use imgui::*;
use imgui_knobs::*;

use imgui_baseview::{HiDpiMode, RenderSettings, Runner, Settings};

use crate::compressor_effect_parameters::CompressorEffectParameters;
use crate::parameter::Parameter;

use vst::editor::Editor;

use baseview::{AppRunner, Parent, Size, WindowOpenOptions, WindowScalePolicy};

use raw_window_handle::RawWindowHandle;
use std::{
    f32::consts::PI,
    sync::{Arc, Mutex},
};
use vst::util::AtomicFloat;

use ringbuf::Consumer;

const WINDOW_WIDTH: usize = 1800;
const WINDOW_HEIGHT: usize = 1000;

pub fn make_knob(
    ui: &Ui,
    perameter: &Parameter,
    format: &ImStr,
    circle_color: &ColorSet,
    wiper_color: &ColorSet,
    track_color: &ColorSet,
) {
    let width = ui.text_line_height() * 8.0;
    let w = ui.push_item_width(width);
    let title = perameter.get_name();
    let knob_id = &ImString::new(format!("##{}_KNOB_CONTORL_", title));
    knob_title(ui, &ImString::new(title.clone()), width);
    let mut val = perameter.get();
    let knob = Knob::new(
        ui,
        knob_id,
        &mut val,
        perameter.min,
        perameter.max,
        perameter.default,
        width * 0.5,
    );
    let drag_id = &ImString::new(format!("##{}_KNOB_DRAG_CONTORL_", title));
    let drag_value_changed = Drag::new(drag_id)
        .range(perameter.min..=perameter.max)
        .display_format(format)
        .speed((perameter.max - perameter.min) / 1000.0)
        .build(ui, knob.p_value);

    if knob.value_changed || drag_value_changed {
        perameter.set(*knob.p_value)
    }

    w.pop(ui);
    draw_wiper_knob(&knob, circle_color, wiper_color, track_color);
}

pub struct EditorOnlyState {
    pub cv_consumer: Consumer<f32>,
    pub amplitude_consumer: Consumer<f32>,
    pub cv_data: Vec<f32>,
    pub amplitude_data: Vec<f32>,
}

pub struct EditorState {
    pub params: Arc<CompressorEffectParameters>,
    pub editor_only: Arc<Mutex<EditorOnlyState>>,
    pub sample_rate: AtomicFloat,
}

pub struct CompressorPluginEditor {
    pub runner: Option<AppRunner>,
    pub state: Arc<EditorState>,
}

fn consume(consumer: &mut Consumer<f32>) -> Vec<f32> {
    let mut data = Vec::new();
    for _ in 0..consumer.len() {
        if let Some(n) = consumer.pop() {
            data.push(n);
        } else {
            break;
        }
    }
    data
}

fn update_data(new_data: &mut Vec<f32>, data_to_update: &mut Vec<f32>, max_size: usize) {
    data_to_update.append(new_data);
    let data_len = data_to_update.len();
    if data_len > max_size {
        data_to_update.drain(0..(data_len - max_size).max(0));
    }
}

fn alt_graph(ui: &Ui, id: &ImStr, size: [f32; 2], v_scale: f32, v_offset: f32, values: &[f32]) {
    let draw_list = ui.get_window_draw_list();

    let mut cursor = ui.cursor_screen_pos();
    ui.invisible_button(id, size);

    let mut color = if ui.is_item_hovered() {
        ui.style_color(StyleColor::PlotLinesHovered)
    } else {
        ui.style_color(StyleColor::PlotLines)
    };
    let scale = (size[0] as f32 / values.len() as f32) as f32;
    //color[3] = (color[3] * scale * 2.0).min(1.0).max(0.0);
    color[3] = (color[3] * 0.5).min(1.0).max(0.0);
    let v_center = size[1] / 2.0;
    let mut last = 0.0 + v_offset;
    for (i, n) in values.iter().enumerate() {
        let fi = i as f32;
        let next = n * v_scale + v_offset;
        let x_ofs = if (next - last).abs() < 1.0 { 1.0 } else { 0.0 };
        draw_list
            .add_line(
                [cursor[0] + fi * scale, cursor[1] + v_center + last],
                [cursor[0] + fi * scale + x_ofs, cursor[1] + v_center + next],
                color,
            )
            .thickness(2.5)
            .build();
        last = next;
    }
}

fn graph(ui: &Ui, a_values: &Vec<f32>, b_values: &Vec<f32>, width: f32) {
    let mut style_colors = Vec::new();
    style_colors.push(ui.push_style_color(StyleColor::FrameBg, [0.0, 0.0, 0.0, 0.0]));
    style_colors.push(ui.push_style_color(StyleColor::PlotHistogram, [0.4, 0.2, 0.4, 0.4]));
    let cursor = ui.cursor_pos();

    ui.plot_lines(im_str!("##Z"), &b_values)
        .graph_size([width, 512.0])
        .scale_min(50.0)
        .scale_max(100.0)
        .build();
    ui.set_cursor_pos(cursor);

    style_colors.push(ui.push_style_color(StyleColor::PlotLines, [0.4, 0.4, 0.4, 0.4]));

    ui.plot_histogram(im_str!("##X"), &a_values)
        .graph_size([width, 512.0])
        .scale_min(-0.5)
        .scale_max(0.5)
        .build();

    ui.set_cursor_pos(cursor);
    ui.plot_lines(im_str!("##Y"), &a_values)
        .graph_size([width, 512.0])
        .scale_min(-0.5)
        .scale_max(0.5)
        .build();

    style_colors.into_iter().for_each(|color| color.pop(ui));
}

impl Editor for CompressorPluginEditor {
    fn position(&self) -> (i32, i32) {
        (0, 0)
    }

    fn size(&self) -> (i32, i32) {
        (WINDOW_WIDTH as i32, WINDOW_HEIGHT as i32)
    }

    fn open(&mut self, parent: *mut ::std::ffi::c_void) -> bool {
        //::log::info!("self.running {}", self.running);
        if self.runner.is_some() {
            return true;
        }

        let parent = raw_window_handle_from_parent(parent);

        let settings = Settings {
            window: WindowOpenOptions {
                title: String::from("imgui-baseview demo window"),
                size: Size::new(WINDOW_WIDTH as f64, WINDOW_HEIGHT as f64),
                scale: WindowScalePolicy::SystemScaleFactor,
                parent: Parent::WithParent(parent),
            },
            clear_color: (0.0, 0.0, 0.0),
            hidpi_mode: HiDpiMode::Default,
            render_settings: RenderSettings::default(),
        };

        let (_handle, runner) = Runner::open(
            settings,
            self.state.clone(),
            |_context: &mut Context, _state: &mut Arc<EditorState>| {},
            |run: &mut bool, ui: &Ui, state: &mut Arc<EditorState>| {
                let mut editor_only = state.editor_only.lock().unwrap();
                let sample_rate = state.sample_rate.get() as usize;
                let window_size = 3000;
                update_data(
                    &mut consume(&mut editor_only.cv_consumer),
                    &mut editor_only.cv_data,
                    window_size,
                );

                update_data(
                    &mut consume(&mut editor_only.amplitude_consumer),
                    &mut editor_only.amplitude_data,
                    window_size,
                );

                ui.show_demo_window(run);
                let w = Window::new(im_str!("Example 1: Basic sliders"))
                    .size([1024.0, 200.0], Condition::Appearing)
                    .position([20.0, 20.0], Condition::Appearing);
                w.build(&ui, || {
                    //graph(
                    //    ui,
                    //    &editor_only.amplitude_data,
                    //    &editor_only.cv_data,
                    //    window_size as f32,
                    //);

                    let cursor = ui.cursor_pos();
                    alt_graph(
                        ui,
                        im_str!("Graph"),
                        [1500.0, 512.0],
                        512.0,
                        0.0,
                        &editor_only.amplitude_data,
                    );
                    ui.set_cursor_pos(cursor);
                    alt_graph(
                        ui,
                        im_str!("Graph"),
                        [1500.0, 512.0],
                        -128.0,
                        -256.0 + 129.0,
                        &editor_only.cv_data,
                    );
                    //ui.plot_lines(im_str!("X"), &editor_only.cv_data)
                    //    .graph_size([800.0, 256.0])
                    //    .build();

                    let highlight = ColorSet::new(
                        [0.4, 0.4, 0.8, 1.0],
                        [0.4, 0.4, 0.9, 1.0],
                        [0.5, 0.5, 1.0, 1.0],
                    );
                    let base = ColorSet::new(
                        [0.4, 0.3, 0.5, 1.0],
                        [0.45, 0.35, 0.55, 1.0],
                        [0.45, 0.35, 0.55, 1.0],
                    );

                    let lowlight = ColorSet::from([0.0, 0.0, 0.0, 1.0]);
                    let params = &state.params;
                    ui.columns(7, im_str!("cols"), false);
                    make_knob(
                        ui,
                        &params.threshold,
                        im_str!("%.2fdB"),
                        &base,
                        &highlight,
                        &lowlight,
                    );
                    ui.next_column();

                    make_knob(
                        ui,
                        &params.ratio,
                        im_str!("%.2f"),
                        &base,
                        &highlight,
                        &lowlight,
                    );
                    ui.next_column();

                    make_knob(
                        ui,
                        &params.attack,
                        im_str!("%.2f"),
                        &base,
                        &highlight,
                        &lowlight,
                    );
                    ui.next_column();

                    make_knob(
                        ui,
                        &params.release,
                        im_str!("%.2f"),
                        &base,
                        &highlight,
                        &lowlight,
                    );
                    ui.next_column();

                    make_knob(
                        ui,
                        &params.gain,
                        im_str!("%.2fdB"),
                        &base,
                        &highlight,
                        &lowlight,
                    );
                    ui.next_column();
                });
            },
        );

        self.runner = runner;

        true
    }

    fn is_open(&mut self) -> bool {
        self.runner.is_some()
    }

    fn close(&mut self) {
        self.runner = None;
    }
}

#[cfg(target_os = "macos")]
fn raw_window_handle_from_parent(parent: *mut ::std::ffi::c_void) -> RawWindowHandle {
    use raw_window_handle::macos::MacOSHandle;

    RawWindowHandle::MacOS(MacOSHandle {
        ns_view: parent as *mut ::std::ffi::c_void,
        ..MacOSHandle::empty()
    })
}

#[cfg(target_os = "windows")]
fn raw_window_handle_from_parent(parent: *mut ::std::ffi::c_void) -> RawWindowHandle {
    use raw_window_handle::windows::WindowsHandle;

    RawWindowHandle::Windows(WindowsHandle {
        hwnd: parent,
        ..WindowsHandle::empty()
    })
}

#[cfg(target_os = "linux")]
fn raw_window_handle_from_parent(parent: *mut ::std::ffi::c_void) -> RawWindowHandle {
    use raw_window_handle::unix::XcbHandle;

    RawWindowHandle::Xcb(XcbHandle {
        window: parent as u32,
        ..XcbHandle::empty()
    })
}
