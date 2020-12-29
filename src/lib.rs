//! Compressor baseview imgui plugin
//TODO Use transfer and smooth https://github.com/RustAudio/vst-rs/blob/master/examples/transfer_and_smooth.rs
//TODO manually draw graph

#[macro_use]
extern crate vst;

mod compressor;
mod compressor_effect_parameters;
mod editor;
mod low_pass_filter;
mod parameter;

use compressor::{db_from_gain, Compressor};
use compressor_effect_parameters::CompressorEffectParameters;
use editor::{CompressorPluginEditor, ConsumerDump, EditorOnlyState, EditorState};

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
    sample_rate: f32,
    compressor: Compressor,
    cv_producer: Producer<f32>,
    amplitude_producer: Producer<f32>,
    cv_low_pass_filter: low_pass_filter::LowPassFilter,
    amplitude_low_pass_filter: low_pass_filter::LowPassFilter,
    data_i: u32,
    block_size: i64,
}

impl Default for CompressorPlugin {
    fn default() -> Self {
        let params = Arc::new(CompressorEffectParameters::default());

        let cv_ring = RingBuffer::<f32>::new(DATA_SIZE);
        let amplitude_ring = RingBuffer::<f32>::new(DATA_SIZE);
        let (cv_producer, cv_consumer) = cv_ring.split();
        let (amplitude_producer, amplitude_consumer) = amplitude_ring.split();
        Self {
            params: params.clone(),
            sample_rate: 44100.0,
            block_size: 128,
            cv_producer,
            amplitude_producer,
            editor: Some(CompressorPluginEditor {
                runner: None,
                state: Arc::new(EditorState {
                    params: params.clone(),
                    sample_rate: AtomicFloat::new(44100.0),
                    editor_only: Arc::new(Mutex::new(EditorOnlyState {
                        cv_data: ConsumerDump::new(cv_consumer, DATA_SIZE),
                        amplitude_data: ConsumerDump::new(amplitude_consumer, DATA_SIZE),
                    })),
                }),
            }),
            compressor: Compressor::new(),
            cv_low_pass_filter: low_pass_filter::LowPassFilter::new(),
            amplitude_low_pass_filter: low_pass_filter::LowPassFilter::new(),
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
            name: "IMGUI Compressor in Rust".to_string(),
            vendor: "DGriffin".to_string(),
            unique_id: 243123123,
            version: 1,
            inputs: 2,
            outputs: 2,
            // This `parameters` bit is important; without it, none of our
            // parameters will be shown!
            parameters: 5,
            category: Category::Effect,
            ..Default::default()
        }
    }

    fn set_sample_rate(&mut self, rate: f32) {
        let rate = rate as f32;
        self.sample_rate = rate;
        self.cv_low_pass_filter.set_sample_rate(rate);
        self.amplitude_low_pass_filter.set_sample_rate(rate);
        if let Some(editor) = &self.editor {
            editor.state.sample_rate.set(rate);
        }
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
            self.params.ratio.get(),
            self.params.attack.get(),
            self.params.release.get(),
            self.params.gain.get(),
            self.sample_rate,
        );
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

            *output_l = *input_l * cv;
            *output_r = *input_r * cv;

            let cv_filtered = self.cv_low_pass_filter.process(cv);
            let amp_filtered = self.amplitude_low_pass_filter.process(input_l + input_r);
            if self.data_i >= 96 {
                if !self.cv_producer.is_full() {
                    self.cv_producer.push(cv_filtered).unwrap();
                }
                if !self.amplitude_producer.is_full() {
                    self.amplitude_producer.push(amp_filtered).unwrap();
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
