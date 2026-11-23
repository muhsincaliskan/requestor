#[cfg(target_os = "macos")]
mod macos;

use std::sync::mpsc::{channel, Receiver};

use eframe::egui::Context;

use crate::state::{Action, Overlay, UpdateCheckOrigin};

#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuCommand {
    OpenSettings,
    CheckForUpdates,
}

impl MenuCommand {
    fn action(self) -> Action {
        match self {
            MenuCommand::OpenSettings => Action::Show(Overlay::Settings),
            MenuCommand::CheckForUpdates => Action::CheckForUpdates(UpdateCheckOrigin::Manual),
        }
    }
}

pub struct AppMenu {
    commands: Receiver<MenuCommand>,
    #[cfg(target_os = "macos")]
    _items: Option<macos::MenuItems>,
}

impl AppMenu {
    #[cfg(target_os = "macos")]
    pub fn install(wake: &Context) -> Self {
        let (sender, commands) = channel();
        AppMenu {
            commands,
            _items: macos::MenuItems::install(sender, wake.clone()),
        }
    }

    #[cfg(not(target_os = "macos"))]
    pub fn install(_wake: &Context) -> Self {
        let (_sender, commands) = channel();
        AppMenu { commands }
    }

    pub fn actions(&self) -> impl Iterator<Item = Action> + '_ {
        self.commands.try_iter().map(MenuCommand::action)
    }
}
