use std::f32::consts::PI;

use crate::low_pass_filter;
use crate::units::gain_from_db;

pub struct Compressor {
    envelope: f32,
    threshold: f32,
    ratio: f32,
    attack: f32,
    release: f32,
    gain: f32,
    attack_gain: f32,
    release_gain: f32,
}

impl Compressor {
    pub fn new() -> Compressor {
        Compressor {
            envelope: 0.0,
            threshold: 0.0,
            ratio: 0.0,
            attack: 0.0,
            release: 0.0,
            gain: 0.0,
            attack_gain: 0.0,
            release_gain: 0.0,
        }
    }

    pub fn update_prams(
        &mut self,
        threshold: f32,
        ratio: f32,
        attack: f32,
        release: f32,
        gain: f32,
        sample_rate: f32,
    ) {
        self.ratio = ratio;
        self.attack = attack;
        self.release = release;
        self.gain = gain_from_db(gain);
        self.threshold = gain_from_db(threshold);
        self.attack_gain = (-2.0 * PI * 1000.0 / attack / sample_rate).exp();
        self.release_gain = (-2.0 * PI * 1000.0 / release / sample_rate).exp();
    }

    //To make detector_input from stereo:
    //detector_input = (input_l + input_r).abs() * 0.5
    //Returns attenuation multiplier
    pub fn process(&mut self, detector_input: f32) -> f32 {
        // Ballistics filter and envelope generation
        let cte = if detector_input >= self.envelope {
            self.attack_gain
        } else {
            self.release_gain
        };
        let detector_input_sq = detector_input.powi(2);
        let env_sq = detector_input_sq + cte * (self.envelope - detector_input_sq);
        self.envelope = env_sq.sqrt();

        // Compressor transfer function
        if self.envelope > self.threshold {
            (self.envelope / self.threshold).powf(1.0 / self.ratio - 1.0) * self.gain
        } else {
            1.0 * self.gain
        }
    }
}
