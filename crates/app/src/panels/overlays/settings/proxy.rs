use eframe::egui::{Context, Id, TextEdit, Ui};
use requestor_core::settings::{ProxySettings, RequestSettings};

use super::row::{row, toggles, Toggle};

const REVEAL_ID: &str = "settings-proxy-reveal";
const BYPASS_ID: &str = "settings-proxy-bypass";
const FIELD_WIDTH: f32 = 260.0;
const URL_HINT: &str = "http://proxy.example.com:8080";
const BYPASS_HINT: &str = "localhost, *.internal.example";
const REVEAL_LABEL: &str = "Show";

const SYSTEM_PROXY_TOGGLES: [Toggle<RequestSettings>; 1] = [Toggle {
    label: "Use the system proxy",
    description: "Use the proxy configured in the operating system.",
    field: |settings| &mut settings.use_system_proxy,
}];

pub fn show(ui: &mut Ui, settings: &mut RequestSettings) -> bool {
    let mut changed = toggles(ui, &SYSTEM_PROXY_TOGGLES, settings);
    changed |= custom_proxy_toggle(ui, settings);
    if let Some(proxy) = settings.proxy.as_mut() {
        changed |= fields(ui, proxy);
    }
    changed
}

pub fn forget_transient_state(ctx: &Context) {
    ctx.data_mut(|data| {
        data.remove_temp::<bool>(Id::new(REVEAL_ID));
        data.remove_temp::<String>(Id::new(BYPASS_ID));
    });
}

fn custom_proxy_toggle(ui: &mut Ui, settings: &mut RequestSettings) -> bool {
    row(
        ui,
        "Use a custom proxy",
        "Send requests through the proxy configured below.",
        |ui| {
            let mut enabled = settings.proxy.is_some();
            let changed = ui.checkbox(&mut enabled, "").changed();
            if changed {
                settings.proxy = enabled.then(ProxySettings::default);
                forget_transient_state(ui.ctx());
            }
            changed
        },
    )
}

fn fields(ui: &mut Ui, proxy: &mut ProxySettings) -> bool {
    let mut changed = proxy_url(ui, proxy);
    changed |= username(ui, proxy);
    changed |= password(ui, proxy);
    changed | bypass_hosts(ui, proxy)
}

fn text_field(ui: &mut Ui, text: &mut String, hint: &str, masked: bool) -> bool {
    ui.add(
        TextEdit::singleline(text)
            .hint_text(hint)
            .password(masked)
            .desired_width(FIELD_WIDTH),
    )
    .changed()
}

fn proxy_url(ui: &mut Ui, proxy: &mut ProxySettings) -> bool {
    row(ui, "Proxy URL", "", |ui| {
        text_field(ui, &mut proxy.url, URL_HINT, false)
    })
}

fn username(ui: &mut Ui, proxy: &mut ProxySettings) -> bool {
    row(ui, "Username", "", |ui| {
        let mut text = proxy.username_text().to_string();
        let changed = text_field(ui, &mut text, "", false);
        if changed {
            proxy.set_username_text(&text);
        }
        changed
    })
}

fn password(ui: &mut Ui, proxy: &mut ProxySettings) -> bool {
    row(ui, "Password", "", |ui| {
        let reveal_id = Id::new(REVEAL_ID);
        let mut revealed = ui
            .ctx()
            .data(|data| data.get_temp(reveal_id))
            .unwrap_or(false);
        if ui.checkbox(&mut revealed, REVEAL_LABEL).changed() {
            ui.ctx()
                .data_mut(|data| data.insert_temp(reveal_id, revealed));
        }
        let mut text = proxy
            .password
            .as_ref()
            .map(|secret| secret.expose().to_string())
            .unwrap_or_default();
        let changed = text_field(ui, &mut text, "", !revealed);
        if changed {
            proxy.set_password_text(&text);
        }
        changed
    })
}

fn bypass_hosts(ui: &mut Ui, proxy: &mut ProxySettings) -> bool {
    row(
        ui,
        "Bypass hosts",
        "Comma-separated hosts that skip the proxy.",
        |ui| {
            let id = Id::new(BYPASS_ID);
            let mut text = ui
                .ctx()
                .data(|data| data.get_temp::<String>(id))
                .unwrap_or_else(|| proxy.bypass_hosts_text());
            let changed = text_field(ui, &mut text, BYPASS_HINT, false);
            if changed {
                proxy.set_bypass_hosts_text(&text);
            }
            ui.ctx().data_mut(|data| data.insert_temp(id, text));
            changed
        },
    )
}
