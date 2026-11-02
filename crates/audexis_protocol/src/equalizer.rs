use serde::{Deserialize, Serialize};

pub const FREQUENCIES: [f64; 10] = [
    31.0, 62.0, 125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0, 16000.0,
];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Bands {
    Five,
    #[default]
    Ten,
    Twenty,
}
impl Bands {
    pub fn frequencies(self) -> &'static [f64] {
        match self {
            Self::Five => &[63., 250., 1000., 4000., 16000.],
            Self::Ten => &FREQUENCIES,
            Self::Twenty => &[
                25., 35.5, 50., 71., 100., 140., 200., 280., 400., 560., 800., 1100., 1600., 2200.,
                3200., 4500., 6300., 9000., 12500., 18000.,
            ],
        }
    }
    pub fn q(self) -> f64 {
        match self {
            Self::Five => 0.67,
            Self::Ten => 1.4,
            Self::Twenty => 2.87,
        }
    }
}

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
    Acoustic,
    Classical,
    Dance,
    HipHop,
    Jazz,
    Metal,
    Pop,
    Rnb,
    SpokenWord,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    pub enabled: bool,
    pub bands: Bands,
    pub five_band_gains: [f64; 5],
    pub twenty_band_gains: [f64; 20],
    pub mode: Mode,
    pub preset: Preset,
    pub manual_gains: [f64; 10],
    pub manual_preamp: f64,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            enabled: false,
            bands: Bands::Ten,
            five_band_gains: [0.0; 5],
            twenty_band_gains: [0.0; 20],
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

// shout out gtt
impl Preset {
    pub fn info(self) -> PresetInfo {
        let (name, gains, preamp) = match self {
            Self::Flat => ("Flat", [0.0; 10], 0.0),
            Self::Acoustic => (
                "Acoustic",
                [-1.0, -0.5, 0.0, 0.5, 0.5, 1.5, 2.0, 1.5, 1.0, 0.5],
                -3.5,
            ),
            Self::Classical => (
                "Classical",
                [1.5, 1.0, 0.5, 0.0, -0.5, -0.5, 0.0, 0.5, 1.0, 1.5],
                -3.0,
            ),
            Self::Dance => (
                "Dance",
                [3.0, 3.5, 2.0, 0.0, -1.5, -0.5, 1.0, 2.0, 2.5, 1.5],
                -5.5,
            ),
            Self::HipHop => (
                "Hip-hop",
                [3.5, 3.0, 2.0, 0.0, -1.0, 0.5, 1.5, 1.0, 0.5, 0.0],
                -5.5,
            ),
            Self::Jazz => (
                "Jazz",
                [0.5, 1.5, 1.0, -0.5, -0.5, 0.5, 1.0, 1.5, 1.0, 0.5],
                -3.0,
            ),
            Self::Metal => (
                "Metal",
                [2.0, 2.5, 1.0, -1.0, -1.5, 0.5, 2.0, 1.0, 0.0, -1.0],
                -4.0,
            ),
            Self::Pop => (
                "Pop",
                [1.0, 2.0, 1.0, -0.5, -0.5, 1.0, 2.0, 1.5, 1.0, 0.5],
                -3.5,
            ),
            Self::Rnb => (
                "R&B",
                [2.5, 3.0, 1.5, 0.5, -0.5, 0.0, 1.0, 0.5, 0.0, -0.5],
                -4.5,
            ),
            Self::SpokenWord => (
                "Spoken word",
                [-6.0, -4.0, -2.0, -1.0, 0.0, 1.5, 2.5, 1.5, -0.5, -2.0],
                -4.0,
            ),
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
        Preset::Acoustic,
        Preset::Classical,
        Preset::Dance,
        Preset::Electronic,
        Preset::HipHop,
        Preset::Jazz,
        Preset::Metal,
        Preset::Pop,
        Preset::Rnb,
        Preset::Rock,
        Preset::BassBoost,
        Preset::TrebleBoost,
        Preset::Vocal,
        Preset::SpokenWord,
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
            .chain(&self.five_band_gains)
            .chain(&self.twenty_band_gains)
            .any(|gain| !gain.is_finite() || !(-12.0..=12.0).contains(gain))
        {
            return Err("EQ bands must be between -12 and +12 dB".into());
        }
        if !self.manual_preamp.is_finite() || !(-24.0..=0.0).contains(&self.manual_preamp) {
            return Err("EQ preamp must be between -24 and 0 dB".into());
        }
        Ok(())
    }
    pub fn gains(self) -> [f64; 20] {
        std::array::from_fn(|index| {
            let Some(&frequency) = self.bands.frequencies().get(index) else {
                return 0.0;
            };
            if self.mode == Mode::Preset {
                let gains = self.preset.info().gains;
                let upper = FREQUENCIES.partition_point(|value| *value < frequency);
                if upper == 0 {
                    return gains[0];
                }
                if upper == FREQUENCIES.len() {
                    return gains[9];
                }
                let fraction = (frequency / FREQUENCIES[upper - 1]).ln()
                    / (FREQUENCIES[upper] / FREQUENCIES[upper - 1]).ln();
                gains[upper - 1] + (gains[upper] - gains[upper - 1]) * fraction
            } else {
                self.manual_values()[index]
            }
        })
    }
    pub fn preamp(self) -> f64 {
        if self.mode == Mode::Preset {
            self.preset.info().preamp
        } else {
            self.manual_preamp
        }
    }
    fn manual_values(&self) -> &[f64] {
        match self.bands {
            Bands::Five => &self.five_band_gains,
            Bands::Ten => &self.manual_gains,
            Bands::Twenty => &self.twenty_band_gains,
        }
    }
    fn manual_values_mut(&mut self) -> &mut [f64] {
        match self.bands {
            Bands::Five => &mut self.five_band_gains,
            Bands::Ten => &mut self.manual_gains,
            Bands::Twenty => &mut self.twenty_band_gains,
        }
    }
    fn make_manual(&mut self) {
        if self.mode == Mode::Preset {
            let gains = self.gains();
            self.manual_preamp = self.preamp();
            let values = self.manual_values_mut();
            let count = values.len();
            values.copy_from_slice(&gains[..count]);
            self.mode = Mode::Manual;
        }
    }
    pub fn set_gain(&mut self, index: usize, gain: f64) {
        self.make_manual();
        if let Some(value) = self.manual_values_mut().get_mut(index) {
            *value = gain;
        }
    }
    pub fn set_preamp(&mut self, gain: f64) {
        self.make_manual();
        self.manual_preamp = gain;
    }
    pub fn reset_curve(&mut self) {
        self.mode = Mode::Manual;
        self.manual_values_mut().fill(0.0);
        self.manual_preamp = 0.0;
    }
}
