use eframe::egui::{
    Align, ComboBox, Event, Frame, Key, Layout, Response, RichText, Stroke, StrokeKind, TextEdit,
    TextStyle, Ui,
};
use requestor_core::method::HttpMethod;

use super::EditorContext;
use crate::state::{Action, RequestTab};
use crate::widgets::variable_text_edit::{VariableHints, VariableTextEdit};
use crate::widgets::{action_button, method_color, PROTECTED_SEND_HINT};

const NAME_HINT: &str = "Request name";
const URL_HINT: &str = "Enter a URL or paste a cURL command";
const SEND_LABEL: &str = "Send";
const CANCEL_LABEL: &str = "Cancel";
const CONNECT_LABEL: &str = "Connect";
const DISCONNECT_LABEL: &str = "Disconnect";
const SAVE_LABEL: &str = "Save";
const MORE_LABEL: &str = "…";
const COPY_CURL_SECRETS_LABEL: &str = "Copy as cURL with secrets";
const GENERATE_CODE_LABEL: &str = "Generate code…";
const SHOW_CODE_PANEL_LABEL: &str = "Show code panel";
const HIDE_CODE_PANEL_LABEL: &str = "Hide code panel";
const QUICK_CURL_LABEL: &str = "Copy cURL";
const QUICK_CURL_HINT: &str =
    "Copy as cURL. Secrets show as <redacted>; Alt-click to include them.";
const METHOD_SALT: &str = "request-method";
const URL_SALT: &str = "request-url";
const METHOD_WIDTH: f32 = 96.0;
const PROTECTED_FRAME_GAP: f32 = 2.0;
const PROTECTED_FRAME_WIDTH: f32 = 1.5;

pub fn show(
    ui: &mut Ui,
    open: &mut RequestTab,
    context: &EditorContext,
    actions: &mut Vec<Action>,
) -> bool {
    let name_changed = ui
        .add(
            TextEdit::singleline(&mut open.draft.name)
                .hint_text(NAME_HINT)
                .font(TextStyle::Heading)
                .frame(Frame::new())
                .desired_width(f32::INFINITY),
        )
        .changed();
    let request_changed = ui
        .horizontal(|ui| request_line(ui, open, context, actions))
        .inner;
    name_changed | request_changed
}

fn request_line(
    ui: &mut Ui,
    open: &mut RequestTab,
    context: &EditorContext,
    actions: &mut Vec<Action>,
) -> bool {
    let method_changed = method_picker(ui, open);
    let url_changed = ui
        .with_layout(Layout::right_to_left(Align::Center), |ui| {
            buttons(ui, open, context, actions);
            url_field(ui, open, context.hints, actions)
        })
        .inner;
    method_changed | url_changed
}

fn method_picker(ui: &mut Ui, open: &mut RequestTab) -> bool {
    let before = open.draft.method.clone();
    ComboBox::from_id_salt(METHOD_SALT)
        .width(METHOD_WIDTH)
        .selected_text(colored_method(ui, &open.draft.method))
        .show_ui(ui, |ui| {
            for method in HttpMethod::COMMON.iter().map(HttpMethod::as_str) {
                let label = colored_method(ui, method);
                ui.selectable_value(&mut open.draft.method, method.to_string(), label);
            }
            if !HttpMethod::is_common(&open.draft.method) {
                let custom = open.draft.method.clone();
                let label = colored_method(ui, &custom);
                ui.selectable_value(&mut open.draft.method, custom, label);
            }
        });
    open.draft.method != before
}

fn colored_method(ui: &Ui, method: &str) -> RichText {
    RichText::new(method)
        .strong()
        .color(method_color(ui, method))
}

fn url_field(
    ui: &mut Ui,
    open: &mut RequestTab,
    hints: &VariableHints,
    actions: &mut Vec<Action>,
) -> bool {
    let response = VariableTextEdit::new(&mut open.draft.url, hints)
        .hint_text(URL_HINT)
        .desired_width(ui.available_width())
        .id(ui.id().with(URL_SALT))
        .show(ui);
    let submitted = response.lost_focus() && ui.input(|input| input.key_pressed(Key::Enter));
    if submitted && !open.is_sending() && !open.is_connected() {
        actions.push(Action::Send(open.id));
    }
    let adopted =
        response.changed() && pasted(ui).is_some_and(|text| open.draft.adopt_pasted_curl(&text));
    if response.changed() && !adopted {
        open.draft.url_edited();
    }
    response.changed()
}

fn pasted(ui: &Ui) -> Option<String> {
    ui.input(|input| {
        input.events.iter().find_map(|event| match event {
            Event::Paste(text) => Some(text.clone()),
            _ => None,
        })
    })
}

fn buttons(ui: &mut Ui, open: &RequestTab, context: &EditorContext, actions: &mut Vec<Action>) {
    ui.menu_button(MORE_LABEL, |ui| {
        copy_menu(ui, open, context.code_panel_open, actions)
    });
    if ui
        .button(QUICK_CURL_LABEL)
        .on_hover_text(QUICK_CURL_HINT)
        .clicked()
    {
        actions.push(Action::CopyAsCurl {
            tab: open.id,
            include_secrets: ui.input(|input| input.modifiers.alt),
        });
    }
    if ui.button(SAVE_LABEL).clicked() {
        actions.push(Action::SaveTab(open.id));
    }
    let stops = open.is_sending() || open.is_connected();
    let clicked = if stops {
        action_button::destructive(ui, stop_label(open)).clicked()
    } else {
        start_button(ui, open, context.protected).clicked()
    };
    if clicked {
        actions.push(if stops {
            Action::CancelSend(open.id)
        } else {
            Action::Send(open.id)
        });
    }
}

fn start_button(ui: &mut Ui, open: &RequestTab, protected: bool) -> Response {
    let button = action_button::primary(ui, start_label(open));
    if protected {
        let frame = button.rect.expand(PROTECTED_FRAME_GAP);
        let stroke = Stroke::new(PROTECTED_FRAME_WIDTH, ui.visuals().error_fg_color);
        ui.painter().rect_stroke(
            frame,
            ui.visuals().widgets.inactive.corner_radius,
            stroke,
            StrokeKind::Outside,
        );
        return button.on_hover_text(PROTECTED_SEND_HINT);
    }
    button
}

fn start_label(open: &RequestTab) -> &'static str {
    if open.draft.is_websocket() {
        CONNECT_LABEL
    } else {
        SEND_LABEL
    }
}

fn stop_label(open: &RequestTab) -> &'static str {
    if open.is_connected() {
        DISCONNECT_LABEL
    } else {
        CANCEL_LABEL
    }
}

fn copy_menu(ui: &mut Ui, open: &RequestTab, code_panel_open: bool, actions: &mut Vec<Action>) {
    if ui.button(GENERATE_CODE_LABEL).clicked() {
        actions.push(Action::OpenCode(open.id));
        ui.close();
    }
    let panel_label = if code_panel_open {
        HIDE_CODE_PANEL_LABEL
    } else {
        SHOW_CODE_PANEL_LABEL
    };
    if ui.button(panel_label).clicked() {
        actions.push(Action::ToggleCodePanel);
        ui.close();
    }
    ui.separator();
    if ui.button(COPY_CURL_SECRETS_LABEL).clicked() {
        actions.push(Action::CopyAsCurl {
            tab: open.id,
            include_secrets: true,
        });
        ui.close();
    }
}
