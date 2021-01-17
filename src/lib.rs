//! Compressor baseview imgui plugin
//TODO Use transfer and smooth https://github.com/RustAudio/vst-rs/blob/master/examples/transfer_and_smooth.rs
//TODO manually draw graph

#[macro_use]
extern crate vst;

pub mod compressor;
pub mod compressor2;
mod compressor_effect_parameters;
mod editor;
pub mod low_pass_filter;
mod parameter;
pub mod units;

use compressor2::Compressor2;
use compressor_effect_parameters::CompressorEffectParameters;
use editor::{CompressorPluginEditor, EditorOnlyState, EditorState};
use units::{gain_from_db, ConsumerDump, Sample};

use vst::buffer::AudioBuffer;
use vst::editor::Editor;
use vst::plugin::{Category, Info, Plugin, PluginParameters};

use std::sync::{Arc, Mutex};

use ringbuf::{Producer, RingBuffer};

use vst::util::AtomicFloat;

const DATA_SIZE: usize = 3000;

struct CompressorPlugin {
    params: Arc<CompressorEffectParameters>,
    editor: Option<CompressorPluginEditor>,
    time: Arc<AtomicFloat>,
    sample_rate: Arc<AtomicFloat>,
    compressor: Compressor2,
    sample_producer: Producer<Sample>,
    cv_lpf: low_pass_filter::LowPassFilter,
    amplitude_lpf_l: low_pass_filter::LowPassFilter,
    amplitude_lpf_r: low_pass_filter::LowPassFilter,
    amplitude_rms_l: units::AccumulatingRMS,
    amplitude_rms_r: units::AccumulatingRMS,
    data_i: u32,
    block_size: i64,
}

impl Default for CompressorPlugin {
    fn default() -> Self {
        let params = Arc::new(CompressorEffectParameters::default());
        let time = Arc::new(AtomicFloat::new(0.0));
        let sample_rate = Arc::new(AtomicFloat::new(44100.0));

        let sample_ring = RingBuffer::<Sample>::new(DATA_SIZE);
        let (sample_producer, sample_consumer) = sample_ring.split();
        Self {
            params: params.clone(),
            sample_rate: sample_rate.clone(),
            block_size: 128,
            sample_producer,
            time: time.clone(),
            editor: Some(CompressorPluginEditor {
                is_open: false,
                state: Arc::new(EditorState {
                    params: params.clone(),
                    sample_rate: sample_rate.clone(),
                    time: time.clone(),
                    editor_only: Arc::new(Mutex::new(EditorOnlyState {
                        sample_data: ConsumerDump::new(sample_consumer, DATA_SIZE),
                        recent_peak_l: 0.0,
                        recent_peak_r: 0.0,
                        recent_peak_cv: 0.0,
                    })),
                }),
            }),
            compressor: Compressor2::new(),
            cv_lpf: low_pass_filter::LowPassFilter::new(50.0, 0.2, 44100.0),
            amplitude_lpf_l: low_pass_filter::LowPassFilter::new(50.0, 0.2, 44100.0),
            amplitude_lpf_r: low_pass_filter::LowPassFilter::new(50.0, 0.2, 44100.0),
            amplitude_rms_l: units::AccumulatingRMS::new(44100, 5.0, 192000),
            amplitude_rms_r: units::AccumulatingRMS::new(44100, 5.0, 192000),
            data_i: 0,
        }
    }
}

fn setup_logging() {
    let log_folder = ::dirs::home_dir().unwrap().join("tmp");

    let _ = ::std::fs::create_dir(log_folder.clone());

    let log_file = ::std::fs::File::create(log_folder.join("IMGUIBaseviewCompressor.log")).unwrap();

    let log_config = ::simplelog::ConfigBuilder::new()
        .set_time_to_local(true)
        .build();

    let _ = ::simplelog::WriteLogger::init(simplelog::LevelFilter::Info, log_config, log_file);

    ::log_panics::init();

    ::log::info!("init");
}

impl Plugin for CompressorPlugin {
    fn get_info(&self) -> Info {
        Info {
            name: "IMGUI Compressor in Rust 0.1".to_string(),
            vendor: "DGriffin".to_string(),
            unique_id: 243123123,
            version: 2,
            inputs: 2,
            outputs: 2,
            // This `parameters` bit is important; without it, none of our
            // parameters will be shown!
            parameters: self.params.len() as i32,
            category: Category::Effect,
            ..Default::default()
        }
    }

    fn set_sample_rate(&mut self, rate: f32) {
        self.sample_rate.set(rate);
        self.cv_lpf.set_sample_rate(rate);
        self.amplitude_lpf_l.set_sample_rate(rate);
        self.amplitude_lpf_r.set_sample_rate(rate);
        self.amplitude_rms_l
            .resize(rate as usize, self.params.rms.get());
        self.amplitude_rms_r
            .resize(rate as usize, self.params.rms.get());
    }

    fn set_block_size(&mut self, block_size: i64) {
        self.block_size = block_size;
    }

    fn init(&mut self) {
        setup_logging()
    }

    fn get_editor(&mut self) -> Option<Box<dyn Editor>> {
        if let Some(editor) = self.editor.take() {
            Some(Box::new(editor) as Box<dyn Editor>)
        } else {
            None
        }
    }

    // Here is where the bulk of our audio processing code goes.
    fn process(&mut self, buffer: &mut AudioBuffer<f32>) {
        self.compressor.update_prams(
            self.params.threshold.get(),
            self.params.knee.get(),
            self.params.pre_smooth.get(),
            self.params.rms.get(),
            self.params.ratio.get(),
            self.params.attack.get(),
            self.params.release.get(),
            self.params.gain.get(),
            self.sample_rate.get(),
        );

        self.time
            .set(self.time.get() + (1.0 / self.sample_rate.get()) * self.block_size as f32);

        let gain = gain_from_db(self.params.gain.get());

        let (inputs, outputs) = buffer.split();
        let (inputs_left, inputs_right) = inputs.split_at(1);
        let (mut outputs_left, mut outputs_right) = outputs.split_at_mut(1);

        let inputs_stereo = inputs_left[0].iter().zip(inputs_right[0].iter());
        let outputs_stereo = outputs_left[0].iter_mut().zip(outputs_right[0].iter_mut());

        for (input_pair, output_pair) in inputs_stereo.zip(outputs_stereo) {
            let (input_l, input_r) = input_pair;
            let (output_l, output_r) = output_pair;

            let detector_input = (input_l + input_r).abs() * 0.5;

            let cv = self.compressor.process(detector_input);

            *output_l = *input_l * cv * gain;
            *output_r = *input_r * cv * gain;

            let cv_filtered = self.cv_lpf.process(cv);

            let amp_filtered_l = self.amplitude_lpf_l.process(*input_l);
            let amp_filtered_r = self.amplitude_lpf_r.process(*input_r);

            let amp_rms_l = self.amplitude_rms_l.process(*input_l);
            let amp_rms_r = self.amplitude_rms_r.process(*input_r);
            if self.data_i >= (self.sample_rate.get() as u32) / 512 {
                if !self.sample_producer.is_full() {
                    self.sample_producer
                        .push(Sample {
                            left: amp_filtered_l,
                            right: amp_filtered_r,
                            left_rms: amp_rms_l,
                            right_rms: amp_rms_r,
                            cv: cv_filtered,
                        })
                        .unwrap_or(());
                }
                self.data_i = 0;
            }
            self.data_i += 1;
        }
    }

    // Return the parameter object. This method can be omitted if the
    // plugin has no parameters.
    fn get_parameter_object(&mut self) -> Arc<dyn PluginParameters> {
        Arc::clone(&self.params) as Arc<dyn PluginParameters>
    }
}

impl PluginParameters for CompressorEffectParameters {
    // the `get_parameter` function reads the value of a parameter.
    fn get_parameter(&self, index: i32) -> f32 {
        if (index as usize) < self.len() {
            self[index as usize].get_normalized()
        } else {
            0.0
        }
    }

    // the `set_parameter` function sets the value of a parameter.
    fn set_parameter(&self, index: i32, val: f32) {
        #[allow(clippy::single_match)]
        if (index as usize) < self.len() {
            self[index as usize].set_normalized(val);
        }
    }

    // This is what will display underneath our control.  We can
    // format it into a string that makes the most since.

    fn get_parameter_text(&self, index: i32) -> String {
        if (index as usize) < self.len() {
            self[index as usize].get_display()
        } else {
            "".to_string()
        }
    }

    // This shows the control's name.
    fn get_parameter_name(&self, index: i32) -> String {
        if (index as usize) < self.len() {
            self[index as usize].get_name()
        } else {
            "".to_string()
        }
    }
}

plugin_main!(CompressorPlugin);
