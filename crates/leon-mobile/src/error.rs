//! The one error type that crosses to the application, in words a person
//! can act on.

use leon_link::client::ClientError;
use leon_link::relay_client::DialError;
use leon_link::{IdentityError, PairError};
use leon_wire::relay::RelayErrorCode;
use thiserror::Error;

/// Why something asked of this library failed.
#[derive(Debug, Clone, PartialEq, Eq, Error, uniffi::Error)]
pub enum MobileError {
    /// The device's key could not be read or written.
    #[error("this device's key could not be stored: {message}")]
    Storage {
        /// What the file system said.
        message: String,
    },
    /// The pairing code is not a complete, well-formed code.
    #[error("that is not a complete pairing code")]
    BadCode,
    /// A stored host record is damaged (its id or its key).
    #[error("the saved computer is damaged; pair with it again")]
    BadHost,
    /// The relay cannot be reached (network, DNS, TLS, the service is down).
    #[error("the relay cannot be reached: {message}")]
    RelayUnreachable {
        /// What failed.
        message: String,
    },
    /// The relay answered and refused.
    #[error("the relay refused: {message}")]
    RelayRefused {
        /// The relay's sentence.
        message: String,
    },
    /// No computer is showing that code on this relay.
    #[error("no computer is showing that code")]
    CodeNotFound,
    /// The code is older than ten minutes.
    #[error("the code has expired; ask for a new one")]
    CodeExpired,
    /// Too many wrong attempts burned the code.
    #[error("too many wrong attempts; ask for a new code")]
    CodeBurned,
    /// The code was already used once.
    #[error("the code was already used; ask for a new one")]
    CodeUsed,
    /// The other computer did not accept the code.
    #[error("the code was not accepted; check it and try again")]
    WrongCode,
    /// The owner of the other computer said no.
    #[error("the other computer declined")]
    Denied,
    /// Nothing arrived in time.
    #[error("the other computer did not answer in time")]
    Timeout,
    /// Not connected, and not connecting soon enough.
    #[error("not connected: {message}")]
    Offline {
        /// Why.
        message: String,
    },
    /// The connection dropped while waiting for the answer.
    #[error("the connection dropped")]
    Disconnected,
    /// The session was closed by the application.
    #[error("the session is closed")]
    Closed,
    /// The other computer answered with an error.
    #[error("{message}")]
    Host {
        /// Its sentence.
        message: String,
    },
    /// The agent named is not one this library can drive.
    #[error("that agent cannot be used from a phone yet")]
    UnknownAgent,
    /// The turn named is not running on the other computer.
    #[error("that turn is no longer on the other computer")]
    TurnNotFound,
    /// The agent is not waiting for an answer to that question (any more).
    #[error("that question is no longer open")]
    UnknownQuestion,
    /// The answer does not fit the question it is given to.
    #[error("that answer does not fit the question")]
    BadAnswer,
    /// This agent is not driven in a way that allows it.
    #[error("this agent cannot do that from here")]
    NotSupported,
    /// The turn has ended: there is nothing left to tell the agent.
    #[error("the turn has already ended")]
    TurnEnded,
    /// The other side did not follow the protocol.
    #[error("the other side did not follow the protocol")]
    Protocol,
    /// A failure inside this library.
    #[error("internal error: {message}")]
    Internal {
        /// What happened.
        message: String,
    },
}

impl From<leon_turn::TurnError> for MobileError {
    fn from(error: leon_turn::TurnError) -> Self {
        match error {
            leon_turn::TurnError::UnknownQuestion => Self::UnknownQuestion,
            leon_turn::TurnError::BadAnswer => Self::BadAnswer,
            leon_turn::TurnError::NotSupported => Self::NotSupported,
            leon_turn::TurnError::TurnEnded => Self::TurnEnded,
        }
    }
}

impl From<IdentityError> for MobileError {
    fn from(error: IdentityError) -> Self {
        MobileError::Storage {
            message: error.to_string(),
        }
    }
}

impl From<ClientError> for MobileError {
    fn from(error: ClientError) -> Self {
        match error {
            ClientError::Offline(message) => MobileError::Offline { message },
            ClientError::Disconnected => MobileError::Disconnected,
            ClientError::Remote(wire) => MobileError::Host {
                message: wire.message,
            },
            ClientError::Protocol => MobileError::Protocol,
            ClientError::Closed => MobileError::Closed,
        }
    }
}

impl From<DialError> for MobileError {
    fn from(error: DialError) -> Self {
        match error {
            DialError::Unreachable(message) | DialError::Secure(message) => {
                MobileError::RelayUnreachable { message }
            }
            DialError::Refused(refusal) if refusal.code == RelayErrorCode::HostNotFound => {
                MobileError::CodeNotFound
            }
            DialError::Refused(refusal) => MobileError::RelayRefused {
                message: refusal.message,
            },
            DialError::Protocol => MobileError::Protocol,
        }
    }
}

impl From<PairError> for MobileError {
    fn from(error: PairError) -> Self {
        match error {
            PairError::Expired => MobileError::CodeExpired,
            PairError::Burned => MobileError::CodeBurned,
            PairError::AlreadyUsed => MobileError::CodeUsed,
            PairError::NotAccepted => MobileError::WrongCode,
            PairError::Denied => MobileError::Denied,
            PairError::Timeout => MobileError::Timeout,
            PairError::Closed => MobileError::Disconnected,
            PairError::Protocol => MobileError::Protocol,
            PairError::Link(message) => MobileError::Internal { message },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use leon_wire::relay::RelayError;

    #[test]
    fn a_room_nobody_opened_reads_as_a_code_nobody_is_showing() {
        let refusal = DialError::Refused(RelayError {
            code: RelayErrorCode::HostNotFound,
            message: "no such room".into(),
        });
        assert_eq!(MobileError::from(refusal), MobileError::CodeNotFound);
    }

    #[test]
    fn a_wrong_code_and_a_refusal_are_told_apart() {
        assert_eq!(
            MobileError::from(PairError::NotAccepted),
            MobileError::WrongCode
        );
        assert_eq!(MobileError::from(PairError::Denied), MobileError::Denied);
    }
}
