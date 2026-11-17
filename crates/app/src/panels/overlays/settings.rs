mod interface;
mod proxy;
mod request;
mod row;
mod shortcuts;
mod themes;

use eframe::egui::{vec2, Align, Context, Id, Layout, Modal, ScrollArea, Sense, Ui, UiBuilder};
use requestor_core::workspace::AppSettings;

use super::{button_row, TITLE_GAP};
use crate::state::{Action, AppState};
use crate::widgets::action_button;

const MODAL_ID: &str = "settings";
const CATEGORY_KEY: &str = "settings-category";
const TITLE: &str = "Settings";
const DONE_LABEL: &str = "Done";
const SIDEBAR_WIDTH: f32 = 140.0;
const CONTENT_WIDTH: f32 = 520.0;
const CONTENT_HEIGHT: f32 = 460.0;
const PANE_GAP: f32 = 24.0;
const ABOUT_NAME: &str = "Requestor";
const ABOUT_TAGLINE: &str = "A lightweight HTTP client.";
const VERSION_LABEL: &str = "Version";

#[derive(Clone, Copy, PartialEq)]
enum Category {
    Request,
    Interface,
    Themes,
    Shortcuts,
    Proxy,
    About,
}

impl Category {
    const ALL: [Category; 6] = [
        Category::Request,
        Category::Interface,
        Category::Themes,
        Category::Shortcuts,
        Category::Proxy,
        Category::About,
    ];

    fn label(self) -> &'static str {
        match self {
            Category::Request => "Request",
            Category::Interface => "User interface",
            Category::Themes => "Themes",
            Category::Shortcuts => "Shortcuts",
            Category::Proxy => "Proxy",
            Category::About => "About",
        }
    }

    fn show(self, ui: &mut Ui, settings: &mut AppSettings) -> bool {
        match self {
            Category::Request => request::show(ui, settings),
            Category::Interface => interface::show(ui, &mut settings.interface),
            Category::Themes => themes::show(ui, &mut settings.interface),
            Category::Shortcuts => unchanged(ui, shortcuts::show),
            Category::Proxy => proxy::show(ui, &mut settings.request),
            Category::About => unchanged(ui, about),
        }
    }
}

pub fn show(ctx: &Context, state: &mut AppState, actions: &mut Vec<Action>) -> bool {
    let category_id = Id::new(CATEGORY_KEY);
    let mut category = ctx
        .data(|data| data.get_temp(category_id))
        .unwrap_or(Category::Request);
    let mut edited = false;
    let mut done = false;
    let shown = Modal::new(Id::new(MODAL_ID)).show(ctx, |ui| {
        ui.set_width(SIDEBAR_WIDTH + CONTENT_WIDTH + PANE_GAP * 2.0);
        ui.heading(TITLE);
        ui.add_space(TITLE_GAP);
        ui.horizontal_top(|ui| {
            pane(ui, SIDEBAR_WIDTH, |ui| {
                category = category_list(ui, category);
            });
            ui.add_space(PANE_GAP);
            pane(ui, CONTENT_WIDTH, |ui| {
                edited = content(ui, category, &mut state.workspace.settings);
            });
        });
        done = button_row(ui, |ui| action_button::primary(ui, DONE_LABEL).clicked());
    });
    ctx.data_mut(|data| data.insert_temp(category_id, category));
    if edited {
        actions.push(Action::SettingsEdited);
    }
    let stays_open = !done && !shown.should_close();
    if !stays_open {
        proxy::forget_transient_state(ctx);
    }
    stays_open
}

fn pane(ui: &mut Ui, width: f32, contents: impl FnOnce(&mut Ui)) {
    let (area, _) = ui.allocate_exact_size(vec2(width, CONTENT_HEIGHT), Sense::hover());
    let builder = UiBuilder::new()
        .max_rect(area)
        .layout(Layout::top_down(Align::Min));
    ui.scope_builder(builder, |ui| {
        ui.set_clip_rect(area.intersect(ui.clip_rect()));
        contents(ui);
    });
}

fn category_list(ui: &mut Ui, current: Category) -> Category {
    let mut selected = current;
    for category in Category::ALL {
        ui.selectable_value(&mut selected, category, category.label());
    }
    selected
}

fn content(ui: &mut Ui, category: Category, settings: &mut AppSettings) -> bool {
    ScrollArea::vertical()
        .id_salt(CATEGORY_KEY)
        .auto_shrink(false)
        .max_height(CONTENT_HEIGHT)
        .show(ui, |ui| category.show(ui, settings))
        .inner
}

fn unchanged(ui: &mut Ui, render: impl FnOnce(&mut Ui)) -> bool {
    render(ui);
    false
}

fn about(ui: &mut Ui) {
    ui.heading(ABOUT_NAME);
    ui.label(ABOUT_TAGLINE);
    ui.label(format!("{VERSION_LABEL} {}", env!("CARGO_PKG_VERSION")));
}
