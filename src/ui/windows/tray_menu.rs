//! The menu of the icon (FR-91) on the shell.
//!
//! On top the line of the state — the program's icon, «LANG SWITCHER» and whether it works, is
//! suspended or needs a restart; under it the items of FR-91 in their order, each with its icon:
//! the key of the hotkey beside «Приостановить» / «Возобновить», the switch of «Запускать при входе
//! в систему», the dot of an unread letter. What the items are, in what order, and which of them
//! are available is the tray's [`Menu`]; this module only says how they look.
//!
//! A chosen item closes the menu, and its command runs once the menu is gone
//! ([`tray::menu_closed`]); the switch changes the setting at once and the menu stays.

use crate::settings::{
    self, IDS_STATUS_RESTART, IDS_STATUS_SUSPENDED, IDS_STATUS_WORKING, Language,
};
use crate::theme::ThemeSetting;
use crate::tray::{
    self, CMD_ABOUT, CMD_AUTOSTART, CMD_EXIT, CMD_SETTINGS, CMD_TOGGLE, CMD_UNREAD, CMD_UPDATE,
    CMD_WHATSNEW, CMD_WRITE, Menu, MenuEntry, MenuNow, Pending,
};
use crate::ui::host::{self, Content, Reply};
use crate::ui::scene::{Event, MenuState, Node, Trailing, Window, WindowKind};
use ::windows::Win32::Foundation::{HWND, POINT};

/// The items of the menu: one for each command of FR-91.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Id {
    Unread,
    Update,
    WhatsNew,
    Toggle,
    Settings,
    Autostart,
    Write,
    About,
    Exit,
    /// Esc — no item: the menu goes, and the tray learns it went by the key (T-120-15).
    Escape,
}

impl Id {
    /// The item of a command of the tray; none for a number that is not one.
    pub fn of(command: u32) -> Option<Self> {
        Some(match command {
            CMD_UNREAD => Self::Unread,
            CMD_UPDATE => Self::Update,
            CMD_WHATSNEW => Self::WhatsNew,
            CMD_TOGGLE => Self::Toggle,
            CMD_SETTINGS => Self::Settings,
            CMD_AUTOSTART => Self::Autostart,
            CMD_WRITE => Self::Write,
            CMD_ABOUT => Self::About,
            CMD_EXIT => Self::Exit,
            _ => return None,
        })
    }

    /// The command of the tray this item gives.
    pub fn command(self) -> u32 {
        match self {
            Self::Unread => CMD_UNREAD,
            Self::Update => CMD_UPDATE,
            Self::WhatsNew => CMD_WHATSNEW,
            Self::Toggle => CMD_TOGGLE,
            Self::Settings => CMD_SETTINGS,
            Self::Autostart => CMD_AUTOSTART,
            Self::Write => CMD_WRITE,
            Self::About => CMD_ABOUT,
            Self::Exit => CMD_EXIT,
            Self::Escape => 0,
        }
    }
}

/// The menu as the tray hands it over, in a language.
pub struct TrayMenu {
    pub now: MenuNow,
    pub language: Language,
}

impl TrayMenu {
    fn state(&self) -> (MenuState, u16) {
        status(self.now.enabled, self.now.fail_safe)
    }
}

/// The state of the program — its dot and its words: «Работает», «Приостановлена», «Нужен
/// перезапуск». One source for the head of this menu and for the block «Состояние» of the window of
/// the settings (T-120-21).
pub fn status(enabled: bool, fail_safe: bool) -> (MenuState, u16) {
    if tray::resume_is_refused(enabled, fail_safe) {
        (MenuState::Restart, IDS_STATUS_RESTART)
    } else if enabled {
        (MenuState::Working, IDS_STATUS_WORKING)
    } else {
        (MenuState::Paused, IDS_STATUS_SUSPENDED)
    }
}

impl Content for TrayMenu {
    type Id = Id;

    fn describe(&self) -> Window<Id> {
        let (state, words) = self.state();
        let mut body = vec![Node::MenuHead {
            state,
            words: settings::text_in(self.language, words),
        }];
        let mut disabled = Vec::new();

        for entry in self.now.menu.entries() {
            let MenuEntry::Item(item) = entry else {
                body.push(Node::MenuRule);
                continue;
            };
            let Some(id) = Id::of(item.command) else {
                continue;
            };
            if !item.enabled {
                disabled.push(id);
            }
            let icon = match id {
                Id::Unread => "mail",
                Id::Update => "down",
                Id::WhatsNew => "star",
                Id::Toggle if self.now.enabled => "pause",
                Id::Toggle => "play",
                Id::Settings => "gear",
                Id::Autostart => "boot",
                Id::Write => "pen",
                Id::About => "info",
                Id::Exit => "exit",
                // Never an item: `Id::of` gives no command `Escape`.
                Id::Escape => continue,
            };
            let trailing = match id {
                // The key that suspends and resumes — not while a restart is the only way back.
                Id::Toggle if state != MenuState::Restart => Trailing::Key(self.now.key.clone()),
                Id::Autostart => Trailing::Switch(item.checked),
                Id::Unread => Trailing::Dot,
                _ => Trailing::None,
            };
            body.push(Node::MenuItem {
                id,
                icon: icon.to_owned(),
                label: item.label.clone(),
                trailing,
                emphasis: id == Id::Unread,
            });
        }

        Window {
            kind: WindowKind::Menu,
            art: None,
            brand: None,
            body: Node::Column(body),
            buttons: Vec::new(),
            disabled,
        }
    }

    fn event(&mut self, event: Event<Id>) -> Reply {
        match event {
            Event::Pressed(_) => Reply::Close,
            _ => Reply::Stay,
        }
    }

    fn title(&self) -> String {
        crate::APP_NAME.to_owned()
    }

    fn language(&self) -> (String, bool) {
        (self.language.tag().to_owned(), self.language.is_rtl())
    }

    fn theme(&self) -> ThemeSetting {
        self.now.theme
    }
}

/// Every state the menu shows, for the instrument of fitting: the plain menu; with an unread
/// letter, a newer version and «Что нового»; suspended; needing a restart; and with the settings
/// window up.
pub fn states(language: Language) -> Vec<(String, Window<Id>)> {
    let pending = Pending {
        unread: true,
        update: Some("1.8.1".to_owned()),
        whats_new: Some("1.8.0".to_owned()),
    };
    let cases = [
        ("menu", true, false, false, Pending::default()),
        ("menu-letters", true, false, false, pending),
        ("menu-paused", false, false, false, Pending::default()),
        ("menu-restart", false, true, false, Pending::default()),
        ("menu-dialog", true, false, true, Pending::default()),
    ];
    cases
        .into_iter()
        .map(|(name, enabled, fail_safe, dialog, pending)| {
            let menu = TrayMenu {
                now: MenuNow {
                    menu: Menu::build_in(
                        language, enabled, true, fail_safe, dialog, false, &pending,
                    ),
                    enabled,
                    fail_safe,
                    key: "Pause".to_owned(),
                    theme: ThemeSetting::Light,
                },
                language,
            };
            (name.to_owned(), menu.describe())
        })
        .collect()
}

/// The menu the icon opens: the description and what the program adds to it.
struct Product {
    menu: TrayMenu,
    /// The program's window: the menu's owner, and the window its chosen command runs on.
    owner: HWND,
    /// The command of the item pressed — run by the tray once the menu is gone.
    chosen: Option<u32>,
    /// The menu went by Esc: no press on the icon closed it, so the next press opens it again.
    escaped: bool,
}

impl Content for Product {
    type Id = Id;

    fn describe(&self) -> Window<Id> {
        self.menu.describe()
    }

    fn event(&mut self, event: Event<Id>) -> Reply {
        match event {
            // FR-93: the switch changes the setting at once, and the menu stays to show it.
            Event::Toggled(Id::Autostart, _) => {
                tray::dispatch_command(self.owner, CMD_AUTOSTART);
                if let Some(now) = tray::menu_now() {
                    self.menu.now = now;
                }
                Reply::Stay
            }
            Event::Pressed(Id::Escape) => {
                self.escaped = true;
                Reply::Close
            }
            Event::Pressed(id) => {
                self.chosen = Some(id.command());
                Reply::Close
            }
            _ => Reply::Stay,
        }
    }

    fn title(&self) -> String {
        self.menu.title()
    }

    fn language(&self) -> (String, bool) {
        self.menu.language()
    }

    fn theme(&self) -> ThemeSetting {
        self.menu.theme()
    }

    fn escape(&self) -> Option<Id> {
        Some(Id::Escape)
    }
}

impl Drop for Product {
    /// The host drops the content when the menu is destroyed, however it went: the tray forgets the
    /// menu and runs the command chosen in it, if one was.
    fn drop(&mut self) {
        tray::menu_closed(self.owner, self.chosen.take(), self.escaped);
    }
}

/// Opens the menu of the icon on `owner`, the program's window, at the point `at` of the screen;
/// from the keyboard (`keyboard`) its first available item is marked. Answers the menu, or none.
pub fn open(owner: HWND, at: POINT, keyboard: bool) -> Option<HWND> {
    let now = tray::menu_now()?;
    let product = Product {
        menu: TrayMenu {
            now,
            language: settings::ui_language(),
        },
        owner,
        chosen: None,
        escaped: false,
    };

    #[cfg(feature = "testing")]
    let made = if testing::hidden() {
        host::create_menu(product, Some(owner), at, keyboard)
    } else {
        host::open_menu(product, Some(owner), at, keyboard)
    };
    #[cfg(not(feature = "testing"))]
    let made = host::open_menu(product, Some(owner), at, keyboard);

    match made {
        Ok(menu) => {
            tray::menu_opened(menu);
            Some(menu)
        }
        Err(error) => {
            crate::app::report_non_critical("CreateWindowExW", &error);
            None
        }
    }
}

/// The door of the tests: [`open`] makes the menu as the product makes it and leaves it hidden.
#[cfg(feature = "testing")]
pub mod testing {
    use std::cell::Cell;

    thread_local! {
        static HIDDEN: Cell<bool> = const { Cell::new(false) };
    }

    pub(super) fn hidden() -> bool {
        HIDDEN.with(Cell::get)
    }

    /// Runs `f` with the menu of [`super::open`] left hidden on this thread.
    pub fn hidden_while<R>(f: impl FnOnce() -> R) -> R {
        struct Reset;
        impl Drop for Reset {
            fn drop(&mut self) {
                HIDDEN.with(|hidden| hidden.set(false));
            }
        }

        HIDDEN.with(|hidden| hidden.set(true));
        let _reset = Reset;
        f()
    }
}
