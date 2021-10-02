#![allow(incomplete_features)]
#![feature(generic_associated_types)]
#![feature(min_specialization)]

pub mod compressor;
mod editor;
pub mod units;

use basic_audio_filters::second_order_iir::{IIR2Coefficients, IIR2};
use compressor::Compressor;
use editor::{editor, EditorState};

use baseplug::{
    AtomicFloat, Model, Plugin, PluginContext, PluginUI, ProcessContext, WindowOpenResult,
};
use baseview::{Size, WindowOpenOptions, WindowScalePolicy};
use imgui::{Context, FontSource, Ui};
use imgui_baseview::{HiDpiMode, ImguiWindow, RenderSettings, Settings};
use raw_window_handle::HasRawWindowHandle;
use ringbuf::{Consumer, Producer, RingBuffer};
use serde::{Deserialize, Serialize};

pub mod logging;

use logging::init_logging;
use units::ConsumerDump;

use std::{
    cell::RefCell,
    sync::{Arc, Mutex},
};

const WINDOW_WIDTH: usize = 1024;
const WINDOW_HEIGHT: usize = 1024;

const DATA_SIZE: usize = 3000;

use keyboard_types::KeyboardEvent;

baseplug::model! {
    #[derive(Debug, Serialize, Deserialize)]
    pub struct CompressorPluginModel {
        #[model(min = -80.0, max = 12.0)]
        #[parameter(name = "Threshold", unit = "Decibels",
            gradient = "Power(0.8)")]
        pub threshold: f32,

        #[model(min = 0.0, max = 48.0)]
        #[parameter(name = "Knee", unit = "Generic",
            gradient = "Linear")]
        pub knee: f32,

        #[model(min = 1.0, max = 300.0, default = 5.0)]
        #[parameter(name = "PreSmooth", unit = "Generic",
            gradient = "Linear")]
        pub pre_smooth: f32,

        #[model(min = 0.0, max = 100.0, default = 5.0)]
        #[parameter(name = "RMS", unit = "Generic",
            gradient = "Linear")]
        pub rms: f32,

        #[model(min = 1.0, max = 20.0, default = 4.0)]
        #[parameter(name = "Ratio", unit = "Generic",
            gradient = "Linear")]
        pub ratio: f32,

        #[model(min = 0.0, max = 300.0, default = 1.0)]
        #[parameter(name = "Attack", unit = "Generic",
            gradient = "Linear")]
        pub attack: f32,

        #[model(min = 0.0, max = 1000.0, default = 100.0)]
        #[parameter(name = "Release", unit = "Generic",
            gradient = "Linear")]
        pub release: f32,

        #[model(min = -24.0, max = 24.0)]
        #[parameter(name = "Gain", unit = "Decibels",
            gradient = "Linear")]
        pub gain: f32,
    }
}

impl Default for CompressorPluginModel {
    fn default() -> Self {
        Self {
            // "CompressorPlugin" is converted from dB to coefficient in the parameter handling code,
            // so in the model here it's a coeff.
            // -0dB == 1.0
            threshold: 1.0,
            knee: 0.0,
            pre_smooth: 5.0,
            rms: 5.0,
            ratio: 4.0,
            attack: 1.0,
            release: 100.0,
            gain: 1.0,
        }
    }
}

pub struct CompressorPluginShared {
    time: Arc<AtomicFloat>,
    sample_rate: Arc<AtomicFloat>,
    sample_producer: Arc<RefCell<Producer<editor::Sample>>>,
    sample_consumer: Arc<Mutex<Consumer<editor::Sample>>>,
}

unsafe impl Send for CompressorPluginShared {}
unsafe impl Sync for CompressorPluginShared {}

impl PluginContext<CompressorPlugin> for CompressorPluginShared {
    fn new() -> Self {
        init_logging("IMGUIBaseviewCompressor.log");

        let time = Arc::new(AtomicFloat::new(0.0));
        let sample_rate = Arc::new(AtomicFloat::new(44100.0));

        let sample_ring = RingBuffer::<editor::Sample>::new(DATA_SIZE);
        let (sample_producer, sample_consumer) = sample_ring.split();

        Self {
            time,
            sample_rate,
            sample_producer: Arc::new(RefCell::new(sample_producer)),
            sample_consumer: Arc::new(Mutex::new(sample_consumer)),
        }
    }
}

pub struct CompressorPlugin {
    time: Arc<AtomicFloat>,
    sample_rate: Arc<AtomicFloat>,
    compressor: Compressor,
    cv_lpf: IIR2,
    amplitude_lpf_l: IIR2,
    amplitude_lpf_r: IIR2,
    amplitude_rms_l: units::AccumulatingRMS,
    amplitude_rms_r: units::AccumulatingRMS,
    data_i: u32,
}

impl Plugin for CompressorPlugin {
    const NAME: &'static str = "DG baseplug Comp";
    const PRODUCT: &'static str = "DG baseplug Comp";
    const VENDOR: &'static str = "DGriffin";

    const INPUT_CHANNELS: usize = 2;
    const OUTPUT_CHANNELS: usize = 2;

    type Model = CompressorPluginModel;
    type PluginContext = CompressorPluginShared;

    #[inline]
    fn new(
        sample_rate: f32,
        _model: &CompressorPluginModel,
        shared: &CompressorPluginShared,
    ) -> Self {
        shared.sample_rate.set(sample_rate);
        Self {
            time: shared.time.clone(),
            compressor: Compressor::new(),
            cv_lpf: IIR2::from(IIR2Coefficients::lowpass(50.0, 0.0, 0.2, sample_rate)),
            amplitude_lpf_l: IIR2::from(IIR2Coefficients::lowpass(50.0, 0.0, 0.2, sample_rate)),
            amplitude_lpf_r: IIR2::from(IIR2Coefficients::lowpass(50.0, 0.0, 0.2, sample_rate)),
            amplitude_rms_l: units::AccumulatingRMS::new(44100, 5.0, 192000),
            amplitude_rms_r: units::AccumulatingRMS::new(44100, 5.0, 192000),
            data_i: 0,
            sample_rate: shared.sample_rate.clone(),
        }
    }

    #[inline]
    fn process(
        &mut self,
        model: &CompressorPluginModelProcess,
        ctx: &mut ProcessContext<Self>,
        shared: &CompressorPluginShared,
    ) {
        let sample_rate = self.sample_rate.get();
        self.time
            .set(self.time.get() + (1.0 / sample_rate) * ctx.nframes as f32);

        let input = &ctx.inputs[0].buffers;
        let output = &mut ctx.outputs[0].buffers;

        let mut sample_producer = shared.sample_producer.borrow_mut();

        for i in 0..ctx.nframes {
            self.compressor.update_prams(
                model.threshold[i],
                model.knee[i],
                model.pre_smooth[i],
                model.rms[i],
                model.ratio[i],
                model.attack[i],
                model.release[i],
                model.gain[i],
                sample_rate,
            ); //TODO Don't update unnecessarily?

            let input_l = if input[0][i].is_nan() || input[0][i].is_infinite() {
                0.0
            } else {
                input[0][i]
            }; //Is there a better way?

            let input_r = if input[1][i].is_nan() || input[1][i].is_infinite() {
                0.0
            } else {
                input[1][i]
            };

            let detector_input = (input_l + input_r).abs() * 0.5;

            let cv = self.compressor.process(detector_input);

            //::log::info!("cv {} cvdb {}", cv, lin_to_db(cv));

            output[0][i] = input_l * cv * model.gain[i];
            output[1][i] = input_r * cv * model.gain[i];

            let cv_filtered = self.cv_lpf.process(cv);

            let amp_filtered_l = self.amplitude_lpf_l.process(input_l);
            let amp_filtered_r = self.amplitude_lpf_r.process(input_r);

            let amp_rms_l = self.amplitude_rms_l.process(input_l);
            let amp_rms_r = self.amplitude_rms_r.process(input_r);
            if self.data_i >= (self.sample_rate.get() as u32) / 512 {
                if !sample_producer.is_full() {
                    sample_producer
                        .push(editor::Sample {
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
}

impl baseplug::PluginUI for CompressorPlugin {
    type Handle = ();

    fn ui_size() -> (i16, i16) {
        (WINDOW_WIDTH as i16, WINDOW_HEIGHT as i16)
    }

    fn ui_open(
        parent: &impl HasRawWindowHandle,
        shared: &CompressorPluginShared,
        model: <Self::Model as Model<Self>>::UI,
    ) -> WindowOpenResult<Self::Handle> {
        let settings = Settings {
            window: WindowOpenOptions {
                title: String::from("egui-baseplug-examples CompressorPlugin"),
                size: Size::new(Self::ui_size().0 as f64, Self::ui_size().1 as f64),
                scale: WindowScalePolicy::SystemScaleFactor,
            },
            render_settings: RenderSettings::default(),
            clear_color: (0.0, 0.0, 0.0),
            hidpi_mode: HiDpiMode::Default,
        };

        ImguiWindow::open_parented(
            parent,
            settings,
            EditorState {
                model,
                sample_rate: shared.sample_rate.clone(),
                time: shared.time.clone(),
                sample_data: ConsumerDump::new(shared.sample_consumer.clone(), DATA_SIZE),
                recent_peak_l: 0.0,
                recent_peak_r: 0.0,
                recent_peak_cv: 0.0,
            },
            // Called once before the first frame. Allows you to do setup code and to
            // call `ctx.set_fonts()`. Optional.
            |ctx: &mut Context, _editor_state: &mut EditorState| {
                ctx.fonts().add_font(&[FontSource::TtfData {
                    data: include_bytes!("../FiraCode-Regular.ttf"),
                    size_pixels: 20.0,
                    config: None,
                }]);
            },
            // Called before each frame. Here you should update the state of your
            // application and build the UI.
            |_run: &mut bool, ui: &Ui, editor_state: &mut EditorState| {
                //ui.show_demo_window(run);
                editor(ui, editor_state);
            },
        );

        Ok(())
    }

    fn ui_close(mut _handle: Self::Handle, _ctx: &CompressorPluginShared) {
        // TODO: Close window once baseview gets the ability to do this.
    }

    fn ui_key_down(_ctx: &CompressorPluginShared, _ev: KeyboardEvent) -> bool {
        true
    }

    fn ui_key_up(_ctx: &CompressorPluginShared, _ev: KeyboardEvent) -> bool {
        true
    }

    fn ui_param_notify(
        _handle: &Self::Handle,
        _param: &'static baseplug::Param<
            Self,
            <Self::Model as Model<Self>>::Smooth,
            <Self as PluginUI>::Handle,
        >,
        _val: f32,
    ) {
    }
}

baseplug::vst2!(CompressorPlugin, b"CANa");
