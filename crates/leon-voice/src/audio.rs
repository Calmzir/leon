//! The arithmetic of sound, without a microphone or a model.
//!
//! A microphone gives what it has: one channel or several, at the rate of
//! its hardware. A speech model listens to one channel at [`RATE`]. This is
//! the way from one to the other, and how loud a stretch of it is.

/// The samples a second a model listens at.
pub const RATE: u32 = 16_000;

/// The shortest recording worth giving a model: less is a click, and a
/// model makes words up for silence that short.
pub const SHORTEST: usize = RATE as usize * 3 / 10;

/// Frames of `channels` samples each as one channel: the mean of each frame.
/// Samples left over at the end, less than a frame, are dropped.
pub fn mono(samples: &[f32], channels: usize) -> Vec<f32> {
    match channels {
        0 => Vec::new(),
        1 => samples.to_vec(),
        _ => samples
            .chunks_exact(channels)
            .map(|frame| frame.iter().sum::<f32>() / channels as f32)
            .collect(),
    }
}

/// One channel recorded at `from` samples a second, as it would be at
/// [`RATE`]: each sample is read between its two neighbours. Speech survives
/// this well, which is all a model needs of it.
pub fn resample(samples: &[f32], from: u32) -> Vec<f32> {
    if from == RATE || from == 0 || samples.is_empty() {
        return samples.to_vec();
    }
    let step = f64::from(from) / f64::from(RATE);
    let count = (samples.len() as f64 / step).floor() as usize;
    (0..count)
        .map(|at| {
            let place = at as f64 * step;
            let before = place.floor() as usize;
            let after = (before + 1).min(samples.len() - 1);
            let part = (place - before as f64) as f32;
            samples[before] + (samples[after] - samples[before]) * part
        })
        .collect()
}

/// How loud `samples` are, from `0.0` (silence) to `1.0`: their root mean
/// square, stretched so that ordinary speech fills most of the range.
pub fn level(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let mean = samples.iter().map(|sample| sample * sample).sum::<f32>() / samples.len() as f32;
    (mean.sqrt() * 6.0).clamp(0.0, 1.0)
}

/// Whether a recording at [`RATE`] holds anything to make text of: long
/// enough, and not silence throughout.
pub fn worth_hearing(samples: &[f32]) -> bool {
    samples.len() >= SHORTEST && samples.iter().any(|sample| sample.abs() > 0.004)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn several_channels_become_the_mean_of_each_frame() {
        assert_eq!(mono(&[0.2, 0.4, -1.0, 1.0], 2), [0.3, 0.0]);
        assert_eq!(mono(&[0.5, 0.25], 1), [0.5, 0.25]);
        // What is left of a frame cut short is not a frame.
        assert_eq!(mono(&[0.3, 0.3, 0.9], 2), [0.3]);
        assert!(mono(&[0.1], 0).is_empty());
    }

    #[test]
    fn any_rate_becomes_the_rate_a_model_listens_at() {
        let second = vec![0.5f32; 48_000];
        let heard = resample(&second, 48_000);
        assert_eq!(heard.len(), RATE as usize);
        assert!(heard.iter().all(|sample| (sample - 0.5).abs() < 1e-6));
        // A ramp stays a ramp: samples are read between their neighbours.
        let ramp: Vec<f32> = (0..8).map(|at| at as f32).collect();
        assert_eq!(resample(&ramp, 32_000), [0.0, 2.0, 4.0, 6.0]);
        // Slower than a model listens: stretched, never read past the end.
        let slow = resample(&[0.0, 1.0], 8_000);
        assert_eq!(slow, [0.0, 0.5, 1.0, 1.0]);
        assert_eq!(resample(&[0.1, 0.2], RATE), [0.1, 0.2]);
        assert!(resample(&[], 44_100).is_empty());
    }

    #[test]
    fn loudness_is_nothing_for_silence_and_full_for_a_shout() {
        assert_eq!(level(&[]), 0.0);
        assert_eq!(level(&[0.0; 64]), 0.0);
        assert_eq!(level(&[1.0; 64]), 1.0);
        let speech = level(&[0.1; 64]);
        assert!(speech > 0.5 && speech < 0.7, "{speech}");
    }

    #[test]
    fn a_click_and_a_silence_are_not_worth_a_model() {
        assert!(!worth_hearing(&[0.5; 100]));
        assert!(!worth_hearing(&vec![0.0; RATE as usize]));
        assert!(worth_hearing(&vec![0.1; RATE as usize]));
    }
}
