use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

pub const FREQUENCIES: [f64; 10] = [
    31.0, 62.0, 125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0, 16000.0,
];

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Mode {
    #[default]
    Preset,
    Manual,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Preset {
    #[default]
    Flat,
    BassBoost,
    Vocal,
    Rock,
    Electronic,
    TrebleBoost,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub enabled: bool,
    pub mode: Mode,
    pub preset: Preset,
    pub manual_gains: [f64; 10],
    pub manual_preamp: f64,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            enabled: false,
            mode: Mode::Preset,
            preset: Preset::Flat,
            manual_gains: [0.0; 10],
            manual_preamp: 0.0,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetInfo {
    pub id: Preset,
    pub name: &'static str,

    pub gains: [f64; 10],
    pub preamp: f64,
}

impl Preset {
    pub fn info(self) -> PresetInfo {
        let (name, gains, preamp) = match self {
            Self::Flat => ("Flat", [0.0; 10], 0.0),
            Self::BassBoost => (
                "Bass boost",
                [5.0, 5.0, 3.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
                -7.0,
            ),
            Self::Vocal => (
                "Vocal",
                [-2.0, -2.0, -1.0, 0.0, 1.0, 3.0, 3.0, 1.0, 0.0, -1.0],
                -5.0,
            ),
            Self::Rock => (
                "Rock",
                [4.0, 3.0, 2.0, 0.0, -1.0, -1.0, 1.0, 3.0, 4.0, 3.0],
                -6.0,
            ),
            Self::Electronic => (
                "Electronic",
                [5.0, 4.0, 2.0, 0.0, -2.0, -1.0, 0.0, 2.0, 4.0, 4.0],
                -7.0,
            ),
            Self::TrebleBoost => (
                "Treble boost",
                [0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 2.0, 3.0, 4.0, 4.0],
                -6.0,
            ),
        };
        PresetInfo {
            id: self,
            name,

            gains,
            preamp,
        }
    }
}

pub fn presets() -> Vec<PresetInfo> {
    [
        Preset::Flat,
        Preset::BassBoost,
        Preset::Vocal,
        Preset::Rock,
        Preset::Electronic,
        Preset::TrebleBoost,
    ]
    .into_iter()
    .map(Preset::info)
    .collect()
}

impl Settings {
    pub fn validate(&self) -> Result<(), String> {
        if self
            .manual_gains
            .iter()
            .any(|gain| !gain.is_finite() || !(-12.0..=12.0).contains(gain))
        {
            return Err("EQ bands must be between -12 and +12 dB".into());
        }
        if !self.manual_preamp.is_finite() || !(-24.0..=0.0).contains(&self.manual_preamp) {
            return Err("EQ preamp must be between -24 and 0 dB".into());
        }
        Ok(())
    }
    fn curve(self) -> ([f64; 10], f64) {
        if !self.enabled {
            return ([0.0; 10], 0.0);
        }
        match self.mode {
            Mode::Preset => {
                let preset = self.preset.info();
                (preset.gains, preset.preamp)
            }
            Mode::Manual => (self.manual_gains, self.manual_preamp),
        }
    }
}

// https://www.w3.org/TR/audio-eq-cookbook/#the-different-types-of-filters

fn coefficients(frequency: f64, gain: f64, rate: f64) -> [f64; 5] {
    if gain == 0.0 || frequency >= rate * 0.49 {
        return [1.0, 0.0, 0.0, 0.0, 0.0];
    }
    let amplitude = 10_f64.powf(gain / 40.0);
    let omega = 2.0 * PI * frequency / rate;
    let alpha = omega.sin() / (2.0 * 1.4);
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
    current: [[f64; 5]; 10],
    target: [[f64; 5]; 10],
    history: Vec<[[f64; 2]; 10]>,
    gain: f64,
    target_gain: f64,
    remaining: usize,
}

impl Equalizer {
    pub fn new(rate: u32, channels: usize, settings: Settings) -> Self {
        let mut eq = Self {
            settings: Settings::default(),
            rate: rate.max(1) as f64,
            current: [[1.0, 0.0, 0.0, 0.0, 0.0]; 10],
            target: [[1.0, 0.0, 0.0, 0.0, 0.0]; 10],
            history: vec![[[0.0; 2]; 10]; channels.max(1)],
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
        let (gains, preamp) = settings.curve();

        let nyquist = self.rate * 0.5;

        self.target = std::array::from_fn(|i| {
            if FREQUENCIES[i] >= nyquist {
                [1.0, 0.0, 0.0, 0.0, 0.0]
            } else {
                coefficients(FREQUENCIES[i], gains[i], self.rate)
            }
        });

        self.target_gain = 10_f64.powf(preamp / 20.0);
        self.remaining = (self.rate * 0.02).max(1.0) as usize;
    }

    pub fn reset(&mut self) {
        self.history.fill([[0.0; 2]; 10]);
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
