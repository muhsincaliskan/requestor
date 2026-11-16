mod app;
mod highlight;
mod response_buffer;
mod synthetic;

use app::RequestorApp;
use response_buffer::{ResponseBuffer, BYTES_PER_MEGABYTE};

const APP_NAME: &str = "Requestor";
const DEFAULT_RESPONSE_MEGABYTES: usize = 5;
const RESPONSE_SIZE_FLAG: &str = "--response-mb";

fn response_megabytes() -> usize {
    std::env::args()
        .skip_while(|argument| argument != RESPONSE_SIZE_FLAG)
        .nth(1)
        .and_then(|value| value.parse().ok())
        .unwrap_or(DEFAULT_RESPONSE_MEGABYTES)
}

fn main() -> eframe::Result {
    let response_bytes = response_megabytes() * BYTES_PER_MEGABYTE;
    let response = ResponseBuffer::from_text(synthetic::json_of_size(response_bytes));
    eframe::run_native(
        APP_NAME,
        eframe::NativeOptions::default(),
        Box::new(|_creation_context| Ok(Box::new(RequestorApp::new(response)))),
    )
}
