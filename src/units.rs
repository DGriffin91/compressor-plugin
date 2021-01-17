pub fn gain_from_db(decibels: f32) -> f32 {
    (10.0f32).powf(decibels * 0.05)
}

pub fn db_from_gain(gain: f32) -> f32 {
    gain.max(0.0).log(10.0) * 20.0
}

pub fn to_range(bottom: f32, top: f32, x: f32) -> f32 {
    x * (top - bottom) + bottom
}

pub fn from_range(bottom: f32, top: f32, x: f32) -> f32 {
    (x - bottom) / (top - bottom)
}

pub fn mix(x: f32, y: f32, a: f32) -> f32 {
    x * (1.0 - a) + y * a
}

pub fn clamp(x: f32, min_v: f32, max_v: f32) -> f32 {
    (x).min(max_v).max(min_v)
}

pub fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    // Scale, bias and saturate x to 0..1 range
    let x = clamp((x - edge0) / (edge1 - edge0), 0.0, 1.0);
    // Evaluate polynomial
    x * x * (3.0 - 2.0 * x)
}

pub fn sign(a: f32, b: f32) -> f32 {
    if b < 0.0 {
        -a
    } else {
        a
    }
}

pub struct VariableRingBuf {
    buffer: Vec<f32>,
    position: usize,
    size: usize,
}

impl VariableRingBuf {
    pub fn new(init_size: usize, max_size: usize) -> VariableRingBuf {
        VariableRingBuf {
            buffer: vec![0.0; max_size],
            position: 0,
            size: init_size,
        }
    }

    pub fn push(&mut self, value: f32) {
        self.buffer[self.position] = value;
        self.position = (self.position + 1) % self.size;
    }

    pub fn oldest(&self) -> f32 {
        self.buffer[self.position]
    }

    pub fn get(&self, index: usize) -> f32 {
        let pos = self.position + index;
        if pos > self.size - 1 {
            self.buffer[pos - self.size]
        } else {
            self.buffer[pos]
        }
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn resize(&mut self, new_size: usize) {
        self.size = new_size.min(self.buffer.len());
        self.position = 0;
        for i in self.buffer.iter_mut() {
            *i = 0.0;
        }
    }
}
pub struct AccumulatingRMS {
    buffer: VariableRingBuf,
    rms: f32,
}

impl AccumulatingRMS {
    pub fn new(sample_rate: usize, rms_size_ms: f32, rms_max_size_samp: usize) -> AccumulatingRMS {
        AccumulatingRMS {
            buffer: VariableRingBuf::new(
                ((sample_rate as f32) * (rms_size_ms / 1000.0)) as usize,
                rms_max_size_samp,
            ),
            rms: 0.0,
        }
    }
    pub fn resize(&mut self, sample_rate: usize, rms_size_ms: f32) {
        let new_size = ((sample_rate as f32) * (rms_size_ms / 1000.0)) as usize;
        if new_size != self.buffer.size() {
            self.buffer.resize(new_size);
            self.rms = 0.0;
        }
    }
    pub fn process(&mut self, value: f32) -> f32 {
        let new_rms_sample = value.powi(2);

        //remove the oldest rms value, add new one
        self.rms += -self.buffer.oldest() + new_rms_sample;
        self.buffer.push(new_rms_sample);
        (self.rms / self.buffer.size() as f32).sqrt()
    }
}
pub struct Sample {
    pub left: f32,
    pub right: f32,
    pub left_rms: f32,
    pub right_rms: f32,
    pub cv: f32,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_from_range() {
        let value: f32 = -40.0;
        let result = from_range(-36.0, 3.0, value.max(-36.0).min(3.0));
        dbg!(result);
    }
}
