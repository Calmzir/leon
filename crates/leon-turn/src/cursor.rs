//! Cursor's agent, spoken to over the Agent Client Protocol.
//!
//! `cursor-agent` serves the protocol on its standard streams (its opening
//! answer checked against 2026.10.01):
//!
//! ```text
//! cursor-agent acp
//! ```
//!
//! The session is [`crate::acp`]'s; here is what is Cursor's own. It refuses
//! a session to a client that did not say `authenticate` with the method
//! `cursor_login` first, which uses the sign-in `cursor-agent login` made.
//! It offers no `session/resume`, so a conversation is continued with
//! `session/load`, and what it prints again is not shown.
//!
//! **Not checked against a real turn**: what was at hand was not signed in.
//! Everything past the opening answer is the protocol as its specification
//! has it, and no model is offered, since how it lists them was not seen.

use crate::acp::{self, Dialect};
use crate::agents::{strings, AgentSession, TurnSpec};
use crate::claude::AgentOptions;
use crate::grok::{two_modes, MODE_ASK};

const DIALECT: Dialect = Dialect {
    name: "Cursor",
    resume: "session/load",
    login: Some("cursor_login"),
    effort: "effort",
    auto_agent: None,
    models_come_late: false,
    refusal_ends_turn: false,
};

/// The command line of a turn. What the turn is about travels in requests.
pub fn argv(turn: &TurnSpec<'_>) -> Vec<String> {
    strings(&[turn.program, "acp"])
}

/// A session for one turn.
pub fn session(unique: u64) -> Box<dyn AgentSession> {
    acp::session(unique, &DIALECT)
}

/// The command that asks Cursor's agent what it offers: its version.
pub const OPTIONS_SCRIPT: &str = r#"program=$1
printf 'leon-version:%s\n' "$("$program" --version 2>/dev/null | head -n 1)"
exit 0"#;

/// What [`OPTIONS_SCRIPT`] is given after the program: nothing.
pub fn options_arguments() -> Vec<String> {
    Vec::new()
}

/// Reads the output of [`OPTIONS_SCRIPT`].
pub fn parse_options(text: &str) -> AgentOptions {
    let version = text
        .lines()
        .find_map(|line| line.trim_end_matches('\r').strip_prefix("leon-version:"))
        .map(|version| version.trim().to_owned())
        .filter(|version| !version.is_empty());
    AgentOptions {
        questions: true,
        follow_ups: false,
        live_changes: true,
        // Its models are not listed: the conversation runs with its own.
        assumed: true,
        version,
        modes: two_modes("Cursor"),
        default_mode: Some(MODE_ASK.to_owned()),
        ..AgentOptions::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::Step;
    use serde_json::{json, Value};

    fn value(line: &str) -> Value {
        serde_json::from_str(line).unwrap()
    }

    #[test]
    fn the_session_is_asked_for_only_once_cursor_was_signed_in_to() {
        let mut session = session(1);
        let first = session.first(&TurnSpec {
            program: "cursor-agent",
            prompt: "hello",
            cwd: Some("/work"),
            ..TurnSpec::default()
        });
        let methods: Vec<Value> = first
            .iter()
            .map(|sent| value(&sent.line)["method"].clone())
            .collect();
        assert_eq!(methods, [json!("initialize"), json!("authenticate")]);
        assert_eq!(value(&first[1].line)["params"]["methodId"], "cursor_login");

        let mut step = Step::default();
        session.line(
            &json!({"jsonrpc":"2.0","id":"leon-login","result":{}}),
            &mut step,
        );
        assert_eq!(step.send.len(), 1);
        let asked = value(&step.send[0].line);
        assert_eq!(asked["method"], "session/new");
        assert_eq!(asked["params"]["cwd"], "/work");
    }

    #[test]
    fn a_sign_in_that_fails_ends_the_turn_with_what_cursor_said() {
        let mut session = session(1);
        session.first(&TurnSpec {
            program: "cursor-agent",
            prompt: "hello",
            ..TurnSpec::default()
        });
        let mut step = Step::default();
        session.line(
            &json!({"jsonrpc":"2.0","id":"leon-login","error":{"code":-32000,"message":"Authentication required"}}),
            &mut step,
        );
        assert!(step.send.is_empty());
        assert!(matches!(
            step.events.as_slice(),
            [crate::TurnEvent::Finished { ok: false, error: Some(error), .. }]
                if error == "Authentication required"
        ));
    }

    #[test]
    fn a_conversation_loaded_again_is_not_said_again() {
        let mut session = session(1);
        let first = session.first(&TurnSpec {
            program: "cursor-agent",
            prompt: "and now?",
            resume: Some("chat-1"),
            cwd: Some("/work"),
            ..TurnSpec::default()
        });
        assert_eq!(first.len(), 2);
        let mut step = Step::default();
        session.line(
            &json!({"jsonrpc":"2.0","id":"leon-login","result":{}}),
            &mut step,
        );
        assert_eq!(value(&step.send[0].line)["method"], "session/load");
        assert_eq!(value(&step.send[0].line)["params"]["sessionId"], "chat-1");
        // What was said before, printed again.
        let mut replay = Step::default();
        session.line(
            &json!({"jsonrpc":"2.0","method":"session/update","params":{"sessionId":"chat-1",
                "update":{"sessionUpdate":"agent_message_chunk","content":{"type":"text","text":"old words"}}}}),
            &mut replay,
        );
        assert!(
            !replay
                .events
                .iter()
                .any(|event| matches!(event, crate::TurnEvent::TextDelta { .. })),
            "{:?}",
            replay.events
        );
        // Loaded: the prompt goes, and what follows is the turn.
        let mut loaded = Step::default();
        session.line(
            &json!({"jsonrpc":"2.0","id":"leon-session","result":{}}),
            &mut loaded,
        );
        assert_eq!(value(&loaded.send[0].line)["method"], "session/prompt");
        let mut live = Step::default();
        session.line(
            &json!({"jsonrpc":"2.0","method":"session/update","params":{"sessionId":"chat-1",
                "update":{"sessionUpdate":"agent_message_chunk","content":{"type":"text","text":"new words"}}}}),
            &mut live,
        );
        assert!(live.events.iter().any(
            |event| matches!(event, crate::TurnEvent::TextDelta { text } if text == "new words")
        ));
    }

    #[test]
    fn its_options_are_the_two_modes_and_its_version() {
        let options = parse_options("leon-version:2026.10.01-e373342\n");
        assert_eq!(options.version.as_deref(), Some("2026.10.01-e373342"));
        assert!(options.models.is_empty());
        let modes: Vec<&str> = options.modes.iter().map(|mode| mode.id.as_str()).collect();
        assert_eq!(modes, [MODE_ASK, acp::MODE_AUTO]);
    }
}
