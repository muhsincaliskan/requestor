use std::sync::Arc;

use eframe::egui::{Context, FontData, FontDefinitions, FontFamily};

const MEDIUM_FAMILY: &str = "medium";
const SEMIBOLD_FAMILY: &str = "semibold";

#[derive(Clone, Copy)]
pub enum Weight {
    Regular,
    Medium,
    SemiBold,
    Monospace,
}

impl Weight {
    const PROPORTIONAL: [Weight; 3] = [Weight::Regular, Weight::Medium, Weight::SemiBold];

    pub fn family(self) -> FontFamily {
        match self {
            Weight::Regular => FontFamily::Proportional,
            Weight::Medium => FontFamily::Name(MEDIUM_FAMILY.into()),
            Weight::SemiBold => FontFamily::Name(SEMIBOLD_FAMILY.into()),
            Weight::Monospace => FontFamily::Monospace,
        }
    }

    fn face(self) -> Option<(&'static str, &'static [u8])> {
        match self {
            Weight::Regular => Some((
                "Inter-Regular",
                include_bytes!("../../../../assets/fonts/Inter-Regular.ttf"),
            )),
            Weight::Medium => Some((
                "Inter-Medium",
                include_bytes!("../../../../assets/fonts/Inter-Medium.ttf"),
            )),
            Weight::SemiBold => Some((
                "Inter-SemiBold",
                include_bytes!("../../../../assets/fonts/Inter-SemiBold.ttf"),
            )),
            Weight::Monospace => None,
        }
    }
}

pub fn install(ctx: &Context) {
    let mut fonts = FontDefinitions::default();
    let fallbacks = fonts
        .families
        .get(&FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();
    for weight in Weight::PROPORTIONAL {
        let Some((name, bytes)) = weight.face() else {
            continue;
        };
        fonts
            .font_data
            .insert(name.to_owned(), Arc::new(FontData::from_static(bytes)));
        let chain = std::iter::once(name.to_owned())
            .chain(fallbacks.iter().cloned())
            .collect();
        fonts.families.insert(weight.family(), chain);
    }
    ctx.set_fonts(fonts);
}
