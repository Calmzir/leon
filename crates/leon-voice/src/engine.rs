//! What turns sound into text.
//!
//! The application sees [`Engine`] and nothing else, so that a recogniser a
//! system already has can stand where the downloaded model does. The one
//! built here is [`Whisper`]: whisper.cpp with a model of the catalogue, on
//! the processor of this computer.

#[cfg(feature = "engine")]
use std::path::Path;

#[cfg(feature = "engine")]
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

use crate::Error;

/// Makes text of what was said.
pub trait Engine: Send + Sync {
    /// The words in `samples` (one channel at [`crate::audio::RATE`]), said
    /// in `language` when it is named and in whichever the engine hears
    /// otherwise. Takes as long as the sound is hard: not for the thread
    /// that draws.
    fn transcribe(&self, samples: &[f32], language: Option<&str>) -> Result<String, Error>;
}

/// whisper.cpp with one model loaded.
#[cfg(feature = "engine")]
pub struct Whisper {
    context: WhisperContext,
}

#[cfg(feature = "engine")]
impl Whisper {
    /// Loads the model in the file at `path`: a second or so, and as much
    /// memory as the file is large.
    pub fn load(path: &Path) -> Result<Self, Error> {
        // What whisper.cpp would print goes nowhere rather than to the
        // terminal Leon was started from.
        whisper_rs::install_logging_hooks();
        let path = path
            .to_str()
            .ok_or_else(|| Error::Model("its path is not text".to_owned()))?;
        let context = WhisperContext::new_with_params(path, WhisperContextParameters::default())
            .map_err(|error| Error::Model(error.to_string()))?;
        Ok(Self { context })
    }
}

#[cfg(feature = "engine")]
impl Engine for Whisper {
    fn transcribe(&self, samples: &[f32], language: Option<&str>) -> Result<String, Error> {
        let failed = |error: whisper_rs::WhisperError| Error::Model(error.to_string());
        let mut state = self.context.create_state().map_err(failed)?;
        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_language(Some(language.unwrap_or("auto")));
        params.set_translate(false);
        // Each dictation stands alone: nothing said before leaks into it.
        params.set_no_context(true);
        params.set_suppress_blank(true);
        params.set_suppress_nst(true);
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);
        params.set_n_threads(threads());
        state.full(params, samples).map_err(failed)?;
        let said: Vec<String> = state
            .as_iter()
            .filter_map(|segment| segment.to_str_lossy().ok().map(|text| text.into_owned()))
            .collect();
        Ok(tidy(&said.join(" ")))
    }
}

/// How many threads a transcription takes: the processors there are, short
/// of all of them (the interface is drawn meanwhile) and no more than a
/// model gains from.
#[cfg(feature = "engine")]
fn threads() -> i32 {
    let cores = std::thread::available_parallelism().map_or(2, |cores| cores.get());
    cores.saturating_sub(1).clamp(1, 8) as i32
}

/// What a model wrote, as a person would have typed it: the marks it puts
/// for what is not speech (`[BLANK_AUDIO]`, `(music)`, `*coughs*`) taken
/// out, and the space between words made one.
pub fn tidy(text: &str) -> String {
    let mut kept = String::with_capacity(text.len());
    let mut closing = None;
    for letter in text.chars() {
        match (closing, letter) {
            (Some(close), _) if letter == close => closing = None,
            (Some(_), _) => {}
            (None, '[') => closing = Some(']'),
            (None, '(') => closing = Some(')'),
            (None, '*') => closing = Some('*'),
            (None, _) => kept.push(letter),
        }
    }
    kept.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn what_a_model_marks_as_not_speech_is_taken_out() {
        assert_eq!(tidy(" Fix the  login timeout.\n"), "Fix the login timeout.");
        assert_eq!(tidy("[BLANK_AUDIO]"), "");
        assert_eq!(
            tidy(" (music) Hola, [ruido] qué tal. *tos*"),
            "Hola, qué tal."
        );
        // A mark left open takes what follows: it was not speech either.
        assert_eq!(tidy("open the file [inaud"), "open the file");
        assert_eq!(tidy(""), "");
    }

    #[cfg(feature = "engine")]
    #[test]
    fn a_file_that_is_not_a_model_is_refused_rather_than_used() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ggml-base.bin");
        std::fs::write(&path, b"not a model").unwrap();
        assert!(matches!(Whisper::load(&path), Err(Error::Model(_))));
        assert!(matches!(
            Whisper::load(&dir.path().join("missing.bin")),
            Err(Error::Model(_))
        ));
    }
}
