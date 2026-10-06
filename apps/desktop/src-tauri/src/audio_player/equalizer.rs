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
        self.target = std::array::from_fn(|i| coefficients(FREQUENCIES[i], gains[i], self.rate));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bypass_and_flat_preserve_samples_exactly() {
        for enabled in [false, true] {
            let input = vec![0.1, -0.4, 0.8, 0.0, -0.9, 0.5];
            let mut output = input.clone();
            Equalizer::new(
                48000,
                2,
                Settings {
                    enabled,
                    ..Settings::default()
                },
            )
            .process(&mut output);
            assert_eq!(input, output);
        }
    }

    #[test]
    fn band_boost_matches_gain_and_does_not_leak_between_channels() {
        let mut settings = Settings {
            enabled: true,
            mode: Mode::Manual,
            ..Settings::default()
        };
        settings.manual_gains[5] = 6.0;
        let mut samples = Vec::new();
        for n in 0..48000 {
            samples.extend([
                (2.0 * PI * 1000.0 * n as f64 / 48000.0).sin() as f32 * 0.1,
                0.0,
            ]);
        }
        Equalizer::new(48000, 2, settings).process(&mut samples);
        let rms = (samples[48000..]
            .as_chunks::<2>()
            .0
            .iter()
            .map(|f| f64::from(f[0]).powi(2))
            .sum::<f64>()
            / 24000.0)
            .sqrt();
        assert!((20.0 * (rms / (0.1 / 2_f64.sqrt())).log10() - 6.0).abs() < 0.05);
        assert!(samples.as_chunks::<2>().0.iter().all(|f| f[1] == 0.0));
    }

    #[test]
    fn settings_round_trip_keeps_manual_curve_while_using_a_preset() {
        let settings = Settings {
            enabled: true,
            preset: Preset::Rock,
            manual_gains: [2.5; 10],
            manual_preamp: -8.0,
            ..Settings::default()
        };
        let loaded: Settings =
            serde_json::from_str(&serde_json::to_string(&settings).unwrap()).unwrap();
        assert_eq!(loaded, settings);
        assert_eq!(loaded.curve().0, Preset::Rock.info().gains);
        assert_eq!(
            Settings {
                mode: Mode::Manual,
                ..loaded
            }
            .curve(),
            ([2.5; 10], -8.0)
        );
    }

    #[test]
    fn validation_rejects_invalid_values() {
        for bad in [f64::NAN, f64::INFINITY, -13.0, 13.0] {
            assert!(Settings {
                manual_gains: [bad; 10],
                ..Settings::default()
            }
            .validate()
            .is_err());
        }
        assert!(Settings {
            manual_preamp: 1.0,
            ..Settings::default()
        }
        .validate()
        .is_err());
    }

    #[test]
    fn changes_are_smoothed_and_low_sample_rates_remain_finite() {
        for rate in [8000, 22050, 44100, 48000, 96000] {
            let mut eq = Equalizer::new(rate, 2, Settings::default());
            eq.update(Settings {
                enabled: true,
                mode: Mode::Manual,
                manual_gains: [12.0; 10],
                manual_preamp: -24.0,
                ..Settings::default()
            });
            let mut samples = vec![0.05; rate as usize * 2];
            eq.process(&mut samples);
            assert!((samples[0] - 0.05).abs() < 0.01);
            assert!(samples.iter().all(|s| s.is_finite()));
            eq.update(Settings::default());
            eq.process(&mut samples);
            let mut bypass = [0.1, -0.1];
            eq.process(&mut bypass);
            assert_eq!(bypass, [0.1, -0.1]);
        }
    }
}
