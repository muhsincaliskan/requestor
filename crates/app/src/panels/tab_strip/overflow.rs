use eframe::egui::scroll_area::ScrollAreaOutput;
use eframe::egui::{pos2, vec2, Color32, Mesh, Popup, Rect, Response, Shape, Ui};

use crate::state::{Action, AppState, TabId};
use crate::widgets::{chevron_button, request_title};

const FADE_WIDTH: f32 = 28.0;
const OVERFLOW_TOLERANCE: f32 = 0.5;
const ALL_TABS_HINT: &str = "All open tabs";
const LAST_ACTIVE_KEY: &str = "tab-strip-last-active";

pub fn overflows<R>(output: &ScrollAreaOutput<R>) -> bool {
    output.content_size.x > output.inner_rect.width() + OVERFLOW_TOLERANCE
}

pub fn fade_edges<R>(ui: &Ui, output: &ScrollAreaOutput<R>) {
    let rect = output.inner_rect;
    let offset = output.state.offset.x;
    let hidden_right = output.content_size.x - rect.width() - offset;
    let fill = ui.visuals().panel_fill;
    if offset > OVERFLOW_TOLERANCE {
        let fade = Rect::from_min_size(rect.min, vec2(FADE_WIDTH, rect.height()));
        ui.painter().add(gradient(fade, fill, Color32::TRANSPARENT));
    }
    if hidden_right > OVERFLOW_TOLERANCE {
        let fade = Rect::from_min_size(
            pos2(rect.max.x - FADE_WIDTH, rect.min.y),
            vec2(FADE_WIDTH, rect.height()),
        );
        ui.painter().add(gradient(fade, Color32::TRANSPARENT, fill));
    }
}

pub fn scroll_wheel_sideways(ui: &mut Ui) {
    if !ui.rect_contains_pointer(ui.clip_rect()) {
        return;
    }
    let delta = ui.input(|input| input.smooth_scroll_delta);
    if delta.x == 0.0 && delta.y != 0.0 {
        ui.scroll_with_delta(vec2(delta.y, 0.0));
    }
}

pub fn reveal_when_activated(ui: &Ui, button: &Response, tab: TabId) {
    let key = ui.make_persistent_id(LAST_ACTIVE_KEY);
    let previous = ui.data_mut(|data| data.get_temp::<TabId>(key));
    if previous != Some(tab) {
        ui.data_mut(|data| data.insert_temp(key, tab));
        button.scroll_to_me(None);
    }
}

pub fn all_tabs_menu(ui: &mut Ui, state: &AppState, actions: &mut Vec<Action>) {
    let button = chevron_button(ui, ALL_TABS_HINT);
    let with_method = state.interface().show_icons_with_tab_names;
    Popup::menu(&button).show(|ui| {
        for tab in &state.tabs {
            let method = with_method.then_some(tab.draft.method.as_str());
            let active = state.active_tab == Some(tab.id);
            let title = request_title(ui, method, &tab.draft.name);
            if ui.selectable_label(active, title).clicked() {
                actions.push(Action::ActivateTab(tab.id));
                ui.close();
            }
        }
    });
}

fn gradient(rect: Rect, left: Color32, right: Color32) -> Shape {
    let mut mesh = Mesh::default();
    mesh.colored_vertex(rect.left_top(), left);
    mesh.colored_vertex(rect.right_top(), right);
    mesh.colored_vertex(rect.right_bottom(), right);
    mesh.colored_vertex(rect.left_bottom(), left);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    Shape::mesh(mesh)
}
