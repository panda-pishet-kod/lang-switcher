//! The window in the corner on the shell (stage Э120, T-120-14 п. 2): the hint of the flag,
//! «Обновлено» and the notice of a letter are one description of the words the program gives —
//! `corner::Content`, built where the window is decided (`corner.rs`, `letters.rs`). The parts
//! and their order are the mock-up's: the header with the icon, «LANG SWITCHER · …» and the ✕; the
//! panda beside the title, the text and the link; the buttons, the second one share at the start
//! and the main two at the end; the check box under them. Mirrored by the language of the program
//! as every window of the shell.

use crate::corner::Content;
use crate::ui::scene::{Art, ButtonKind, Node, Taskbar, Window, WindowKind};

/// The elements of the window in the corner.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Id {
    Close,
    Main,
    Second,
    Link,
    Check,
}

/// The window in the corner saying `content`; its check box ticked when `checked`; `picture` —
/// the moment of the picture of the taskbar, for a content that has one.
pub fn describe(content: &Content, checked: bool, picture: Option<Taskbar>) -> Window<Id> {
    let mut parts = vec![
        Node::CornerHeader {
            about: format!("{} · {}", crate::APP_NAME, content.about),
            close: Id::Close,
        },
        Node::Message {
            title: content.title.clone(),
            words: content.words().to_owned(),
            link: content
                .link
                .as_ref()
                .map(|(label, _)| (Id::Link, label.clone())),
        },
    ];
    if let Some(picture) = picture.filter(|_| content.picture) {
        parts.push(Node::Taskbar(picture));
    }
    let mut buttons = Vec::new();
    if let Some((label, _)) = &content.second {
        buttons.push(Node::Button {
            id: Id::Second,
            label: label.clone(),
            kind: ButtonKind::Normal,
            browser: false,
        });
    }
    buttons.push(Node::Button {
        id: Id::Main,
        label: content.main.0.clone(),
        kind: ButtonKind::Primary,
        browser: false,
    });
    parts.push(Node::Buttons(buttons));
    if let Some(label) = &content.check {
        parts.push(Node::Check {
            id: Id::Check,
            label: label.clone(),
            on: checked,
            hint: None,
        });
    }
    Window {
        kind: WindowKind::Corner,
        art: Some(Art::Avatar),
        brand: None,
        body: Node::Column(parts),
        buttons: Vec::new(),
        disabled: Vec::new(),
    }
}
