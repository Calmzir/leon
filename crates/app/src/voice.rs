//! Dictation, as the interface asks for it.
//!
//! [`Voice`] is everything a conversation's field needs to turn what a
//! person says into text: which models are on this computer, getting one,
//! opening the microphone, and making text of what it heard. The interface
//! holds it as a trait so that tests speak with a scripted one, and so that
//! a recogniser a system already has can stand behind it one day where the
//! downloaded model does.
//!
//! [`Local`] is the one a running Leon has: `leon-voice`'s models in
//! `<data folder>/voice`, the system's microphone, and whisper.cpp. A model
//! is loaded the first time it is used and kept, so only the first
//! dictation waits for it. Nothing is downloaded and the microphone is
//! never opened unless the person asks.

use std::future::Future;
use std::pin::Pin;

use leon_voice::catalog::Model;
pub use leon_voice::Error;

/// What a download is told: the bytes there are so far.
pub type Progress = leon_update::http::Progress;

/// What a download answers, off the thread that draws.
pub type Downloading = Pin<Box<dyn Future<Output = Result<(), Error>> + Send>>;

/// The microphone, open.
pub trait Listening {
    /// How loud what is being said is, from `0.0` to `1.0`.
    fn level(&self) -> f32;
    /// Closes the microphone: what it heard, one channel at the rate a
    /// model listens at.
    fn finish(self: Box<Self>) -> Vec<f32>;
}

/// Dictation on this computer.
pub trait Voice: Send + Sync {
    /// The language the system is set to, when it is one a model is offered
    /// for by name.
    fn language(&self) -> Option<&'static str>;
    /// The ids of the models installed, in the catalogue's order.
    fn installed(&self) -> Vec<String>;
    /// Downloads `model` and holds it to its checksum.
    fn download(&self, model: &'static Model, progress: Progress) -> Downloading;
    /// Opens the microphone.
    fn listen(&self) -> Result<Box<dyn Listening>, Error>;
    /// The words in `samples`, said in `language` when it is named. Takes
    /// seconds: not for the thread that draws.
    fn transcribe(
        &self,
        model: &'static Model,
        samples: &[f32],
        language: Option<&str>,
    ) -> Result<String, Error>;
}

#[cfg(unix)]
mod local {
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex};

    use leon_update::Http;
    use leon_voice::{capture, catalog, store, Engine, Whisper};

    use super::{Downloading, Error, Listening, Model, Progress, Voice};

    /// The models in a folder, the system's microphone and whisper.cpp.
    pub struct Local {
        dir: PathBuf,
        handle: tokio::runtime::Handle,
        http: Arc<dyn Http>,
        /// The model loaded last, by its id.
        loaded: Mutex<Option<(&'static str, Arc<Whisper>)>>,
    }

    impl Local {
        /// Dictation with the models of `<data_dir>/voice`; downloads run
        /// on `handle`.
        pub fn new(data_dir: &std::path::Path, handle: tokio::runtime::Handle) -> Self {
            Self {
                dir: data_dir.join("voice"),
                handle,
                http: Arc::new(leon_update::CurlHttp::models()),
                loaded: Mutex::new(None),
            }
        }

        /// `model`, loaded: the one kept when it is that one.
        fn engine(&self, model: &'static Model) -> Result<Arc<Whisper>, Error> {
            let mut loaded = self
                .loaded
                .lock()
                .map_err(|_| Error::Model("it failed before".to_owned()))?;
            if let Some((id, engine)) = loaded.as_ref() {
                if *id == model.id {
                    return Ok(engine.clone());
                }
            }
            let engine = Arc::new(Whisper::load(&store::path(&self.dir, model))?);
            *loaded = Some((model.id, engine.clone()));
            Ok(engine)
        }
    }

    impl Listening for capture::Recording {
        fn level(&self) -> f32 {
            capture::Recording::level(self)
        }

        fn finish(self: Box<Self>) -> Vec<f32> {
            capture::Recording::finish(*self)
        }
    }

    impl Voice for Local {
        fn language(&self) -> Option<&'static str> {
            catalog::language_of_system(|name| std::env::var(name).ok())
        }

        fn installed(&self) -> Vec<String> {
            store::installed(&self.dir)
        }

        fn download(&self, model: &'static Model, progress: Progress) -> Downloading {
            let (dir, http) = (self.dir.clone(), self.http.clone());
            let task = self.handle.spawn(async move {
                store::install(&*http, &dir, model, progress)
                    .await
                    .map(|_| ())
            });
            Box::pin(async move {
                task.await
                    .unwrap_or_else(|error| Err(Error::Download(error.to_string())))
            })
        }

        fn listen(&self) -> Result<Box<dyn Listening>, Error> {
            Ok(Box::new(capture::start()?))
        }

        fn transcribe(
            &self,
            model: &'static Model,
            samples: &[f32],
            language: Option<&str>,
        ) -> Result<String, Error> {
            self.engine(model)?.transcribe(samples, language)
        }
    }
}

#[cfg(unix)]
pub use local::Local;

/// Dictation where there is none: a system without conversations in Leon's
/// own interface, which is where a person dictates.
pub struct Unavailable;

impl Voice for Unavailable {
    fn language(&self) -> Option<&'static str> {
        None
    }

    fn installed(&self) -> Vec<String> {
        Vec::new()
    }

    fn download(&self, _: &'static Model, _: Progress) -> Downloading {
        Box::pin(async { Err(Error::Download("not on this system".to_owned())) })
    }

    fn listen(&self) -> Result<Box<dyn Listening>, Error> {
        Err(Error::Microphone("not on this system".to_owned()))
    }

    fn transcribe(&self, _: &'static Model, _: &[f32], _: Option<&str>) -> Result<String, Error> {
        Err(Error::Model("not on this system".to_owned()))
    }
}

/// A scripted [`Voice`], for the tests of what the interface does with one.
#[cfg(test)]
pub mod testing {
    use std::sync::{Arc, Mutex};

    use super::{Downloading, Error, Listening, Model, Progress, Voice};

    /// What a test tells the scripted voice, and what it was asked.
    #[derive(Default)]
    pub struct Told {
        /// The language the system is set to.
        pub language: Option<&'static str>,
        /// The ids of the models installed.
        pub installed: Vec<String>,
        /// A download fails with this instead of installing the model.
        pub refuse: Option<Error>,
        /// The microphone fails with this instead of opening.
        pub deaf: Option<Error>,
        /// What the microphone hears: a second of speech unless told.
        pub heard: Option<Vec<f32>>,
        /// What is made of what was heard.
        pub says: String,
        /// The models that were downloaded, by id.
        pub downloaded: Vec<String>,
        /// How many microphones are open.
        pub open: usize,
        /// The transcriptions asked for: model, samples, language.
        pub asked: Vec<(String, usize, Option<String>)>,
    }

    /// A voice that does what [`Told`] says.
    #[derive(Clone, Default)]
    pub struct Scripted(pub Arc<Mutex<Told>>);

    struct Open(Arc<Mutex<Told>>);

    impl Listening for Open {
        fn level(&self) -> f32 {
            0.5
        }

        fn finish(self: Box<Self>) -> Vec<f32> {
            let told = self.0.lock().unwrap();
            told.heard.clone().unwrap_or_else(|| vec![0.1; 16_000])
        }
    }

    impl Drop for Open {
        fn drop(&mut self) {
            self.0.lock().unwrap().open -= 1;
        }
    }

    impl Voice for Scripted {
        fn language(&self) -> Option<&'static str> {
            self.0.lock().unwrap().language
        }

        fn installed(&self) -> Vec<String> {
            self.0.lock().unwrap().installed.clone()
        }

        fn download(&self, model: &'static Model, progress: Progress) -> Downloading {
            let told = self.0.clone();
            Box::pin(async move {
                let mut told = told.lock().unwrap();
                if let Some(error) = told.refuse.clone() {
                    return Err(error);
                }
                progress(model.bytes);
                told.downloaded.push(model.id.to_owned());
                told.installed.push(model.id.to_owned());
                Ok(())
            })
        }

        fn listen(&self) -> Result<Box<dyn Listening>, Error> {
            let mut told = self.0.lock().unwrap();
            if let Some(error) = told.deaf.clone() {
                return Err(error);
            }
            told.open += 1;
            Ok(Box::new(Open(self.0.clone())))
        }

        fn transcribe(
            &self,
            model: &'static Model,
            samples: &[f32],
            language: Option<&str>,
        ) -> Result<String, Error> {
            let mut told = self.0.lock().unwrap();
            told.asked.push((
                model.id.to_owned(),
                samples.len(),
                language.map(str::to_owned),
            ));
            Ok(told.says.clone())
        }
    }
}
