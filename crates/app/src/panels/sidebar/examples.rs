use eframe::egui::text::LayoutJob;
use eframe::egui::{Response, TextFormat, TextStyle, Ui};
use requestor_core::collection::Item;
use requestor_core::response::StatusClass;

use crate::state::{Action, ExampleRef};
use crate::widgets::{list_row, row_label, status_color};

const EXAMPLE_INDENT: f32 = 18.0;
const OPEN_HINT: &str = "Open this saved example in a new tab";
const DELETE_LABEL: &str = "Delete example";

pub(super) fn rows(
    ui: &mut Ui,
    collection_id: &str,
    item_id: &str,
    item: &Item,
    actions: &mut Vec<Action>,
) {
    for summary in item.example_summaries() {
        let reference = ExampleRef {
            collection_id: collection_id.to_string(),
            item_id: item_id.to_string(),
            index: summary.index,
        };
        let title = example_title(ui, summary.code, summary.display_name());
        let response = ui
            .horizontal(|ui| {
                ui.add_space(EXAMPLE_INDENT);
                ui.add(list_row(ui, false, row_label(title)))
            })
            .inner
            .on_hover_text(OPEN_HINT);
        if response.clicked() {
            actions.push(Action::OpenExample(reference.clone()));
        }
        menu(&response, reference, actions);
    }
}

fn menu(response: &Response, reference: ExampleRef, actions: &mut Vec<Action>) {
    response.context_menu(|ui| {
        if ui.button(DELETE_LABEL).clicked() {
            actions.push(Action::DeleteExample(reference));
            ui.close();
        }
    });
}

fn example_title(ui: &Ui, code: Option<u16>, name: &str) -> LayoutJob {
    let font = TextStyle::Small.resolve(ui.style());
    let mut job = LayoutJob::default();
    if let Some(code) = code {
        let color = status_color(ui, StatusClass::of(code));
        job.append(
            &format!("{code} "),
            0.0,
            TextFormat::simple(font.clone(), color),
        );
    }
    let text_color = ui.visuals().weak_text_color();
    job.append(name, 0.0, TextFormat::simple(font, text_color));
    job
}
