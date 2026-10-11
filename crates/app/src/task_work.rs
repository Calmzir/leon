//! Starting work on a task: the branch of its worktree and what the agent is
//! told, decided without a window.
//!
//! The branch is the slug of the title and the task's short id
//! (`fix-the-login-redirect-fcb6d0ab`), so the link between the branch and
//! the task survives outside the database and a title that is used twice does
//! not make two branches of one name. The prompt is the task, in the words a
//! person would give it, and says how to tell the board it is done: the
//! agent runs the same `leon task move` an agent always could.

use crate::fanout;
use crate::launch::MAX_PROMPT;

/// The room the closing sentence of the prompt needs, in characters, kept out
/// of what the body may take.
const SENTENCE_ROOM: usize = 400;

/// The branch for a task: its title's slug and its short id, numbered when
/// `taken` (the branches the project and its worktrees have) holds it.
pub fn branch_name(title: &str, short_id: &str, taken: &[String]) -> String {
    let base = format!("{}-{short_id}", fanout::prompt_slug(title));
    let mut branch = base.clone();
    let mut number = 2;
    while taken.contains(&branch) {
        branch = format!("{base}-{number}");
        number += 1;
    }
    branch
}

/// What the agent is asked: the task as a sentence, its body, and how to
/// move the task to review when it is done. `cli` is how the program is
/// reached from the agent's terminal (`"$LEON_BIN"`).
pub fn prompt(title: &str, body: &str, short_id: &str, cli: &str) -> String {
    let title = title.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut text = format!("Work on this task: {title}");
    let body = body.trim();
    if !body.is_empty() {
        let room = MAX_PROMPT.saturating_sub(text.chars().count() + SENTENCE_ROOM);
        let body: String = body.chars().take(room).collect();
        text.push_str("\n\n");
        text.push_str(body.trim_end());
    }
    text.push_str(&format!(
        "\n\nIt is task {short_id} of this project's task board. When you are done, \
         move it to review with: {cli} task move {short_id} review"
    ));
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::launch::clean_prompt;

    #[test]
    fn the_branch_is_the_slug_of_the_title_and_the_short_id() {
        assert_eq!(
            branch_name("Fix the login redirect loop", "fcb6d0ab", &[]),
            "fix-the-login-redirect-loop-fcb6d0ab"
        );
        // A title with nothing in it to slug still gives a valid branch.
        assert_eq!(branch_name("¿¿??", "fcb6d0ab", &[]), "prompt-fcb6d0ab");
        // A long title is cut at a word.
        let long = branch_name(&"migrate ".repeat(12), "fcb6d0ab", &[]);
        assert!(long.len() <= 28 + 9, "{long}");
        assert!(long.ends_with("-fcb6d0ab"));
    }

    #[test]
    fn a_branch_that_exists_is_numbered() {
        let taken = vec!["fix-it-1a2b3c4d".to_owned(), "fix-it-1a2b3c4d-2".to_owned()];
        assert_eq!(
            branch_name("Fix it", "1a2b3c4d", &taken),
            "fix-it-1a2b3c4d-3"
        );
        assert_eq!(branch_name("Fix it", "99999999", &taken), "fix-it-99999999");
    }

    #[test]
    fn the_prompt_holds_the_task_and_how_to_close_it() {
        let text = prompt(
            "Fix   the\nlogin",
            "Users land back on the login screen.\nUse a cleared cookie jar.",
            "fcb6d0ab",
            "\"$LEON_BIN\"",
        );
        assert!(text.starts_with("Work on this task: Fix the login\n\nUsers land back"));
        assert!(text.contains("Use a cleared cookie jar."));
        assert!(text.ends_with("\"$LEON_BIN\" task move fcb6d0ab review"));
        // The terminal takes it as a prompt: no dash first, no control keys.
        assert_eq!(clean_prompt(&text).as_deref(), Ok(text.as_str()));
        let without = prompt("Only a title", "", "fcb6d0ab", "x");
        assert!(!without.contains("\n\n\n"));
    }

    #[test]
    fn a_body_too_long_for_a_prompt_is_cut_and_the_prompt_still_goes() {
        let text = prompt("Big", &"word ".repeat(5_000), "fcb6d0ab", "\"$LEON_BIN\"");
        assert!(text.chars().count() <= MAX_PROMPT);
        assert!(clean_prompt(&text).is_ok());
        assert!(text.ends_with("task move fcb6d0ab review"));
    }
}
