//! The speech models there are, and which of them serves a language.
//!
//! The models are whisper.cpp's, kept on Hugging Face. Two kinds: one that
//! knows only English, smaller for what it does, and one that knows about a
//! hundred languages. So which model is offered depends on the language the
//! person speaks: an English speaker is offered the English ones, everybody
//! else the others. Each is pinned to one revision of the repository and to
//! the SHA-256 of its file, written here: a file that is not that one is not
//! installed, wherever it came from.

/// The revision of the repository every model is downloaded from.
pub const REVISION: &str = "5359861c739e955e79d9a303bcbc70fb988958b1";

/// A model a person can download.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Model {
    /// What settings keep it under.
    pub id: &'static str,
    /// Its name.
    pub label: &'static str,
    /// What it is good for, in a sentence.
    pub detail: &'static str,
    /// Its file's name, in the repository and on this computer.
    pub file: &'static str,
    /// Its file's size, in bytes.
    pub bytes: u64,
    /// Its file's SHA-256, in hex.
    pub sha256: &'static str,
    /// It knows only English.
    pub english: bool,
}

/// Every model, the lightest first within each kind.
pub const MODELS: &[Model] = &[
    Model {
        id: "tiny.en",
        label: "Fast",
        detail: "The quickest to answer; mishears more.",
        file: "ggml-tiny.en.bin",
        bytes: 77_704_715,
        sha256: "921e4cf8686fdd993dcd081a5da5b6c365bfde1162e72b08d75ac75289920b1f",
        english: true,
    },
    Model {
        id: "base.en",
        label: "Balanced",
        detail: "Quick and seldom wrong: the one to start with.",
        file: "ggml-base.en.bin",
        bytes: 147_964_211,
        sha256: "a03779c86df3323075f5e796cb2ce5029f00ec8869eee3fdfb897afe36c6d002",
        english: true,
    },
    Model {
        id: "small.en",
        label: "Accurate",
        detail: "The fewest mistakes; slower, and a large download.",
        file: "ggml-small.en.bin",
        bytes: 487_614_201,
        sha256: "c6138d6d58ecc8322097e0f987c32f1be8bb0a18532a3f88f734d1bbf9c41e5d",
        english: true,
    },
    Model {
        id: "tiny",
        label: "Fast",
        detail: "The quickest to answer; mishears more.",
        file: "ggml-tiny.bin",
        bytes: 77_691_713,
        sha256: "be07e048e1e599ad46341c8d2a135645097a538221678b7acdd1b1919c6e1b21",
        english: false,
    },
    Model {
        id: "base",
        label: "Balanced",
        detail: "Quick and seldom wrong: the one to start with.",
        file: "ggml-base.bin",
        bytes: 147_951_465,
        sha256: "60ed5bc3dd14eea856493d334349b405782ddcaf0028d4b5df4088345fba2efe",
        english: false,
    },
    Model {
        id: "small",
        label: "Accurate",
        detail: "The fewest mistakes; slower to answer.",
        file: "ggml-small-q5_1.bin",
        bytes: 190_085_487,
        sha256: "ae85e4a935d7a567bd102fe55afc16bb595bdb618e11b2fc7591bc08120411bb",
        english: false,
    },
];

/// The languages a person can say they speak, by the code a model takes.
/// The models know more; these are the ones offered by name.
pub const LANGUAGES: &[(&str, &str)] = &[
    ("ar", "Arabic"),
    ("ca", "Catalan"),
    ("zh", "Chinese"),
    ("cs", "Czech"),
    ("da", "Danish"),
    ("nl", "Dutch"),
    ("en", "English"),
    ("fi", "Finnish"),
    ("fr", "French"),
    ("de", "German"),
    ("el", "Greek"),
    ("he", "Hebrew"),
    ("hi", "Hindi"),
    ("hu", "Hungarian"),
    ("id", "Indonesian"),
    ("it", "Italian"),
    ("ja", "Japanese"),
    ("ko", "Korean"),
    ("no", "Norwegian"),
    ("pl", "Polish"),
    ("pt", "Portuguese"),
    ("ro", "Romanian"),
    ("ru", "Russian"),
    ("es", "Spanish"),
    ("sv", "Swedish"),
    ("th", "Thai"),
    ("tr", "Turkish"),
    ("uk", "Ukrainian"),
    ("vi", "Vietnamese"),
];

/// The name of the language a code stands for.
pub fn language_name(code: &str) -> Option<&'static str> {
    LANGUAGES
        .iter()
        .find(|(have, _)| *have == code)
        .map(|(_, name)| *name)
}

/// The language of a locale as the system writes it (`es_MX.UTF-8`,
/// `en-GB`, `pt_BR@euro`), when it is one that is offered. `C` and `POSIX`
/// are no language.
pub fn language_of_locale(locale: &str) -> Option<&'static str> {
    let code = locale
        .split(['_', '-', '.', '@'])
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    // Norwegian is written three ways.
    let code = match code.as_str() {
        "nb" | "nn" => "no",
        other => other,
    };
    LANGUAGES
        .iter()
        .find(|(have, _)| *have == code)
        .map(|(have, _)| *have)
}

/// The language a person speaks, from the first of the system's locale
/// variables that names one, in the order the system reads them.
pub fn language_of_system(variable: impl Fn(&str) -> Option<String>) -> Option<&'static str> {
    ["LC_ALL", "LC_MESSAGES", "LANG"]
        .into_iter()
        .filter_map(variable)
        .find_map(|locale| language_of_locale(&locale))
}

/// The model kept under `id`.
pub fn find(id: &str) -> Option<&'static Model> {
    MODELS.iter().find(|model| model.id == id)
}

/// Whether `model` makes text of `language`; `None` is a language nobody
/// named, which only a model of many can be asked to tell.
pub fn serves(model: &Model, language: Option<&str>) -> bool {
    !model.english || language == Some("en")
}

/// The models offered to somebody who speaks `language`, the lightest
/// first: the English ones for English, the ones of many for the rest.
pub fn offered(language: Option<&str>) -> Vec<&'static Model> {
    let english = language == Some("en");
    MODELS
        .iter()
        .filter(|model| model.english == english)
        .collect()
}

/// The model to use for `language` among the `installed` (by id): the one
/// `chosen` when it is there and serves the language, else the most
/// accurate that is.
pub fn in_use(
    language: Option<&str>,
    chosen: Option<&str>,
    installed: &[String],
) -> Option<&'static Model> {
    let has = |model: &&Model| installed.iter().any(|id| id == model.id) && serves(model, language);
    chosen
        .and_then(find)
        .filter(has)
        // Among those of the language's own kind first: they do it best.
        .or_else(|| offered(language).into_iter().rev().find(has))
        .or_else(|| MODELS.iter().rev().find(has))
}

/// Where `model` is downloaded from.
pub fn url(model: &Model) -> String {
    format!(
        "https://huggingface.co/ggerganov/whisper.cpp/resolve/{REVISION}/{}",
        model.file
    )
}

/// A size as a person reads it: `148 MB`.
pub fn megabytes(bytes: u64) -> String {
    format!("{} MB", (bytes + 500_000) / 1_000_000)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(models: &[&Model]) -> Vec<&'static str> {
        models.iter().map(|model| model.id).collect()
    }

    #[test]
    fn the_models_offered_depend_on_the_language_spoken() {
        assert_eq!(
            ids(&offered(Some("en"))),
            ["tiny.en", "base.en", "small.en"]
        );
        assert_eq!(ids(&offered(Some("es"))), ["tiny", "base", "small"]);
        // A language nobody named is told by a model of many.
        assert_eq!(ids(&offered(None)), ["tiny", "base", "small"]);
        assert!(serves(find("base").unwrap(), Some("en")));
        assert!(!serves(find("base.en").unwrap(), Some("es")));
        assert!(!serves(find("base.en").unwrap(), None));
    }

    #[test]
    fn every_model_is_pinned_and_none_is_listed_twice() {
        for (at, model) in MODELS.iter().enumerate() {
            assert_eq!(model.sha256.len(), 64, "{}", model.id);
            assert!(model.sha256.bytes().all(|b| b.is_ascii_hexdigit()));
            assert!(model.bytes > 1_000_000 && model.bytes <= 512 * 1024 * 1024);
            assert!(MODELS[..at].iter().all(|other| other.id != model.id));
            assert!(MODELS[..at].iter().all(|other| other.file != model.file));
            assert!(leon_update::http::HostPolicy::Models.allows(&url(model)));
        }
        assert_eq!(
            url(find("base").unwrap()),
            format!(
                "https://huggingface.co/ggerganov/whisper.cpp/resolve/{REVISION}/ggml-base.bin"
            )
        );
        assert_eq!(megabytes(147_951_465), "148 MB");
    }

    #[test]
    fn the_language_is_read_from_the_systems_locale() {
        assert_eq!(language_of_locale("es_MX.UTF-8"), Some("es"));
        assert_eq!(language_of_locale("en-GB"), Some("en"));
        assert_eq!(language_of_locale("pt_BR@euro"), Some("pt"));
        assert_eq!(language_of_locale("nb_NO.UTF-8"), Some("no"));
        assert_eq!(language_of_locale("C.UTF-8"), None);
        assert_eq!(language_of_locale("POSIX"), None);
        assert_eq!(language_of_locale(""), None);
        assert_eq!(language_name("es"), Some("Spanish"));

        let set = |pairs: &'static [(&str, &str)]| {
            move |name: &str| {
                pairs
                    .iter()
                    .find(|(have, _)| *have == name)
                    .map(|(_, value)| (*value).to_owned())
            }
        };
        assert_eq!(
            language_of_system(set(&[("LANG", "en_US.UTF-8"), ("LC_ALL", "es_ES.UTF-8")])),
            Some("es")
        );
        // One that names no language is passed over for the next.
        assert_eq!(
            language_of_system(set(&[("LC_ALL", "C"), ("LANG", "fr_FR")])),
            Some("fr")
        );
        assert_eq!(language_of_system(set(&[])), None);
    }

    #[test]
    fn the_model_in_use_is_the_one_chosen_or_the_best_that_is_there() {
        let have = |ids: &[&str]| ids.iter().map(|id| (*id).to_owned()).collect::<Vec<_>>();
        assert_eq!(in_use(Some("es"), None, &have(&[])), None);
        let both = have(&["tiny", "base"]);
        assert_eq!(in_use(Some("es"), None, &both).unwrap().id, "base");
        assert_eq!(in_use(Some("es"), Some("tiny"), &both).unwrap().id, "tiny");
        // One chosen that was removed, or is not a model, is not in use.
        assert_eq!(in_use(Some("es"), Some("small"), &both).unwrap().id, "base");
        assert_eq!(in_use(Some("es"), Some("nope"), &both).unwrap().id, "base");
        // An English one is no use for Spanish, chosen or not.
        let english = have(&["base.en"]);
        assert_eq!(in_use(Some("es"), Some("base.en"), &english), None);
        assert_eq!(in_use(Some("en"), None, &english).unwrap().id, "base.en");
        // English is also spoken by a model of many, when that is all there is.
        assert_eq!(in_use(Some("en"), None, &both).unwrap().id, "base");
        assert_eq!(
            in_use(Some("en"), None, &have(&["base", "tiny.en"]))
                .unwrap()
                .id,
            "tiny.en"
        );
    }
}
