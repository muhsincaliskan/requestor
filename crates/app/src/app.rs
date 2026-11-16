use eframe::egui;
use egui::{ComboBox, Id, ScrollArea, TextStyle, TextWrapMode, Ui, WidgetText};
use egui_dock::{DockArea, DockState, NodeIndex, Style, TabViewer};
use requestor_core::method::HttpMethod;

use crate::highlight::JsonHighlighter;
use crate::response_buffer::ResponseBuffer;

const REQUEST_TAB_TITLE: &str = "Request";
const RESPONSE_TAB_TITLE: &str = "Response";
const SPLIT_FRACTION: f32 = 0.45;
const DEFAULT_URL: &str = "https://api.example.com/users";
const METHOD_COMBO_ID: &str = "method";
const SPIKE_STATUS_TEXT: &str = "200 OK";
const METHODS: [HttpMethod; 5] = [
    HttpMethod::Get,
    HttpMethod::Post,
    HttpMethod::Put,
    HttpMethod::Patch,
    HttpMethod::Delete,
];

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Request,
    Response,
}

pub struct RequestorApp {
    dock: DockState<Tab>,
    session: Session,
}

struct Session {
    method: HttpMethod,
    url: String,
    response: ResponseBuffer,
    highlighter: JsonHighlighter,
}

impl RequestorApp {
    pub fn new(response: ResponseBuffer) -> Self {
        let mut dock = DockState::new(vec![Tab::Request]);
        dock.main_surface_mut()
            .split_right(NodeIndex::root(), SPLIT_FRACTION, vec![Tab::Response]);
        let session = Session {
            method: HttpMethod::Get,
            url: DEFAULT_URL.to_string(),
            response,
            highlighter: JsonHighlighter::new(),
        };
        RequestorApp { dock, session }
    }
}

impl eframe::App for RequestorApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        DockArea::new(&mut self.dock)
            .style(Style::from_egui(ui.style().as_ref()))
            .show_inside(ui, &mut self.session);
    }
}

impl TabViewer for Session {
    type Tab = Tab;

    fn id(&mut self, tab: &mut Tab) -> Id {
        Id::new(match tab {
            Tab::Request => REQUEST_TAB_TITLE,
            Tab::Response => RESPONSE_TAB_TITLE,
        })
    }

    fn title(&mut self, tab: &mut Tab) -> WidgetText {
        match tab {
            Tab::Request => REQUEST_TAB_TITLE.into(),
            Tab::Response => RESPONSE_TAB_TITLE.into(),
        }
    }

    fn is_closeable(&self, _tab: &Tab) -> bool {
        false
    }

    fn ui(&mut self, ui: &mut Ui, tab: &mut Tab) {
        match tab {
            Tab::Request => self.request_ui(ui),
            Tab::Response => self.response_ui(ui),
        }
    }
}

impl Session {
    fn request_ui(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ComboBox::from_id_salt(METHOD_COMBO_ID)
                .selected_text(self.method.as_str())
                .show_ui(ui, |ui| {
                    for method in METHODS {
                        let label = method.as_str().to_string();
                        ui.selectable_value(&mut self.method, method, label);
                    }
                });
            ui.add(egui::TextEdit::singleline(&mut self.url).desired_width(f32::INFINITY));
        });
    }

    fn response_ui(&self, ui: &mut Ui) {
        ui.label(format!(
            "{SPIKE_STATUS_TEXT}  {:.1} MB  {} lines",
            self.response.megabytes(),
            self.response.line_count()
        ));
        ui.separator();
        let font_id = TextStyle::Monospace.resolve(ui.style());
        let row_height = ui.text_style_height(&TextStyle::Monospace);
        ScrollArea::both().auto_shrink(false).show_rows(
            ui,
            row_height,
            self.response.line_count(),
            |ui, rows| {
                for index in rows {
                    let job = self
                        .highlighter
                        .layout_line(self.response.line(index), &font_id);
                    ui.add(egui::Label::new(job).wrap_mode(TextWrapMode::Extend));
                }
            },
        );
    }
}
