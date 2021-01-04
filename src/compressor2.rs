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

pub struct Compressor2 {
    envelope: f32,
    threshold: f32,
    knee: f32,
    ratio: f32,
    attack: f32,
    release: f32,
    gain: f32,
    attack_gain: f32,
    release_gain: f32,

    slope: f32,
    cv_env: f32,
    cv_gain: f32,

    transition: f32,
    cte_env: f32,
    cte_gain: f32,
}

impl Compressor2 {
    pub fn new() -> Compressor2 {
        Compressor2 {
            envelope: 0.0,
            threshold: 0.0,
            knee: 0.0,
            ratio: 0.0,
            attack: 0.0,
            release: 0.0,
            gain: 0.0,
            attack_gain: 0.0,
            release_gain: 0.0,

            slope: 0.0,
            cv_env: 1.0,
            cv_gain: 0.0,

            transition: 0.0,
            cte_env: 0.0,
            cte_gain: 0.0,
        }
    }

    pub fn update_prams(
        &mut self,
        threshold: f32,
        knee: f32,
        transition: f32,
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
        self.threshold = threshold;
        self.knee = knee;
        self.transition = transition;
        self.attack_gain = (-2.0 * PI * 1000.0 / attack / sample_rate).exp();
        self.release_gain = (-2.0 * PI * 1000.0 / release / sample_rate).exp();

        self.slope = 1.0 / self.ratio - 1.0;
        self.cte_gain = (-2.0 * PI * 1000.0 / (self.transition) / sample_rate).exp();
    }

    //To make detector_input from stereo:
    //detector_input = (input_l + input_r).abs() * 0.5
    //Returns attenuation multiplier
    pub fn process(&mut self, detector_input: f32) -> f32 {
        //let detector_input = detector_input / self.threshold;
        // Ballistics filter and envelope generation
        let cte = if detector_input >= self.envelope {
            self.attack_gain
        } else {
            self.release_gain
        };

        //self.cte_env = cte + self.cte_gain * (self.cte_env - cte);

        self.envelope = detector_input + cte * (self.envelope - detector_input);
        let env_db = db_from_gain(self.envelope);

        let cv =
            gain_from_db(reiss(env_db, self.threshold, self.knee, self.ratio, self.slope) - env_db);

        if cv.is_finite() {
            cv
        } else {
            1.0
        }

        //self.cv_env = cv + self.cv_gain * (self.cv_env - cv);

        //self.cv_env
    }
}
