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

struct VariableRingBuf {
    buffer: Vec<f32>,
    position: usize,
    size: usize,
}

impl VariableRingBuf {
    fn new(init_size: usize, max_size: usize) -> VariableRingBuf {
        VariableRingBuf {
            buffer: vec![0.0; max_size],
            position: 0,
            size: init_size,
        }
    }

    fn push(&mut self, value: f32) {
        self.buffer[self.position] = value;
        self.position = (self.position + 1) % self.size;
    }

    fn oldest(&self) -> f32 {
        self.buffer[self.position]
    }

    fn get(&self, index: usize) -> f32 {
        let pos = self.position + index;
        if pos > self.size - 1 {
            self.buffer[pos - self.size]
        } else {
            self.buffer[pos]
        }
    }

    fn size(&self) -> usize {
        self.size
    }

    fn resize(&mut self, new_size: usize) {
        self.size = new_size.min(self.buffer.len());
        self.position = 0;
        for i in self.buffer.iter_mut() {
            *i = 0.0;
        }
    }
}

struct DecoupledPeakDetector {
    attack_input: f32,
    release_input: f32,
    attack: f32,
    release: f32,
    env: f32,
    env2: f32,
    sample_rate: f32,
}

impl DecoupledPeakDetector {
    pub fn new(attack: f32, release: f32, sample_rate: f32) -> DecoupledPeakDetector {
        let mut detector = DecoupledPeakDetector {
            attack_input: attack,
            release_input: release,
            attack: 0.0,
            release: 0.0,
            env: 0.0,
            env2: 0.0,
            sample_rate,
        };
        detector.update();
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

    fn update(&mut self) {
        self.attack = (-1.0 * PI * 1000.0 / self.attack_input / self.sample_rate).exp();
        self.release = (-1.0 * PI * 1000.0 / self.release_input / self.sample_rate).exp();
    }

    fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        self.update();
    }

    fn set_attack(&mut self, attack: f32) {
        self.attack_input = attack;
        self.update();
    }

    fn set_release(&mut self, release: f32) {
        self.release_input = release;
        self.update();
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

    pre_smooth: f32,
    cte_env: f32,
    pre_smooth_gain: f32,
    decoupled_peak_detector: DecoupledPeakDetector,
    rms_size: usize,
    rms_buffer: VariableRingBuf,
    running_rms: f32,
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

            pre_smooth: 0.0,
            cte_env: 0.0,
            pre_smooth_gain: 0.0,
            decoupled_peak_detector: DecoupledPeakDetector::new(0.0, 0.0, 48000.0),

            rms_size: 0,
            rms_buffer: VariableRingBuf::new(10, 1000000),

            running_rms: 0.0,
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
        self.pre_smooth = pre_smooth;

        self.attack = attack;
        self.attack_gain = (-2.0 * PI * 1000.0 / attack / sample_rate).exp(); //UNUSED

        self.release = release;
        self.release_gain = (-2.0 * PI * 1000.0 / release / sample_rate).exp(); //UNUSED

        self.slope = 1.0 / self.ratio - 1.0;
        self.pre_smooth_gain = (-2.0 * PI * 1000.0 / (self.pre_smooth) / sample_rate).exp();
        self.decoupled_peak_detector.set_attack(attack);
        self.decoupled_peak_detector.set_release(release);
        self.decoupled_peak_detector.set_sample_rate(sample_rate);

        let rms_size = rms_size as usize;

        if rms_size != self.rms_size {
            self.rms_size = rms_size;
            if self.rms_size >= 1 {
                self.rms_buffer
                    .resize((sample_rate * ((self.rms_size as f32) / 1000.0)) as usize);
                self.running_rms = 0.0;
            }
        }
    }

    //To make detector_input from stereo:
    //detector_input = (input_l + input_r).abs() * 0.5
    //Returns attenuation multiplier
    pub fn process(&mut self, detector_input: f32) -> f32 {
        if false {
            // Ballistics filter and envelope generation
            let cte = if detector_input >= self.envelope {
                self.attack_gain
            } else {
                self.release_gain
            };

            self.envelope = detector_input + cte * (self.envelope - detector_input);
            let env_db = db_from_gain(self.envelope);

            let cv = gain_from_db(
                reiss(env_db, self.threshold, self.knee, self.ratio, self.slope) - env_db,
            );

            if cv.is_finite() {
                cv
            } else {
                1.0
            }
        } else {
            let mut detector_input = detector_input;
            if self.rms_size >= 1 {
                let new_rms_sample = detector_input.powi(2);
                //remove the oldest rms value, add new one
                self.running_rms = self.running_rms - self.rms_buffer.oldest() + new_rms_sample;
                self.rms_buffer.push(new_rms_sample);
                //let mut rms = 0.0;
                //for i in 0..self.rms_buffer.len() {
                //    let multiplier = 0.5
                //        * (1.0
                //            - (2.0 * PI * (i as f32) / (self.rms_buffer.len() - 1) as f32).cos());
                //    rms += (multiplier * self.rms_buffer.get(i)).powi(2);
                //}
                detector_input = (self.running_rms / self.rms_buffer.size() as f32).sqrt();
            }

            self.envelope =
                detector_input + self.pre_smooth_gain * (self.envelope - detector_input);

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
