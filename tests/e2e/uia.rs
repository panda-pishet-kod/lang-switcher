//! UI Automation: how the bench decides a window is ready and how it reads the result back.
//!
//! Requirements 1 and 3 of §11.5 both live here — readiness is a property the tree is asked
//! about, and the result is read through `ValuePattern` or `TextPattern`.
//!
//! # Why the tree is walked rather than searched
//!
//! `IUIAutomation::CreatePropertyCondition` and `IUIAutomationElement::GetCurrentPropertyValue`
//! both take a `VARIANT`, and the `windows` crate gates them behind the features
//! `Win32_System_Variant` and `Win32_System_Ole`. Neither is in the closed dependency list of
//! §3.2 of SPEC, and the task allows exactly one new section in `Cargo.toml` — so the bench
//! does not use them. What it uses instead is `CreateTrueCondition`, the raw-view tree walker
//! and the ungated `Current*` accessors, and it filters in Rust. This costs nothing in
//! capability: the predicates the matrix needs are over class, name, control type and process,
//! and all four are readable without a `VARIANT`.
//!
//! Both walks are **bounded** — by depth and by node count. An unbounded descendant search on
//! a Word or VS Code tree is a request that can take longer than the scenario it serves.

use std::time::Duration;

use windows::Win32::Foundation::HWND;
use windows::Win32::System::Com::{CLSCTX_INPROC_SERVER, CoCreateInstance};
use windows::Win32::UI::Accessibility::{
    CUIAutomation, IUIAutomation, IUIAutomationElement, IUIAutomationInvokePattern,
    IUIAutomationTextPattern, IUIAutomationTreeWalker, IUIAutomationValuePattern,
    TextUnit_Document, TreeScope_Children, UIA_CONTROLTYPE_ID, UIA_InvokePatternId,
    UIA_TextPatternId, UIA_ValuePatternId,
};

use crate::wait;

/// How deep a walk goes before it gives up.
///
/// The deepest thing the matrix reaches for is the document element of Word, which sits about
/// eight levels down inside `OpusApp`; VS Code's editor is of a similar order. Twenty is
/// comfortably past both and still bounds the work.
const MAX_DEPTH: u32 = 20;

/// How many elements a single walk may visit.
///
/// A guard against a tree that is being rebuilt underneath the walk, which is what an
/// application that is still starting looks like.
const MAX_NODES: u32 = 4000;

/// Longest string the bench will read out of a text range.
///
/// The scenarios type six characters. A bound exists so that pointing the bench at a document
/// that already has content cannot turn a read into a transfer of somebody's file.
const MAX_TEXT: i32 = 4096;

/// The UI Automation client object, plus the raw-view walker it hands out.
///
/// One per thread, created after `CoInitializeEx`. Held for the whole run: creating it per
/// scenario would add a COM activation to every position for no benefit.
pub struct Automation {
    automation: IUIAutomation,
    walker: IUIAutomationTreeWalker,
}

impl Automation {
    /// Creates the client. COM must already be initialised on this thread.
    pub fn new() -> windows::core::Result<Self> {
        // SAFETY: `CoCreateInstance` is called on an apartment that `main` initialised, with
        // the documented class id of the UI Automation client and no aggregation. The returned
        // interface is reference-counted by the `windows` crate wrapper. NFR-13: the call
        // returns a `Result` and the `?` propagates a failure rather than continuing with an
        // unusable object.
        let automation: IUIAutomation =
            unsafe { CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER)? };

        // SAFETY: `automation` is a live interface pointer just obtained above. `RawViewWalker`
        // is a property read that returns a new reference-counted interface.
        let walker = unsafe { automation.RawViewWalker()? };

        Ok(Self { automation, walker })
    }

    /// The element that currently has keyboard focus.
    ///
    /// Used to confirm, before typing into Word, that focus really did land on `_WwG` — the
    /// second half of rake 3 of §2 of `TOOLCHAIN.md`.
    pub fn focused(&self) -> Option<Element> {
        // SAFETY: no arguments, and the result is a new reference-counted interface or an
        // error. An error means nothing has focus at this instant, which is a `None`.
        unsafe { self.automation.GetFocusedElement() }
            .ok()
            .map(Element)
    }

    /// Top-level windows belonging to `pid`.
    ///
    /// The children of the root element are the desktop's top-level windows; a true condition
    /// plus a filter in Rust is what stands in for a property condition here.
    pub fn top_level_of(&self, pid: u32) -> Vec<Element> {
        // SAFETY: both calls act on live interface pointers held by `self`. `CreateTrueCondition`
        // allocates a condition object; `FindAll` returns an array or an error, and an error is
        // answered with an empty list rather than a panic — a desktop can refuse a query while
        // it is switching.
        let found = unsafe {
            self.automation.CreateTrueCondition().and_then(|any| {
                self.automation
                    .GetRootElement()?
                    .FindAll(TreeScope_Children, &any)
            })
        };

        let Ok(array) = found else {
            return Vec::new();
        };

        // SAFETY: `array` is a live element array; `Length` and `GetElement` are its accessors
        // and the index is kept inside the length just read.
        let count = unsafe { array.Length() }.unwrap_or(0);
        (0..count)
            .filter_map(|index| unsafe { array.GetElement(index) }.ok())
            .map(Element)
            .filter(|element| element.pid() == Some(pid))
            .collect()
    }

    /// Top-level windows satisfying `predicate`, whatever process they belong to.
    ///
    /// Needed because several applications the matrix names do not own their own window in the
    /// process `spawn` returned: Chrome and VS Code spread over a process tree, `wt.exe` is a
    /// launcher that exits, and `explorer.exe` hands the request to the running shell. For
    /// those, the window is identified first and the scenario then adopts **its** process id
    /// as the target of the foreground check — so the check still compares against a window
    /// the bench opened and identified, which is what the safety rule asks.
    pub fn top_level_of_any(&self, predicate: &dyn Fn(&Element) -> bool) -> Vec<Element> {
        // SAFETY: as in `top_level_of` — live interface pointers held by `self`, and an error
        // is answered with an empty list.
        let found = unsafe {
            self.automation.CreateTrueCondition().and_then(|any| {
                self.automation
                    .GetRootElement()?
                    .FindAll(TreeScope_Children, &any)
            })
        };

        let Ok(array) = found else {
            return Vec::new();
        };

        // SAFETY: `array` is live; the index stays inside the length just read.
        let count = unsafe { array.Length() }.unwrap_or(0);
        (0..count)
            .filter_map(|index| unsafe { array.GetElement(index) }.ok())
            .map(Element)
            .filter(|element| predicate(element))
            .collect()
    }

    /// Depth-first search under `root` for the first element satisfying `predicate`.
    ///
    /// Bounded by [`MAX_DEPTH`] and [`MAX_NODES`]; `None` means "not found within those
    /// bounds", which the caller turns into a wait or a verdict, never into an assumption.
    pub fn find(&self, root: &Element, predicate: &dyn Fn(&Element) -> bool) -> Option<Element> {
        let mut budget = MAX_NODES;
        self.descend(root, predicate, 0, &mut budget)
    }

    /// Every element under `root` satisfying `predicate`, within the same bounds as [`find`].
    pub fn find_all(&self, root: &Element, predicate: &dyn Fn(&Element) -> bool) -> Vec<Element> {
        let mut budget = MAX_NODES;
        let mut found = Vec::new();
        self.collect(root, predicate, 0, &mut budget, &mut found);
        found
    }

    fn descend(
        &self,
        node: &Element,
        predicate: &dyn Fn(&Element) -> bool,
        depth: u32,
        budget: &mut u32,
    ) -> Option<Element> {
        if depth > MAX_DEPTH || *budget == 0 {
            return None;
        }
        *budget -= 1;

        if predicate(node) {
            return Some(node.clone());
        }

        let mut child = self.first_child(node);
        while let Some(current) = child {
            if let Some(hit) = self.descend(&current, predicate, depth + 1, budget) {
                return Some(hit);
            }
            child = self.next_sibling(&current);
        }

        None
    }

    fn collect(
        &self,
        node: &Element,
        predicate: &dyn Fn(&Element) -> bool,
        depth: u32,
        budget: &mut u32,
        found: &mut Vec<Element>,
    ) {
        if depth > MAX_DEPTH || *budget == 0 {
            return;
        }
        *budget -= 1;

        if predicate(node) {
            found.push(node.clone());
        }

        let mut child = self.first_child(node);
        while let Some(current) = child {
            self.collect(&current, predicate, depth + 1, budget, found);
            child = self.next_sibling(&current);
        }
    }

    fn first_child(&self, node: &Element) -> Option<Element> {
        // SAFETY: `self.walker` and `node.0` are live interface pointers. The walker returns a
        // new reference or an error; an error, and a null return, both mean "no child" and are
        // answered with `None`.
        unsafe { self.walker.GetFirstChildElement(&node.0) }
            .ok()
            .map(Element)
    }

    fn next_sibling(&self, node: &Element) -> Option<Element> {
        // SAFETY: as `first_child` above.
        unsafe { self.walker.GetNextSiblingElement(&node.0) }
            .ok()
            .map(Element)
    }

    /// Waits for a top-level window of `pid` matching `predicate` to exist — requirement 1.
    ///
    /// This is what "readiness through UI Automation, not through a delay" means in practice:
    /// the question asked is "does the tree contain this window yet", and the answer arrives
    /// the moment it does.
    pub fn await_window(
        &self,
        pid: u32,
        timeout: Duration,
        predicate: &dyn Fn(&Element) -> bool,
    ) -> Option<Element> {
        wait::until(timeout, || {
            self.top_level_of(pid).into_iter().find(|w| predicate(w))
        })
    }

    /// Waits for an element under `root` matching `predicate` — the same idea one level down.
    ///
    /// A window existing is not the same as its content element existing: Word's `OpusApp` is
    /// in the tree well before `_WwG` is, and typing in between goes nowhere. Every scenario
    /// therefore waits for the element it is going to *read*, not merely for a window.
    pub fn await_element(
        &self,
        root: &Element,
        timeout: Duration,
        predicate: &dyn Fn(&Element) -> bool,
    ) -> Option<Element> {
        wait::until(timeout, || self.find(root, predicate))
    }
}

/// One node of the UI Automation tree.
#[derive(Clone)]
pub struct Element(pub IUIAutomationElement);

impl Element {
    /// `Name`, or an empty string when it cannot be read.
    pub fn name(&self) -> String {
        // SAFETY: `self.0` is a live interface pointer; `CurrentName` returns an allocated
        // string the wrapper frees, or an error when the element has gone away.
        unsafe { self.0.CurrentName() }
            .map(|s| s.to_string())
            .unwrap_or_default()
    }

    /// `ClassName`, or an empty string.
    pub fn class(&self) -> String {
        // SAFETY: as `name` above.
        unsafe { self.0.CurrentClassName() }
            .map(|s| s.to_string())
            .unwrap_or_default()
    }

    /// Owning process id.
    pub fn pid(&self) -> Option<u32> {
        // SAFETY: as `name` above; the result is an `i32` the system fills in.
        unsafe { self.0.CurrentProcessId() }
            .ok()
            .map(|id| id as u32)
    }

    /// Control type — `UIA_EditControlTypeId`, `UIA_DocumentControlTypeId` and so on.
    pub fn control_type(&self) -> Option<UIA_CONTROLTYPE_ID> {
        // SAFETY: as `name` above.
        unsafe { self.0.CurrentControlType() }.ok()
    }

    /// `IsPasswordProperty` — **the property level 3 of FR-72 reads**, asked of the same element.
    ///
    /// `Some(true)` is a password field; `Some(false)` an element that answered and is not one;
    /// `None` is "no answer" — the provider does not implement the property, or stopped
    /// replying. The three outcomes are kept apart on purpose (NFR-13): the product's own
    /// `guard::is_password_element` distinguishes exactly the same three, and a bench that
    /// folded `None` into `false` could report "the field is ordinary" about a field nobody ever
    /// answered for.
    ///
    /// ⚠ This is the one reading that says **which level of FR-72 a scenario exercises**.
    /// Position 14's `ES_PASSWORD` box is caught by level 2 (`EM_GETPASSWORDCHAR`) and never
    /// reaches level 3; a browser's `<input type="password">` has no `Edit` window class at all,
    /// so level 3 is the only level that can see it. Asserting `Some(true)` here, before a key
    /// is sent, is how the HTML pillar of position 14 states in its own report row that the
    /// branch under test is that one.
    pub fn is_password(&self) -> Option<bool> {
        // SAFETY: `self.0` is a live interface pointer, as in `name` above. The property read
        // takes no arguments of ours and returns a `BOOL` or a failing `HRESULT`. NFR-13: the
        // failure is examined and becomes `None` rather than a `false` that would read as a
        // verdict about the element.
        unsafe { self.0.CurrentIsPassword() }
            .ok()
            .map(|flag| flag.as_bool())
    }

    /// The window handle behind this element, when it has one.
    pub fn hwnd(&self) -> Option<HWND> {
        // SAFETY: as `name` above. A zero handle means the element is not a window, which is
        // reported as `None` rather than as a handle that would fail on first use.
        unsafe { self.0.CurrentNativeWindowHandle() }
            .ok()
            .filter(|h| !h.is_invalid())
    }

    /// The text of this element through `ValuePattern` — requirement 3, first choice.
    pub fn value(&self) -> Option<String> {
        // SAFETY: `self.0` is live. `GetCurrentPatternAs` asks the element for an interface by
        // pattern id and fails when the pattern is unsupported, which is a `None` here.
        let pattern: IUIAutomationValuePattern =
            unsafe { self.0.GetCurrentPatternAs(UIA_ValuePatternId) }.ok()?;

        // SAFETY: `pattern` is a live interface pointer obtained immediately above.
        unsafe { pattern.CurrentValue() }
            .ok()
            .map(|s| s.to_string())
    }

    /// The text of this element through `TextPattern` — requirement 3, second choice.
    ///
    /// Word's document supports this one and not `ValuePattern`, which is why the requirement
    /// names both.
    pub fn text(&self) -> Option<String> {
        // SAFETY: as `value` above.
        let pattern: IUIAutomationTextPattern =
            unsafe { self.0.GetCurrentPatternAs(UIA_TextPatternId) }.ok()?;

        // SAFETY: `pattern` is live; `DocumentRange` returns a new range object, and
        // `ExpandToEnclosingUnit` widens it to the whole document. `GetText` is bounded by
        // `MAX_TEXT` so the call cannot be turned into an unbounded read of a real document.
        let text = unsafe {
            let range = pattern.DocumentRange().ok()?;
            let _ = range.ExpandToEnclosingUnit(TextUnit_Document);
            range.GetText(MAX_TEXT).ok()?
        };

        Some(text.to_string())
    }

    /// What the scenario compares against the expected value: `ValuePattern` if the element
    /// has one, otherwise `TextPattern`, together with which of the two answered.
    pub fn read(&self) -> Option<(String, Source)> {
        if let Some(value) = self.value() {
            return Some((value, Source::Value));
        }
        self.text().map(|text| (text, Source::Text))
    }

    /// Invokes the element, when it supports being invoked.
    ///
    /// Used to open «Избранное» in Telegram: the alternative — typing a chat name into a search
    /// box and pressing `Enter` — is the exact shape of action footnote 7 of §11.3 forbids, so
    /// the bench opens the chat through the accessibility tree or not at all.
    pub fn invoke(&self) -> bool {
        // SAFETY: `self.0` is live. `GetCurrentPatternAs` fails when the pattern is
        // unsupported, and `Invoke` when the element has gone; both are answered with `false`,
        // which the caller turns into a recorded refusal rather than a fallback to the mouse.
        let Ok(pattern) = (unsafe {
            self.0
                .GetCurrentPatternAs::<IUIAutomationInvokePattern>(UIA_InvokePatternId)
        }) else {
            return false;
        };

        // SAFETY: `pattern` is a live interface pointer obtained immediately above.
        unsafe { pattern.Invoke() }.is_ok()
    }

    /// A one-line description for the report.
    pub fn describe(&self) -> String {
        format!(
            "class={:?} name={:?} pid={:?}",
            self.class(),
            self.name(),
            self.pid()
        )
    }
}

/// Which pattern produced a reading — requirement 3 wants this said out loud.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Value,
    Text,
}

impl Source {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Value => "ValuePattern",
            Self::Text => "TextPattern",
        }
    }
}

/// Normalises a reading for comparison: line breaks and the object replacement characters some
/// controls pad with become spaces, and the ends are trimmed.
///
/// Word returns `"ghbdtn\r"`; a console returns the whole visible screen. Comparison is
/// therefore "contains the expected string", against a normalised reading, and every scenario
/// reports the raw value it actually saw.
pub fn normalise(raw: &str) -> String {
    raw.replace(['\r', '\n', '\u{fffc}', '\u{200b}'], " ")
        .trim()
        .to_owned()
}
