use eframe::egui::{RichText, Ui};
use requestor_core::collection::InheritedHeader;
use requestor_core::draft::{AuthDraft, AuthKind, RequestDraft};

use crate::state::InheritedAuth;

const INHERITED_HEADERS_TITLE: &str = "From folders and the collection";
const NOTHING_INHERITED: &str =
    "No folder or the collection sets authorization, so this request is sent without it.";
const SECTION_GAP: f32 = 10.0;
const REDACTED: &str = "••••••";

pub fn auth(ui: &mut Ui, draft: &AuthDraft, inherited: Option<&InheritedAuth>) {
    if draft.kind != AuthKind::Inherit {
        return;
    }
    match inherited {
        Some(inherited) => ui.weak(format!(
            "Uses {} from \"{}\".",
            inherited.kind, inherited.owner
        )),
        None => ui.weak(NOTHING_INHERITED),
    };
}

pub fn headers(ui: &mut Ui, draft: &RequestDraft, inherited: &[InheritedHeader]) {
    let own_names: Vec<&str> = draft
        .headers
        .iter()
        .filter(|row| row.enabled)
        .map(|row| row.key.as_str())
        .collect();
    let effective = InheritedHeader::effective(inherited, &own_names);
    if effective.is_empty() {
        return;
    }
    ui.add_space(SECTION_GAP);
    ui.strong(INHERITED_HEADERS_TITLE);
    for entry in effective {
        let key = entry.header.key.as_deref().unwrap_or_default();
        let value = if entry.header.has_secret_name() {
            REDACTED
        } else {
            entry.header.value.as_deref().unwrap_or_default()
        };
        ui.horizontal(|ui| {
            ui.monospace(format!("{key}: {value}"));
            ui.label(
                RichText::new(format!("from {}", entry.owner))
                    .weak()
                    .small(),
            );
        });
    }
}
