//! The microphone of this computer.
//!
//! [`start`] opens the microphone the system has as its own and records
//! until [`Recording::finish`], which gives what was heard as one channel at
//! the rate a model listens at. The stream belongs to the thread that made
//! it (on some systems it cannot leave it), so a thread of its own keeps it
//! and the recording is a handle to that thread.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, Sample, SampleFormat, SizedSample};

use crate::audio;
use crate::Error;

/// The longest a recording is kept: what is said after that is not heard.
/// A prompt is seldom a minute; this only bounds a microphone left open.
pub const LONGEST_SECONDS: usize = 300;

/// What the microphone's thread shares with its handle.
#[derive(Default)]
struct Heard {
    /// The samples as the microphone gives them, channel after channel.
    samples: Mutex<Vec<f32>>,
    /// How loud the last of them were, as the bits of an `f32`.
    level: AtomicU32,
}

/// A recording in progress.
pub struct Recording {
    heard: Arc<Heard>,
    channels: usize,
    rate: u32,
    stop: Option<Sender<()>>,
    thread: Option<JoinHandle<()>>,
}

/// Opens the system's microphone and starts recording.
pub fn start() -> Result<Recording, Error> {
    let heard = Arc::new(Heard::default());
    let (ready, opened) = mpsc::channel();
    let (stop, stopped) = mpsc::channel::<()>();
    let shared = heard.clone();
    let thread = std::thread::Builder::new()
        .name("leon-voice".to_owned())
        .spawn(move || match open(shared) {
            Ok((stream, channels, rate)) => {
                let _ = ready.send(Ok((channels, rate)));
                // Until told to stop, or until the handle is gone.
                let _ = stopped.recv();
                drop(stream);
            }
            Err(error) => {
                let _ = ready.send(Err(error));
            }
        })
        .map_err(|error| Error::Microphone(error.to_string()))?;
    let (channels, rate) = opened
        .recv()
        .map_err(|_| Error::Microphone("its thread ended".to_owned()))??;
    Ok(Recording {
        heard,
        channels,
        rate,
        stop: Some(stop),
        thread: Some(thread),
    })
}

/// Opens the stream on the calling thread: the stream, how many channels it
/// has and its rate.
fn open(heard: Arc<Heard>) -> Result<(cpal::Stream, usize, u32), Error> {
    let failed = |error: &dyn std::fmt::Display| Error::Microphone(error.to_string());
    let device = cpal::default_host()
        .default_input_device()
        .ok_or_else(|| Error::Microphone("this computer has none".to_owned()))?;
    let config = device
        .default_input_config()
        .map_err(|error| failed(&error))?;
    let channels = usize::from(config.channels());
    let rate = config.sample_rate();
    let most = LONGEST_SECONDS * rate as usize * channels.max(1);
    let stream = match config.sample_format() {
        SampleFormat::F32 => listen::<f32>(&device, &config, heard, most),
        SampleFormat::I16 => listen::<i16>(&device, &config, heard, most),
        SampleFormat::U16 => listen::<u16>(&device, &config, heard, most),
        SampleFormat::I32 => listen::<i32>(&device, &config, heard, most),
        SampleFormat::I8 => listen::<i8>(&device, &config, heard, most),
        SampleFormat::U8 => listen::<u8>(&device, &config, heard, most),
        other => return Err(Error::Microphone(format!("it records as {other}"))),
    }
    .map_err(|error| failed(&error))?;
    stream.play().map_err(|error| failed(&error))?;
    Ok((stream, channels, rate))
}

/// A stream of samples of type `T`, kept as `f32` in `heard` up to `most`.
fn listen<T>(
    device: &cpal::Device,
    config: &cpal::SupportedStreamConfig,
    heard: Arc<Heard>,
    most: usize,
) -> Result<cpal::Stream, cpal::Error>
where
    T: SizedSample,
    f32: FromSample<T>,
{
    device.build_input_stream(
        config.config(),
        move |data: &[T], _: &cpal::InputCallbackInfo| {
            let block: Vec<f32> = data
                .iter()
                .map(|sample| f32::from_sample(*sample))
                .collect();
            heard
                .level
                .store(audio::level(&block).to_bits(), Ordering::Relaxed);
            let Ok(mut samples) = heard.samples.lock() else {
                return;
            };
            let room = most.saturating_sub(samples.len());
            samples.extend_from_slice(&block[..block.len().min(room)]);
        },
        // A microphone that fails while it records leaves what it heard.
        |_| {},
        None,
    )
}

impl Recording {
    /// How loud what is being said is, from `0.0` to `1.0`.
    pub fn level(&self) -> f32 {
        f32::from_bits(self.heard.level.load(Ordering::Relaxed))
    }

    /// Stops recording: what was heard, as one channel at
    /// [`audio::RATE`].
    pub fn finish(mut self) -> Vec<f32> {
        self.close();
        let samples = self
            .heard
            .samples
            .lock()
            .map(|mut samples| std::mem::take(&mut *samples))
            .unwrap_or_default();
        audio::resample(&audio::mono(&samples, self.channels), self.rate)
    }

    /// Closes the microphone and waits for its thread.
    fn close(&mut self) {
        drop(self.stop.take());
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

impl Drop for Recording {
    fn drop(&mut self) {
        self.close();
    }
}
