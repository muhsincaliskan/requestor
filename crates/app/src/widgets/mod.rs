mod environment_color;
mod highlight;
mod icon_button;
mod list_row;
mod method_badge;
mod status_color;

pub use environment_color::{environment_color, environment_swatch};
pub use highlight::{append_row, layout_row, SyntaxPalette};
pub use icon_button::{chevron_button, close_button, icon_button, ICON_BUTTON_SIZE};
pub use list_row::{list_row, row_label};
pub use method_badge::{aligned_request_title, method_color, request_title};
pub use status_color::{status_color, success_color};

pub const UNSAVED_MARK: &str = " •";
pub const UNSAVED_DOT: &str = "•";
pub const PROTECTED_SEND_HINT: &str =
    "This environment is protected. Requestor asks before sending.";

pub mod action_button;
pub mod auth_editor;
pub mod code_view;
pub mod key_value_table;
pub mod switch;
pub mod variable_text_edit;
