use requestor_core::draft::KeyValueRow;
use requestor_core::prepared::CodeLanguage;

use super::{open, state_with_request};
use crate::state::{Action, AppState, Effect, Overlay, TabId};

const TOKEN: &str = "Bearer s3cret";

fn opened_with_token() -> (AppState, TabId) {
    let (mut state, collection_id, item_id) = state_with_request();
    let tab = open(&mut state, &collection_id, &item_id);
    let draft = &mut state.tab_mut(tab).unwrap().draft;
    draft
        .headers
        .insert(0, KeyValueRow::new("Authorization", TOKEN));
    state.apply(Action::TabEdited(tab), 0.0);
    (state, tab)
}

fn shown_code(state: &AppState) -> (CodeLanguage, bool, String) {
    let Some(Overlay::Code(form)) = &state.overlay else {
        panic!("expected the code dialog");
    };
    (form.language, form.include_secrets, form.code.clone())
}

#[test]
fn opening_code_uses_the_remembered_language_with_secrets_hidden() {
    let (mut state, tab) = opened_with_token();
    state.workspace.settings.interface.code_language = CodeLanguage::Python;
    state.apply(Action::OpenCode(tab), 0.0);
    let (language, include_secrets, code) = shown_code(&state);
    assert_eq!(language, CodeLanguage::Python);
    assert!(!include_secrets);
    assert!(code.contains("requests.request(\"GET\""), "{code}");
    assert!(!code.contains(TOKEN), "{code}");
}

#[test]
fn switching_language_regenerates_and_is_remembered() {
    let (mut state, tab) = opened_with_token();
    state.apply(Action::OpenCode(tab), 0.0);
    let show = Action::ShowCode {
        tab,
        language: CodeLanguage::Java,
        include_secrets: true,
    };
    state.apply(show, 0.0);
    let (language, _, code) = shown_code(&state);
    assert_eq!(language, CodeLanguage::Java);
    assert!(code.contains(TOKEN), "{code}");
    assert_eq!(
        state.workspace.settings.interface.code_language,
        CodeLanguage::Java
    );
}

#[test]
fn copy_as_curl_still_masks_secrets_by_default() {
    let (mut state, tab) = opened_with_token();
    let effects = state.apply(
        Action::CopyAsCurl {
            tab,
            include_secrets: false,
        },
        0.0,
    );
    let copied = effects.iter().find_map(|effect| match effect {
        Effect::CopyText(text) => Some(text.clone()),
        _ => None,
    });
    let copied = copied.expect("copied text");
    assert!(copied.starts_with("curl"));
    assert!(!copied.contains(TOKEN));
}

#[test]
fn the_code_panel_follows_edits_and_its_options() {
    let (mut state, tab) = opened_with_token();
    state.workspace.settings.interface.code_language = CodeLanguage::Curl;
    state.apply(Action::ToggleCodePanel, 0.0);
    let code = |state: &AppState| state.code_panel.as_ref().unwrap().code.clone().unwrap();
    assert!(code(&state).starts_with("curl"), "{}", code(&state));
    assert!(!code(&state).contains(TOKEN));
    state.tab_mut(tab).unwrap().draft.url = "https://example.test/changed".to_string();
    state.apply(Action::TabEdited(tab), 0.0);
    assert!(code(&state).contains("/changed"), "{}", code(&state));
    let panel = state.code_panel.clone().unwrap();
    state.apply(panel.with_options(CodeLanguage::Python, true), 0.0);
    assert!(code(&state).contains(TOKEN), "{}", code(&state));
    assert_eq!(
        state.workspace.settings.interface.code_language,
        CodeLanguage::Python
    );
    state.apply(Action::ToggleCodePanel, 0.0);
    assert!(state.code_panel.is_none());
}
