use std::sync::{Arc, Mutex};

use ringbuf::Consumer;

pub fn db_to_lin(decibels: f32) -> f32 {
    (10.0f32).powf(decibels * 0.05)
}

pub fn lin_to_db(gain: f32) -> f32 {
    gain.max(0.0).log(10.0) * 20.0
}

pub fn to_range(bottom: f32, top: f32, x: f32) -> f32 {
    x * (top - bottom) + bottom
}

pub fn from_range(bottom: f32, top: f32, x: f32) -> f32 {
    (x - bottom) / (top - bottom)
}

pub fn sign(a: f32, b: f32) -> f32 {
    if b < 0.0 {
        -a
    } else {
        a
    }
}

pub struct VariableRingBuffer {
    buffer: Vec<f32>,
    position: usize,
    size: usize,
}

impl VariableRingBuffer {
    pub fn new(init_size: usize, max_size: usize) -> VariableRingBuffer {
        VariableRingBuffer {
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
    buffer: VariableRingBuffer,
    rms: f32,
}

impl AccumulatingRMS {
    pub fn new(sample_rate: usize, rms_size_ms: f32, rms_max_size_samp: usize) -> AccumulatingRMS {
        AccumulatingRMS {
            buffer: VariableRingBuffer::new(
                ((sample_rate as f32) * (rms_size_ms / 1000.0)) as usize,
                rms_max_size_samp,
            ),
            rms: 0.0,
        }
    }
    pub fn resize(&mut self, sample_rate: usize, rms_size_ms: f32) {
        let new_size = (((sample_rate as f32) * (rms_size_ms / 1000.0)) as usize).max(1);
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
        let res = (self.rms / self.buffer.size() as f32).sqrt();

        if res.is_nan() || res.is_infinite() {
            0.0
        } else {
            res
        }
    }
}

//find a better name?
pub struct ConsumerDump<T> {
    pub data: Vec<T>,
    consumer: Arc<Mutex<Consumer<T>>>,
    max_size: usize,
}

impl<T> ConsumerDump<T> {
    pub fn new(consumer: Arc<Mutex<Consumer<T>>>, max_size: usize) -> ConsumerDump<T> {
        ConsumerDump {
            data: Vec::new(),
            consumer,
            max_size,
        }
    }

    pub fn consume(&mut self) {
        {
            let mut consumer = self.consumer.lock().unwrap();
            for _ in 0..consumer.len() {
                if let Some(n) = consumer.pop() {
                    self.data.push(n);
                } else {
                    break;
                }
            }
        }
        self.trim_data()
    }

    pub fn set_max_size(&mut self, max_size: usize) {
        self.max_size = max_size;
        self.trim_data();
    }

    pub fn trim_data(&mut self) {
        //Trims from the start of the vec
        let data_len = self.data.len();
        if data_len > self.max_size {
            self.data.drain(0..(data_len - self.max_size).max(0));
        }
    }
}
