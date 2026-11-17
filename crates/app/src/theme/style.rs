use std::collections::BTreeMap;

use eframe::egui::{vec2, Context, FontId, Margin, Style, TextStyle};

use super::typeface::Weight;

const TEXT_SIZES: [(TextStyle, f32, Weight); 5] = [
    (TextStyle::Heading, 19.0, Weight::SemiBold),
    (TextStyle::Body, 13.5, Weight::Regular),
    (TextStyle::Button, 13.5, Weight::Medium),
    (TextStyle::Small, 11.5, Weight::Regular),
    (TextStyle::Monospace, 13.0, Weight::Monospace),
];
const ITEM_SPACING: (f32, f32) = (8.0, 6.0);
const BUTTON_PADDING: (f32, f32) = (12.0, 5.0);
const CONTROL_HEIGHT: f32 = 28.0;
const WINDOW_MARGIN: i8 = 14;
const MENU_MARGIN: i8 = 6;
const INDENT: f32 = 16.0;

pub fn apply(ctx: &Context) {
    ctx.all_styles_mut(restyle);
}

fn restyle(style: &mut Style) {
    style.text_styles = TEXT_SIZES
        .iter()
        .map(|(text_style, size, weight)| (text_style.clone(), FontId::new(*size, weight.family())))
        .collect::<BTreeMap<_, _>>();
    let spacing = &mut style.spacing;
    spacing.item_spacing = vec2(ITEM_SPACING.0, ITEM_SPACING.1);
    spacing.button_padding = vec2(BUTTON_PADDING.0, BUTTON_PADDING.1);
    spacing.interact_size.y = CONTROL_HEIGHT;
    spacing.window_margin = Margin::same(WINDOW_MARGIN);
    spacing.menu_margin = Margin::same(MENU_MARGIN);
    spacing.indent = INDENT;
}
