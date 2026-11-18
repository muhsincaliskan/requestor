use super::{BodyLayout, JsonInspection, ResponseView};

pub struct BodyPresentation {
    layout: BodyLayout,
    inspection: JsonInspection,
}

impl ResponseView {
    pub fn take_presentation(&mut self) -> BodyPresentation {
        let mut inspection = std::mem::take(&mut self.inspection);
        inspection.release();
        BodyPresentation {
            layout: self.layout,
            inspection,
        }
    }

    pub fn with_presentation(mut self, presentation: Option<BodyPresentation>) -> Self {
        if let Some(presentation) = presentation {
            self.layout = presentation.layout;
            self.inspection = presentation.inspection;
        }
        self
    }
}
