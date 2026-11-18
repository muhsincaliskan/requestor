use std::ops::Range;

use eframe::egui::collapsing_header::CollapsingState;
use eframe::egui::text::LayoutJob;
use eframe::egui::{Color32, FontId, Id, Label, ScrollArea, TextFormat, TextStyle, Ui};
use requestor_core::json_tree::{JsonKey, JsonKind, JsonNode};

use crate::response::JsonTreeView;
use crate::widgets::SyntaxPalette;

const SCROLL_ID: &str = "response-tree";
const TREE_ID: &str = "response-tree-root";
const CHUNK_SIZE: usize = 100;
const KEY_SEPARATOR: &str = ": ";
const ROOT_LABEL: &str = "root";
const CHUNK_LABEL_GAP: &str = " … ";
const ROW_PADDING: f32 = 4.0;
const ROW_GAP: f32 = 2.0;

type ChildAt<'a, 'n> = dyn Fn(usize) -> Option<(JsonKey<'n>, &'n JsonNode)> + 'a;

struct TreeStyle {
    palette: SyntaxPalette,
    font: FontId,
}

pub fn show(ui: &mut Ui, root: &JsonTreeView) {
    let style = TreeStyle {
        palette: SyntaxPalette::for_mode(ui.visuals().dark_mode),
        font: TextStyle::Monospace.resolve(ui.style()),
    };
    ScrollArea::both()
        .id_salt(SCROLL_ID)
        .auto_shrink(false)
        .show(ui, |ui| {
            let row_height = ui.text_style_height(&TextStyle::Monospace) + ROW_PADDING;
            let spacing = ui.spacing_mut();
            spacing.interact_size.y = row_height;
            spacing.item_spacing.y = ROW_GAP;
            let id = Id::new(TREE_ID);
            let header = style.row(
                ROOT_LABEL,
                &root.summary(),
                style.palette.json_color(root_kind(root)),
            );
            CollapsingState::load_with_default_open(ui.ctx(), id, true)
                .show_header(ui, |ui| ui.add(Label::new(header).extend()))
                .body(|ui| {
                    let child = |position| root.child(position);
                    range(ui, &style, id, 0..root.child_count(), &child);
                });
        });
}

fn root_kind(root: &JsonTreeView) -> JsonKind {
    match root {
        JsonTreeView::Whole(node) => node.kind(),
        JsonTreeView::Matches(..) => JsonKind::Array,
    }
}

fn node(ui: &mut Ui, style: &TreeStyle, id: Id, key: JsonKey, value: &JsonNode) {
    let color = style.palette.json_color(value.kind());
    let row = style.row(&key.to_string(), &value.summary(), color);
    if !value.is_container() {
        ui.add(Label::new(row).extend());
        return;
    }
    CollapsingState::load_with_default_open(ui.ctx(), id, false)
        .show_header(ui, |ui| ui.add(Label::new(row).extend()))
        .body(|ui| {
            let child = |position| value.child(position);
            range(ui, style, id, 0..value.child_count(), &child);
        });
}

fn range(ui: &mut Ui, style: &TreeStyle, id: Id, positions: Range<usize>, child: &ChildAt) {
    let length = positions.len();
    if length <= CHUNK_SIZE {
        for position in positions {
            if let Some((key, value)) = child(position) {
                node(ui, style, id.with(position), key, value);
            }
        }
        return;
    }
    let mut span = CHUNK_SIZE;
    while length > span * CHUNK_SIZE {
        span *= CHUNK_SIZE;
    }
    for start in positions.clone().step_by(span) {
        let end = (start + span).min(positions.end);
        CollapsingState::load_with_default_open(ui.ctx(), id.with((start, end)), false)
            .show_header(ui, |ui| {
                ui.weak(format!("[{start}{CHUNK_LABEL_GAP}{}]", end - 1))
            })
            .body(|ui| range(ui, style, id, start..end, child));
    }
}

impl TreeStyle {
    fn row(&self, key: &str, summary: &str, value_color: Color32) -> LayoutJob {
        let mut job = LayoutJob::default();
        let key_format = TextFormat::simple(self.font.clone(), self.palette.key_color());
        job.append(key, 0.0, key_format.clone());
        job.append(KEY_SEPARATOR, 0.0, key_format);
        job.append(
            summary,
            0.0,
            TextFormat::simple(self.font.clone(), value_color),
        );
        job
    }
}
