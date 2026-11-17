use eframe::egui::{ComboBox, DragValue, Ui};
use requestor_core::settings::{HttpVersion, RequestSettings};
use requestor_core::workspace::AppSettings;

use super::row::{row, toggles, Toggle};

const HTTP_VERSION_ID: &str = "settings-http-version";
const MILLISECONDS_SUFFIX: &str = " ms";
const MEGABYTES_SUFFIX: &str = " MB";
const TIMEOUT_DRAG_SPEED: f64 = 100.0;
const MAX_REDIRECT_LIMIT: usize = 50;

const REQUEST_TOGGLES: [Toggle<RequestSettings>; 3] = [
    Toggle {
        label: "SSL certificate verification",
        description: "Verify server certificates when connecting over HTTPS.",
        field: |settings| &mut settings.verify_ssl,
    },
    Toggle {
        label: "Send no-cache header",
        description: "Add a Cache-Control: no-cache header to every request.",
        field: |settings| &mut settings.send_no_cache_header,
    },
    Toggle {
        label: "Send Postman-Token header",
        description: "Add a unique Postman-Token header to every request.",
        field: |settings| &mut settings.send_postman_token_header,
    },
];

const REDIRECT_TOGGLES: [Toggle<RequestSettings>; 1] = [Toggle {
    label: "Automatically follow redirects",
    description: "Follow 3xx responses to their target.",
    field: |settings| &mut settings.follow_redirects,
}];

const COOKIE_TOGGLES: [Toggle<RequestSettings>; 1] = [Toggle {
    label: "Enable cookies",
    description: "Store cookies from responses and send them back.",
    field: |settings| &mut settings.cookies_enabled,
}];

pub fn show(ui: &mut Ui, settings: &mut AppSettings) -> bool {
    let request = &mut settings.request;
    let mut changed = http_version(ui, request);
    changed |= milliseconds(
        ui,
        "Request timeout",
        "0 means no timeout.",
        |s| &mut s.request_timeout_ms,
        request,
    );
    changed |= milliseconds(
        ui,
        "Connect timeout",
        "0 means no timeout.",
        |s| &mut s.connect_timeout_ms,
        request,
    );
    changed |= max_response_size(ui, request);
    changed |= toggles(ui, &REQUEST_TOGGLES, request);
    changed |= toggles(ui, &REDIRECT_TOGGLES, &mut settings.request);
    changed |= max_redirects(ui, &mut settings.request);
    changed | toggles(ui, &COOKIE_TOGGLES, &mut settings.request)
}

fn http_version(ui: &mut Ui, settings: &mut RequestSettings) -> bool {
    row(ui, "HTTP version", "Protocol used for requests.", |ui| {
        let mut changed = false;
        ComboBox::from_id_salt(HTTP_VERSION_ID)
            .selected_text(settings.http_version.label())
            .show_ui(ui, |ui| {
                for version in HttpVersion::ALL {
                    changed |= ui
                        .selectable_value(&mut settings.http_version, version, version.label())
                        .changed();
                }
            });
        changed
    })
}

fn milliseconds(
    ui: &mut Ui,
    label: &str,
    description: &str,
    field: fn(&mut RequestSettings) -> &mut u64,
    settings: &mut RequestSettings,
) -> bool {
    row(ui, label, description, |ui| {
        ui.add(
            DragValue::new(field(settings))
                .suffix(MILLISECONDS_SUFFIX)
                .speed(TIMEOUT_DRAG_SPEED),
        )
        .changed()
    })
}

fn max_response_size(ui: &mut Ui, settings: &mut RequestSettings) -> bool {
    row(ui, "Max response size", "0 means unlimited.", |ui| {
        let mut megabytes = settings.max_response_megabytes();
        let changed = ui
            .add(DragValue::new(&mut megabytes).suffix(MEGABYTES_SUFFIX))
            .changed();
        if changed {
            settings.set_max_response_megabytes(megabytes);
        }
        changed
    })
}

fn max_redirects(ui: &mut Ui, settings: &mut RequestSettings) -> bool {
    row(
        ui,
        "Max redirects",
        "Stop after this many redirects.",
        |ui| {
            ui.add(DragValue::new(&mut settings.max_redirects).range(0..=MAX_REDIRECT_LIMIT))
                .changed()
        },
    )
}
