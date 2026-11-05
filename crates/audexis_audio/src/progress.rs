#[derive(Default)]
pub(super) struct FrameProgress {
    fractional_microseconds: u64,
    fractional_milliseconds: u64,
}

impl FrameProgress {
    pub fn advance(&mut self, frames: u64, sample_rate: u64) -> (u64, u64) {
        if sample_rate == 0 {
            return (0, 0);
        }
        let numerator = u128::from(frames) * 1_000_000 + u128::from(self.fractional_microseconds);
        let microseconds = (numerator / u128::from(sample_rate)) as u64;
        self.fractional_microseconds = (numerator % u128::from(sample_rate)) as u64;
        let total = microseconds + self.fractional_milliseconds;
        self.fractional_milliseconds = total % 1000;
        (total / 1000, microseconds)
    }
}
