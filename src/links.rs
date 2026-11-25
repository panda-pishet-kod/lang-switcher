//! The addresses the program links to, and the one rule that governs all of them.
//!
//! # What is settled — решения 138, 139.2 и 147.3
//!
//! All five addresses are the author's own and permanent. The channel, the support page, the
//! download page and the program's page arrived in one word of the user's (2026-09-17); the
//! feed's followed on 2026-09-25 (147.3, stage Э88). They are string literals here and nothing
//! else — which is what the paragraph below promised the day the placeholders went in.
//!
//! No placeholder address is left in this module, and [`is_placeholder`] stays all the same: it
//! is the one question every button with a link stands behind, and the link of a feed entry is
//! written by the author's hand, not by this module. For the five constants it answers «no».
//!
//! # Placeholders, and why a button is drawn disabled rather than hidden
//!
//! The user's answer (полномочия П5, П7 и П8 of the mandate) was to build the windows now and
//! put a placeholder in each unsettled address — «кнопка пусть будет, вместо пути заглушка». So
//! each is a constant of this module, [`is_placeholder`] recognises a placeholder by its domain,
//! and every button that would open one is drawn in the disabled role the own-drawn buttons
//! already have. The window is the finished window; what is missing is visibly missing.
//!
//! `example.invalid` and not a made-up host: RFC 2606 reserves `.invalid` precisely so that it
//! never resolves. A placeholder that fell out of this rule and reached the network would fail
//! slowly, against somebody else's machine; this one fails at once, against nobody's.
//!
//! # ⚠ Two different things, and the difference matters
//!
//! **The network surface of this program is [`FEED_URLS`] and nothing else** — that is the one
//! address the program itself goes out and *fetches*, unasked, on a timer. The six below are
//! never fetched by this program at all: they are handed to the shell, one at a time, because a
//! person clicked a button, and it is the browser that goes to them. Ворота 3 of
//! `tools\verify-perimeter.ps1` reads the strings of the Release binary and refuses any
//! `https://` that is not one of the seven, which is why every one of them lives here.

use crate::settings::Language;

/// The reserved domain every placeholder address is under.
const PLACEHOLDER_HOST: &str = "example.invalid";

/// Where the feed is read from — FR-102, in order, the first good answer winning.
///
/// ⚠ **The whole of the network surface of this program is this array** — see the module's
/// own doc for what that does and does not mean about the six addresses below.
///
/// The author's own address since `e88` — word of the user, 2026-09-25 (решения 147.3 и
/// 148.1): the host of the other four, and **one** address, with no second road behind it.
/// Until then this was a placeholder by decision (138.4), and the guard that said so is now
/// the test `the_hard_wired_addresses_are_real_and_so_is_the_feed`.
pub const FEED_URLS: [&str; 1] = ["https://panda-pishet-kod.dev/langswitcher/news.toml"];

/// The author's channel — FR-103, «Что нового» and the thanks of the wizard: the «Открыть
/// канал» buttons, the road to the news.
///
/// ⚠ **A channel is read, not written to** — there is nowhere in it for a reader to write
/// (вопрос 181). So the two places that ask a person to write go to [`feedback_url`] since
/// решение 181.1: «Скопировать и открыть…» of the wizard of FR-104 and the letter «Спасибо».
pub const CHANNEL_URL: &str = "https://t.me/panda_pishet_kod";

/// The topic of Lang Switcher in the author's group — where a person who reads this program
/// in Russian or Ukrainian writes to the author (решение 181.1). The topic the site sends the
/// questions about the program to.
pub const GROUP_URL: &str = "https://t.me/panda_koding/45";

/// The issues of the program on GitHub — where a person who reads this program in any of the
/// other twelve languages writes to the author (решение 181.1), as the English pages of the
/// site send them.
///
/// The list of the issues and not the form of a new one: the address the site and README name,
/// and one that does not depend on the templates of the repository.
pub const ISSUES_URL: &str = "https://github.com/panda-pishet-kod/lang-switcher/issues";

/// The box of the appeals about this program — **the way back when the site did not answer**,
/// решение 199.3: «отдельная почта … (создать отдельный ящик для обращений по программе)».
/// The wizard sends an appeal to [`APPEAL_URL`]; when that fails, its last step shows this
/// address and copies it, and the person writes the letter themselves. Any appeal, a trouble
/// or an idea, in any language — and not the author's own address (`send.me@…`, решение 181.6,
/// which the wizard named until 1.2.0).
///
/// ⚠ **Not a link, and not in [`allowed_hosts`].** Nothing opens it: there is no `mailto:` and
/// no mail program is started. It is shown on the last step of the wizard and copied to the
/// clipboard by a button. The tables of FR-94 carry it only as `{0}`, so the fourteen
/// translations hold no copy of it.
pub const MAIL_ADDRESS: &str = "lang.switcher@panda-pishet-kod.dev";

/// **Where the wizard sends an appeal** — решения 199.1, 199.7 и 199.9: one `POST` of
/// [`super::appeal::body`] over HTTPS, when the person presses «Отправить», and at no other
/// moment.
///
/// ⚠ Not a page and not opened by anything, so not in [`allowed_hosts`] — the shell is never
/// handed it. It is in this module for the reason [`UPDATE_FILES_URL`] is: ворота 3 of
/// `tools\verify-perimeter.ps1` judge every `https://` of an image against the addresses
/// written here, and the image carries this one.
pub const APPEAL_URL: &str = "https://panda-pishet-kod.dev/langswitcher/feedback";

/// **Which side of the author's places a person reads, by the language they read the program
/// in** — решения 181.1, 181.33 и 202.6: the Russian side (the topic of the group, the pages of
/// the site without `/en/`) for the six languages below, the English one (GitHub Issues, the pages
/// under `/en/`) for every other. ⚠ Until stage E114 this sentence said «Russian and Ukrainian …
/// the other twelve», which was true of fourteen languages and stopped being true at sixty-six.
///
/// The one place the line is drawn — [`feedback_url`], [`download_url`] and [`program_url`]
/// all ask here, so a person is never sent to write on one side and to read on the other. A
/// `match` without a wildcard on purpose: a language added to [`Language`] does not compile
/// until somebody has said which side its readers are on.
///
/// ⚠ **Since stage E112 the pages ask [`OWN_PAGES`] first** (Q-206-1): a language of that table
/// reads the site in its own language and goes on writing to the author on its side here —
/// Ukrainian to the group, Arabic, Persian and Thai to GitHub Issues.
///
/// ⭐ **Stage E108, решение 202.6** — дословно: «Делаем так, пока нет адаптированных страниц
/// сайта под каждый из языков»: Kazakh, Belarusian, Tatar and Uzbek in Cyrillic read the
/// Russian side too; the other twenty-six of group A — the English one.
/// Stage E109: all twenty-two of group B — the English one (TZ E109 §0 п. 3).
fn reads_the_russian_side(language: Language) -> bool {
    match language {
        Language::Ru
        | Language::Uk
        | Language::Kk
        | Language::Be
        | Language::Tt
        | Language::UzCyrl => true,
        Language::En
        | Language::De
        | Language::Fr
        | Language::Es
        | Language::Pt
        | Language::It
        | Language::Pl
        | Language::Cs
        | Language::Tr
        | Language::El
        | Language::He
        | Language::Ar
        | Language::Hi
        | Language::Bn
        | Language::Ur
        | Language::Pa
        | Language::Mr
        | Language::Te
        | Language::Ta
        | Language::Fa
        | Language::Gu
        | Language::Th
        | Language::Kn
        | Language::Or
        | Language::Ml
        | Language::Uz
        | Language::Ne
        | Language::As
        | Language::Az
        | Language::Km
        | Language::Sr
        | Language::SrLatn
        | Language::Ug
        | Language::Bg
        | Language::Hy
        | Language::Lo
        | Language::Ka
        | Language::Mk
        | Language::Vi
        | Language::Nl
        | Language::Ro
        | Language::Hu
        | Language::Sv
        | Language::Sq
        | Language::Da
        | Language::Fi
        | Language::Hr
        | Language::Sk
        | Language::Nb
        | Language::Lt
        | Language::Sl
        | Language::Bs
        | Language::Lv
        | Language::Et
        | Language::Cy
        | Language::Mt
        | Language::Is
        | Language::Lb
        | Language::Mi
        | Language::Nn => false,
    }
}

/// **Which pages of the site a person without pages of their own reads** — the side of
/// [`reads_the_russian_side`], and, since stage E114 (task T-114-3, решение 208.6, 1а; Q-208-5),
/// the Russian pages for Armenian and Uzbek in the Latin script too: a reader of either script of
/// Uzbek and a reader of Armenian read Russian more often than English, while their channel stays
/// where it was ([`feedback_url`]). The pages and the channel are two decisions, as in task
/// T-112-1.
fn reads_the_russian_pages(language: Language) -> bool {
    reads_the_russian_side(language) || matches!(language, Language::Hy | Language::Uz)
}

/// **Where a person writes to the author, by the language they read the program in** —
/// решение 181.1: Russian and Ukrainian go to the topic of the group, the other twelve to the
/// issues on GitHub ([`reads_the_russian_side`]).
///
/// The one place that choice is made — the wizard of FR-104 and the letter «Спасибо» both ask
/// here, with the language of their own window.
///
/// ⚠ **Only the channel.** The pages are another decision ([`reads_the_russian_pages`], stage
/// E114): Armenian and Uzbek in the Latin script read the pages of the Russian side and still
/// write to the issues on GitHub.
pub fn feedback_url(language: Language) -> &'static str {
    if reads_the_russian_side(language) {
        GROUP_URL
    } else {
        ISSUES_URL
    }
}

/// **The pages the site has in the language of a person** — Q-206-1, решение 207.3:
    /// `(language, download page, program's page)`. One row is one language, and a language with a
    /// row leaves the pages of its side ([`reads_the_russian_side`]) for these.
    ///
    /// ⛔ **A row only on a line from the controller of the languages of the site** that the pages
    /// are out: the mark outranks the browser and the site remembers it, so `?lang=` of a language
    /// the site does not have would strand the reader. `he` comes next.
    ///
    /// ⚠ The addresses are written out whole and not put together by `concat!`: ворота 3 of
    /// `tools\verify-perimeter.ps1` read the declared addresses as string literals of this file, and
    /// one built by a macro would stand in the image undeclared.
    #[rustfmt::skip]
    ///
    /// ⭐ **Stage E114, task T-114-3** (Q-208-1…4, Q-208-6, Q-208-8, решения 208.6 и 208.8) — fourteen
    /// more, each on the line of the controller that its pages are out (2026-11-10: every address
    /// 200 and in its own `lang`). Both Serbian scripts read `/sr/` under the mark `sr` (the page is
    /// `lang="sr-Latn"`), Portuguese reads `/pt/` under `pt` (the page is `lang="pt-BR"`). In the
    /// order of [`Language::ALL`]. Hungarian after them, on the line of the controller of the same
    /// day (208.1: both addresses 200, `lang=hu`). ⭐ Stage E116, task T-116-6 (Q-208-13 п. 1):
    /// Swedish and Slovak, on the line of the controller of the languages of the site of 2026-11-12;
    /// all four addresses checked in Chrome before the rows were written (2026-11-14: 200,
    /// `lang=sv` and `lang=sk`).
    ///
    /// ⭐ **Z12 of AUDIT-180 (решение 208.209, слово владельца П-291 PandaKod)** — the third address
    /// of a row: the page «Как пользоваться: флаг раскладки», `/xx/langswitcher/docs/flag/?lang=xx`,
    /// the target of «Как пользоваться» of «Обновлено» ([`flag_url`]). ⚠ Until the day of the
    /// release the site keeps it as a draft and answers 404 — no test asks it to answer.
    const OWN_PAGES: [(Language, &str, &str, &str); 21] = [
        (Language::Uk, "https://panda-pishet-kod.dev/uk/langswitcher/download/?lang=uk", "https://panda-pishet-kod.dev/uk/langswitcher/?lang=uk", "https://panda-pishet-kod.dev/uk/langswitcher/docs/flag/?lang=uk"),
        (Language::De, "https://panda-pishet-kod.dev/de/langswitcher/download/?lang=de", "https://panda-pishet-kod.dev/de/langswitcher/?lang=de", "https://panda-pishet-kod.dev/de/langswitcher/docs/flag/?lang=de"),
        (Language::Fr, "https://panda-pishet-kod.dev/fr/langswitcher/download/?lang=fr", "https://panda-pishet-kod.dev/fr/langswitcher/?lang=fr", "https://panda-pishet-kod.dev/fr/langswitcher/docs/flag/?lang=fr"),
        (Language::Pt, "https://panda-pishet-kod.dev/pt/langswitcher/download/?lang=pt", "https://panda-pishet-kod.dev/pt/langswitcher/?lang=pt", "https://panda-pishet-kod.dev/pt/langswitcher/docs/flag/?lang=pt"),
        (Language::It, "https://panda-pishet-kod.dev/it/langswitcher/download/?lang=it", "https://panda-pishet-kod.dev/it/langswitcher/?lang=it", "https://panda-pishet-kod.dev/it/langswitcher/docs/flag/?lang=it"),
        (Language::Pl, "https://panda-pishet-kod.dev/pl/langswitcher/download/?lang=pl", "https://panda-pishet-kod.dev/pl/langswitcher/?lang=pl", "https://panda-pishet-kod.dev/pl/langswitcher/docs/flag/?lang=pl"),
        (Language::Cs, "https://panda-pishet-kod.dev/cs/langswitcher/download/?lang=cs", "https://panda-pishet-kod.dev/cs/langswitcher/?lang=cs", "https://panda-pishet-kod.dev/cs/langswitcher/docs/flag/?lang=cs"),
        (Language::Tr, "https://panda-pishet-kod.dev/tr/langswitcher/download/?lang=tr", "https://panda-pishet-kod.dev/tr/langswitcher/?lang=tr", "https://panda-pishet-kod.dev/tr/langswitcher/docs/flag/?lang=tr"),
        (Language::El, "https://panda-pishet-kod.dev/el/langswitcher/download/?lang=el", "https://panda-pishet-kod.dev/el/langswitcher/?lang=el", "https://panda-pishet-kod.dev/el/langswitcher/docs/flag/?lang=el"),
        (Language::He, "https://panda-pishet-kod.dev/he/langswitcher/download/?lang=he", "https://panda-pishet-kod.dev/he/langswitcher/?lang=he", "https://panda-pishet-kod.dev/he/langswitcher/docs/flag/?lang=he"),
        (Language::Ar, "https://panda-pishet-kod.dev/ar/langswitcher/download/?lang=ar", "https://panda-pishet-kod.dev/ar/langswitcher/?lang=ar", "https://panda-pishet-kod.dev/ar/langswitcher/docs/flag/?lang=ar"),
        (Language::Fa, "https://panda-pishet-kod.dev/fa/langswitcher/download/?lang=fa", "https://panda-pishet-kod.dev/fa/langswitcher/?lang=fa", "https://panda-pishet-kod.dev/fa/langswitcher/docs/flag/?lang=fa"),
        (Language::Th, "https://panda-pishet-kod.dev/th/langswitcher/download/?lang=th", "https://panda-pishet-kod.dev/th/langswitcher/?lang=th", "https://panda-pishet-kod.dev/th/langswitcher/docs/flag/?lang=th"),
        (Language::Az, "https://panda-pishet-kod.dev/az/langswitcher/download/?lang=az", "https://panda-pishet-kod.dev/az/langswitcher/?lang=az", "https://panda-pishet-kod.dev/az/langswitcher/docs/flag/?lang=az"),
        (Language::Sr, "https://panda-pishet-kod.dev/sr/langswitcher/download/?lang=sr", "https://panda-pishet-kod.dev/sr/langswitcher/?lang=sr", "https://panda-pishet-kod.dev/sr/langswitcher/docs/flag/?lang=sr"),
        (Language::SrLatn, "https://panda-pishet-kod.dev/sr/langswitcher/download/?lang=sr", "https://panda-pishet-kod.dev/sr/langswitcher/?lang=sr", "https://panda-pishet-kod.dev/sr/langswitcher/docs/flag/?lang=sr"),
        (Language::Bg, "https://panda-pishet-kod.dev/bg/langswitcher/download/?lang=bg", "https://panda-pishet-kod.dev/bg/langswitcher/?lang=bg", "https://panda-pishet-kod.dev/bg/langswitcher/docs/flag/?lang=bg"),
        (Language::Nl, "https://panda-pishet-kod.dev/nl/langswitcher/download/?lang=nl", "https://panda-pishet-kod.dev/nl/langswitcher/?lang=nl", "https://panda-pishet-kod.dev/nl/langswitcher/docs/flag/?lang=nl"),
        (Language::Hu, "https://panda-pishet-kod.dev/hu/langswitcher/download/?lang=hu", "https://panda-pishet-kod.dev/hu/langswitcher/?lang=hu", "https://panda-pishet-kod.dev/hu/langswitcher/docs/flag/?lang=hu"),
        (Language::Sv, "https://panda-pishet-kod.dev/sv/langswitcher/download/?lang=sv", "https://panda-pishet-kod.dev/sv/langswitcher/?lang=sv", "https://panda-pishet-kod.dev/sv/langswitcher/docs/flag/?lang=sv"),
        (Language::Sk, "https://panda-pishet-kod.dev/sk/langswitcher/download/?lang=sk", "https://panda-pishet-kod.dev/sk/langswitcher/?lang=sk", "https://panda-pishet-kod.dev/sk/langswitcher/docs/flag/?lang=sk"),
    ];

/// The row of [`OWN_PAGES`] for this language, if it has one.
fn own_pages(language: Language) -> Option<(Language, &'static str, &'static str, &'static str)> {
    OWN_PAGES
        .iter()
        .copied()
        .find(|(own, _, _, _)| *own == language)
}

/// The support page — FR-103, one link and no payment details of any kind in the program
/// (вопрос 101 п. 6).
pub const SUPPORT_URL: &str = "https://panda-pishet-kod.dev/donate/";

/// The download page — FR-103, the target of «Открыть страницу загрузки».
///
/// # Why the build's own address beats the one in the feed entry
///
/// An `update` entry of the feed carries a `link` of its own, and until task T-78-2 that
/// link was what the button opened. It loses to this constant on one argument: a feed entry
/// is **signed and dated**, so its address is as old as the entry and goes stale, whereas
/// this page is permanent and always shows the current version. Word of the user, 2026-09-17:
/// «если новость об обновлении пришла, значит оно уже лежит на сайте, и перейдя по ссылке
/// загрузки программы, мы уже попадаем на актуальную версию».
///
/// The `link` field stays in the feed format — it is the format's, not ours — and this
/// program simply no longer reads it for this button.
///
/// ⭐ **The Russian page** — since решение 181.33 the button opens [`download_url`] of the
/// language of its window, and this is the page of the Russian side, marked `?lang=ru` for the
/// site (решение 181.35, see [`download_url`]).
pub const DOWNLOAD_URL: &str = "https://panda-pishet-kod.dev/langswitcher/download/?lang=ru";

/// The English download page — the pair of [`DOWNLOAD_URL`] under `/en/`, решение 181.33, marked
/// `?lang=en` (решение 181.35): what «Open the download page» opens for a person who reads the
/// program in any of the twelve languages of the English side.
pub const DOWNLOAD_URL_EN: &str = "https://panda-pishet-kod.dev/en/langswitcher/download/?lang=en";

/// **The download page in the language a person reads the program in** — [`OWN_PAGES`], or else
/// решение 181.33 ([`reads_the_russian_side`]): «Открыть страницу загрузки» of «От автора» and
/// of the letter about an update.
///
/// ⚠ **Why each address carries `?lang=`** — решение 181.35. The page alone does not decide the
/// language a person sees: the script of the language of a page on the site (П-179 of the site)
/// moves a person to the choice they made with the site's own language button, or else to the
/// language of the browser (поправка 181.34). The mark tells the site the language of the
/// program; the name `lang` and its values are the ones the site already gives that word. A
/// site that does not read the mark yet simply ignores it.
pub fn download_url(language: Language) -> &'static str {
    match own_pages(language) {
        Some((_, download, _, _)) => download,
        None if reads_the_russian_pages(language) => DOWNLOAD_URL,
        None => DOWNLOAD_URL_EN,
    }
}

/// The program's own page — the target of a click on the name «Lang Switcher» in «О
/// программе», task T-78-3.
///
/// The only address of the five with no button of its own: it is opened by a click on the
/// name itself, which is a push button without a border and looks like the label it used to
/// be until the pointer is over it.
///
/// ⭐ **The Russian page** — since решение 181.33 the name opens [`program_url`] of the
/// language of its window; marked `?lang=ru` for the site (решение 181.35).
pub const PROGRAM_URL: &str = "https://panda-pishet-kod.dev/langswitcher/?lang=ru";

/// The English page of the program — the pair of [`PROGRAM_URL`] under `/en/`, решение 181.33,
/// marked `?lang=en` (решение 181.35).
pub const PROGRAM_URL_EN: &str = "https://panda-pishet-kod.dev/en/langswitcher/?lang=en";

/// **The program's page in the language a person reads the program in** — [`OWN_PAGES`], or else
/// решение 181.33 ([`reads_the_russian_side`]): the name in «О программе» and in «От автора». The
/// mark of the language is there for the reason [`download_url`] gives.
pub fn program_url(language: Language) -> &'static str {
    match own_pages(language) {
        Some((_, _, program, _)) => program,
        None if reads_the_russian_pages(language) => PROGRAM_URL,
        None => PROGRAM_URL_EN,
    }
}

/// The page «Как пользоваться: флаг раскладки» — Z12 of AUDIT-180 (решения 208.92 п. 10, 208.209):
/// the target of «Как пользоваться» of the window «Обновлено», the Russian side, marked `?lang=ru`.
/// ⚠ A draft on the site until the day of the release (404 until then).
pub const FLAG_URL: &str = "https://panda-pishet-kod.dev/langswitcher/docs/flag/?lang=ru";

/// The English page about the flag — the pair of [`FLAG_URL`] under `/en/`, marked `?lang=en`.
pub const FLAG_URL_EN: &str = "https://panda-pishet-kod.dev/en/langswitcher/docs/flag/?lang=en";

/// **The page about the flag in the language a person reads the program in** — [`OWN_PAGES`], or
/// else the side of решение 181.33, by the rule of [`program_url`]: «Как пользоваться» of
/// «Обновлено» (Z12).
pub fn flag_url(language: Language) -> &'static str {
    match own_pages(language) {
        Some((_, _, _, flag)) => flag,
        None if reads_the_russian_pages(language) => FLAG_URL,
        None => FLAG_URL_EN,
    }
}

/// The page «Что нового» — the target of a click on the notification «Lang Switcher обновлён
/// до X» (TZ E104 §0 п. 5, task T-104-7): the Russian side, marked `?lang=ru` for the site
/// (решение 181.35).
pub const CHANGELOG_URL: &str = "https://panda-pishet-kod.dev/langswitcher/changelog/?lang=ru";

/// The English page «What's new» — the pair of [`CHANGELOG_URL`] under `/en/`, marked
/// `?lang=en`.
pub const CHANGELOG_URL_EN: &str =
    "https://panda-pishet-kod.dev/en/langswitcher/changelog/?lang=en";

/// The English page «What's new» **with no mark** — for a language of [`OWN_PAGES`], whose page
/// «Что нового» the site does not have yet (Q-206-1). ⛔ `?lang=en` would take the reader off
/// their language: the mark outranks the browser and the site remembers it.
pub const CHANGELOG_URL_UNMARKED: &str = "https://panda-pishet-kod.dev/en/langswitcher/changelog/";

/// **The page «Что нового» in the language a person reads the program in** — the side of
/// [`reads_the_russian_side`], as for the two pages above; a language of [`OWN_PAGES`] has no
/// such page of its own and reads [`CHANGELOG_URL_UNMARKED`].
pub fn changelog_url(language: Language) -> &'static str {
    match own_pages(language) {
        Some(_) => CHANGELOG_URL_UNMARKED,
        None if reads_the_russian_pages(language) => CHANGELOG_URL,
        None => CHANGELOG_URL_EN,
    }
}

/// **The one folder the updater takes an installer from** — TZ E104 §0 п. 7 и 15: the
/// author's site, `/langswitcher/files/`, and nowhere else; no second road (решение 189.10).
///
/// ⚠ Not a page and not opened by anything: the program never touches it. It is here because
/// ворота 3 of `tools\verify-perimeter.ps1` judge every `https://` of an image against this
/// module, and the updater carries this one ([`crate::update::decide`] holds the address of
/// a `[download]` against it).
pub const UPDATE_FILES_URL: &str = "https://panda-pishet-kod.dev/langswitcher/files/";

/// ⭐ **The one folder the picture of an author's card is taken from** — stage Э121, task T-121-3
/// (решение 208.244 п. 8): the author's site, `/langswitcher/cards/`, and nowhere else. A whole
/// literal, for ворота 3 of `tools\verify-perimeter.ps1`; [`crate::cards::fetch_pictures`] holds
/// the address a `[[card]]` names against it, by the updater's own check
/// ([`crate::update::address_is_ours`]).
pub const CARDS_URL: &str = "https://panda-pishet-kod.dev/langswitcher/cards/";

/// Whether this address is one of the placeholders — the one test every button that opens
/// a link is behind.
pub fn is_placeholder(url: &str) -> bool {
    url.contains(PLACEHOLDER_HOST)
}

/// Whether the program has a real feed address to read from at all.
pub fn feed_is_configured() -> bool {
    FEED_URLS.iter().any(|url| !is_placeholder(url))
}

/// The one scheme this program opens anything under — finding С52, task T-41-1.
const HTTPS_SCHEME: &str = "https://";

/// The host of an `https://` address, or `None` when the string is not one.
///
/// The scheme is compared **without regard to case**, because `HTTPS://` is the same scheme
/// to every browser and a check that missed it would be a check anybody could walk past.
///
/// The host ends at the first `/`, `\`, `?` or `#`. The backslash is not decoration: every
/// browser reads `https://evil.example\ours.invalid/` as a page at `evil.example`, so a
/// reader that did not stop there would compare a host no browser will ever visit.
///
/// `:` and `@` in what is left make the answer `None` rather than a shorter host — a port
/// and a set of credentials are both ways of writing a host that is not the host it looks
/// like, and none of the seven addresses of this program has either. The same two
/// characters are refused for the same reason by [`super::feed::split_https`].
fn host_of(url: &str) -> Option<&str> {
    if !url
        .get(..HTTPS_SCHEME.len())
        .is_some_and(|scheme| scheme.eq_ignore_ascii_case(HTTPS_SCHEME))
    {
        return None;
    }

    let host = url[HTTPS_SCHEME.len()..]
        .split(['/', '\\', '?', '#'])
        .next()
        .unwrap_or_default();

    (!host.is_empty() && !host.contains(':') && !host.contains('@')).then_some(host)
}

/// Every host this program is allowed to reach — the constants above **and nothing else**.
///
/// Read out of the constants rather than written down a second time: a separate list is a
/// second place to forget, and when the author's real addresses replaced the placeholders
/// (task T-78-1) this gate followed them with no edit but the two new names below.
///
/// ⚠ Every **address** of this module belongs in this chain — [`MAIL_ADDRESS`] is not one: it
/// is shown and copied, never opened — and being on a host some other
/// constant already names is **not** a reason to leave one out: that is an accident of where
/// the author happens to host the pages today, and the day one of them moves, the address
/// would stop opening for a reason nobody could see from the line that changed.
fn allowed_hosts() -> impl Iterator<Item = &'static str> {
    FEED_URLS
        .iter()
        .copied()
        .chain([
            CHANNEL_URL,
            GROUP_URL,
            ISSUES_URL,
            SUPPORT_URL,
            DOWNLOAD_URL,
            DOWNLOAD_URL_EN,
            PROGRAM_URL,
            PROGRAM_URL_EN,
            CHANGELOG_URL,
            CHANGELOG_URL_EN,
            CHANGELOG_URL_UNMARKED,
            FLAG_URL,
            FLAG_URL_EN,
        ])
        .chain(
            OWN_PAGES
                .iter()
                .flat_map(|(_, download, program, flag)| [*download, *program, *flag]),
        )
        .filter_map(host_of)
}

/// Whether this program may hand `url` to the shell — finding С52, task T-41-1.
///
/// # Why a white list, and why here
///
/// The process carries `uiAccess`, so **whatever `ShellExecuteW` starts inherits that pass**
/// — the raised rights and the right to reach across the window boundary Windows otherwise
/// keeps. The address comes out of the feed; the feed is signed; and a signature attests the
/// author, not the safety of a string. `file:///…`, `ms-settings:`, a UNC path, a path to an
/// `.exe` and a foreign host would every one of them be opened by the shell, and one typo in
/// the author's TOML is enough to do it.
///
/// Pure, allocation-free, and **the answer does not depend on the placeholder check**: that
/// one is a separate fact about a separate question (is there a real address yet), and
/// [`is_placeholder`] goes on answering it beside this.
pub fn is_allowed(url: &str) -> bool {
    host_of(url)
        .is_some_and(|host| allowed_hosts().any(|allowed| allowed.eq_ignore_ascii_case(host)))
}
