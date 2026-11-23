use eframe::egui::{Popup, Ui};
use requestor_core::workspace::{SidebarOrder, SidebarSort};

use crate::widgets::icon_button;

const SORT_GLYPH: &str = "↕";
const SORT_HINT: &str = "Sort";
const FOLDERS_FIRST_LABEL: &str = "Folders first";

pub(super) fn show(ui: &mut Ui, order: &mut SidebarOrder) -> bool {
    let hint = format!("{SORT_HINT}: {}", order.sort.label());
    let mut button = icon_button(ui, SORT_GLYPH, &hint);
    if !order.is_manual() {
        button = button.highlight();
    }
    let mut changed = false;
    Popup::menu(&button).show(|ui| {
        for sort in SidebarSort::ALL {
            changed |= ui
                .radio_value(&mut order.sort, sort, sort.label())
                .changed();
        }
        ui.separator();
        changed |= ui
            .checkbox(&mut order.folders_first, FOLDERS_FIRST_LABEL)
            .changed();
    });
    changed
}
