//! The agents a phone can talk to, and how each one is driven.
//!
//! An agent is a row of `ADAPTERS`. Each runs behind the bridge
//! (`crate::bridge`) and is spoken to both ways for as long as the turn
//! lasts. Its `AgentSession` writes what the agent must read (its handshake,
//! the prompt, the answers to what it asks) and reads what it prints. This
//! is how a person on a phone gets the agent's own questions and modes.
//!
//! The command is started through the owner's login shell, as the desktop
//! application starts agents, so that `PATH` is the one of a terminal on
//! that computer. The prompt never becomes shell text: it is bytes written
//! to the agent's input.

use leon_wire::ExecSpec;
use serde_json::Value;

use crate::claude::AgentOptions;
use crate::error::MobileError;
use crate::events::{Answer, TurnEvent};

/// An agent this library can drive.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct AgentInfo {
    /// The catalogue tag, the same the desktop application uses (`claude`).
    pub id: String,
    /// The name to show.
    pub name: String,
    /// The program that must be installed on the other computer.
    pub program: String,
}

/// One turn, as an adapter needs it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct TurnSpec<'a> {
    /// The program to run (the adapter's, unless a test replaced it).
    pub program: &'a str,
    /// What the person wrote.
    pub prompt: &'a str,
    /// The conversation to continue, by the id the agent gave it.
    pub resume: Option<&'a str>,
    /// The agent's permission mode, by the id [`crate::AgentOptions`] lists.
    pub mode: Option<&'a str>,
    /// The model, by the id [`crate::AgentOptions`] lists.
    pub model: Option<&'a str>,
    /// How hard it should think, by the id [`crate::AgentOptions`] lists.
    pub effort: Option<&'a str>,
    /// The folder it works in, as a whole path, when it is known.
    pub cwd: Option<&'a str>,
    /// More folders it may work in.
    pub add_dirs: &'a [String],
}

/// One line for the agent's input, and the tag its delivery is known by.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Outgoing {
    /// Names this message in the bridge's acknowledgement. Deterministic
    /// for anything sent in reaction to what the agent printed, so that a
    /// replayed turn recognises what it already sent.
    pub tag: String,
    /// One JSON value, without a line end.
    pub line: String,
}

/// What a person can do to a turn that is running.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Action {
    /// Say more.
    Prompt(String),
    /// Answer the question with this id.
    Answer(String, Answer),
    /// Stop working, keep the conversation.
    Interrupt,
    /// Work in another permission mode from now on.
    SetMode(String),
    /// Answer with another model from now on.
    SetModel(String),
}

/// What reading one line led to.
#[derive(Debug, Default)]
pub(crate) struct Step {
    /// What to tell the chat.
    pub events: Vec<TurnEvent>,
    /// What to write to the agent without anybody asking: the next message
    /// of a handshake, the refusal of a request this client does not serve.
    pub send: Vec<Outgoing>,
}

/// One agent process spoken to both ways, for one turn.
///
/// The engine ([`crate::turn`]) owns the terminal and the bridge; a session
/// only knows the agent's protocol. It must be able to start in the middle:
/// a turn found again after the application was closed is read from the
/// output the host kept, by a session made with [`Adapter::session`] that
/// was never told the prompt and sees only what the agent printed and which
/// of the phone's messages the bridge acknowledged ([`Self::delivered`]).
///
/// For a protocol with numbered requests (JSON-RPC, as Codex's app server
/// and opencode's ACP speak): [`Self::first`] returns the opening request;
/// the next steps of the handshake are returned from [`Self::line`] as
/// [`Step::send`] when the answer they wait for is read, each with a tag
/// derived from its request id (`h.2`), never from a counter of this
/// process; a request *from* the agent that needs an answer becomes a
/// [`TurnEvent::Question`] whose id is the request's, answered through
/// [`Self::encode`] with the tag `a.<id>` ([`crate::bridge::id_tag`]); a
/// request this client cannot serve is refused at once through
/// [`Step::send`], so the agent never waits for nobody.
pub(crate) trait AgentSession: Send {
    /// What the agent must read first, in order: its handshake, then the
    /// prompt if the protocol lets it follow at once. Delivered by the
    /// bridge itself when it fits its arguments.
    fn first(&mut self, turn: &TurnSpec<'_>) -> Vec<Outgoing>;

    /// One JSON value the agent printed.
    fn line(&mut self, value: &Value, step: &mut Step);

    /// A line that was not JSON: kept to explain a failure.
    fn noise(&mut self, line: &str);

    /// The bridge acknowledged the message with this tag: the agent has it.
    fn delivered(&mut self, tag: &str, step: &mut Step);

    /// What to write so that `action` happens.
    fn encode(&mut self, action: Action) -> Result<Vec<Outgoing>, MobileError>;

    /// The agent has nothing more to do and waits for input that will not
    /// come: its input is to be closed, which ends it.
    fn idle(&self) -> bool;

    /// The process ended with `code`.
    fn exited(&mut self, code: u32) -> Vec<TurnEvent>;
}

/// How an agent is asked what it offers, in one command on the other
/// computer: `sh -c <script> leon <program> <arguments…>`.
pub(crate) struct Probe {
    /// The script; `$1` is the agent's program.
    pub script: &'static str,
    /// What follows the program.
    pub arguments: fn() -> Vec<String>,
    /// Reads what the script printed.
    pub parse: fn(&str) -> AgentOptions,
}

/// How one agent is driven.
pub(crate) struct Adapter {
    /// The catalogue tag.
    pub id: &'static str,
    /// The name to show.
    pub name: &'static str,
    /// The program.
    pub program: &'static str,
    /// The argument vector of the agent for a turn, program first.
    pub argv: fn(&TurnSpec<'_>) -> Vec<String>,
    /// A session for one turn; `unique` seeds the ids it makes up, so that
    /// two processes following the same turn never reuse one.
    pub session: fn(unique: u64) -> Box<dyn AgentSession>,
    /// How the agent on the other computer is asked what it offers.
    pub options: Probe,
    /// Its protocol must be told the folder to work in as a whole path:
    /// [`TurnSpec::cwd`] is found out when the turn names none.
    pub needs_folder: bool,
}

/// Every agent a phone can talk to. Nothing else is offered: a session of
/// another agent in the history is shown, and cannot be continued.
pub(crate) const ADAPTERS: &[Adapter] = &[
    Adapter {
        id: "claude",
        name: "Claude Code",
        program: "claude",
        argv: crate::claude::argv,
        session: crate::claude::session,
        options: Probe {
            script: crate::claude::OPTIONS_SCRIPT,
            arguments: crate::claude::options_arguments,
            parse: crate::claude::parse_options,
        },
        needs_folder: false,
    },
    Adapter {
        id: "codex",
        name: "Codex",
        program: "codex",
        argv: crate::codex::argv,
        session: crate::codex::session,
        options: Probe {
            script: crate::codex::OPTIONS_SCRIPT,
            arguments: crate::codex::options_arguments,
            parse: crate::codex::parse_options,
        },
        needs_folder: false,
    },
    Adapter {
        id: "opencode",
        name: "opencode",
        program: "opencode",
        argv: crate::opencode::argv,
        session: crate::opencode::session,
        options: Probe {
            script: crate::opencode::OPTIONS_SCRIPT,
            arguments: crate::opencode::options_arguments,
            parse: crate::opencode::parse_options,
        },
        needs_folder: true,
    },
];

/// The adapter of the agent with this tag.
pub(crate) fn adapter(id: &str) -> Option<&'static Adapter> {
    ADAPTERS.iter().find(|adapter| adapter.id == id)
}

/// The agents this library can drive.
#[uniffi::export]
pub fn agents() -> Vec<AgentInfo> {
    ADAPTERS
        .iter()
        .map(|adapter| AgentInfo {
            id: adapter.id.to_owned(),
            name: adapter.name.to_owned(),
            program: adapter.program.to_owned(),
        })
        .collect()
}

pub(crate) fn strings(items: &[&str]) -> Vec<String> {
    items.iter().map(|item| (*item).to_owned()).collect()
}

/// Runs its arguments as a command inside the owner's login shell.
///
/// `sh -c LOGIN_SHELL leon <program> <args…>`: the shell named by `SHELL`
/// starts as an interactive login shell (the files that set `PATH` for a
/// terminal are read), then replaces itself with the command. The arguments
/// are passed on as arguments at each step and never spliced into text. fish
/// has no `"$@"`; its `$argv` is the same list. A shell this does not know
/// would not understand either form, so `/bin/sh` stands in for it.
const LOGIN_SHELL: &str = r#"shell="${SHELL:-/bin/sh}"
case "${shell##*/}" in
    fish) exec "$shell" -l -i -c 'exec $argv' "$@" ;;
    sh|bash|zsh|dash|ksh|mksh|ash) exec "$shell" -l -i -c 'exec "$0" "$@"' "$@" ;;
    *) exec /bin/sh -l -c 'exec "$0" "$@"' "$@" ;;
esac"#;

/// What the host is asked to start for `argv` (program first) in `cwd`.
///
/// `login_shell` off runs the program directly, with the host service's own
/// environment: for tests, which must not depend on anybody's shell.
pub(crate) fn login_spec(argv: Vec<String>, cwd: Option<String>, login_shell: bool) -> ExecSpec {
    let mut argv = argv.into_iter();
    if login_shell {
        let mut args = strings(&["-c", LOGIN_SHELL, "leon"]);
        args.extend(argv);
        return ExecSpec {
            program: "sh".to_owned(),
            args,
            env: Vec::new(),
            cwd,
            stdin: None,
        };
    }
    ExecSpec {
        program: argv.next().unwrap_or_default(),
        args: argv.collect(),
        env: Vec::new(),
        cwd,
        stdin: None,
    }
}

/// The label of a turn's terminal on the host: `leon-mobile/<agent>/<chat>`.
/// It is how a turn is found again after the application was closed.
pub(crate) fn turn_label(agent: &str, chat_id: &str) -> String {
    format!("{LABEL_PREFIX}{agent}/{chat_id}")
}

/// Reads a label written by [`turn_label`]: the agent and the chat.
pub(crate) fn parse_label(label: &str) -> Option<(&str, &str)> {
    label.strip_prefix(LABEL_PREFIX)?.split_once('/')
}

const LABEL_PREFIX: &str = "leon-mobile/";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_login_shell_receives_the_command_as_arguments_not_as_text() {
        let spec = login_spec(
            strings(&["claude", "-p", "--", "it's $HOME"]),
            Some("/work".into()),
            true,
        );
        assert_eq!(spec.program, "sh");
        assert_eq!(spec.args[0], "-c");
        assert_eq!(spec.args[1], LOGIN_SHELL);
        assert_eq!(
            &spec.args[2..],
            ["leon", "claude", "-p", "--", "it's $HOME"]
        );
        assert!(!LOGIN_SHELL.contains("it's"));
        assert_eq!(spec.cwd.as_deref(), Some("/work"));
    }

    #[test]
    fn without_the_login_shell_the_program_runs_as_it_is() {
        let spec = login_spec(strings(&["/bin/echo", "a", "b"]), None, false);
        assert_eq!(spec.program, "/bin/echo");
        assert_eq!(spec.args, ["a", "b"]);
    }

    /// The wrapper itself, run by a real `sh` with a known shell: every
    /// argument arrives whole.
    #[cfg(unix)]
    #[test]
    fn the_wrapper_passes_every_argument_through_whole() {
        let output = std::process::Command::new("/bin/sh")
            .args([
                "-c",
                LOGIN_SHELL,
                "leon",
                "printf",
                "[%s]",
                "a b",
                "$HOME",
                "it's",
                "--x",
            ])
            .env("SHELL", "/bin/sh")
            .env("ENV", "/dev/null")
            .stdin(std::process::Stdio::null())
            .output()
            .unwrap();
        let text = String::from_utf8_lossy(&output.stdout);
        assert!(text.ends_with("[a b][$HOME][it's][--x]"), "{text:?}");
    }

    #[test]
    fn labels_round_trip_and_foreign_ones_are_not_turns() {
        let label = turn_label("claude", "chat/with/slashes");
        assert_eq!(parse_label(&label), Some(("claude", "chat/with/slashes")));
        assert_eq!(parse_label("zsh @ /home"), None);
    }

    #[test]
    fn the_catalogue_names_the_three_agents_once_each() {
        let listed: Vec<String> = agents().into_iter().map(|agent| agent.id).collect();
        assert_eq!(listed, ["claude", "codex", "opencode"]);
        assert!(adapter("claude").is_some());
        assert!(adapter("vim").is_none());
        // Every one of them starts its server and is told the rest there.
        let turn = TurnSpec {
            program: "agent",
            prompt: "--help; rm -rf ~",
            ..TurnSpec::default()
        };
        for adapter in ADAPTERS {
            assert!(!(adapter.argv)(&turn).contains(&turn.prompt.to_owned()));
        }
    }
}
