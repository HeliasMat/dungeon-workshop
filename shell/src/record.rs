//! What the microphone hears, kept until stopped: one channel, at the speakers' rate.

use std::sync::{Arc, Mutex};

use rodio::DeviceTrait;
use rodio::cpal;
use rodio::cpal::traits::{HostTrait, StreamTrait};

use crate::audio::OUTPUT_RATE;

/// A recording going on: the stream the microphone feeds, what it has given so far, at its own rate.
pub struct Recording {
    _stream: cpal::Stream,
    heard: Arc<Mutex<Vec<f32>>>,
    rate: u32,
}

impl Recording {
    /// The default microphone, listened to now (why not, if it cannot be).
    pub fn start() -> Result<Recording, crate::Error> {
        let host = cpal::default_host();
        let device = host.default_input_device().ok_or(crate::Error::NoMicrophone)?;
        let config = device.default_input_config().map_err(|why| crate::Error::Microphone(why.to_string()))?;
        let (rate, channels) = (config.sample_rate().0, config.channels() as usize);
        let heard = Arc::new(Mutex::new(Vec::new()));
        let kept = heard.clone();
        let failed = |why: cpal::StreamError| eprintln!("microphone: {why}");
        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => device.build_input_stream(
                &config.into(),
                move |data: &[f32], _: &_| {
                    if let Ok(mut kept) = kept.lock() {
                        kept.extend(data.chunks(channels).map(|frame| frame.iter().sum::<f32>() / channels as f32));
                    }
                },
                failed,
                None,
            ),
            cpal::SampleFormat::I16 => device.build_input_stream(
                &config.into(),
                move |data: &[i16], _: &_| {
                    if let Ok(mut kept) = kept.lock() {
                        kept.extend(data.chunks(channels).map(|frame| frame.iter().map(|value| f32::from(*value) / 32768.0).sum::<f32>() / channels as f32));
                    }
                },
                failed,
                None,
            ),
            cpal::SampleFormat::U16 => device.build_input_stream(
                &config.into(),
                move |data: &[u16], _: &_| {
                    if let Ok(mut kept) = kept.lock() {
                        kept.extend(
                            data.chunks(channels).map(|frame| frame.iter().map(|value| (f32::from(*value) - 32768.0) / 32768.0).sum::<f32>() / channels as f32),
                        );
                    }
                },
                failed,
                None,
            ),
            other => return Err(crate::Error::MicrophoneFormat(format!("{other:?}"))),
        }
        .map_err(|why| crate::Error::Microphone(why.to_string()))?;
        stream.play().map_err(|why| crate::Error::Microphone(why.to_string()))?;
        Ok(Recording { _stream: stream, heard, rate })
    }

    /// How long it has heard, in seconds.
    pub fn seconds(&self) -> f32 {
        self.heard.lock().map_or(0.0, |heard| heard.len() as f32 / self.rate as f32)
    }

    /// Stopped: what it heard, one channel at the speakers' rate.
    pub fn stop(self) -> Vec<f32> {
        let heard = self.heard.lock().map(|heard| heard.clone()).unwrap_or_default();
        let step = self.rate as f32 / OUTPUT_RATE as f32;
        let count = (heard.len() as f32 / step) as usize;
        (0..count)
            .map(|at| {
                let from = at as f32 * step;
                let below = (from as usize).min(heard.len().saturating_sub(1));
                let above = (below + 1).min(heard.len().saturating_sub(1));
                heard[below] * (1.0 - from.fract()) + heard[above] * from.fract()
            })
            .collect()
    }
}
