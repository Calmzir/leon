//! Why something asked of a turn cannot be written to its agent.

use thiserror::Error;

/// Why an action was not encoded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum TurnError {
    /// The question was already answered, withdrawn, or never asked.
    #[error("that question is no longer open")]
    UnknownQuestion,
    /// The answer is not one the question takes.
    #[error("that is not an answer to this question")]
    BadAnswer,
    /// This agent cannot do that (see [`crate::AgentOptions`]).
    #[error("this agent cannot do that")]
    NotSupported,
    /// The turn is over.
    #[error("the turn has ended")]
    TurnEnded,
}
