use std::f32::consts::PI;

pub struct LowPassFilter {
    fd0: f32,
    fd1: f32,
    fd2: f32,
    fa0: f32,
    fa1: f32,
    fk: f32,
    freq: f32,
    sharp: f32,
    sample_rate: f32,
}

impl LowPassFilter {
    pub fn new(freq: f32, sharp: f32, sample_rate: f32) -> LowPassFilter {
        LowPassFilter {
            fd0: 0.0,
            fd1: 0.0,
            fd2: 0.0,
            fa0: 0.0,
            fa1: 0.0,
            fk: 0.0,
            freq,
            sharp,
            sample_rate,
        }
    }

    pub fn set_freq(&mut self, freq: f32) {
        self.freq = freq;
        self.update();
    }

    pub fn set_sharp(&mut self, sharp: f32) {
        self.sharp = sharp;
        self.update();
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        self.update();
    }

    fn update(&mut self) {
        let damp = 0.01 + self.sharp * 20.0;
        let c = 1.0 / (PI * self.freq / self.sample_rate).tan();
        self.fk = 1.0 / (1.0 + c * (c + damp));
        self.fa1 = 2.0 * (1.0 - c * c) * self.fk;
        self.fa0 = (1.0 + c * (c - damp)) * self.fk;
    }

    pub fn process(&mut self, x: f32) -> f32 {
        self.fd0 = (self.fk * x) - (self.fa1 * self.fd1) - (self.fa0 * self.fd2);
        let y = self.fd0 + self.fd1 + self.fd1 + self.fd2;
        self.fd2 = self.fd1;
        self.fd1 = self.fd0;
        y
    }
}
