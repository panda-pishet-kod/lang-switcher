//! Window descriptions: which elements, in what order, which strings. No numbers, colours or
//! coordinates.

pub mod about;

use crate::settings::Language;
use crate::ui::scene::Window;

/// Every window of the shell in every state it shows — each step and each section — in
/// `language`, named for the reports. The instrument of fitting (`tests\ui_fit.rs`) goes over this
/// list: a new window adds its line here, and from then on it is fitted with the others.
pub fn catalogue(language: Language) -> Vec<(String, Window<u16>)> {
    let mut all = Vec::new();
    all.extend(
        about::states(language)
            .into_iter()
            .map(|(name, window)| (name, window.map(&|id| id as u16))),
    );
    all
}
