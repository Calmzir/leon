//! The runtime every connection and every callback of this crate runs on.
//!
//! A phone application has no Tokio runtime of its own, and the client of
//! `leon-link` spawns tasks, so the crate keeps one for the life of the
//! process. Exported `async` functions do their work on it and only await the
//! result on the caller's executor, which therefore needs to be nothing more
//! than a future poller.

use std::future::Future;
use std::sync::OnceLock;

use tokio::runtime::{Builder, Runtime};

use crate::error::MobileError;

/// The process-wide runtime, started on first use.
pub(crate) fn runtime() -> &'static Runtime {
    static RUNTIME: OnceLock<Runtime> = OnceLock::new();
    RUNTIME.get_or_init(|| {
        Builder::new_multi_thread()
            .worker_threads(2)
            .thread_name("leon-mobile")
            .enable_all()
            .build()
            .expect("the Tokio runtime starts")
    })
}

/// Runs `work` on the runtime and waits for it from any executor.
pub(crate) async fn on_runtime<T, F>(work: F) -> Result<T, MobileError>
where
    T: Send + 'static,
    F: Future<Output = Result<T, MobileError>> + Send + 'static,
{
    match runtime().spawn(work).await {
        Ok(result) => result,
        Err(error) => Err(MobileError::Internal {
            message: error.to_string(),
        }),
    }
}
