use eframe::egui::{Button, Context, Id, Slider, Ui};
use requestor_core::workspace::InterfaceSettings;

use super::row::{row, toggles, Toggle};
use crate::state::{Action, UpdateCheckOrigin};

const SCALE_ID: &str = "settings-interface-scale";
const SCALE_STEP: f64 = 0.05;
const CHECK_NOW_TITLE: &str = "Check for updates now";
const CHECK_NOW_LABEL: &str = "Check now";
const CHECKING_LABEL: &str = "Checking…";
const INSTALLED_VERSION_LABEL: &str = "Installed version";

const INTERFACE_TOGGLES: [Toggle<InterfaceSettings>; 11] = [
    Toggle {
        label: "Autosave",
        description: "Save changes automatically.",
        field: |settings| &mut settings.autosave,
    },
    Toggle {
        label: "Variable autocomplete",
        description: "Suggest variable names while typing.",
        field: |settings| &mut settings.variable_autocomplete,
    },
    Toggle {
        label: "Response format detection",
        description: "Detect the response format from its content.",
        field: |settings| &mut settings.response_format_detection,
    },
    Toggle {
        label: "Open sidebar item in new tab",
        description: "A single click in the sidebar opens a new tab.",
        field: |settings| &mut settings.open_sidebar_item_in_new_tab,
    },
    Toggle {
        label: "Always ask when closing unsaved tabs",
        description: "Confirm before discarding unsaved changes.",
        field: |settings| &mut settings.ask_when_closing_unsaved_tabs,
    },
    Toggle {
        label: "Show icons with tab names",
        description: "",
        field: |settings| &mut settings.show_icons_with_tab_names,
    },
    Toggle {
        label: "Two-pane view",
        description: "Show the request and response side by side.",
        field: |settings| &mut settings.two_pane_view,
    },
    Toggle {
        label: "Show middle workbench",
        description: "Turn off to give the whole window to the sidebar.",
        field: |settings| &mut settings.show_workbench,
    },
    Toggle {
        label: "Show sidebar",
        description: "Show collections and environments.",
        field: |settings| &mut settings.show_sidebar,
    },
    Toggle {
        label: "Swap left and right sidebars",
        description: "Move the sidebar to the other side.",
        field: |settings| &mut settings.sidebars_swapped,
    },
    Toggle {
        label: "Check for updates",
        description: "Look for a newer version on GitHub when the app starts.",
        field: |settings| &mut settings.check_for_updates,
    },
];

pub fn show(
    ui: &mut Ui,
    settings: &mut InterfaceSettings,
    checking_for_updates: bool,
    actions: &mut Vec<Action>,
) -> bool {
    let changed = toggles(ui, &INTERFACE_TOGGLES, settings);
    check_now(ui, checking_for_updates, actions);
    changed | scale(ui, settings)
}

fn check_now(ui: &mut Ui, checking: bool, actions: &mut Vec<Action>) {
    let description = format!("{INSTALLED_VERSION_LABEL} {}", env!("CARGO_PKG_VERSION"));
    row(ui, CHECK_NOW_TITLE, &description, |ui| {
        let label = if checking {
            CHECKING_LABEL
        } else {
            CHECK_NOW_LABEL
        };
        if ui.add_enabled(!checking, Button::new(label)).clicked() {
            actions.push(Action::CheckForUpdates(UpdateCheckOrigin::Manual));
        }
        false
    });
}

fn scale(ui: &mut Ui, settings: &mut InterfaceSettings) -> bool {
    row(
        ui,
        "Interface scale",
        "Applied when you release the slider.",
        |ui| {
            let id = Id::new(SCALE_ID);
            let mut value = pending_scale(ui.ctx(), id).unwrap_or(settings.interface_scale);
            let slider =
                ui.add(Slider::new(&mut value, InterfaceSettings::SCALE_RANGE).step_by(SCALE_STEP));
            if slider.is_pointer_button_down_on() {
                ui.ctx().data_mut(|data| data.insert_temp(id, value));
                return false;
            }
            ui.ctx().data_mut(|data| data.remove_temp::<f32>(id));
            let changed = value != settings.interface_scale;
            settings.interface_scale = value;
            changed
        },
    )
}

fn pending_scale(ctx: &Context, id: Id) -> Option<f32> {
    ctx.data(|data| data.get_temp::<f32>(id))
}
