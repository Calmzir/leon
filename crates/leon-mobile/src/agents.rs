//! The agents a phone can talk to, and how their command is started.
//!
//! Which agents there are and how each is spoken to is Leon's unified agent
//! protocol (`leon-turn`): a phone drives exactly the agents it lists. What
//! this file adds is where the command runs. It is started through the
//! owner's login shell, as the desktop application starts agents, so that
//! `PATH` is the one of a terminal on that computer. The prompt never
//! becomes shell text: it is bytes written to the agent's input.

use leon_wire::ExecSpec;

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

/// The agents this library can drive.
#[uniffi::export]
pub fn agents() -> Vec<AgentInfo> {
    leon_turn::ADAPTERS
        .iter()
        .map(|adapter| AgentInfo {
            id: adapter.id.to_owned(),
            name: adapter.name.to_owned(),
            program: adapter.program.to_owned(),
        })
        .collect()
}

/// What the host is asked to start for `argv` (program first) in `cwd`.
///
/// `login_shell` off runs the program directly, with the host service's own
/// environment: for tests, which must not depend on anybody's shell.
pub(crate) fn login_spec(argv: Vec<String>, cwd: Option<String>, login_shell: bool) -> ExecSpec {
    let mut argv = argv.into_iter();
    if login_shell {
        let mut wrapped = leon_turn::shell::login_argv(argv.collect()).into_iter();
        return ExecSpec {
            program: wrapped.next().unwrap_or_default(),
            args: wrapped.collect(),
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
    use leon_turn::agents::strings;
    use leon_turn::shell::LOGIN_SHELL;

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
}
