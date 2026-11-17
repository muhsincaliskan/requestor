use eframe::egui::{Context, Key, KeyboardShortcut, Modifiers};

use requestor_core::workspace::InterfaceSettings;

use crate::state::{Action, AppState, Overlay};

#[derive(Clone, Copy)]
enum Command {
    NewTab,
    CloseTab,
    Send,
    Save,
    DuplicateTab,
    NextTab,
    PreviousTab,
    Import,
    Settings,
    ToggleSidebar,
    ToggleTwoPaneView,
    ToggleWorkbench,
    SwapSidebars,
}

pub struct Shortcut {
    pub label: &'static str,
    pub keys: KeyboardShortcut,
    command: Command,
}

const COMMAND_SHIFT: Modifiers = Modifiers::COMMAND.plus(Modifiers::SHIFT);
const COMMAND_ALT: Modifiers = Modifiers::COMMAND.plus(Modifiers::ALT);
const CONTROL_SHIFT: Modifiers = Modifiers::CTRL.plus(Modifiers::SHIFT);

pub const ALL: [Shortcut; 13] = [
    Shortcut::new(
        "Send request",
        Modifiers::COMMAND,
        Key::Enter,
        Command::Send,
    ),
    Shortcut::new("Save request", Modifiers::COMMAND, Key::S, Command::Save),
    Shortcut::new("New tab", Modifiers::COMMAND, Key::T, Command::NewTab),
    Shortcut::new("Close tab", Modifiers::COMMAND, Key::W, Command::CloseTab),
    Shortcut::new(
        "Duplicate tab",
        Modifiers::COMMAND,
        Key::D,
        Command::DuplicateTab,
    ),
    Shortcut::new(
        "Previous tab",
        CONTROL_SHIFT,
        Key::Tab,
        Command::PreviousTab,
    ),
    Shortcut::new("Next tab", Modifiers::CTRL, Key::Tab, Command::NextTab),
    Shortcut::new("Import", Modifiers::COMMAND, Key::O, Command::Import),
    Shortcut::new(
        "Settings",
        Modifiers::COMMAND,
        Key::Comma,
        Command::Settings,
    ),
    Shortcut::new(
        "Toggle sidebar",
        Modifiers::COMMAND,
        Key::B,
        Command::ToggleSidebar,
    ),
    Shortcut::new(
        "Toggle two-pane view",
        COMMAND_SHIFT,
        Key::L,
        Command::ToggleTwoPaneView,
    ),
    Shortcut::new(
        "Show middle workbench",
        COMMAND_ALT,
        Key::M,
        Command::ToggleWorkbench,
    ),
    Shortcut::new(
        "Swap left and right sidebars",
        COMMAND_ALT,
        Key::S,
        Command::SwapSidebars,
    ),
];

impl Shortcut {
    const fn new(label: &'static str, modifiers: Modifiers, key: Key, command: Command) -> Self {
        Shortcut {
            label,
            keys: KeyboardShortcut::new(modifiers, key),
            command,
        }
    }

    fn is_chorded(&self) -> bool {
        self.keys.modifiers.alt || self.keys.modifiers.shift
    }

    pub fn keys_text(&self, ctx: &Context) -> String {
        ctx.format_shortcut(&self.keys)
    }
}

impl Command {
    fn run(self, state: &mut AppState, actions: &mut Vec<Action>) {
        match self {
            Command::NewTab => actions.push(Action::NewTab),
            Command::Import => actions.push(Action::Show(Overlay::Import(Default::default()))),
            Command::Settings => actions.push(Action::Show(Overlay::Settings)),
            Command::NextTab => actions.push(Action::ActivateNeighbourTab { forward: true }),
            Command::PreviousTab => actions.push(Action::ActivateNeighbourTab { forward: false }),
            Command::ToggleSidebar => toggle(state, actions, |ui| &mut ui.show_sidebar),
            Command::ToggleTwoPaneView => toggle(state, actions, |ui| &mut ui.two_pane_view),
            Command::ToggleWorkbench => toggle(state, actions, |ui| &mut ui.show_workbench),
            Command::SwapSidebars => toggle(state, actions, |ui| &mut ui.sidebars_swapped),
            tab_command => actions.extend(tab_command.for_active_tab(state)),
        }
    }

    fn for_active_tab(self, state: &AppState) -> Option<Action> {
        let tab = state.active_tab?;
        match self {
            Command::CloseTab => Some(Action::CloseTab(tab)),
            Command::Send => Some(Action::Send(tab)),
            Command::Save => Some(Action::SaveTab(tab)),
            Command::DuplicateTab => Some(Action::DuplicateTab(tab)),
            _ => None,
        }
    }
}

fn toggle(
    state: &mut AppState,
    actions: &mut Vec<Action>,
    flag: fn(&mut InterfaceSettings) -> &mut bool,
) {
    let value = flag(&mut state.workspace.settings.interface);
    *value = !*value;
    actions.push(Action::SettingsEdited);
}

pub fn collect(ctx: &Context, state: &mut AppState, actions: &mut Vec<Action>) {
    if state.overlay.is_some() {
        return;
    }
    let (chorded, plain): (Vec<_>, Vec<_>) = ALL.iter().partition(|shortcut| shortcut.is_chorded());
    for shortcut in chorded.into_iter().chain(plain) {
        if ctx.input_mut(|input| input.consume_shortcut(&shortcut.keys)) {
            shortcut.command.run(state, actions);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ALL;

    #[test]
    fn every_shortcut_has_its_own_keys() {
        for (index, shortcut) in ALL.iter().enumerate() {
            let duplicate = ALL[index + 1..]
                .iter()
                .any(|other| other.keys == shortcut.keys);
            assert!(!duplicate, "{} shares its keys", shortcut.label);
        }
    }

    #[test]
    fn chorded_shortcuts_are_the_ones_with_alt_or_shift() {
        let chorded: Vec<&str> = ALL
            .iter()
            .filter(|shortcut| shortcut.is_chorded())
            .map(|shortcut| shortcut.label)
            .collect();
        assert_eq!(
            chorded,
            [
                "Previous tab",
                "Toggle two-pane view",
                "Show middle workbench",
                "Swap left and right sidebars"
            ]
        );
    }
}
