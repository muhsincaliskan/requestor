use std::collections::BTreeMap;

use eframe::egui::{vec2, Context, FontFamily, FontId, Margin, Style, TextStyle};

const TEXT_SIZES: [(TextStyle, f32, FontFamily); 5] = [
    (TextStyle::Heading, 19.0, FontFamily::Proportional),
    (TextStyle::Body, 14.0, FontFamily::Proportional),
    (TextStyle::Button, 14.0, FontFamily::Proportional),
    (TextStyle::Small, 11.5, FontFamily::Proportional),
    (TextStyle::Monospace, 13.0, FontFamily::Monospace),
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
        .map(|(text_style, size, family)| (text_style.clone(), FontId::new(*size, family.clone())))
        .collect::<BTreeMap<_, _>>();
    let spacing = &mut style.spacing;
    spacing.item_spacing = vec2(ITEM_SPACING.0, ITEM_SPACING.1);
    spacing.button_padding = vec2(BUTTON_PADDING.0, BUTTON_PADDING.1);
    spacing.interact_size.y = CONTROL_HEIGHT;
    spacing.window_margin = Margin::same(WINDOW_MARGIN);
    spacing.menu_margin = Margin::same(MENU_MARGIN);
    spacing.indent = INDENT;
}
