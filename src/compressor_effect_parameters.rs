use super::parameter::Parameter;

pub struct CompressorEffectParameters {
    // The plugin's state consists of a single parameter: amplitude.
    pub threshold: Parameter,
    pub knee: Parameter,
    pub pre_smooth: Parameter,
    pub rms: Parameter,
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
            1 => &self.knee,
            2 => &self.pre_smooth,
            3 => &self.rms,
            4 => &self.ratio,
            5 => &self.attack,
            6 => &self.release,
            7 => &self.gain,
            _ => &self.gain,
        }
    }
}

impl CompressorEffectParameters {
    pub fn len(&self) -> usize {
        8
    }
}

impl Default for CompressorEffectParameters {
    fn default() -> CompressorEffectParameters {
        CompressorEffectParameters {
            threshold: Parameter::new("Threshold", 0.0, -80.0, 12.0, |x| format!("{:.2}dB", x)),
            knee: Parameter::new("Knee", 0.0, 0.0, 48.0, |x| format!("{:.2}dB", x)),
            pre_smooth: Parameter::new("PreSmooth", 5.0, 1.0, 300.0, |x| format!("{:.2}", x)),
            rms: Parameter::new("RMS", 5.0, 0.0, 100.0, |x| format!("{:.2}ms", x)),
            ratio: Parameter::new("Ratio", 4.0, 1.0, 20.0, |x| format!("{:.2}", x)),
            attack: Parameter::new("Attack", 1.0, 0.0, 300.0, |x| format!("{:.2}ms", x)),
            release: Parameter::new("Release", 100.0, 0.0, 1000.0, |x| format!("{:.2}ms", x)),
            gain: Parameter::new("Gain", 0.0, -24.0, 24.0, |x| format!("{:.2}dB", x)),
        }
    }
}
