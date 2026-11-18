use eframe::egui::{ComboBox, Context, Ui};
use requestor_core::environment::{
    Environment, EnvironmentColor, EnvironmentValue, PRODUCTION_NAMES,
};

use super::dialog;
use super::table::{
    delete_cell, edit_rows, enabled_cell, footer, forget_reveal, name_field, reveal_toggle,
    text_cell, RowEdit, KEY_HINT, VALUE_HINT,
};
use crate::state::{Action, AppState};
use crate::widgets::environment_swatch;

const DIALOG_ID: &str = "environment-editor";
const TABLE_ID: &str = "environment-values";
const REVEAL_ID: &str = "environment-reveal-secrets";
const TITLE: &str = "Edit environment";
const SECRET_LABEL: &str = "Secret";
const COLUMNS: usize = 5;
const COLOR_ID: &str = "environment-color";
const COLOR_LABEL: &str = "Color";
const NO_COLOR_LABEL: &str = "None";
const AUTOMATIC_RED_LABEL: &str = "Red (protected)";
const PROTECT_LABEL: &str = "Ask before sending";
const PROTECT_HINT: &str = "Use this for production. Send gets a red frame and asks first. Protected by default when the name contains:";
const NAME_LIST_SEPARATOR: &str = ", ";

pub fn show(
    ctx: &Context,
    state: &mut AppState,
    environment_id: &str,
    actions: &mut Vec<Action>,
) -> bool {
    let Some(environment) = state.workspace.environment_mut(environment_id) else {
        return false;
    };
    let stays_open = dialog(ctx, DIALOG_ID, TITLE, |ui| {
        let mut changed = name_field(ui, &mut environment.name);
        changed |= marking(ui, environment);
        let reveal = reveal_toggle(ui, REVEAL_ID);
        changed |= edit_rows(
            ui,
            TABLE_ID,
            COLUMNS,
            &mut environment.values,
            |ui, entry| row(ui, entry, reveal),
        );
        let (add, done) = footer(ui);
        if add {
            environment.values.push(EnvironmentValue::blank());
            changed = true;
        }
        if changed {
            actions.push(Action::EnvironmentEdited(environment_id.to_string()));
        }
        !done
    });
    if !stays_open {
        forget_reveal(ctx, REVEAL_ID);
    }
    stays_open
}

fn row(ui: &mut Ui, entry: &mut EnvironmentValue, reveal: bool) -> RowEdit {
    let mut changed = false;
    if let Some(enabled) = enabled_cell(ui, entry.is_enabled()) {
        entry.set_enabled(enabled);
        changed = true;
    }
    changed |= text_cell(ui, &mut entry.key, KEY_HINT, false);
    changed |= value_cell(ui, entry, reveal);
    changed |= secret_cell(ui, entry);
    RowEdit::from_flags(changed, delete_cell(ui))
}

fn value_cell(ui: &mut Ui, entry: &mut EnvironmentValue, reveal: bool) -> bool {
    let mut text = entry.value_text();
    let masked = entry.is_secret() && !reveal;
    let edited = text_cell(ui, &mut text, VALUE_HINT, masked);
    if edited {
        entry.set_value_text(&text);
    }
    edited
}

fn secret_cell(ui: &mut Ui, entry: &mut EnvironmentValue) -> bool {
    let mut secret = entry.is_secret();
    let toggled = ui.checkbox(&mut secret, SECRET_LABEL).changed();
    if toggled {
        entry.set_secret(secret);
    }
    toggled
}

fn marking(ui: &mut Ui, environment: &mut Environment) -> bool {
    ui.horizontal(|ui| {
        let mut changed = color_picker(ui, environment);
        let mut protected = environment.is_protected();
        if ui
            .checkbox(&mut protected, PROTECT_LABEL)
            .on_hover_text(format!(
                "{PROTECT_HINT} {}",
                PRODUCTION_NAMES.join(NAME_LIST_SEPARATOR)
            ))
            .changed()
        {
            environment.set_protected(protected);
            changed = true;
        }
        changed
    })
    .inner
}

fn color_picker(ui: &mut Ui, environment: &mut Environment) -> bool {
    ui.label(COLOR_LABEL);
    if let Some(color) = environment.color() {
        environment_swatch(ui, color);
    }
    let unset = if environment.is_protected() {
        AUTOMATIC_RED_LABEL
    } else {
        NO_COLOR_LABEL
    };
    let mut chosen = environment.chosen_color();
    let selected = chosen.map_or(unset, EnvironmentColor::label);
    let mut changed = false;
    ComboBox::from_id_salt(COLOR_ID)
        .selected_text(selected)
        .show_ui(ui, |ui| {
            changed |= ui.selectable_value(&mut chosen, None, unset).changed();
            for color in EnvironmentColor::ALL {
                changed |= ui
                    .selectable_value(&mut chosen, Some(color), color.label())
                    .changed();
            }
        });
    if changed {
        environment.set_color(chosen);
    }
    changed
}
