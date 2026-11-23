use std::sync::mpsc::Sender;

use eframe::egui::Context;
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, NSObject, NSObjectProtocol, Sel};
use objc2::{define_class, msg_send, sel, DefinedClass, MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{NSApplication, NSMenu, NSMenuItem};
use objc2_foundation::NSString;

use super::MenuCommand;

const CHECK_FOR_UPDATES_TITLE: &str = "Check for Updates…";
const SETTINGS_TITLE: &str = "Settings…";
const SETTINGS_KEY: &str = ",";
const NO_KEY: &str = "";
const APPLICATION_MENU_INDEX: isize = 0;
const FIRST_ITEM_AFTER_ABOUT: isize = 1;

pub struct MenuTargetIvars {
    commands: Sender<MenuCommand>,
    wake: Context,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "RequestorMenuTarget"]
    #[ivars = MenuTargetIvars]
    pub struct MenuTarget;

    impl MenuTarget {
        #[unsafe(method(openSettings:))]
        fn open_settings(&self, _sender: Option<&AnyObject>) {
            self.forward(MenuCommand::OpenSettings);
        }

        #[unsafe(method(checkForUpdates:))]
        fn check_for_updates(&self, _sender: Option<&AnyObject>) {
            self.forward(MenuCommand::CheckForUpdates);
        }
    }

    unsafe impl NSObjectProtocol for MenuTarget {}
);

impl MenuTarget {
    fn new(mtm: MainThreadMarker, ivars: MenuTargetIvars) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(ivars);
        unsafe { msg_send![super(this), init] }
    }

    fn forward(&self, command: MenuCommand) {
        let ivars = self.ivars();
        if ivars.commands.send(command).is_ok() {
            ivars.wake.request_repaint();
        }
    }
}

enum MenuEntry {
    Command {
        title: &'static str,
        action: Sel,
        key: &'static str,
    },
    Separator,
}

impl MenuEntry {
    fn all() -> [MenuEntry; 3] {
        [
            MenuEntry::Command {
                title: CHECK_FOR_UPDATES_TITLE,
                action: sel!(checkForUpdates:),
                key: NO_KEY,
            },
            MenuEntry::Separator,
            MenuEntry::Command {
                title: SETTINGS_TITLE,
                action: sel!(openSettings:),
                key: SETTINGS_KEY,
            },
        ]
    }

    fn item(&self, mtm: MainThreadMarker, target: &MenuTarget) -> Retained<NSMenuItem> {
        match self {
            MenuEntry::Command { title, action, key } => {
                let item = unsafe {
                    NSMenuItem::initWithTitle_action_keyEquivalent(
                        NSMenuItem::alloc(mtm),
                        &NSString::from_str(title),
                        Some(*action),
                        &NSString::from_str(key),
                    )
                };
                unsafe { item.setTarget(Some(target)) };
                item
            }
            MenuEntry::Separator => NSMenuItem::separatorItem(mtm),
        }
    }
}

pub struct MenuItems {
    _target: Retained<MenuTarget>,
}

impl MenuItems {
    pub fn install(commands: Sender<MenuCommand>, wake: Context) -> Option<Self> {
        let mtm = MainThreadMarker::new()?;
        let menu = application_menu(mtm)?;
        let target = MenuTarget::new(mtm, MenuTargetIvars { commands, wake });
        for (index, entry) in (FIRST_ITEM_AFTER_ABOUT..).zip(MenuEntry::all()) {
            menu.insertItem_atIndex(&entry.item(mtm, &target), index);
        }
        Some(MenuItems { _target: target })
    }
}

fn application_menu(mtm: MainThreadMarker) -> Option<Retained<NSMenu>> {
    NSApplication::sharedApplication(mtm)
        .mainMenu()?
        .itemAtIndex(APPLICATION_MENU_INDEX)?
        .submenu()
}
