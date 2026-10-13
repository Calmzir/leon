//! Dictation: what a person says, as text.
//!
//! Nothing here is on unless the person asks for it. The speech model is a
//! file they choose to download, chosen for the language they speak; until
//! then Leon holds no model and never opens the microphone. Everything is
//! done on this computer: no sound and no text leaves it.
//!
//! * [`catalog`]: the models there are, which serve a language, and where
//!   each is downloaded from with the checksum it is held to. Pure.
//! * [`store`]: the models on this computer: which are installed, getting
//!   one (resumable, verified) and removing it.
//! * [`audio`]: the arithmetic of sound: many channels into one, any rate
//!   into the 16 kHz a model listens at, how loud it is. Pure.
//! * [`capture`]: the microphone of this computer, recorded until told to
//!   stop.
//! * [`engine`]: what turns sound into text ([`engine::Engine`]). One is
//!   built here, whisper.cpp with a downloaded model; a system that has a
//!   recogniser of its own (macOS, Windows) can be a second one behind the
//!   same trait, which is why the trait is all the application sees.
//!
//! The microphone and the model are behind the feature `engine`: they need
//! a C++ toolchain to build. The rest builds anywhere.

pub mod audio;
#[cfg(feature = "engine")]
pub mod capture;
pub mod catalog;
pub mod engine;
pub mod store;

pub use catalog::Model;
pub use engine::Engine;
#[cfg(feature = "engine")]
pub use engine::Whisper;

/// Why dictation could not do what was asked.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// This computer has no microphone to record from, or refuses it.
    #[error("no microphone to record from: {0}")]
    Microphone(String),
    /// The model could not be downloaded.
    #[error("the model could not be downloaded: {0}")]
    Download(String),
    /// What was downloaded is not the model that was meant.
    #[error("the downloaded file is not the model that was meant")]
    Checksum,
    /// The model's file could not be read, written or removed.
    #[error("the model's file: {0}")]
    Io(String),
    /// The model could not be loaded, or could not make text of the sound.
    #[error("the model could not be used: {0}")]
    Model(String),
}
