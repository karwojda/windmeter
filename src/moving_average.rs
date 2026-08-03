
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