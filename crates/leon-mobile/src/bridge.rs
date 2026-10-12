//! The bridge of a turn's terminal, as the phone asks the host to start it.
//!
//! The bridge itself (its script, the frames written to it and the marks it
//! answers with) is `leon_turn::bridge`, shared with the desktop
//! application. Here is only what the host is asked to run.

use leon_wire::ExecSpec;

pub(crate) use leon_turn::bridge::{end, frame, safe_tag, Mark, Marks, ARGUMENT_BYTES, END_TAG};

use crate::agents::login_spec;

/// What the host is asked to start: the bridge, given `first` to hand to the
/// agent before anything else, then the agent's `argv` (program first).
pub(crate) fn bridge_spec(
    first: &[String],
    argv: Vec<String>,
    cwd: Option<String>,
    login_shell: bool,
) -> ExecSpec {
    login_spec(leon_turn::bridge::argv(first, argv), cwd, login_shell)
}

#[cfg(test)]
mod tests {
    use super::*;
    use leon_turn::bridge::BRIDGE;

    #[test]
    fn the_bridge_is_started_in_the_folder_of_the_turn() {
        let spec = bridge_spec(
            &["{\"one\":1}".to_owned()],
            vec!["claude".to_owned(), "-p".to_owned()],
            Some("/work".into()),
            false,
        );
        assert_eq!(spec.program, "sh");
        assert_eq!(
            spec.args,
            [
                "-c",
                BRIDGE,
                "leon-bridge",
                "1",
                "{\"one\":1}",
                "claude",
                "-p"
            ]
        );
        assert_eq!(spec.cwd.as_deref(), Some("/work"));
    }
}
