//! Where an agent's command runs: inside the owner's login shell.
//!
//! An agent is installed for a person, and found by the `PATH` their
//! terminal has. A program started by an application has another `PATH`, so
//! every client starts the agent the same way: through the login shell.

/// Runs its arguments as a command inside the owner's login shell.
///
/// `sh -c LOGIN_SHELL leon <program> <args…>`: the shell named by `SHELL`
/// starts as an interactive login shell (the files that set `PATH` for a
/// terminal are read), then replaces itself with the command. The arguments
/// are passed on as arguments at each step and never spliced into text. fish
/// has no `"$@"`; its `$argv` is the same list. A shell this does not know
/// would not understand either form, so `/bin/sh` stands in for it.
pub const LOGIN_SHELL: &str = r#"shell="${SHELL:-/bin/sh}"
case "${shell##*/}" in
    fish) exec "$shell" -l -i -c 'exec $argv' "$@" ;;
    sh|bash|zsh|dash|ksh|mksh|ash) exec "$shell" -l -i -c 'exec "$0" "$@"' "$@" ;;
    *) exec /bin/sh -l -c 'exec "$0" "$@"' "$@" ;;
esac"#;

/// `argv` (program first) as the command that runs it in the login shell.
pub fn login_argv(argv: Vec<String>) -> Vec<String> {
    let mut wrapped = crate::agents::strings(&["sh", "-c", LOGIN_SHELL, "leon"]);
    wrapped.extend(argv);
    wrapped
}
