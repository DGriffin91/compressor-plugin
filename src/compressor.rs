use std::f32::consts::PI;
pub struct Compressor {
    prev_envelope: f32,
    threshold: f32,
    ratio: f32,
    attack: f32,
    release: f32,
    gain: f32,
    cte_attack: f32,
    cte_release: f32,
}

pub fn gain_from_db(decibels: f32) -> f32 {
    (10.0f32).powf(decibels * 0.05)
}

pub fn db_from_gain(gain: f32) -> f32 {
    gain.max(0.0).log(10.0) * 20.0
}

impl Compressor {
    pub fn new() -> Compressor {
        Compressor {
            prev_envelope: 0.0,
            threshold: 0.0,
            ratio: 0.0,
            attack: 0.0,
            release: 0.0,
            gain: 0.0,
            cte_attack: 0.0,
            cte_release: 0.0,
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
        self.cte_attack = (-2.0 * PI * 1000.0 / attack / sample_rate).exp();
        self.cte_release = (-2.0 * PI * 1000.0 / release / sample_rate).exp();
    }

    //To make detector_input from stereo:
    //detector_input = (input_l + input_r).abs() * 0.5
    //Returns attenuation multiplier
    pub fn process(&mut self, detector_input: f32) -> f32 {
        // Ballistics filter and envelope generation
        let cte = if detector_input >= self.prev_envelope {
            self.cte_attack
        } else {
            self.cte_release
        };
        let env = detector_input + cte * (self.prev_envelope - detector_input);
        self.prev_envelope = env;

        // Compressor transfer function
        if env <= self.threshold {
            1.0 * self.gain
        } else {
            (env / self.threshold).powf(1.0 / self.ratio - 1.0) * self.gain
        }
    }
}
