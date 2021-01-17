use std::f32::consts::PI;

use crate::units::*;

//x, threshold, & width units are dB
//slope is: 1.0 / ratio - 1.0 (Computed ahead of time for performance)
fn reiss(x: f32, threshold: f32, width: f32, ratio: f32, slope: f32) -> f32 {
    let x_minus_threshold = x - threshold;
    if 2.0 * (x_minus_threshold).abs() <= width {
        x + slope * (x_minus_threshold + width / 2.0).powi(2) / (2.0 * width)
    } else if 2.0 * (x_minus_threshold) > width {
        threshold + (x_minus_threshold) / ratio
    } else {
        //2.0 * (x_minus_threshold) < -width
        x
    }
}

struct DecoupledPeakDetector {
    attack: f32,
    release: f32,
    env: f32,
    env2: f32,
}

impl DecoupledPeakDetector {
    pub fn new(attack: f32, release: f32, sample_rate: f32) -> DecoupledPeakDetector {
        let mut detector = DecoupledPeakDetector {
            attack: 0.0,
            release: 0.0,
            env: 0.0,
            env2: 0.0,
        };
        detector.update(attack, release, sample_rate);
        detector
    }

    fn process(&mut self, x: f32) -> f32 {
        self.env = x.max(self.release * self.env);
        self.env2 = self.attack * self.env2 + (1.0 - self.attack) * self.env;
        self.env2
    }

    fn process_smooth(&mut self, x: f32) -> f32 {
        self.env = x.max(self.release * self.env + (1.0 - self.release) * x);
        self.env2 = self.attack * self.env2 + (1.0 - self.attack) * self.env;

        self.env = if self.env.is_finite() { self.env } else { 1.0 };
        self.env2 = if self.env2.is_finite() {
            self.env2
        } else {
            1.0
        };
        self.env2
    }

    fn update(&mut self, attack: f32, release: f32, sample_rate: f32) {
        self.attack = (-1.0 * PI * 1000.0 / attack / sample_rate).exp();
        self.release = (-1.0 * PI * 1000.0 / release / sample_rate).exp();
    }
}

pub struct Compressor2 {
    envelope: f32,
    threshold: f32,
    knee: f32,
    ratio: f32,
    gain: f32,

    slope: f32,

    pre_smooth_gain: f32,
    decoupled_peak_detector: DecoupledPeakDetector,
    rms_size: f32,
    rms: AccumulatingRMS,
}

impl Compressor2 {
    pub fn new() -> Compressor2 {
        Compressor2 {
            envelope: 0.0,
            threshold: 0.0,
            knee: 0.0,
            ratio: 0.0,
            gain: 0.0,

            slope: 0.0,

            pre_smooth_gain: 0.0,
            decoupled_peak_detector: DecoupledPeakDetector::new(0.0, 0.0, 48000.0),

            rms_size: 0.0,
            rms: AccumulatingRMS::new(48000, 5.0, 192000),
        }
    }

    pub fn update_prams(
        &mut self,
        threshold: f32,
        knee: f32,
        pre_smooth: f32,
        rms_size: f32,
        ratio: f32,
        attack: f32,
        release: f32,
        gain: f32,
        sample_rate: f32,
    ) {
        //TODO don't update here unnecessarily
        self.ratio = ratio;
        self.gain = gain_from_db(gain);
        self.threshold = threshold;
        self.knee = knee;

        self.slope = 1.0 / self.ratio - 1.0;
        self.pre_smooth_gain = (-2.0 * PI * 1000.0 / pre_smooth / sample_rate).exp();
        self.decoupled_peak_detector
            .update(attack, release, sample_rate);

        if rms_size != self.rms_size {
            self.rms_size = rms_size;
            self.rms.resize(sample_rate as usize, self.rms_size)
        }
    }

    //To make detector_input from stereo:
    //detector_input = (input_l + input_r).abs() * 0.5
    //Returns attenuation multiplier
    pub fn process(&mut self, detector_input: f32) -> f32 {
        let mut detector_input = detector_input;
        if self.rms_size >= 1.0 {
            detector_input = self.rms.process(detector_input);
        }

        self.envelope = detector_input + self.pre_smooth_gain * (self.envelope - detector_input);

        self.envelope = if self.envelope.is_finite() {
            self.envelope
        } else {
            1.0
        };

        let db = db_from_gain(self.envelope);

        let mut cv = db - reiss(db, self.threshold, self.knee, self.ratio, self.slope);
        cv = gain_from_db(-self.decoupled_peak_detector.process_smooth(cv));
        if cv.is_finite() {
            cv
        } else {
            1.0
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_ring() {
        let mut ring = VariableRingBuf::new(10, 15);
        for i in 0..35 {
            ring.push(i as f32)
        }
        for i in 0..ring.size() {
            dbg!(ring.get(i));
        }
        println!("oldest {}", ring.oldest());
        println!("");
        ring.resize(15);
        for i in 100..200 {
            ring.push(i as f32)
        }
        println!("");
        for i in 0..ring.size() {
            dbg!(ring.get(i));
        }
        println!("oldest {}", ring.oldest());
        ring.resize(8);
        for i in 200..300 {
            ring.push(i as f32)
        }
        println!("");
        for i in 0..ring.size() {
            dbg!(ring.get(i));
        }
        println!("oldest {}", ring.oldest());
    }
}
