use eframe::egui::{Align, ComboBox, Event, Key, Layout, TextEdit, Ui};
use requestor_core::method::HttpMethod;

use crate::state::{Action, RequestTab};
use crate::widgets::action_button;
use crate::widgets::variable_text_edit::VariableTextEdit;

const NAME_HINT: &str = "Request name";
const URL_HINT: &str = "Enter a URL or paste a cURL command";
const SEND_LABEL: &str = "Send";
const CANCEL_LABEL: &str = "Cancel";
const SAVE_LABEL: &str = "Save";
const MORE_LABEL: &str = "…";
const COPY_CURL_LABEL: &str = "Copy as cURL";
const COPY_CURL_SECRETS_LABEL: &str = "Copy as cURL with secrets";
const METHOD_SALT: &str = "request-method";
const URL_SALT: &str = "request-url";
const METHOD_WIDTH: f32 = 96.0;

pub fn show(
    ui: &mut Ui,
    open: &mut RequestTab,
    names: &[String],
    actions: &mut Vec<Action>,
) -> bool {
    let name_changed = ui
        .add(
            TextEdit::singleline(&mut open.draft.name)
                .hint_text(NAME_HINT)
                .desired_width(f32::INFINITY),
        )
        .changed();
    let request_changed = ui
        .horizontal(|ui| request_line(ui, open, names, actions))
        .inner;
    name_changed | request_changed
}

fn request_line(
    ui: &mut Ui,
    open: &mut RequestTab,
    names: &[String],
    actions: &mut Vec<Action>,
) -> bool {
    let method_changed = method_picker(ui, open);
    let url_changed = ui
        .with_layout(Layout::right_to_left(Align::Center), |ui| {
            buttons(ui, open, actions);
            url_field(ui, open, names, actions)
        })
        .inner;
    method_changed | url_changed
}

fn method_picker(ui: &mut Ui, open: &mut RequestTab) -> bool {
    let before = open.draft.method.clone();
    ComboBox::from_id_salt(METHOD_SALT)
        .width(METHOD_WIDTH)
        .selected_text(open.draft.method.as_str())
        .show_ui(ui, |ui| {
            for method in HttpMethod::COMMON.iter().map(HttpMethod::as_str) {
                ui.selectable_value(&mut open.draft.method, method.to_string(), method);
            }
            if !HttpMethod::is_common(&open.draft.method) {
                let custom = open.draft.method.clone();
                ui.selectable_value(&mut open.draft.method, custom.clone(), custom);
            }
        });
    open.draft.method != before
}

fn url_field(
    ui: &mut Ui,
    open: &mut RequestTab,
    names: &[String],
    actions: &mut Vec<Action>,
) -> bool {
    let response = VariableTextEdit::new(&mut open.draft.url, names)
        .hint_text(URL_HINT)
        .desired_width(ui.available_width())
        .id(ui.id().with(URL_SALT))
        .show(ui);
    let submitted = response.lost_focus() && ui.input(|input| input.key_pressed(Key::Enter));
    if submitted && !open.is_sending() {
        actions.push(Action::Send(open.id));
    }
    if response.changed() && !(pasted(ui) && open.draft.adopt_pasted_curl()) {
        open.draft.url_edited();
    }
    response.changed()
}

fn pasted(ui: &Ui) -> bool {
    ui.input(|input| {
        input
            .events
            .iter()
            .any(|event| matches!(event, Event::Paste(_)))
    })
}

fn buttons(ui: &mut Ui, open: &RequestTab, actions: &mut Vec<Action>) {
    ui.menu_button(MORE_LABEL, |ui| copy_menu(ui, open, actions));
    if ui.button(SAVE_LABEL).clicked() {
        actions.push(Action::SaveTab(open.id));
    }
    let clicked = if open.is_sending() {
        action_button::destructive(ui, CANCEL_LABEL).clicked()
    } else {
        action_button::primary(ui, SEND_LABEL).clicked()
    };
    if clicked {
        actions.push(if open.is_sending() {
            Action::CancelSend(open.id)
        } else {
            Action::Send(open.id)
        });
    }
}

fn copy_menu(ui: &mut Ui, open: &RequestTab, actions: &mut Vec<Action>) {
    for (label, include_secrets) in [(COPY_CURL_LABEL, false), (COPY_CURL_SECRETS_LABEL, true)] {
        if ui.button(label).clicked() {
            actions.push(Action::CopyAsCurl {
                tab: open.id,
                include_secrets,
            });
            ui.close();
        }
    }
}
