//! Replay buffer for AlphaZero self-play data.

use std::collections::VecDeque;
use rand::Rng;

// ── Training sample ────────────────────────────────────────────────────────

/// One training example produced by self-play.
#[derive(Clone, Debug)]
pub struct TrainSample {
    /// Observation tensor from the acting player's perspective (`obs_size` floats).
    pub obs: Vec<f32>,
    /// MCTS policy target: normalized visit counts (`action_space` floats, sums to 1).
    pub policy: Vec<f32>,
    /// Game outcome from the acting player's perspective: +1 win, -1 loss, 0 draw.
    pub value: f32,
}

// ── Replay buffer ──────────────────────────────────────────────────────────

/// Fixed-capacity circular buffer of [`TrainSample`]s.
///
/// When the buffer is full, the oldest sample is evicted on push.
/// Samples are drawn without replacement using a Fisher-Yates partial shuffle.
pub struct ReplayBuffer {
    data: VecDeque<TrainSample>,
    capacity: usize,
}

impl ReplayBuffer {
    /// Create a buffer with the given maximum capacity.
    pub fn new(capacity: usize) -> Self {
        Self {
            data: VecDeque::with_capacity(capacity.min(1024)),
            capacity,
        }
    }

    /// Add a sample; evicts the oldest if at capacity.
    pub fn push(&mut self, sample: TrainSample) {
        if self.data.len() == self.capacity {
            self.data.pop_front();
        }
        self.data.push_back(sample);
    }

    /// Add all samples from an episode.
    pub fn extend(&mut self, samples: impl IntoIterator<Item = TrainSample>) {
        for s in samples {
            self.push(s);
        }
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Sample up to `n` distinct samples, without replacement.
    ///
    /// If the buffer has fewer than `n` samples, all are returned (shuffled).
    pub fn sample_batch(&self, n: usize, rng: &mut impl Rng) -> Vec<&TrainSample> {
        let len = self.data.len();
        let n = n.min(len);
        // Partial Fisher-Yates using index shuffling.
        let mut indices: Vec<usize> = (0..len).collect();
        for i in 0..n {
            let j = rng.random_range(i..len);
            indices.swap(i, j);
        }
        indices[..n].iter().map(|&i| &self.data[i]).collect()
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{SeedableRng, rngs::SmallRng};

    fn dummy(value: f32) -> TrainSample {
        TrainSample { obs: vec![value], policy: vec![1.0], value }
    }

    #[test]
    fn push_and_len() {
        let mut buf = ReplayBuffer::new(10);
        assert!(buf.is_empty());
        buf.push(dummy(1.0));
        buf.push(dummy(2.0));
        assert_eq!(buf.len(), 2);
    }

    #[test]
    fn evicts_oldest_at_capacity() {
        let mut buf = ReplayBuffer::new(3);
        buf.push(dummy(1.0));
        buf.push(dummy(2.0));
        buf.push(dummy(3.0));
        buf.push(dummy(4.0)); // evicts 1.0
        assert_eq!(buf.len(), 3);
        // Oldest remaining should be 2.0
        assert_eq!(buf.data[0].value, 2.0);
    }

    #[test]
    fn sample_batch_size() {
        let mut buf = ReplayBuffer::new(20);
        for i in 0..10 {
            buf.push(dummy(i as f32));
        }
        let mut rng = SmallRng::seed_from_u64(0);
        let batch = buf.sample_batch(5, &mut rng);
        assert_eq!(batch.len(), 5);
    }

    #[test]
    fn sample_batch_capped_at_len() {
        let mut buf = ReplayBuffer::new(20);
        buf.push(dummy(1.0));
        buf.push(dummy(2.0));
        let mut rng = SmallRng::seed_from_u64(0);
        let batch = buf.sample_batch(100, &mut rng);
        assert_eq!(batch.len(), 2);
    }

    #[test]
    fn sample_batch_no_duplicates() {
        let mut buf = ReplayBuffer::new(20);
        for i in 0..10 {
            buf.push(dummy(i as f32));
        }
        let mut rng = SmallRng::seed_from_u64(1);
        let batch = buf.sample_batch(10, &mut rng);
        let mut seen: Vec<f32> = batch.iter().map(|s| s.value).collect();
        seen.sort_by(f32::total_cmp);
        seen.dedup();
        assert_eq!(seen.len(), 10, "sample contained duplicates");
    }
}
