//! Dictation without the application, to try a model or a microphone by
//! hand:
//!
//! ```sh
//! cargo run -p leon-voice --example dictate -- get tiny /tmp/models
//! cargo run -p leon-voice --example dictate -- wav /tmp/models tiny speech.wav en
//! cargo run -p leon-voice --example dictate -- mic /tmp/models tiny 5 es
//! ```
//!
//! `wav` reads 16-bit PCM, any rate and any number of channels.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use leon_voice::{audio, capture, catalog, store, Engine, Whisper};

fn main() {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let words: Vec<&str> = arguments.iter().map(String::as_str).collect();
    let done = match words.as_slice() {
        ["get", id, dir] => get(id, Path::new(dir)),
        ["wav", dir, id, file, rest @ ..] => {
            read_wav(Path::new(file)).and_then(|heard| say(Path::new(dir), id, &heard, rest))
        }
        ["mic", dir, id, seconds, rest @ ..] => {
            record(seconds).and_then(|heard| say(Path::new(dir), id, &heard, rest))
        }
        _ => Err("usage: dictate get <model> <dir> | wav <dir> <model> <file> [language] | mic <dir> <model> <seconds> [language]".to_owned()),
    };
    if let Err(error) = done {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn model(id: &str) -> Result<&'static catalog::Model, String> {
    catalog::find(id).ok_or_else(|| format!("no model is called {id}"))
}

fn get(id: &str, dir: &Path) -> Result<(), String> {
    let model = model(id)?;
    let runtime = tokio::runtime::Runtime::new().map_err(|error| error.to_string())?;
    let http = leon_update::CurlHttp::models();
    let total = model.bytes;
    let progress: leon_update::http::Progress = Arc::new(move |bytes| {
        eprint!(
            "\r{} of {}",
            catalog::megabytes(bytes),
            catalog::megabytes(total)
        );
    });
    let at: PathBuf = runtime
        .block_on(store::install(&http, dir, model, progress))
        .map_err(|error| error.to_string())?;
    eprintln!();
    println!("{}", at.display());
    Ok(())
}

fn record(seconds: &str) -> Result<Vec<f32>, String> {
    let seconds: u64 = seconds
        .parse()
        .map_err(|_| "seconds is a number".to_owned())?;
    let recording = capture::start().map_err(|error| error.to_string())?;
    let until = Instant::now() + Duration::from_secs(seconds);
    while Instant::now() < until {
        let bars = (recording.level() * 40.0) as usize;
        eprint!("\rspeak {:<40}", "#".repeat(bars));
        std::thread::sleep(Duration::from_millis(80));
    }
    eprintln!();
    Ok(recording.finish())
}

fn say(dir: &Path, id: &str, heard: &[f32], rest: &[&str]) -> Result<(), String> {
    let model = model(id)?;
    if !audio::worth_hearing(heard) {
        return Err("nothing was heard".to_owned());
    }
    let loaded = Instant::now();
    let engine = Whisper::load(&store::path(dir, model)).map_err(|error| error.to_string())?;
    let started = Instant::now();
    let text = engine
        .transcribe(heard, rest.first().copied())
        .map_err(|error| error.to_string())?;
    eprintln!(
        "{:.1} s of sound, loaded in {:.1} s, heard in {:.1} s",
        heard.len() as f32 / audio::RATE as f32,
        (started - loaded).as_secs_f32(),
        started.elapsed().as_secs_f32()
    );
    println!("{text}");
    Ok(())
}

/// The samples of a 16-bit PCM WAV file, as a model listens to them.
fn read_wav(path: &Path) -> Result<Vec<f32>, String> {
    let bytes = std::fs::read(path).map_err(|error| error.to_string())?;
    if bytes.len() < 12 || &bytes[..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err("not a WAV file".to_owned());
    }
    let (mut at, mut channels, mut rate, mut bits) = (12usize, 1usize, audio::RATE, 16u16);
    while at + 8 <= bytes.len() {
        let size = u32::from_le_bytes([bytes[at + 4], bytes[at + 5], bytes[at + 6], bytes[at + 7]]);
        let body = at + 8;
        let end = (body + size as usize).min(bytes.len());
        match &bytes[at..at + 4] {
            b"fmt " if end - body >= 16 => {
                channels = usize::from(u16::from_le_bytes([bytes[body + 2], bytes[body + 3]]));
                rate = u32::from_le_bytes([
                    bytes[body + 4],
                    bytes[body + 5],
                    bytes[body + 6],
                    bytes[body + 7],
                ]);
                bits = u16::from_le_bytes([bytes[body + 14], bytes[body + 15]]);
            }
            b"data" => {
                if bits != 16 {
                    return Err(format!("{bits}-bit sound, and 16-bit is read"));
                }
                let samples: Vec<f32> = bytes[body..end]
                    .chunks_exact(2)
                    .map(|pair| f32::from(i16::from_le_bytes([pair[0], pair[1]])) / 32768.0)
                    .collect();
                return Ok(audio::resample(&audio::mono(&samples, channels), rate));
            }
            _ => {}
        }
        at = end + (size as usize & 1);
    }
    Err("the WAV file has no sound".to_owned())
}
