use super::parameter::Parameter;

pub struct CompressorEffectParameters {
    // The plugin's state consists of a single parameter: amplitude.
    pub threshold: Parameter,
    pub ratio: Parameter,
    pub attack: Parameter,
    pub release: Parameter,
    pub gain: Parameter,
}

use std::ops::Index;

impl Index<usize> for CompressorEffectParameters {
    type Output = Parameter;
    fn index(&self, i: usize) -> &Self::Output {
        match i {
            0 => &self.threshold,
            1 => &self.ratio,
            2 => &self.attack,
            3 => &self.release,
            4 => &self.gain,
            _ => &self.gain,
        }
    }
}

impl CompressorEffectParameters {
    pub fn len(&self) -> usize {
        5
    }
}

impl Default for CompressorEffectParameters {
    fn default() -> CompressorEffectParameters {
        CompressorEffectParameters {
            threshold: Parameter::new("Threshold", 0.0, -80.0, 12.0, |x| format!("{:.2}dB", x)),
            ratio: Parameter::new("Ratio", 4.0, 0.0, 20.0, |x| format!("{:.2}", x)),
            attack: Parameter::new("Attack", 1.0, 0.0, 300.0, |x| format!("{:.2}", x)),
            release: Parameter::new("Release", 100.0, 0.0, 1000.0, |x| format!("{:.2}", x)),
            gain: Parameter::new("Gain", 0.0, -24.0, 24.0, |x| format!("{:.2}dB", x)),
        }
    }
}
