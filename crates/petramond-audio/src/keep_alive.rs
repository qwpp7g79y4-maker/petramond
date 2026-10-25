use std::time::Duration;

use rodio::source::Source;
use rodio::{ChannelCount, Sample, SampleRate};

const AMPLITUDE: f32 = 1.0 / 16_384.0;

pub(super) struct KeepAlive {
    channels: ChannelCount,
    sample_rate: SampleRate,
    rng: u64,
}

impl KeepAlive {
    pub(super) fn new(channels: ChannelCount, sample_rate: SampleRate) -> Self {
        Self {
            channels,
            sample_rate,
            rng: 0x9E37_79B9_7F4A_7C15,
        }
    }
}

impl Iterator for KeepAlive {
    type Item = Sample;

    #[inline]
    fn next(&mut self) -> Option<Sample> {
        let mut x = self.rng;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.rng = x;
        let unit = (x >> 40) as f32 / (1u32 << 24) as f32;
        Some((unit * 2.0 - 1.0) * AMPLITUDE)
    }
}

impl Source for KeepAlive {
    #[inline]
    fn current_span_len(&self) -> Option<usize> {
        None
    }

    #[inline]
    fn channels(&self) -> ChannelCount {
        self.channels
    }

    #[inline]
    fn sample_rate(&self) -> SampleRate {
        self.sample_rate
    }

    #[inline]
    fn total_duration(&self) -> Option<Duration> {
        None
    }
}
