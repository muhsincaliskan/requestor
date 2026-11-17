use eframe::egui::{Button, Grid, Label, RichText, ScrollArea, Ui};

use super::cookie::CookieEntry;
use crate::response::ResponseView;
use crate::state::Action;

const COPY_LABEL: &str = "Copy";
const HEADERS_GRID_ID: &str = "response-headers";
const COOKIES_GRID_ID: &str = "response-cookies";
const NO_HEADERS_TEXT: &str = "This response has no headers.";
const NO_COOKIES_TEXT: &str = "This response did not set any cookies.";
const GRID_SPACING: [f32; 2] = [16.0, 4.0];

fn selectable_mono(ui: &mut Ui, text: &str) {
    ui.add(Label::new(RichText::new(text).monospace()).selectable(true));
}

fn copy_button(ui: &mut Ui, text: &str, actions: &mut Vec<Action>) {
    let label = RichText::new(COPY_LABEL).small().weak();
    if ui.add(Button::new(label).frame(false)).clicked() {
        actions.push(Action::CopyText(text.to_string()));
    }
}

pub fn headers(ui: &mut Ui, view: &ResponseView, actions: &mut Vec<Action>) {
    if view.headers.is_empty() {
        ui.weak(NO_HEADERS_TEXT);
        return;
    }
    ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
        Grid::new(HEADERS_GRID_ID)
            .num_columns(3)
            .spacing(GRID_SPACING)
            .striped(true)
            .show(ui, |ui| {
                for header in &view.headers {
                    ui.label(RichText::new(&header.name).strong());
                    selectable_mono(ui, &header.value);
                    copy_button(ui, &header.value, actions);
                    ui.end_row();
                }
            });
    });
}

pub fn cookies(ui: &mut Ui, view: &ResponseView, actions: &mut Vec<Action>) {
    if view.cookies().next().is_none() {
        ui.weak(NO_COOKIES_TEXT);
        return;
    }
    ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
        Grid::new(COOKIES_GRID_ID)
            .num_columns(4)
            .spacing(GRID_SPACING)
            .striped(true)
            .show(ui, |ui| {
                for raw in view.cookies() {
                    let cookie = CookieEntry::parse(raw);
                    ui.label(RichText::new(cookie.name).strong());
                    selectable_mono(ui, cookie.value);
                    ui.weak(cookie.attributes);
                    copy_button(ui, raw, actions);
                    ui.end_row();
                }
            });
    });
}
