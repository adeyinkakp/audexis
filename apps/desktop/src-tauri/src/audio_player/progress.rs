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

#[cfg(test)]
mod tests {
    use super::FrameProgress;

    #[test]
    fn callback_sizes_do_not_change_elapsed_time_over_ten_minutes() {
        for rate in [44_100, 48_000, 96_000, 192_000] {
            for buffer in [128, 256, 512, 1024] {
                let mut clock = FrameProgress::default();
                let mut remaining = rate * 600;
                let (mut ms, mut us) = (0, 0);
                while remaining > 0 {
                    let frames = remaining.min(buffer);
                    let elapsed = clock.advance(frames, rate);
                    ms += elapsed.0;
                    us += elapsed.1;
                    remaining -= frames;
                }
                assert_eq!(ms, 600_000, "rate {rate}, buffer {buffer}");
                assert_eq!(us, 600_000_000, "rate {rate}, buffer {buffer}");
            }
        }
    }

    #[test]
    fn pauses_and_empty_buffers_preserve_fractional_progress() {
        let mut clock = FrameProgress::default();
        assert_eq!(clock.advance(44, 44_100).0, 0);
        assert_eq!(clock.advance(0, 44_100), (0, 0));
        assert_eq!(clock.advance(1, 44_100).0, 1);
    }

    #[test]
    fn flush_starts_a_new_clock_at_the_seek_position() {
        let mut clock = FrameProgress::default();
        clock.advance(44, 44_100);
        clock = FrameProgress::default();
        let mut position_ms = 5250;
        position_ms += clock.advance(1, 44_100).0;
        assert_eq!(position_ms, 5250);
        position_ms += clock.advance(44_099, 44_100).0;
        assert_eq!(position_ms, 6250);
    }
}
