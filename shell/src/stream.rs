//! Sound made as it plays (an emulator's, a frame at a time) on the
//! machine's speakers: samples pushed in, heard a moment later. Too few:
//! silence until more come; too many (the maker went ahead): the oldest
//! dropped, so that what is heard keeps up with what is seen.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use rodio::{OutputStream, Sink, Source};

/// The samples waiting, left, right, left…
type Queue = Arc<Mutex<VecDeque<i16>>>;

/// A stream of stereo sound at a rate, playing.
pub struct Stream {
    _stream: OutputStream,
    sink: Sink,
    queue: Queue,
    rate: u32,
}

impl Stream {
    /// One playing at that rate (samples a second, each a left and a right); none without a device.
    pub fn open(rate: u32) -> Option<Stream> {
        let (stream, handle) = OutputStream::try_default().ok()?;
        let sink = Sink::try_new(&handle).ok()?;
        let queue = Queue::default();
        sink.append(Pulled { queue: queue.clone(), rate, at: 0, silent: true });
        Some(Stream { _stream: stream, sink, queue, rate })
    }

    /// The rate it plays at.
    pub fn rate(&self) -> u32 {
        self.rate
    }

    /// Stereo samples to play after those waiting; past `most` seconds waiting, the oldest dropped.
    pub fn push(&self, stereo: &[i16], most: f32) {
        let mut queue = self.queue.lock().unwrap_or_else(PoisonError::into_inner);
        queue.extend(stereo);
        let kept = ((self.rate as f32 * most) as usize * 2).max(2);
        if queue.len() > kept {
            let over = (queue.len() - kept) & !1;
            queue.drain(..over);
        }
    }

    /// What waits, forgotten (paused, sped up).
    pub fn clear(&self) {
        self.queue.lock().unwrap_or_else(PoisonError::into_inner).clear();
    }

    pub fn set_volume(&self, volume: f32) {
        self.sink.set_volume(volume);
    }
}

/// The source the speakers pull from: the queue, or silence.
struct Pulled {
    queue: Queue,
    rate: u32,
    /// Samples given: even, a left comes next.
    at: u64,
    /// This pair is silence (the queue had less than a pair when it began).
    silent: bool,
}

impl Iterator for Pulled {
    type Item = i16;

    fn next(&mut self) -> Option<i16> {
        let mut queue = self.queue.lock().unwrap_or_else(PoisonError::into_inner);
        // (Out of samples: silence, a whole pair at a time, so left stays left.)
        if self.at.is_multiple_of(2) {
            self.silent = queue.len() < 2;
        }
        let sample = if self.silent { 0 } else { queue.pop_front().unwrap_or(0) };
        self.at += 1;
        Some(sample)
    }
}

impl Source for Pulled {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> u16 {
        2
    }

    fn sample_rate(&self) -> u32 {
        self.rate
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn silence_comes_in_pairs_and_samples_in_order() {
        let queue = Queue::default();
        let mut pulled = Pulled { queue: queue.clone(), rate: 32768, at: 0, silent: true };
        assert_eq!(pulled.next(), Some(0));
        queue.lock().unwrap().extend([5, 6]);
        // (After one sample of silence, the right one: silence still, then the pair.)
        assert_eq!([pulled.next(), pulled.next(), pulled.next()], [Some(0), Some(5), Some(6)]);
    }
}
