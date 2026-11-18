use eframe::egui::{ComboBox, Grid, RichText, TextEdit, Ui};
use requestor_core::settings::{HttpVersion, ProxyChoice, RequestOverrides, RequestSettings};

const INTRO: &str =
    "These settings apply to this request only. Anything left on the app setting follows Settings.";
const SECURITY_WARNING: &str = "This request turns off SSL verification or sends through its own proxy. Check these settings when the collection came from someone else.";
const CREDENTIALS_WARNING: &str = "This address is saved in the collection as plain text. Use Settings › Proxy for a proxy that needs a password.";
const RESET_LABEL: &str = "Use app settings for everything";
const GRID_ID: &str = "request-settings-grid";
const HTTP_VERSION_ID: &str = "request-settings-http-version";
const VERIFY_ID: &str = "request-settings-verify";
const REDIRECTS_ID: &str = "request-settings-redirects";
const PROXY_ID: &str = "request-settings-proxy";
const CERTIFICATES_ID: &str = "request-settings-certificates";
const HTTP_VERSION_LABEL: &str = "HTTP version";
const TIMEOUT_LABEL: &str = "Request timeout (ms)";
const VERIFY_LABEL: &str = "SSL certificate verification";
const REDIRECTS_LABEL: &str = "Follow redirects";
const MAX_REDIRECTS_LABEL: &str = "Maximum redirects";
const PROXY_LABEL: &str = "Proxy";
const CERTIFICATES_LABEL: &str = "Client certificates";
const PROXY_URL_HINT: &str = "http://proxy.example.com:8080";
const APP_SETTING: &str = "App setting";
const NO_PROXY: &str = "No proxy";
const CUSTOM_PROXY: &str = "Custom proxy";
const ON: &str = "On";
const OFF: &str = "Off";
const SEND: &str = "Send";
const DONT_SEND: &str = "Don't send";
const NO_TIMEOUT: &str = "no timeout";
const CONTROL_WIDTH: f32 = 240.0;
const GRID_SPACING: [f32; 2] = [16.0, 8.0];
const SECTION_GAP: f32 = 8.0;

pub fn show(ui: &mut Ui, overrides: &mut RequestOverrides, global: &RequestSettings) -> bool {
    ui.weak(INTRO);
    if overrides.lowers_security() {
        ui.label(RichText::new(SECURITY_WARNING).color(ui.visuals().warn_fg_color));
    }
    ui.add_space(SECTION_GAP);
    let mut changed = false;
    Grid::new(GRID_ID)
        .num_columns(2)
        .spacing(GRID_SPACING)
        .show(ui, |ui| {
            changed |= http_version(ui, overrides, global.http_version);
            changed |= timeout(ui, overrides, global.request_timeout_ms);
            changed |= switch(
                ui,
                (VERIFY_LABEL, VERIFY_ID),
                &mut overrides.verify_ssl,
                global.verify_ssl,
            );
            changed |= switch(
                ui,
                (REDIRECTS_LABEL, REDIRECTS_ID),
                &mut overrides.follow_redirects,
                global.follow_redirects,
            );
            changed |= max_redirects(ui, overrides, global.max_redirects);
            changed |= proxy(ui, &mut overrides.proxy);
            changed |= certificates(ui, &mut overrides.skip_client_certificates);
        });
    if !overrides.is_empty() {
        ui.add_space(SECTION_GAP);
        if ui.button(RESET_LABEL).clicked() {
            *overrides = RequestOverrides::default();
            changed = true;
        }
    }
    changed
}

fn app_setting(value: &str) -> String {
    format!("{APP_SETTING} ({value})")
}

fn http_version(ui: &mut Ui, overrides: &mut RequestOverrides, global: HttpVersion) -> bool {
    ui.label(HTTP_VERSION_LABEL);
    let inherited = app_setting(global.label());
    let selected = overrides
        .http_version
        .map_or(inherited.clone(), |version| version.label().to_string());
    let mut changed = false;
    ComboBox::from_id_salt(HTTP_VERSION_ID)
        .width(CONTROL_WIDTH)
        .selected_text(selected)
        .show_ui(ui, |ui| {
            changed |= ui
                .selectable_value(&mut overrides.http_version, None, inherited)
                .changed();
            for version in HttpVersion::ALL {
                changed |= ui
                    .selectable_value(&mut overrides.http_version, Some(version), version.label())
                    .changed();
            }
        });
    ui.end_row();
    changed
}

fn timeout(ui: &mut Ui, overrides: &mut RequestOverrides, global: u64) -> bool {
    ui.label(TIMEOUT_LABEL);
    let inherited = match global {
        0 => app_setting(NO_TIMEOUT),
        milliseconds => app_setting(&milliseconds.to_string()),
    };
    let mut text = overrides.timeout_text();
    let changed = number_field(ui, &mut text, &inherited);
    if changed {
        overrides.set_timeout_text(&text);
    }
    ui.end_row();
    changed
}

fn max_redirects(ui: &mut Ui, overrides: &mut RequestOverrides, global: usize) -> bool {
    ui.label(MAX_REDIRECTS_LABEL);
    let mut text = overrides.max_redirects_text();
    let changed = number_field(ui, &mut text, &app_setting(&global.to_string()));
    if changed {
        overrides.set_max_redirects_text(&text);
    }
    ui.end_row();
    changed
}

fn number_field(ui: &mut Ui, text: &mut String, hint: &str) -> bool {
    ui.add(
        TextEdit::singleline(text)
            .hint_text(hint)
            .desired_width(CONTROL_WIDTH),
    )
    .changed()
}

fn switch(ui: &mut Ui, (label, id): (&str, &str), value: &mut Option<bool>, global: bool) -> bool {
    ui.label(label);
    let name = |enabled: bool| if enabled { ON } else { OFF };
    let inherited = app_setting(name(global));
    let selected = value.map_or(inherited.clone(), |enabled| name(enabled).to_string());
    let mut changed = false;
    ComboBox::from_id_salt(id)
        .width(CONTROL_WIDTH)
        .selected_text(selected)
        .show_ui(ui, |ui| {
            let choices = [
                (None, inherited),
                (Some(true), ON.to_string()),
                (Some(false), OFF.to_string()),
            ];
            for (choice, text) in choices {
                changed |= ui.selectable_value(value, choice, text).changed();
            }
        });
    ui.end_row();
    changed
}

fn certificates(ui: &mut Ui, skip: &mut bool) -> bool {
    ui.label(CERTIFICATES_LABEL);
    let inherited = app_setting(SEND);
    let selected = if *skip {
        DONT_SEND.to_string()
    } else {
        inherited.clone()
    };
    let mut changed = false;
    ComboBox::from_id_salt(CERTIFICATES_ID)
        .width(CONTROL_WIDTH)
        .selected_text(selected)
        .show_ui(ui, |ui| {
            changed |= ui.selectable_value(skip, false, inherited).changed();
            changed |= ui.selectable_value(skip, true, DONT_SEND).changed();
        });
    ui.end_row();
    changed
}

fn proxy_label(choice: &ProxyChoice) -> &'static str {
    match choice {
        ProxyChoice::Global => APP_SETTING,
        ProxyChoice::Direct => NO_PROXY,
        ProxyChoice::Custom(_) => CUSTOM_PROXY,
    }
}

fn proxy(ui: &mut Ui, proxy: &mut ProxyChoice) -> bool {
    ui.label(PROXY_LABEL);
    let mut changed = false;
    ui.vertical(|ui| {
        ComboBox::from_id_salt(PROXY_ID)
            .width(CONTROL_WIDTH)
            .selected_text(proxy_label(proxy))
            .show_ui(ui, |ui| {
                let custom = ProxyChoice::Custom(String::new());
                for choice in [ProxyChoice::Global, ProxyChoice::Direct, custom] {
                    let selected = proxy.same_kind(&choice);
                    if ui
                        .selectable_label(selected, proxy_label(&choice))
                        .clicked()
                        && !selected
                    {
                        *proxy = choice;
                        changed = true;
                    }
                }
            });
        changed |= custom_proxy_address(ui, proxy);
    });
    ui.end_row();
    changed
}

fn custom_proxy_address(ui: &mut Ui, proxy: &mut ProxyChoice) -> bool {
    let warn = proxy.has_credentials();
    let ProxyChoice::Custom(url) = proxy else {
        return false;
    };
    let changed = ui
        .add(
            TextEdit::singleline(url)
                .hint_text(PROXY_URL_HINT)
                .desired_width(CONTROL_WIDTH),
        )
        .changed();
    if warn {
        ui.label(RichText::new(CREDENTIALS_WARNING).color(ui.visuals().warn_fg_color));
    }
    changed
}
