use baseplug::{AtomicFloat, UIFloatParam};
use imgui::*;
use imgui_knobs::*;

use crate::{
    units::{db_to_lin, from_range, lin_to_db, sign, ConsumerDump},
    CompressorPlugin, CompressorPluginModelUI,
};

use raw_window_handle::{HasRawWindowHandle, RawWindowHandle};
use std::sync::Arc;

const WINDOW_WIDTH: usize = 1024;
const WINDOW_HEIGHT: usize = 1024;
const WINDOW_WIDTH_F: f32 = WINDOW_WIDTH as f32;
const WINDOW_HEIGHT_F: f32 = WINDOW_HEIGHT as f32;

const BLACK: [f32; 4] = [0.0, 0.0, 0.0, 1.0];
const BG_COLOR: [f32; 4] = [0.21 * 1.4, 0.11 * 1.7, 0.25 * 1.4, 1.0];
const BG_COLOR_TRANSP: [f32; 4] = [0.21 * 1.4, 0.11 * 1.7, 0.25 * 1.4, 0.0];
const GREEN: [f32; 4] = [0.23, 0.68, 0.23, 1.0];
const RED: [f32; 4] = [0.98, 0.02, 0.22, 1.0];
const ORANGE: [f32; 4] = [1.0, 0.58, 0.0, 1.0];
const ORANGE_HOVERED: [f32; 4] = [1.0, 0.68, 0.1, 1.0];
const WAVEFORM_LINES: [f32; 4] = [1.0, 1.0, 1.0, 0.2];
const TEXT: [f32; 4] = [1.0, 1.0, 1.0, 0.75];
const DB_LINES: [f32; 4] = [1.0, 1.0, 1.0, 0.15];

pub struct Sample {
    pub left: f32,
    pub right: f32,
    pub left_rms: f32,
    pub right_rms: f32,
    pub cv: f32,
}

unsafe impl Send for Sample {}
unsafe impl Sync for Sample {}

pub fn draw_knob(knob: &Knob, wiper_color: &ColorSet, track_color: &ColorSet) {
    knob.draw_arc(
        0.8,
        0.20,
        knob.angle_min,
        knob.angle_max,
        track_color,
        16,
        2,
    );
    if knob.t > 0.01 {
        knob.draw_arc(0.8, 0.21, knob.angle_min, knob.angle, wiper_color, 16, 2);
    }
}

pub fn make_knob(
    ui: &Ui,
    parameter: &mut UIFloatParam<crate::CompressorPluginModel, crate::CompressorPluginModelSmooth>,
    wiper_color: &ColorSet,
    track_color: &ColorSet,
    title_fix: f32,
) {
    let width = ui.text_line_height() * 4.75;
    let w = ui.push_item_width(width);
    let title = parameter.name();
    let knob_id = &ImString::new(format!("##{}_KNOB_CONTORL_", title));
    knob_title(ui, &ImString::new(title.clone().to_uppercase()), width);
    let cursor = ui.cursor_pos();
    ui.set_cursor_pos([cursor[0], cursor[1] + 5.0]);
    let mut val = parameter.normalized();
    let value_changed = {
        let knob = Knob::new(ui, knob_id, &mut val, 0.0, 1.0, 0.5, width * 0.5, true);
        let cursor = ui.cursor_pos();
        ui.set_cursor_pos([cursor[0] + title_fix, cursor[1] - 10.0]);
        knob_title(
            ui,
            &ImString::new(format!("{:.2}", parameter.unit_value())),
            width,
        );

        w.pop(ui);
        draw_knob(&knob, wiper_color, track_color);
        knob.value_changed
    };

    if value_changed {
        parameter.set_from_normalized(val)
    }
}

pub struct EditorState {
    pub model: CompressorPluginModelUI<CompressorPlugin>,
    pub sample_rate: Arc<AtomicFloat>,
    pub time: Arc<AtomicFloat>,
    pub sample_data: ConsumerDump<Sample>,
    pub recent_peak_l: f32,
    pub recent_peak_r: f32,
    pub recent_peak_cv: f32,
}

fn draw_graph<F: Fn(usize) -> f32>(
    ui: &Ui,
    id: &ImStr,
    size: [f32; 2],
    v_scale: f32,
    v_offset: f32,
    thinkness: f32,
    length: usize,
    value_fn: F,
) {
    let draw_list = ui.get_window_draw_list();

    let cursor = ui.cursor_screen_pos();
    ui.invisible_button(id, size);

    let mut color = if ui.is_item_hovered() {
        ui.style_color(StyleColor::PlotLinesHovered)
    } else {
        ui.style_color(StyleColor::PlotLines)
    };
    let scale = (size[0] as f32 / length as f32) as f32;
    //color[3] = (color[3] * scale * 2.0).min(1.0).max(0.0);
    color[3] = (color[3] * 0.9).min(1.0).max(0.0);
    let v_center = size[1] / 2.0;
    let mut last = 0.0 + v_offset;
    for i in 0..length {
        let fi = i as f32;
        let next = value_fn(i) * v_scale + v_offset;
        let x_ofs = if (next - last).abs() < 1.0 { 1.0 } else { 0.0 };
        draw_list
            .add_line(
                [cursor[0] + fi * scale, cursor[1] + v_center + last],
                [cursor[0] + fi * scale + x_ofs, cursor[1] + v_center + next],
                color,
            )
            .thickness(thinkness)
            .build();
        last = next;
    }
}

fn draw_meter(
    ui: &Ui,
    size: [f32; 2],
    value: f32,
    peak_value: f32,
    bottom: f32,
    top: f32,
    bg_color: [f32; 4],
    color: [f32; 4],
    reduction: bool,
) {
    let draw_list = ui.get_window_draw_list();
    let cursor = ui.cursor_screen_pos();
    draw_list
        .add_rect(
            [cursor[0], cursor[1]],
            [cursor[0] + size[0], cursor[1] + size[1]],
            bg_color,
        )
        .filled(true)
        .build();
    if !reduction {
        let pos = from_range(bottom, top, value.max(bottom).min(top));
        let peak_pos = from_range(bottom, top, peak_value.max(bottom).min(top));
        draw_list
            .add_rect(
                [cursor[0], cursor[1]],
                [cursor[0] + size[0] * pos, cursor[1] + size[1]], //size[0] * pos
                color,
            )
            .filled(true)
            .build();
        draw_list
            .add_rect(
                [cursor[0] + size[0] * peak_pos, cursor[1]],
                [cursor[0] + size[0] * peak_pos + 4.0, cursor[1] + size[1]],
                color,
            )
            .filled(true)
            .build();
    } else {
        let pos = from_range(top, bottom, value.max(bottom).min(top));
        let peak_pos = from_range(top, bottom, peak_value.max(bottom).min(top));
        draw_list
            .add_rect(
                [cursor[0] + size[0] - size[0] * pos, cursor[1]],
                [cursor[0] + size[0], cursor[1] + size[1]], //size[0] * pos
                color,
            )
            .filled(true)
            .build();
        draw_list
            .add_rect(
                [cursor[0] + size[0] - size[0] * peak_pos - 4.0, cursor[1]],
                [
                    cursor[0] + size[0] - size[0] * peak_pos,
                    cursor[1] + size[1],
                ],
                color,
            )
            .filled(true)
            .build();
    }
}

fn draw_db_lines(
    ui: &Ui,
    bottom: f32,
    top: f32,
    bottom_scale: f32,
    top_scale: f32,
    indv_width: f32,
    size: [f32; 2],
    step: usize,
    color: [f32; 4],
    text_color: [f32; 4],
) {
    let draw_list = ui.get_window_draw_list();
    let cursor = ui.cursor_screen_pos();
    for i in (bottom as i32..(top as i32 + step as i32)).step_by(step) {
        let pos = from_range(
            bottom_scale as f32,
            top_scale,
            (i as f32).max(bottom).min(top),
        ) * size[0];
        draw_list
            .add_rect(
                [cursor[0] + pos, cursor[1]],
                [cursor[0] + pos + indv_width, cursor[1] + size[1]],
                color,
            )
            .filled(true)
            .build();
        let s = format!("{}", i);
        let offset = s.len() as f32 * 4.0;
        draw_list.add_text(
            [cursor[0] + pos - offset, cursor[1] + size[1] + 15.0],
            text_color,
            s,
        )
    }
}

fn draw_meter_knob(
    ui: &Ui,
    value: f32,
    peak_value: f32,
    bottom: f32,
    top: f32,
    width: f32,
    radius: f32,
    color: [f32; 4],
    bg_color: [f32; 4],
) {
    let mut value = value;
    let mut peak_value = peak_value;
    let cursor = ui.cursor_pos();
    {
        let main_knob = Knob::new(
            ui,
            im_str!("___THRESHOLD METER___"),
            &mut value,
            bottom,
            top,
            0.0,
            width * 0.5,
            false,
        );

        main_knob.draw_arc(
            radius,
            0.20,
            main_knob.angle_min,
            main_knob.angle_max,
            &ColorSet::new(bg_color, bg_color, bg_color),
            16,
            2,
        );
        if main_knob.t > 0.01 {
            main_knob.draw_arc(
                radius,
                0.21,
                main_knob.angle_min,
                main_knob.angle,
                &ColorSet::new(color, color, color),
                16,
                2,
            );
        }
    }
    ui.set_cursor_pos(cursor);
    {
        let peak_knob = Knob::new(
            ui,
            im_str!("___THRESHOLD METER PEAK___"),
            &mut peak_value,
            bottom,
            top,
            0.0,
            width * 0.5,
            false,
        );
        if peak_knob.t > 0.01 {
            peak_knob.draw_arc(
                radius,
                0.21,
                peak_knob.angle,
                peak_knob.angle + 0.1,
                &ColorSet::new(color, color, color),
                8,
                1,
            );
        }
    }
}

fn move_cursor(ui: &Ui, x: f32, y: f32) {
    let cursor = ui.cursor_pos();
    ui.set_cursor_pos([cursor[0] + x, cursor[1] + y])
}

fn floating_text(ui: &Ui, text: &str) {
    ui.get_window_draw_list()
        .add_text(ui.cursor_pos(), ui.style_color(StyleColor::Text), text)
}

fn draw_meters(
    ui: &Ui,
    left: f32,
    right: f32,
    cv: f32,
    recent_peak_l: f32,
    recent_peak_r: f32,
    recent_peak_cv: f32,
    gain: f32,
) {
    let distance_between_pairs = 30.0;
    let distance_between_meters = 15.0;

    let start_cursor_x = ui.cursor_pos()[0];

    move_cursor(ui, 62.0, -45.0);
    draw_db_lines(
        ui,
        -36.0,
        0.0,
        -39.0,
        3.0,
        1.0,
        [WINDOW_WIDTH_F - 65.0, 200.0],
        3,
        DB_LINES,
        TEXT,
    );

    move_cursor(ui, 15.0, 35.0);

    move_cursor(ui, -45.0, 0.0);
    floating_text(ui, "IN");
    move_cursor(ui, 45.0, 0.0);

    draw_meter(
        ui,
        [WINDOW_WIDTH_F - 65.0, 4.0],
        lin_to_db(left),
        lin_to_db(recent_peak_l),
        -39.0,
        3.0,
        BLACK,
        GREEN,
        false,
    );

    move_cursor(ui, 0.0, distance_between_meters);

    draw_meter(
        ui,
        [WINDOW_WIDTH_F - 65.0, 4.0],
        lin_to_db(right),
        lin_to_db(recent_peak_r),
        -39.0,
        3.0,
        BLACK,
        GREEN,
        false,
    );

    move_cursor(ui, -45.0, distance_between_pairs);
    floating_text(ui, "GR");
    move_cursor(ui, 45.0, 0.0);

    draw_meter(
        ui,
        [WINDOW_WIDTH_F - 65.0, 4.0],
        lin_to_db(cv),
        lin_to_db(recent_peak_cv),
        -39.0,
        3.0,
        BLACK,
        RED,
        true,
    );
    move_cursor(ui, 0.0, distance_between_meters);
    draw_meter(
        ui,
        [WINDOW_WIDTH_F - 65.0, 4.0],
        lin_to_db(cv),
        lin_to_db(recent_peak_cv),
        -39.0,
        3.0,
        BLACK,
        RED,
        true,
    );
    move_cursor(ui, -55.0, distance_between_pairs);
    floating_text(ui, "OUT");
    move_cursor(ui, 55.0, 0.0);
    draw_meter(
        ui,
        [WINDOW_WIDTH_F - 65.0, 4.0],
        lin_to_db(left * cv * gain),
        lin_to_db(recent_peak_cv * recent_peak_l * gain),
        -39.0,
        3.0,
        BLACK,
        GREEN,
        false,
    );
    move_cursor(ui, 0.0, distance_between_meters);
    draw_meter(
        ui,
        [WINDOW_WIDTH_F - 65.0, 4.0],
        lin_to_db(right * cv * gain),
        lin_to_db(recent_peak_cv * recent_peak_r * gain),
        -39.0,
        3.0,
        BLACK,
        GREEN,
        false,
    );
    let cursor = ui.cursor_pos();
    ui.set_cursor_pos([start_cursor_x, cursor[1] + 160.0]);
}

fn draw_graphs(ui: &Ui, graph_v_center: f32, graph_height: f32, state: &mut EditorState) {
    let init_cursor = ui.cursor_pos();
    let sample_data = &state.sample_data.data;
    let col = ui.push_style_color(StyleColor::PlotLines, ORANGE);
    let col2 = ui.push_style_color(StyleColor::PlotLinesHovered, ORANGE);
    draw_graph(
        ui,
        im_str!("Graph"),
        [WINDOW_WIDTH_F, graph_height],
        225.0 / db_to_lin(state.model.threshold.unit_value()).powf(0.8), // / 256.0
        0.0,
        2.5,
        sample_data.len(),
        |i| {
            let val = sample_data[i].left + sample_data[i].right;
            sign((val.abs()).powf(0.8), val)
        },
    );
    {
        let draw_list = ui.get_window_draw_list();
        draw_list.add_rect_filled_multicolor(
            [0.0, graph_v_center + 92.0],
            [WINDOW_WIDTH_F, graph_v_center + 92.0 + 128.0],
            BG_COLOR_TRANSP,
            BG_COLOR_TRANSP,
            BG_COLOR,
            BG_COLOR,
        );
        draw_list
            .add_rect(
                [0.0, graph_v_center + 92.0 + 128.0],
                [WINDOW_WIDTH_F, WINDOW_HEIGHT_F],
                BG_COLOR,
            )
            .filled(true)
            .build();
    }
    col.pop(ui);
    col2.pop(ui);
    {
        //threshold line
        let draw_list = ui.get_window_draw_list();
        draw_list
            .add_rect(
                [0.0, 0.0],
                [WINDOW_WIDTH_F, graph_v_center - 92.0],
                [0.0, 0.0, 0.0, 0.65],
            )
            .filled(true)
            .build();
        draw_list
            .add_rect(
                [0.0, graph_v_center + 92.0],
                [WINDOW_WIDTH_F, WINDOW_HEIGHT_F],
                [0.0, 0.0, 0.0, 0.65],
            )
            .filled(true)
            .build();
        let knee_setting = state.model.knee.unit_value();
        if knee_setting > 0.1 {
            let knee = db_to_lin(knee_setting).powf(0.5) * 6.0;

            draw_list.add_rect_filled_multicolor(
                [0.0, graph_v_center - 92.0],
                [WINDOW_WIDTH_F, graph_v_center - 92.0 + knee],
                [0.8, 0.1, 0.1, 0.5],
                [0.8, 0.1, 0.1, 0.5],
                [0.8, 0.1, 0.1, 0.0],
                [0.8, 0.1, 0.1, 0.0],
            );
            draw_list.add_rect_filled_multicolor(
                [0.0, graph_v_center + 92.0],
                [WINDOW_WIDTH_F, graph_v_center + 92.0 - knee],
                [0.8, 0.1, 0.1, 0.5],
                [0.8, 0.1, 0.1, 0.5],
                [0.8, 0.1, 0.1, 0.0],
                [0.8, 0.1, 0.1, 0.0],
            );
        }
        draw_list
            .add_line(
                [0.0, graph_v_center - 92.0],
                [WINDOW_WIDTH_F, graph_v_center - 92.0],
                WAVEFORM_LINES,
            )
            .thickness(2.0)
            .build();
        draw_list
            .add_line(
                [0.0, graph_v_center + 92.0],
                [WINDOW_WIDTH_F, graph_v_center + 92.0],
                WAVEFORM_LINES,
            )
            .thickness(2.0)
            .build();
    }

    let col = ui.push_style_color(StyleColor::PlotLines, RED);
    let col2 = ui.push_style_color(StyleColor::PlotLinesHovered, RED);
    ui.set_cursor_pos(init_cursor);
    move_cursor(ui, 0.0, 20.0);
    draw_graph(
        ui,
        im_str!("Graph"),
        [WINDOW_WIDTH_F, graph_height],
        -128.0,
        -256.0 + 129.0,
        3.0,
        sample_data.len(),
        |i| sample_data[i].cv,
    );
    col.pop(ui);
    col2.pop(ui);
    ui.set_cursor_pos(init_cursor);
    move_cursor(ui, 12.0, 108.0);
    floating_text(ui, &state.model.threshold.name());
    ui.set_cursor_pos(init_cursor);
    move_cursor(ui, 0.0, graph_height);
}

pub fn editor(ui: &Ui, editor_state: &mut EditorState) {
    editor_state.sample_data.consume();

    let w = Window::new(im_str!("Example 1: Basic sliders"))
        .size([WINDOW_WIDTH_F, WINDOW_HEIGHT_F], Condition::Appearing)
        .position([0.0, 0.0], Condition::Appearing)
        .draw_background(false)
        .no_decoration()
        .movable(false);
    w.build(&ui, || {
        let text_style_color = ui.push_style_color(StyleColor::Text, TEXT);
        let graph_v_center = 225.0 + 25.0;
        {
            let draw_list = ui.get_window_draw_list();
            draw_list.add_rect_filled_multicolor(
                [0.0, 0.0],
                [WINDOW_WIDTH_F, 200.0],
                BLACK,
                BLACK,
                BG_COLOR,
                BG_COLOR,
            );
            draw_list
                .add_rect([0.0, 200.0], [WINDOW_WIDTH_F, WINDOW_HEIGHT_F], BG_COLOR)
                .filled(true)
                .build();
            draw_list
                .add_rect(
                    [0.0, graph_v_center - 92.0],
                    [WINDOW_WIDTH_F, graph_v_center + 92.0],
                    [0.0, 0.0, 0.0, 0.65],
                )
                .filled(true)
                .build();
        }
        ui.set_cursor_pos([0.0, 25.0]);
        draw_graphs(ui, graph_v_center, 450.0, editor_state);
        let mut left = 0.0;
        let mut right = 0.0;
        let mut cv = 0.0;
        if editor_state.sample_data.data.len() > 0 {
            let last = editor_state.sample_data.data.len() - 1;
            left = editor_state.sample_data.data[last].left_rms;
            right = editor_state.sample_data.data[last].right_rms;
            cv = editor_state.sample_data.data[last].cv;
            if (editor_state.time.get() * 10.0) as u32 % 10 == 0 {
                editor_state.recent_peak_l = left;
                editor_state.recent_peak_r = right;
                editor_state.recent_peak_cv = cv;
            } else {
                editor_state.recent_peak_l = editor_state.recent_peak_l.max(left);
                editor_state.recent_peak_r = editor_state.recent_peak_r.max(right);
                editor_state.recent_peak_cv = editor_state.recent_peak_cv.min(cv);
            }
        }

        let highlight = ColorSet::new(ORANGE, ORANGE_HOVERED, ORANGE_HOVERED);

        let model = &mut editor_state.model;

        let line_height = ui.text_line_height();

        let lowlight = ColorSet::from(BLACK);
        ui.columns(7, im_str!("cols"), false);
        let width = WINDOW_WIDTH_F / 6.75;
        for i in 1..7 {
            ui.set_column_width(i, width);
        }
        ui.set_column_width(0, width * 0.5);

        ui.next_column();
        make_knob(ui, &mut model.threshold, &highlight, &lowlight, 0.0);
        move_cursor(ui, 0.0, -113.0);
        draw_meter_knob(
            ui,
            lin_to_db((left + right) * 0.5),
            lin_to_db((editor_state.recent_peak_l + editor_state.recent_peak_r) * 0.5),
            model.threshold.min_max().0,
            model.threshold.min_max().1,
            line_height * 4.75,
            1.0,
            GREEN,
            BLACK,
        );
        ui.next_column();

        make_knob(ui, &mut model.knee, &highlight, &lowlight, 0.0);
        ui.next_column();

        //make_knob(ui, &mut model.pre_smooth, &highlight, &lowlight);
        //ui.next_column();

        //make_knob(ui, &mut model.rms, &highlight, &lowlight);
        //ui.next_column();

        make_knob(ui, &mut model.ratio, &highlight, &lowlight, 0.0);
        ui.next_column();

        make_knob(ui, &mut model.attack, &highlight, &lowlight, 0.0);
        ui.next_column();

        make_knob(ui, &mut model.release, &highlight, &lowlight, 0.0);
        ui.next_column();

        make_knob(ui, &mut model.gain, &highlight, &lowlight, 0.0);
        ui.next_column();

        ui.columns(1, im_str!("nocols"), false);

        //ui.label_text(im_str!("left"), &ImString::new(left.to_string()));

        move_cursor(ui, 0.0, 84.0);
        draw_meters(
            ui,
            left,
            right,
            cv,
            editor_state.recent_peak_l,
            editor_state.recent_peak_r,
            editor_state.recent_peak_cv,
            db_to_lin(model.gain.unit_value()),
        );

        text_style_color.pop(ui);
    });
}

struct VstParent(*mut ::std::ffi::c_void);

#[cfg(target_os = "macos")]
unsafe impl HasRawWindowHandle for VstParent {
    fn raw_window_handle(&self) -> RawWindowHandle {
        use raw_window_handle::macos::MacOSHandle;

        RawWindowHandle::MacOS(MacOSHandle {
            ns_view: self.0 as *mut ::std::ffi::c_void,
            ..MacOSHandle::empty()
        })
    }
}

#[cfg(target_os = "windows")]
unsafe impl HasRawWindowHandle for VstParent {
    fn raw_window_handle(&self) -> RawWindowHandle {
        use raw_window_handle::windows::WindowsHandle;

        RawWindowHandle::Windows(WindowsHandle {
            hwnd: self.0,
            ..WindowsHandle::empty()
        })
    }
}

#[cfg(target_os = "linux")]
unsafe impl HasRawWindowHandle for VstParent {
    fn raw_window_handle(&self) -> RawWindowHandle {
        use raw_window_handle::unix::XcbHandle;

        RawWindowHandle::Xcb(XcbHandle {
            window: self.0 as u32,
            ..XcbHandle::empty()
        })
    }
}
