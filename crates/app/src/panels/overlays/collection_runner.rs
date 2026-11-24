mod results;

use eframe::egui::{Align, Context, Layout, RichText, ScrollArea, TextEdit, Ui};

use super::{button_row, dialog, SECTION_GAP};
use crate::state::{Action, AppState, RunnerForm};
use crate::widgets::{action_button, checkbox, icon_button, method_color};

const DIALOG_ID: &str = "collection-runner";
const TITLE_PREFIX: &str = "Run";
const ITERATIONS_LABEL: &str = "Iterations";
const DELAY_LABEL: &str = "Delay between requests (ms)";
const REQUESTS_LABEL: &str = "Requests";
const RUN_LABEL: &str = "Run";
const RUN_ANYWAY_LABEL: &str = "Run anyway";
const CONFIRM_PROTECTED: &str =
    "Click Run anyway to send these requests to the protected environment.";
const STOP_LABEL: &str = "Stop";
const CLOSE_LABEL: &str = "Close";
const EARLIER_LABEL: &str = "↑";
const LATER_LABEL: &str = "↓";
const EARLIER_HINT: &str = "Run earlier";
const LATER_HINT: &str = "Run later";
const PROTECTED_WARNING: &str =
    "The active environment is protected. These requests may change real data.";
const NO_REQUESTS_TEXT: &str = "This collection has no requests yet.";
const NUMBER_WIDTH: f32 = 56.0;
const LIST_HEIGHT: f32 = 220.0;
const DIALOG_WIDTH: f32 = 560.0;

pub fn show(
    ctx: &Context,
    state: &AppState,
    form: &mut RunnerForm,
    actions: &mut Vec<Action>,
) -> bool {
    let title = format!("{TITLE_PREFIX} {}", form.collection_name);
    let protected = state.sends_to_protected_environment();
    dialog(ctx, DIALOG_ID, &title, |ui| {
        ui.set_width(DIALOG_WIDTH);
        if protected {
            ui.label(RichText::new(PROTECTED_WARNING).color(ui.visuals().warn_fg_color));
        }
        if form.is_confirming() {
            ui.label(RichText::new(CONFIRM_PROTECTED).color(ui.visuals().warn_fg_color));
        }
        if let Some(failure) = form.failure() {
            ui.label(RichText::new(failure).color(ui.visuals().error_fg_color));
        }
        pacing(ui, form);
        ui.add_space(SECTION_GAP);
        steps(ui, form);
        if !form.results().is_empty() || form.is_running() {
            ui.add_space(SECTION_GAP);
            results::show(ui, form);
        }
        buttons(ui, form, protected, actions)
    })
}

fn pacing(ui: &mut Ui, form: &mut RunnerForm) {
    ui.add_enabled_ui(!form.is_running(), |ui| {
        ui.horizontal(|ui| {
            ui.label(ITERATIONS_LABEL);
            number_field(ui, &mut form.iterations_text);
            ui.add_space(SECTION_GAP);
            ui.label(DELAY_LABEL);
            number_field(ui, &mut form.delay_text);
        });
    });
}

fn number_field(ui: &mut Ui, text: &mut String) {
    ui.add(TextEdit::singleline(text).desired_width(NUMBER_WIDTH));
}

fn steps(ui: &mut Ui, form: &mut RunnerForm) {
    ui.strong(REQUESTS_LABEL);
    if form.steps.is_empty() {
        ui.weak(NO_REQUESTS_TEXT);
        return;
    }
    let mut moved = None;
    let running = form.is_running();
    ScrollArea::vertical()
        .id_salt(REQUESTS_LABEL)
        .max_height(LIST_HEIGHT)
        .show(ui, |ui| {
            ui.add_enabled_ui(!running, |ui| {
                let last = form.steps.len() - 1;
                for (index, step) in form.steps.iter_mut().enumerate() {
                    ui.horizontal(|ui| {
                        checkbox(ui, &mut step.enabled, "");
                        ui.label(
                            RichText::new(step.method())
                                .color(method_color(ui, step.method()))
                                .small(),
                        );
                        ui.label(step.name());
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            moved = moved.or(order_buttons(ui, index, last));
                        });
                    });
                }
            });
        });
    if let Some((index, earlier)) = moved {
        form.move_step(index, earlier);
    }
}

fn order_buttons(ui: &mut Ui, index: usize, last: usize) -> Option<(usize, bool)> {
    let later = ui
        .add_enabled_ui(index < last, |ui| icon_button(ui, LATER_LABEL, LATER_HINT))
        .inner;
    let earlier = ui
        .add_enabled_ui(index > 0, |ui| icon_button(ui, EARLIER_LABEL, EARLIER_HINT))
        .inner;
    if later.clicked() {
        Some((index, false))
    } else {
        earlier.clicked().then_some((index, true))
    }
}

fn buttons(ui: &mut Ui, form: &RunnerForm, protected: bool, actions: &mut Vec<Action>) -> bool {
    button_row(ui, |ui| {
        if let Some(job) = form.running_job() {
            if ui.button(STOP_LABEL).clicked() {
                actions.push(Action::StopRun(job));
            }
        } else {
            let run = if form.is_confirming() {
                action_button::destructive(ui, RUN_ANYWAY_LABEL)
            } else if protected {
                action_button::destructive(ui, RUN_LABEL)
            } else {
                action_button::primary(ui, RUN_LABEL)
            };
            if run.clicked() && form.can_start() {
                actions.push(Action::StartRun);
            }
        }
        !ui.button(CLOSE_LABEL).clicked()
    })
}
