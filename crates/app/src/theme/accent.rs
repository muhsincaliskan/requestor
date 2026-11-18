use eframe::egui::Color32;
use requestor_core::workspace::AccentName;

pub fn accent_color(accent: AccentName, dark_mode: bool) -> Option<Color32> {
    let (on_light, on_dark) = match accent {
        AccentName::Theme => return None,
        AccentName::Blue => ((29, 88, 214), (96, 150, 255)),
        AccentName::Teal => ((0, 118, 118), (45, 190, 180)),
        AccentName::Green => ((22, 122, 58), (84, 190, 110)),
        AccentName::Purple => ((118, 62, 200), (170, 140, 250)),
        AccentName::Orange => ((164, 74, 0), (245, 150, 70)),
        AccentName::Pink => ((190, 30, 110), (245, 130, 180)),
    };
    let (red, green, blue) = if dark_mode { on_dark } else { on_light };
    Some(Color32::from_rgb(red, green, blue))
}

#[cfg(test)]
mod tests {
    use requestor_core::workspace::AccentName;

    use super::accent_color;
    use crate::theme::contrast::{contrast, label_on, BODY_TEXT_MINIMUM};
    use crate::theme::palette::{DAYLIGHT, MIDNIGHT, NIGHT, SEPIA};

    #[test]
    fn the_theme_accent_has_no_override() {
        assert_eq!(accent_color(AccentName::Theme, true), None);
        assert_eq!(accent_color(AccentName::Theme, false), None);
    }

    #[test]
    fn every_accent_stays_readable_as_text_and_as_a_fill() {
        let modes = [(false, [DAYLIGHT, SEPIA]), (true, [NIGHT, MIDNIGHT])];
        let named = AccentName::ALL
            .into_iter()
            .filter(|accent| *accent != AccentName::Theme);
        for accent in named {
            for (dark_mode, palettes) in &modes {
                let color = accent_color(accent, *dark_mode).unwrap();
                assert!(contrast(label_on(color), color) >= BODY_TEXT_MINIMUM);
                for palette in palettes {
                    for background in [palette.panel, palette.window, palette.input] {
                        assert!(
                            contrast(color, background) >= BODY_TEXT_MINIMUM,
                            "{accent:?}"
                        );
                    }
                }
            }
        }
    }
}
