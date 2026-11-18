use eframe::egui::Color32;

const LIGHT_LABEL: Color32 = Color32::WHITE;
const DARK_LABEL: Color32 = Color32::from_rgb(16, 18, 22);
const LINEAR_THRESHOLD: f32 = 0.039_28;
const LINEAR_DIVISOR: f32 = 12.92;
const GAMMA_OFFSET: f32 = 0.055;
const GAMMA_SCALE: f32 = 1.055;
const GAMMA_EXPONENT: f32 = 2.4;
const RED_WEIGHT: f32 = 0.2126;
const GREEN_WEIGHT: f32 = 0.7152;
const BLUE_WEIGHT: f32 = 0.0722;
const FLARE: f32 = 0.05;
#[cfg(test)]
pub(super) const BODY_TEXT_MINIMUM: f32 = 4.5;

pub fn label_on(fill: Color32) -> Color32 {
    match contrast(LIGHT_LABEL, fill) >= contrast(DARK_LABEL, fill) {
        true => LIGHT_LABEL,
        false => DARK_LABEL,
    }
}

pub fn contrast(first: Color32, second: Color32) -> f32 {
    let (a, b) = (luminance(first), luminance(second));
    (a.max(b) + FLARE) / (a.min(b) + FLARE)
}

fn luminance(color: Color32) -> f32 {
    RED_WEIGHT * channel(color.r())
        + GREEN_WEIGHT * channel(color.g())
        + BLUE_WEIGHT * channel(color.b())
}

fn channel(value: u8) -> f32 {
    let value = f32::from(value) / f32::from(u8::MAX);
    match value <= LINEAR_THRESHOLD {
        true => value / LINEAR_DIVISOR,
        false => ((value + GAMMA_OFFSET) / GAMMA_SCALE).powf(GAMMA_EXPONENT),
    }
}

#[cfg(test)]
mod tests {
    use eframe::egui::Color32;

    use super::{contrast, label_on, BODY_TEXT_MINIMUM, DARK_LABEL, LIGHT_LABEL};
    use crate::theme::palette::{DAYLIGHT, MIDNIGHT, NIGHT, SEPIA};

    #[test]
    fn black_on_white_is_the_maximum_contrast() {
        assert!((contrast(Color32::BLACK, Color32::WHITE) - 21.0).abs() < 0.01);
    }

    #[test]
    fn labels_pick_the_more_readable_color() {
        assert_eq!(label_on(Color32::from_rgb(37, 99, 235)), LIGHT_LABEL);
        assert_eq!(label_on(Color32::from_rgb(76, 139, 255)), DARK_LABEL);
    }

    #[test]
    fn every_palette_keeps_text_readable() {
        for palette in [DAYLIGHT, NIGHT, MIDNIGHT, SEPIA] {
            for background in [palette.panel, palette.window, palette.input] {
                assert!(contrast(palette.text, background) >= BODY_TEXT_MINIMUM);
                assert!(contrast(palette.muted, background) >= BODY_TEXT_MINIMUM);
                assert!(contrast(palette.danger, background) >= BODY_TEXT_MINIMUM);
            }
            for fill in [palette.accent, palette.danger] {
                assert!(contrast(label_on(fill), fill) >= BODY_TEXT_MINIMUM);
            }
        }
    }
}
