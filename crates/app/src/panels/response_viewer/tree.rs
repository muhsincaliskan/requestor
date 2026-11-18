use eframe::egui::collapsing_header::CollapsingState;
use eframe::egui::text::LayoutJob;
use eframe::egui::{FontId, Id, Label, ScrollArea, TextFormat, TextStyle, Ui};
use requestor_core::json_tree::{JsonKey, JsonNode};

use crate::widgets::SyntaxPalette;

const SCROLL_ID: &str = "response-tree";
const TREE_ID: &str = "response-tree-root";
const CHUNK_SIZE: usize = 100;
const KEY_SEPARATOR: &str = ": ";
const ROOT_LABEL: &str = "root";
const ROW_PADDING: f32 = 4.0;
const ROW_GAP: f32 = 2.0;

struct TreeStyle {
    palette: SyntaxPalette,
    font: FontId,
}

pub fn show(ui: &mut Ui, root: &JsonNode) {
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
            node(ui, &style, (Id::new(TREE_ID), None), root);
        });
}

fn node(ui: &mut Ui, style: &TreeStyle, (id, key): (Id, Option<JsonKey>), value: &JsonNode) {
    let row = style.row(key, value);
    if !value.is_container() {
        ui.add(Label::new(row).extend());
        return;
    }
    CollapsingState::load_with_default_open(ui.ctx(), id, key.is_none())
        .show_header(ui, |ui| ui.add(Label::new(row).extend()))
        .body(|ui| children(ui, style, id, value));
}

fn children(ui: &mut Ui, style: &TreeStyle, id: Id, value: &JsonNode) {
    let count = value.child_count();
    if count <= CHUNK_SIZE {
        child_range(ui, style, id, value, 0..count);
        return;
    }
    for start in (0..count).step_by(CHUNK_SIZE) {
        let end = (start + CHUNK_SIZE).min(count);
        CollapsingState::load_with_default_open(ui.ctx(), id.with(start), false)
            .show_header(ui, |ui| ui.weak(format!("[{start} … {}]", end - 1)))
            .body(|ui| child_range(ui, style, id, value, start..end));
    }
}

fn child_range(
    ui: &mut Ui,
    style: &TreeStyle,
    id: Id,
    value: &JsonNode,
    positions: std::ops::Range<usize>,
) {
    for position in positions {
        if let Some((key, child)) = value.child(position) {
            node(ui, style, (id.with(key_text(key)), Some(key)), child);
        }
    }
}

fn key_text(key: JsonKey) -> String {
    match key {
        JsonKey::Name(name) => name.to_string(),
        JsonKey::Index(index) => index.to_string(),
    }
}

impl TreeStyle {
    fn row(&self, key: Option<JsonKey>, value: &JsonNode) -> LayoutJob {
        let mut job = LayoutJob::default();
        let key = key.map_or(ROOT_LABEL.to_string(), key_text);
        let key_format = TextFormat::simple(self.font.clone(), self.palette.key_color());
        job.append(&key, 0.0, key_format.clone());
        job.append(KEY_SEPARATOR, 0.0, key_format);
        let value_color = self.palette.json_color(value.kind());
        job.append(
            &value.summary(),
            0.0,
            TextFormat::simple(self.font.clone(), value_color),
        );
        job
    }
}
