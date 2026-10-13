//! The models on this computer.
//!
//! A model is one file in a folder of Leon's own data. It is there only
//! once it was downloaded whole and its SHA-256 is the one the catalogue
//! pins: until then it has another name, which a later attempt carries on
//! from, so what [`installed`] lists is always a model that can be used.

use std::path::{Path, PathBuf};

use leon_update::checksums;
use leon_update::download::fetch_asset;
use leon_update::http::{HostPolicy, Http, Progress};

use crate::catalog::{self, Model};
use crate::Error;

/// Where `model` is on this computer once it is installed.
pub fn path(dir: &Path, model: &Model) -> PathBuf {
    dir.join(model.file)
}

/// The name a model's file has while it is not verified yet.
fn unverified(model: &Model) -> String {
    format!("{}.download", model.file)
}

/// The ids of the models installed in `dir`, in the catalogue's order. A
/// file of another size than its model's is not one: it was cut off, or it
/// is something else.
pub fn installed(dir: &Path) -> Vec<String> {
    catalog::MODELS
        .iter()
        .filter(|model| {
            std::fs::metadata(path(dir, model)).is_ok_and(|meta| meta.len() == model.bytes)
        })
        .map(|model| model.id.to_owned())
        .collect()
}

/// Downloads `model` into `dir`, carrying on from what an earlier attempt
/// left, and holds it to its checksum. `progress` is told the bytes there
/// are so far. A file that is not the model is deleted.
pub async fn install(
    http: &dyn Http,
    dir: &Path,
    model: &Model,
    progress: Progress,
) -> Result<PathBuf, Error> {
    let done = path(dir, model);
    if installed(dir).iter().any(|id| id == model.id) {
        progress(model.bytes);
        return Ok(done);
    }
    let fetched = fetch_asset(
        http,
        &HostPolicy::Models,
        &catalog::url(model),
        dir,
        &unverified(model),
        model.bytes,
        progress,
    )
    .await
    .map_err(|error| Error::Download(error.to_string()))?;
    let pinned = checksums::decode_hex(model.sha256).ok_or(Error::Checksum)?;
    let (actual, _) =
        checksums::hash_file(&fetched).map_err(|error| Error::Io(error.to_string()))?;
    if actual != pinned {
        let _ = std::fs::remove_file(&fetched);
        return Err(Error::Checksum);
    }
    std::fs::rename(&fetched, &done).map_err(|error| Error::Io(error.to_string()))?;
    Ok(done)
}

/// Removes `model` from `dir`, with whatever a download of it left. One
/// that is not there is removed already.
pub fn remove(dir: &Path, model: &Model) -> Result<(), Error> {
    let part = format!("{}.part", unverified(model));
    for name in [model.file.to_owned(), unverified(model), part] {
        match std::fs::remove_file(dir.join(name)) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(Error::Io(error.to_string())),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::Arc;

    use leon_update::http::{Reply, ScriptedHttp};

    use super::*;

    const CDN: &str = "https://us.aws.cdn.hf.co/xet-bridge-us/a-model";

    /// A model of the catalogue's shape whose file is `data`.
    fn model_of(data: &[u8]) -> Model {
        let (hash, size) = {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("data");
            std::fs::write(&path, data).unwrap();
            checksums::hash_file(&path).unwrap()
        };
        Model {
            id: "base",
            label: "Balanced",
            detail: "",
            file: "ggml-base.bin",
            bytes: size,
            sha256: Box::leak(checksums::hex(&hash).into_boxed_str()),
            english: false,
        }
    }

    fn data() -> Vec<u8> {
        (0..6000u32).map(|at| (at % 251) as u8).collect()
    }

    fn quiet() -> Progress {
        Arc::new(|_| {})
    }

    #[tokio::test]
    async fn a_model_is_installed_once_it_is_whole_and_is_the_one_pinned() {
        let model = model_of(&data());
        let http = ScriptedHttp::new()
            .route(&catalog::url(&model), Reply::redirect(CDN))
            .route(CDN, Reply::file(data()));
        let dir = tempfile::tempdir().unwrap();
        assert!(installed(dir.path()).is_empty());
        let seen = Arc::new(AtomicU64::new(0));
        let seen_in = seen.clone();
        let progress: Progress = Arc::new(move |bytes| seen_in.store(bytes, Ordering::SeqCst));
        // The catalogue's own `base` is another size: this one is found by
        // the file it is kept as.
        let at = install(&http, dir.path(), &model, progress).await.unwrap();
        assert_eq!(at, path(dir.path(), &model));
        assert_eq!(std::fs::read(&at).unwrap(), data());
        assert_eq!(seen.load(Ordering::SeqCst), model.bytes);
        assert!(!dir.path().join("ggml-base.bin.download").exists());

        remove(dir.path(), &model).unwrap();
        assert!(!at.exists());
        // Removed twice is removed.
        remove(dir.path(), &model).unwrap();
    }

    #[tokio::test]
    async fn a_file_that_is_not_the_model_is_deleted_and_never_listed() {
        let model = model_of(&data());
        let mut other = data();
        other[10] ^= 0xff;
        let http = ScriptedHttp::new()
            .route(&catalog::url(&model), Reply::redirect(CDN))
            .route(CDN, Reply::file(other));
        let dir = tempfile::tempdir().unwrap();
        let error = install(&http, dir.path(), &model, quiet()).await;
        assert_eq!(error, Err(Error::Checksum));
        assert!(std::fs::read_dir(dir.path()).unwrap().next().is_none());
    }

    #[tokio::test]
    async fn a_download_cut_off_is_carried_on_and_an_address_elsewhere_is_refused() {
        let model = model_of(&data());
        let cut = Reply::File {
            bytes: data(),
            cut: Some(2500),
            ranges: true,
            extra: 0,
        };
        let http = ScriptedHttp::new()
            .route(&catalog::url(&model), Reply::redirect(CDN))
            .route(CDN, cut)
            .route(CDN, Reply::file(data()));
        let dir = tempfile::tempdir().unwrap();
        let at = install(&http, dir.path(), &model, quiet()).await.unwrap();
        assert_eq!(std::fs::read(at).unwrap(), data());

        let elsewhere = ScriptedHttp::new().route(
            &catalog::url(&model),
            Reply::redirect("https://evil.example/ggml-base.bin"),
        );
        let dir = tempfile::tempdir().unwrap();
        let error = install(&elsewhere, dir.path(), &model, quiet()).await;
        assert!(matches!(error, Err(Error::Download(_))), "{error:?}");
    }

    #[test]
    fn only_a_file_of_its_models_size_is_an_installed_model() {
        let dir = tempfile::tempdir().unwrap();
        let tiny = catalog::find("tiny").unwrap();
        std::fs::write(path(dir.path(), tiny), b"cut off").unwrap();
        assert!(installed(dir.path()).is_empty());
        assert!(installed(&dir.path().join("not-there")).is_empty());
    }
}
