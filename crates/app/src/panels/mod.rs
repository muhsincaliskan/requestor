mod overlays;
mod request_editor;
mod response_viewer;
mod sidebar;
mod status_bar;
mod tab_strip;
mod toolbar;
mod welcome;

use eframe::egui::{
    Align2, CentralPanel, Color32, FontId, Frame, Id, LayerId, Margin, Order, Panel, TextStyle, Ui,
};
use requestor_core::workspace::InterfaceSettings;

use crate::state::{Action, AppState};

const TOOLBAR_ID: &str = "toolbar";
const STATUS_BAR_ID: &str = "status-bar";
const SIDEBAR_ID: &str = "sidebar";
const TAB_STRIP_ID: &str = "tab-strip";
const REQUEST_BESIDE_ID: &str = "request-beside-response";
const REQUEST_ABOVE_ID: &str = "request-above-response";
const SIDEBAR_WIDTH: f32 = 260.0;
const SIDEBAR_MIN_WIDTH: f32 = 180.0;
const REQUEST_SHARE: f32 = 0.5;
const CONTENT_MARGIN: i8 = 14;
const DROP_HINT: &str = "Drop to import";
const DROP_LAYER_ID: &str = "drop-hint";
const DROP_SHADE: Color32 = Color32::from_black_alpha(140);
const BAR_MARGIN_X: i8 = 12;
const BAR_MARGIN_Y: i8 = 6;

pub fn show(ui: &mut Ui, state: &mut AppState, now: f64, actions: &mut Vec<Action>) {
    Panel::top(TOOLBAR_ID)
        .frame(bar_frame(ui))
        .show(ui, |ui| toolbar::show(ui, state, actions));
    Panel::bottom(STATUS_BAR_ID)
        .frame(bar_frame(ui))
        .show(ui, |ui| status_bar::show(ui, state, now, actions));
    let interface = state.interface().clone();
    if interface.show_sidebar && interface.show_workbench {
        sidebar_panel(ui, &interface).show(ui, |ui| sidebar::show(ui, state, actions));
    }
    CentralPanel::default()
        .frame(content_frame(ui))
        .show(ui, |ui| {
            if interface.show_workbench || !interface.show_sidebar {
                workbench(ui, state, &interface, actions);
            } else {
                sidebar::show(ui, state, actions);
            }
        });
    overlays::show(ui.ctx(), state, actions);
    import_dropped_files(ui, actions);
    show_drop_hint(ui);
}

fn show_drop_hint(ui: &Ui) {
    if ui.input(|input| input.raw.hovered_files.is_empty()) {
        return;
    }
    let painter = ui
        .ctx()
        .layer_painter(LayerId::new(Order::Foreground, Id::new(DROP_LAYER_ID)));
    let screen = ui.ctx().content_rect();
    painter.rect_filled(screen, 0.0, DROP_SHADE);
    let font: FontId = TextStyle::Heading.resolve(ui.style());
    painter.text(
        screen.center(),
        Align2::CENTER_CENTER,
        DROP_HINT,
        font,
        Color32::WHITE,
    );
}

fn import_dropped_files(ui: &Ui, actions: &mut Vec<Action>) {
    ui.input(|input| {
        actions.extend(
            input
                .raw
                .dropped_files
                .iter()
                .map(|file| file.path().to_path_buf())
                .map(Action::ImportFile),
        )
    });
}

fn bar_frame(ui: &Ui) -> Frame {
    Frame::side_top_panel(ui.style()).inner_margin(Margin::symmetric(BAR_MARGIN_X, BAR_MARGIN_Y))
}

fn content_frame(ui: &Ui) -> Frame {
    Frame::central_panel(ui.style()).inner_margin(Margin::same(CONTENT_MARGIN))
}

fn sidebar_panel(ui: &Ui, interface: &InterfaceSettings) -> Panel {
    let panel = if interface.sidebars_swapped {
        Panel::right(SIDEBAR_ID)
    } else {
        Panel::left(SIDEBAR_ID)
    };
    panel
        .frame(content_frame(ui))
        .resizable(true)
        .default_size(SIDEBAR_WIDTH)
        .min_size(SIDEBAR_MIN_WIDTH)
}

fn workbench(
    ui: &mut Ui,
    state: &mut AppState,
    interface: &InterfaceSettings,
    actions: &mut Vec<Action>,
) {
    Panel::top(TAB_STRIP_ID)
        .frame(bar_frame(ui))
        .show(ui, |ui| tab_strip::show(ui, state, actions));
    let Some(tab) = state.active_tab else {
        welcome::show(ui, actions);
        return;
    };
    let request_panel = if interface.two_pane_view {
        Panel::left(REQUEST_BESIDE_ID).default_size(ui.available_width() * REQUEST_SHARE)
    } else {
        Panel::top(REQUEST_ABOVE_ID).default_size(ui.available_height() * REQUEST_SHARE)
    };
    request_panel
        .frame(content_frame(ui))
        .resizable(true)
        .show(ui, |ui| request_editor::show(ui, state, tab, actions));
    CentralPanel::default()
        .frame(content_frame(ui))
        .show(ui, |ui| response_viewer::show(ui, state, tab, actions));
}
