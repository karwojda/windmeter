
pub struct MovingAverage<const N: usize> {
    buffer: [f32; N],
    index: usize,
    count: usize,
    sum: f32,
}

impl<const N: usize> MovingAverage<N> {
    /// Creates a new, empty moving average filter.
    pub const fn new() -> Self {
        assert!(N > 0, "Window size must be greater than zero");
        Self {
            buffer: [0.0; N],
            index: 0,
            count: 0,
            sum: 0.0,
        }
    }

    /// Adds a new sample to the filter and returns the updated average.
    pub fn update(&mut self, sample: f32) -> f32 {
        if self.count < N {
            // Buffer is filling up initially
            self.buffer[self.index] = sample;
            self.sum += sample;
            self.count += 1;
        } else {
            // Buffer is full; subtract oldest sample, add new one
            self.sum -= self.buffer[self.index];
            self.buffer[self.index] = sample;
            self.sum += sample;
        }

        // Advance ring buffer index
        self.index = (self.index + 1) % N;

        // Calculate average safely
        self.sum / (self.count as f32)
    }

    /// Resets the filter state.
    pub fn reset(&mut self) {
        self.buffer = [0.0; N];
        self.index = 0;
        self.count = 0;
        self.sum = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accumulates_average_while_filling() {
        let mut ma: MovingAverage<2> = MovingAverage::new();
        assert_eq!(ma.update(2.0), 2.0); // sum=2, count=1
        assert_eq!(ma.update(4.0), 3.0); // sum=6, count=2
    }

    #[test]
    fn wraps_and_evicts_oldest_sample_once_full() {
        let mut ma: MovingAverage<3> = MovingAverage::new();
        ma.update(1.0);
        ma.update(2.0);
        ma.update(3.0); // window full: [1, 2, 3], avg = 2.0
        // Window is full; this must evict 1.0 (the oldest), not just
        // accumulate onto count=4. Window becomes [2, 3, 4], avg = 3.0 --
        // a plain running average of all 4 values would give 2.5 instead,
        // and a broken ring-buffer index would corrupt which slot gets
        // overwritten, giving yet another wrong value.
        let avg = ma.update(4.0);
        assert!((avg - 3.0).abs() < 1e-6, "expected 3.0, got {avg}");
    }

    #[test]
    fn continues_evicting_correctly_across_multiple_wraps() {
        // Exercises the ring index cycling through 0,1,2,0,1,... more than
        // once, not just the first wrap.
        let mut ma: MovingAverage<3> = MovingAverage::new();
        for v in [10.0, 20.0, 30.0, 40.0, 50.0, 60.0] {
            ma.update(v);
        }
        // Window should now hold the last 3 values: [40, 50, 60].
        assert!((ma.update(70.0) - 60.0).abs() < 1e-6); // [50,60,70] -> 60.0
    }

    #[test]
    fn reset_clears_accumulated_state() {
        let mut ma: MovingAverage<3> = MovingAverage::new();
        ma.update(10.0);
        ma.update(20.0); // sum=30, count=2 -- still mid-fill (count < N)

        ma.reset();

        // If reset() were a no-op, count would still be 2 (< N=3), so this
        // call would take the fill branch and land on (30+5)/3 ~= 11.67
        // instead of a fresh 5.0/1 = 5.0.
        let avg = ma.update(5.0);
        assert!((avg - 5.0).abs() < 1e-6, "expected 5.0, got {avg}");
    }
}