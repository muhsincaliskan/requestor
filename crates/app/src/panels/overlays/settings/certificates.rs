use eframe::egui::{vec2, Align, Button, Label, Layout, RichText, TextEdit, Ui};
use requestor_core::settings::{ClientCertificate, RequestSettings};

use crate::state::{Action, CertificateFile, CertificatePart};

const INTRO: &str = "Requestor presents a client certificate to hosts that ask for one (mutual TLS). Use PEM files; a key file is only needed when the key is not in the certificate file.";
const EMPTY_TEXT: &str = "No client certificates yet.";
const ADD_LABEL: &str = "Add certificate";
const REMOVE_LABEL: &str = "Remove";
const CHOOSE_LABEL: &str = "Choose…";
const HOST_LABEL: &str = "Host";
const CERTIFICATE_LABEL: &str = "Certificate";
const KEY_LABEL: &str = "Key";
const HOST_HINT: &str = "api.example.com, *.example.com:8443 or *";
const CERTIFICATE_HINT: &str = "client.pem";
const KEY_HINT: &str = "Optional key.pem";
const LABEL_WIDTH: f32 = 84.0;
const CHOOSE_WIDTH: f32 = 84.0;
const ENTRY_GAP: f32 = 12.0;
const SCROLLBAR_GUTTER: f32 = 14.0;

pub fn show(ui: &mut Ui, settings: &mut RequestSettings, actions: &mut Vec<Action>) -> bool {
    ui.label(RichText::new(INTRO).weak().small());
    ui.add_space(ENTRY_GAP);
    if settings.client_certificates.is_empty() {
        ui.weak(EMPTY_TEXT);
    }
    let mut changed = false;
    let mut removed = None;
    for (index, certificate) in settings.client_certificates.iter_mut().enumerate() {
        ui.push_id(index, |ui| {
            changed |= entry(ui, index, certificate, actions);
            if remove_button(ui) {
                removed = Some(index);
            }
        });
        ui.separator();
    }
    if let Some(index) = removed {
        settings.client_certificates.remove(index);
        changed = true;
    }
    ui.add_space(ENTRY_GAP);
    if ui.button(ADD_LABEL).clicked() {
        settings
            .client_certificates
            .push(ClientCertificate::default());
        changed = true;
    }
    changed
}

fn entry(
    ui: &mut Ui,
    index: usize,
    certificate: &mut ClientCertificate,
    actions: &mut Vec<Action>,
) -> bool {
    let mut changed = field(ui, HOST_LABEL, |ui| {
        let width = ui.available_width() - SCROLLBAR_GUTTER;
        ui.add(
            TextEdit::singleline(&mut certificate.host)
                .hint_text(HOST_HINT)
                .desired_width(width),
        )
        .changed()
    });
    let mut path = certificate.certificate_path_text();
    if path_field(ui, CERTIFICATE_LABEL, &mut path, CERTIFICATE_HINT, || {
        actions.push(Action::ChooseCertificateFile(CertificateFile {
            index,
            part: CertificatePart::Certificate,
        }));
    }) {
        certificate.set_certificate_path_text(&path);
        changed = true;
    }
    let mut key = certificate.key_path_text();
    if path_field(ui, KEY_LABEL, &mut key, KEY_HINT, || {
        actions.push(Action::ChooseCertificateFile(CertificateFile {
            index,
            part: CertificatePart::Key,
        }));
    }) {
        certificate.set_key_path_text(&key);
        changed = true;
    }
    changed
}

fn remove_button(ui: &mut Ui) -> bool {
    ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
        ui.add_space(SCROLLBAR_GUTTER);
        ui.small_button(REMOVE_LABEL).clicked()
    })
    .inner
}

fn field(ui: &mut Ui, label: &str, control: impl FnOnce(&mut Ui) -> bool) -> bool {
    ui.horizontal(|ui| {
        let size = vec2(LABEL_WIDTH, ui.spacing().interact_size.y);
        ui.allocate_ui_with_layout(size, Layout::left_to_right(Align::Center), |ui| {
            ui.set_min_size(size);
            ui.add(Label::new(label));
        });
        control(ui)
    })
    .inner
}

fn path_field(
    ui: &mut Ui,
    label: &str,
    text: &mut String,
    hint: &str,
    choose: impl FnOnce(),
) -> bool {
    field(ui, label, |ui| {
        let width =
            ui.available_width() - CHOOSE_WIDTH - SCROLLBAR_GUTTER - ui.spacing().item_spacing.x;
        let changed = ui
            .add(
                TextEdit::singleline(text)
                    .hint_text(hint)
                    .desired_width(width),
            )
            .changed();
        let size = vec2(CHOOSE_WIDTH, ui.spacing().interact_size.y);
        if ui.add_sized(size, Button::new(CHOOSE_LABEL)).clicked() {
            choose();
        }
        changed
    })
}
