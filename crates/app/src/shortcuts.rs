use eframe::egui::{Context, Key, KeyboardShortcut, Modifiers};

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
}

pub struct Shortcut {
    pub label: &'static str,
    pub keys: KeyboardShortcut,
    command: Command,
}

const COMMAND_SHIFT: Modifiers = Modifiers::COMMAND.plus(Modifiers::SHIFT);
const CONTROL_SHIFT: Modifiers = Modifiers::CTRL.plus(Modifiers::SHIFT);

pub const ALL: [Shortcut; 11] = [
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
];

impl Shortcut {
    const fn new(label: &'static str, modifiers: Modifiers, key: Key, command: Command) -> Self {
        Shortcut {
            label,
            keys: KeyboardShortcut::new(modifiers, key),
            command,
        }
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
            Command::ToggleSidebar => {
                let interface = &mut state.workspace.settings.interface;
                interface.show_sidebar = !interface.show_sidebar;
                actions.push(Action::SettingsEdited);
            }
            Command::ToggleTwoPaneView => {
                let interface = &mut state.workspace.settings.interface;
                interface.two_pane_view = !interface.two_pane_view;
                actions.push(Action::SettingsEdited);
            }
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

pub fn collect(ctx: &Context, state: &mut AppState, actions: &mut Vec<Action>) {
    if state.overlay.is_some() {
        return;
    }
    for shortcut in &ALL {
        if ctx.input_mut(|input| input.consume_shortcut(&shortcut.keys)) {
            shortcut.command.run(state, actions);
        }
    }
}
