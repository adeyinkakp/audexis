pub use audexis_protocol::equalizer::*;
use std::f64::consts::PI;

// https://www.w3.org/TR/audio-eq-cookbook/#the-different-types-of-filters

fn coefficients(frequency: f64, gain: f64, rate: f64, q: f64) -> [f64; 5] {
    if gain == 0.0 || frequency >= rate * 0.49 {
        return [1.0, 0.0, 0.0, 0.0, 0.0];
    }
    let amplitude = 10_f64.powf(gain / 40.0);
    let omega = 2.0 * PI * frequency / rate;
    let alpha = omega.sin() / (2.0 * q);
    let a0 = 1.0 + alpha / amplitude;
    [
        (1.0 + alpha * amplitude) / a0,
        -2.0 * omega.cos() / a0,
        (1.0 - alpha * amplitude) / a0,
        -2.0 * omega.cos() / a0,
        (1.0 - alpha / amplitude) / a0,
    ]
}

pub struct Equalizer {
    settings: Settings,
    rate: f64,
    current: [[f64; 5]; 20],
    target: [[f64; 5]; 20],
    history: Vec<[[f64; 2]; 20]>,
    gain: f64,
    target_gain: f64,
    remaining: usize,
}

impl Equalizer {
    pub fn new(rate: u32, channels: usize, settings: Settings) -> Self {
        let mut eq = Self {
            settings: Settings::default(),
            rate: rate.max(1) as f64,
            current: [[1.0, 0.0, 0.0, 0.0, 0.0]; 20],
            target: [[1.0, 0.0, 0.0, 0.0, 0.0]; 20],
            history: vec![[[0.0; 2]; 20]; channels.max(1)],
            gain: 1.0,
            target_gain: 1.0,
            remaining: 0,
        };
        eq.update(settings);
        eq.current = eq.target;
        eq.gain = eq.target_gain;
        eq.remaining = 0;
        eq
    }

    pub fn update(&mut self, settings: Settings) {
        if settings == self.settings || settings.validate().is_err() {
            return;
        }
        self.settings = settings;
        let gains = if settings.enabled {
            settings.gains()
        } else {
            [0.0; 20]
        };
        let preamp = if settings.enabled {
            settings.preamp()
        } else {
            0.0
        };
        let frequencies = settings.bands.frequencies();

        self.target = std::array::from_fn(|i| {
            if i >= frequencies.len() {
                [1.0, 0.0, 0.0, 0.0, 0.0]
            } else {
                coefficients(frequencies[i], gains[i], self.rate, settings.bands.q())
            }
        });

        self.target_gain = 10_f64.powf(preamp / 20.0);
        self.remaining = (self.rate * 0.02).max(1.0) as usize;
    }

    pub fn reset(&mut self) {
        self.history.fill([[0.0; 2]; 20]);
    }

    pub fn process(&mut self, samples: &mut [f32]) {
        if !self.settings.enabled && self.remaining == 0 {
            return;
        }
        for frame in samples.chunks_mut(self.history.len()) {
            if self.remaining > 0 {
                let fraction = 1.0 / self.remaining as f64;
                for (current, target) in self.current.iter_mut().zip(&self.target) {
                    for (value, goal) in current.iter_mut().zip(target) {
                        *value += (goal - *value) * fraction;
                    }
                }
                self.gain += (self.target_gain - self.gain) * fraction;
                self.remaining -= 1;
                if self.remaining == 0 && !self.settings.enabled {
                    self.reset();
                }
            }
            for (channel, sample) in frame.iter_mut().enumerate() {
                let mut value = f64::from(*sample) * self.gain;
                for (c, h) in self.current.iter().zip(&mut self.history[channel]) {
                    let output = c[0] * value + h[0];
                    h[0] = c[1] * value - c[3] * output + h[1];
                    h[1] = c[2] * value - c[4] * output;
                    value = output;
                }
                *sample = value as f32;
            }
        }
    }
}
