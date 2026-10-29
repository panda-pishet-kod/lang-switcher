//! The updater — `LangSwitcherUpdate.exe`, task T-104-5 (TZ E104 §0 п. 13–19, 22): the decisions
//! that need no network and no system account, each against a state and a feed written out here.
//!
//! Every rule of §0 п. 15 has its own test, so that a test which passed for the wrong reason
//! shows: the version not newer, equal and older; `[stop]` of the reserve key and of the working
//! one; a version that failed here; a foreign host, a foreign folder, `http://`; a size over
//! 16 MB and one that differs from the feed's; a sum that is not the feed's; the revocation of the
//! working key surviving a restart; a feed without `[download]`.

use lang_switcher::letters::{self, FeedItem, feed};
use lang_switcher::update::{self, Allowed, Pending, Refused, Signer, Skip, State};

/// The folder every test feed names — the product's own.
const FILES: &str = letters::links::UPDATE_FILES_URL;

/// A sum the tests' downloads «have».
const SUM: [u8; 32] = [7u8; 32];

/// The `[download]` of a version, in the author's folder.
fn download_of(version: &str) -> feed::Download {
    feed::Download {
        version: version.to_owned(),
        url: format!(
            "{FILES}e{}/LangSwitcher-setup.exe",
            version.split('.').nth(1).unwrap_or("0")
        ),
        sha256: SUM,
        size: 3_500_000,
    }
}

/// A feed that offers `version`: the `update` entry and the `[download]` of the same version.
fn offering(version: &str) -> feed::Feed {
    feed::Feed {
        update: Some(FeedItem {
            id: 12,
            date: None,
            title: "Version is out".to_owned(),
            text: "New.".to_owned(),
            link: String::new(),
            version: Some(version.to_owned()),
        }),
        news: Vec::new(),
        download: feed::Table::Read(download_of(version)),
        stop: feed::Table::Absent,
    }
}

/// The version and the sum of the decision — what a test compares.
fn chosen(result: Result<&feed::Download, Skip>) -> Result<String, Skip> {
    result.map(|download| download.version.clone())
}

// =========================================================================================
// The version — strictly newer, no going back
// =========================================================================================

/// **Only a version strictly newer than the installed one is installed** — §0 п. 15: equal and
/// older are refused (no going back), and so is an installed version that could not be read.
#[test]
fn only_a_strictly_newer_version_is_installed() {
    let state = State::default();
    let feed = offering("0.104.1");

    assert_eq!(
        chosen(update::decide(
            "0.104.0",
            &feed,
            Signer::Working,
            &state,
            FILES
        )),
        Ok("0.104.1".to_owned()),
        "newer — installed"
    );
    assert_eq!(
        chosen(update::decide(
            "0.104.1",
            &feed,
            Signer::Working,
            &state,
            FILES
        )),
        Err(Skip::NotNewer),
        "equal — nothing"
    );
    assert_eq!(
        chosen(update::decide(
            "0.105.0",
            &feed,
            Signer::Working,
            &state,
            FILES
        )),
        Err(Skip::NotNewer),
        "older — no going back"
    );
    assert_eq!(
        chosen(update::decide("", &feed, Signer::Working, &state, FILES)),
        Err(Skip::NotNewer),
        "an installed version that could not be read — nothing"
    );
}

/// **A `[download]` goes with the `update` entry of its version** (§0 п. 16), and a feed without
/// one — the ordinary day — installs nothing.
#[test]
fn a_feed_without_a_download_or_with_a_mismatched_one_installs_nothing() {
    let state = State::default();

    let mut plain = offering("0.104.1");
    plain.download = feed::Table::Absent;
    assert_eq!(
        chosen(update::decide(
            "0.104.0",
            &plain,
            Signer::Working,
            &state,
            FILES
        )),
        Err(Skip::NoDownload)
    );

    let mut broken = offering("0.104.1");
    broken.download = feed::Table::Malformed;
    assert_eq!(
        chosen(update::decide(
            "0.104.0",
            &broken,
            Signer::Working,
            &state,
            FILES
        )),
        Err(Skip::DownloadMalformed)
    );

    let mut mismatched = offering("0.104.1");
    mismatched.download = feed::Table::Read(download_of("0.104.2"));
    assert_eq!(
        chosen(update::decide(
            "0.104.0",
            &mismatched,
            Signer::Working,
            &state,
            FILES
        )),
        Err(Skip::NoUpdateEntry)
    );

    let mut no_entry = offering("0.104.1");
    no_entry.update = None;
    assert_eq!(
        chosen(update::decide(
            "0.104.0",
            &no_entry,
            Signer::Working,
            &state,
            FILES
        )),
        Err(Skip::NoUpdateEntry)
    );
}

// =========================================================================================
// `[stop]`, the failed versions, the revoked key
// =========================================================================================

/// **`[stop]` counts in a feed of the reserve key, and only there** — §0 п. 16; and what the
/// reserve stopped stays stopped when a later feed of the working key offers it again.
#[test]
fn a_stop_of_the_reserve_key_stops_and_a_stop_of_the_working_key_does_not() {
    let mut feed = offering("0.104.1");
    feed.stop = feed::Table::Read(feed::Stop {
        versions: vec!["0.104.1".to_owned()],
        revoke_working: false,
    });

    let mut state = State::default();

    assert_eq!(
        chosen(update::decide(
            "0.104.0",
            &feed,
            Signer::Reserve,
            &state,
            FILES
        )),
        Err(Skip::Stopped),
        "the reserve stops it"
    );
    assert_eq!(
        chosen(update::decide(
            "0.104.0",
            &feed,
            Signer::Working,
            &state,
            FILES
        )),
        Ok("0.104.1".to_owned()),
        "a [stop] the working key signed is not heard"
    );

    assert!(state.take_stop(&feed, Signer::Reserve));
    assert!(!state.take_stop(&feed, Signer::Reserve), "taken once");

    let later = offering("0.104.1");
    assert_eq!(
        chosen(update::decide(
            "0.104.0",
            &later,
            Signer::Working,
            &state,
            FILES
        )),
        Err(Skip::Stopped),
        "what the reserve stopped, a later feed of the working key cannot bring back"
    );

    let mut unreadable = offering("0.104.1");
    unreadable.stop = feed::Table::Malformed;
    assert_eq!(
        chosen(update::decide(
            "0.104.0",
            &unreadable,
            Signer::Reserve,
            &State::default(),
            FILES
        )),
        Err(Skip::StopMalformed),
        "a [stop] of the reserve that cannot be read — nothing at all"
    );
}

/// **A version that failed here is never tried again** — §0 п. 18.
#[test]
fn a_version_that_failed_here_is_not_tried_again() {
    let mut state = State::default();
    state.mark_failed("0.104.1");
    state.mark_failed("0.104.1");

    assert_eq!(state.failed, vec!["0.104.1".to_owned()], "marked once");
    assert_eq!(
        chosen(update::decide(
            "0.104.0",
            &offering("0.104.1"),
            Signer::Working,
            &state,
            FILES
        )),
        Err(Skip::FailedBefore)
    );
    assert_eq!(
        chosen(update::decide(
            "0.104.0",
            &offering("0.104.2"),
            Signer::Working,
            &state,
            FILES
        )),
        Ok("0.104.2".to_owned()),
        "the next version is not the failed one"
    );
}

/// **The reserve key revokes the working one for good** — §0 п. 16: after a feed of the reserve
/// with `revoke_working`, a feed of the working key is refused, and the state that says so
/// survives being written and read back (a restart of the updater).
#[test]
fn a_revoked_working_key_stays_revoked_across_a_restart() {
    let mut revoking = offering("0.104.1");
    revoking.stop = feed::Table::Read(feed::Stop {
        versions: Vec::new(),
        revoke_working: true,
    });

    let mut state = State::default();

    assert!(
        !state.take_stop(&revoking, Signer::Working),
        "the working key cannot revoke itself"
    );
    assert!(!state.working_key_revoked);
    assert!(state.take_stop(&revoking, Signer::Reserve));
    assert!(state.working_key_revoked);

    let folder = scratch("revoked");
    let file = folder.join(update::STATE_FILE);

    assert!(state.save(&file), "the state must be written");

    let again = State::load(&file).expect("and read back");
    let _ = std::fs::remove_dir_all(&folder);

    assert_eq!(again, state);
    assert_eq!(
        chosen(update::decide(
            "0.104.0",
            &offering("0.104.1"),
            Signer::Working,
            &again,
            FILES
        )),
        Err(Skip::WorkingKeyRevoked),
        "after the restart the working key is still refused"
    );
    assert_eq!(
        chosen(update::decide(
            "0.104.0",
            &offering("0.104.1"),
            Signer::Reserve,
            &again,
            FILES
        )),
        Ok("0.104.1".to_owned()),
        "the reserve key still is heard"
    );
}

// =========================================================================================
// The address, the size, the sum
// =========================================================================================

/// **The installer comes from one folder of one host** — §0 п. 7 и 15: a foreign host, a host
/// that only starts like the author's, another folder, `http://`, a step up, a query — refused.
#[test]
fn only_a_file_in_the_authors_folder_is_downloaded() {
    let state = State::default();
    let refused = [
        "https://evil.example/langswitcher/files/e104/LangSwitcher-setup.exe",
        "https://panda-pishet-kod.dev.evil.example/langswitcher/files/e104/x.exe",
        "https://panda-pishet-kod.dev/langswitcher/other/x.exe",
        "https://panda-pishet-kod.dev/files/x.exe",
        "http://panda-pishet-kod.dev/langswitcher/files/e104/x.exe",
        "HTTPS://panda-pishet-kod.dev/langswitcher/files/e104/x.exe",
        "https://panda-pishet-kod.dev:8443/langswitcher/files/e104/x.exe",
        "https://user@panda-pishet-kod.dev/langswitcher/files/e104/x.exe",
        "https://panda-pishet-kod.dev/langswitcher/files/../x.exe",
        "https://panda-pishet-kod.dev/langswitcher/files/e104/x.exe?a=1",
        "https://panda-pishet-kod.dev/langswitcher/files/e104\\x.exe",
        "https://panda-pishet-kod.dev/langswitcher/files/e104/%2e%2e/x.exe",
        "https://panda-pishet-kod.dev/langswitcher/files//x.exe",
        "https://panda-pishet-kod.dev/langswitcher/files/",
    ];

    for url in refused {
        let mut feed = offering("0.104.1");
        let mut download = download_of("0.104.1");
        download.url = url.to_owned();
        feed.download = feed::Table::Read(download);

        assert_eq!(
            chosen(update::decide(
                "0.104.0",
                &feed,
                Signer::Working,
                &state,
                FILES
            )),
            Err(Skip::ForeignAddress),
            "{url} must be refused"
        );
    }

    assert!(update::address_is_ours(
        "https://panda-pishet-kod.dev/langswitcher/files/e104/LangSwitcher-setup.exe",
        FILES
    ));
    assert!(
        !update::address_is_ours("anything", ""),
        "an empty folder is no folder"
    );
}

/// **At most 16 MB, and exactly the size the feed named** — §0 п. 15: the size named is checked
/// before the download, the size received after it.
#[test]
fn the_size_is_at_most_sixteen_megabytes_and_the_one_named() {
    let state = State::default();
    let mut feed = offering("0.104.1");
    let mut big = download_of("0.104.1");
    big.size = update::MAX_INSTALLER_BYTES + 1;
    feed.download = feed::Table::Read(big);

    assert_eq!(
        chosen(update::decide(
            "0.104.0",
            &feed,
            Signer::Working,
            &state,
            FILES
        )),
        Err(Skip::TooBig)
    );

    let named = download_of("0.104.1");

    assert_eq!(update::received_matches(named.size, &SUM, &named), Ok(()));
    assert_eq!(
        update::received_matches(named.size - 1, &SUM, &named),
        Err(Refused::Size),
        "one byte short"
    );
    assert_eq!(
        update::received_matches(named.size + 1, &SUM, &named),
        Err(Refused::Size),
        "one byte over"
    );

    let mut at_the_ceiling = download_of("0.104.1");
    at_the_ceiling.size = update::MAX_INSTALLER_BYTES + 1;
    assert_eq!(
        update::received_matches(at_the_ceiling.size, &SUM, &at_the_ceiling),
        Err(Refused::Size),
        "over the ceiling even when the feed named it"
    );
}

/// **The sum is the feed's or the file is refused** — §0 п. 15.
#[test]
fn a_sum_that_is_not_the_feeds_is_refused() {
    let named = download_of("0.104.1");
    let mut other = SUM;
    other[31] ^= 1;

    assert_eq!(
        update::received_matches(named.size, &other, &named),
        Err(Refused::Sum)
    );
}

/// **The sum the updater computes is SHA-256, in pieces or whole** — the hasher of the feed
/// module, against the vector everyone knows: `abc`.
#[test]
fn the_hasher_is_sha256_in_pieces_and_whole() {
    const ABC: [u8; 32] = [
        0xBA, 0x78, 0x16, 0xBF, 0x8F, 0x01, 0xCF, 0xEA, 0x41, 0x41, 0x40, 0xDE, 0x5D, 0xAE, 0x22,
        0x23, 0xB0, 0x03, 0x61, 0xA3, 0x96, 0x17, 0x7A, 0x9C, 0xB4, 0x10, 0xFF, 0x61, 0xF2, 0x00,
        0x15, 0xAD,
    ];

    let mut whole = feed::Sha256::new().expect("a hasher");
    assert!(whole.update(b"abc"));
    assert_eq!(whole.finish(), Some(ABC));

    let mut pieces = feed::Sha256::new().expect("a hasher");
    assert!(pieces.update(b"a"));
    assert!(pieces.update(b""));
    assert!(pieces.update(b"bc"));
    assert_eq!(pieces.finish(), Some(ABC));
}

// =========================================================================================
// The folder, the state, the journal
// =========================================================================================

/// **The folder is SYSTEM's and Administrators' only** — §0 п. 15, посылка П10, the pure half:
/// an owner that is not one of them, a list that lets anybody else write (inherited or not), no
/// list at all — refused; reading for everybody is fine.
#[test]
fn the_folder_is_trusted_only_when_nobody_else_may_write() {
    let system = Allowed {
        mask: 0x001F_01FF,
        trusted: true,
    };
    let users_read = Allowed {
        mask: 0x0012_00A9,
        trusted: false,
    };

    assert!(update::folder_verdict(true, Some(&[system, users_read])));
    assert!(
        !update::folder_verdict(false, Some(&[system])),
        "a foreign owner"
    );
    assert!(
        !update::folder_verdict(true, None),
        "no list — open to everybody"
    );

    for right in [
        0x0002,
        0x0004,
        0x0010,
        0x0040,
        0x0100,
        0x0001_0000,
        0x0004_0000,
        0x0008_0000,
        0x1000_0000,
        0x4000_0000,
    ] {
        let writer = Allowed {
            mask: 0x0012_00A9 | right,
            trusted: false,
        };

        assert!(
            !update::folder_verdict(true, Some(&[system, writer])),
            "the right {right:#x} for somebody else must be refused"
        );
    }
}

/// **A folder made by a test is not trusted** — the impure half, on a real folder: the owner of
/// a folder under the temporary one is the person running the tests, not SYSTEM.
#[test]
fn a_folder_of_the_person_is_not_trusted() {
    let folder = scratch("untrusted");

    assert!(!update::folder_is_trusted(&folder));
    assert!(
        !update::folder_is_trusted(&folder.join("missing")),
        "a folder that does not exist"
    );

    let _ = std::fs::remove_dir_all(&folder);
}

/// **The state goes to the file and back whole** — every field, the in-flight installation
/// included; a missing file is the empty state, a file of nonsense is no state at all.
#[test]
fn the_state_goes_to_the_file_and_back() {
    let state = State {
        working_key_revoked: true,
        stopped: vec!["0.105.0".to_owned()],
        failed: vec!["0.104.1".to_owned()],
        last_try: letters::Date::from_ymd(2026, 10, 25),
        pending: Some(Pending {
            version: "0.104.1".to_owned(),
            from: "0.104.0".to_owned(),
            sessions: vec![1, 2],
            rollback_copy: true,
            rolling_back: false,
            since: 1_761_350_400,
        }),
    };

    let folder = scratch("state");
    let file = folder.join(update::STATE_FILE);

    assert_eq!(
        State::load(&file),
        Some(State::default()),
        "no file — the empty state"
    );
    assert!(state.save(&file));
    assert_eq!(State::load(&file), Some(state));

    std::fs::write(&file, "pending = [").expect("written");
    assert_eq!(
        State::load(&file),
        None,
        "a file that cannot be read is not a default"
    );

    let _ = std::fs::remove_dir_all(&folder);
}

/// **The journal says when, what and a code — and turns at 64 KB** (§0 п. 17).
#[test]
fn the_journal_turns_at_its_cap() {
    assert_eq!(update::stamp_of(0), "1970-01-01T00:00:00Z");
    assert_eq!(update::stamp_of(1_761_394_245), "2025-10-25T12:10:45Z");

    let folder = scratch("journal");
    let journal = update::Journal::at(&folder);

    for _ in 0..3000 {
        journal.line("feed request", 0);
    }

    let size = std::fs::metadata(folder.join(update::LOG_FILE))
        .expect("the journal")
        .len();
    let kept = std::fs::metadata(folder.join(update::LOG_KEPT))
        .expect("the turned journal")
        .len();

    let _ = std::fs::remove_dir_all(&folder);

    assert!(size <= update::LOG_CAP, "{size} bytes — over the cap");
    assert!(
        kept <= update::LOG_CAP && kept > update::LOG_CAP / 2,
        "{kept} bytes kept"
    );
}

/// **Phase one closes the program before it starts the installer** — решение 189.33: on Test_VM the
/// Restart Manager of a Setup under the system account did not close the program of the user's
/// session («Session Mismatch», 189.32), and the installation aborted. The order of the code lines
/// of phase one: the sessions written, every one closed, and only then the installer.
#[test]
fn phase_one_closes_the_program_before_the_installer_starts() {
    let source = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/update.rs"),
    )
    .expect("src/update.rs must be readable");

    let start = source.find("fn phase_one_of(").expect("phase one");
    let body = &source[start..start + source[start..].find("\nfn ").expect("its end")];
    let at = |needle: &str| {
        body.find(needle)
            .unwrap_or_else(|| panic!("phase one has no «{needle}»"))
    };

    assert!(at("sessions: sessions_running(&places.program)") < at("close_program_in("));
    assert!(
        at("close_program_in(") < at("if !start_setup(&setup"),
        "the program must be closed BEFORE the installer starts"
    );
    assert_eq!(update::CLOSE_WAIT_SECONDS, 10, "the ten seconds of 155.25");
}

/// The signer of each key of FR-102.
#[test]
fn the_working_key_is_the_first_and_the_reserve_the_second() {
    assert_eq!(Signer::of_index(0), Some(Signer::Working));
    assert_eq!(Signer::of_index(1), Some(Signer::Reserve));
    assert_eq!(Signer::of_index(2), None);
}

/// A folder of the test's own under the temporary folder.
fn scratch(name: &str) -> std::path::PathBuf {
    let folder = std::env::temp_dir().join(format!("langsw-update-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder).expect("a scratch folder");
    folder
}

/// A hidden top-level window of a class of the test's own, made on a thread of its own after
/// `appear_after` and destroyed after `live` (never, for `None`) — the program's window as the
/// helper `/window` sees it, without the program: on the machine that runs the tests the
/// program's own `LangSwitcher.Hidden` may well be up.
fn window_of_class_on_a_thread(
    class: &'static str,
    appear_after: std::time::Duration,
    live: Option<std::time::Duration>,
) -> std::thread::JoinHandle<()> {
    use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DestroyWindow, RegisterClassW, WINDOW_EX_STYLE, WNDCLASSW,
        WS_POPUP,
    };
    use windows::core::PCWSTR;

    unsafe extern "system" fn procedure(
        window: HWND,
        message: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        // SAFETY: the arguments are the ones the system handed this procedure.
        unsafe { DefWindowProcW(window, message, wparam, lparam) }
    }

    std::thread::spawn(move || {
        std::thread::sleep(appear_after);

        let name: Vec<u16> = class.encode_utf16().chain(Some(0)).collect();
        // SAFETY: the module of this test binary.
        let instance = unsafe { GetModuleHandleW(None) }.expect("the module of the test");
        let description = WNDCLASSW {
            lpfnWndProc: Some(procedure),
            hInstance: instance.into(),
            lpszClassName: PCWSTR(name.as_ptr()),
            ..Default::default()
        };

        // SAFETY: the description lives for this frame and names a procedure of this file.
        assert_ne!(unsafe { RegisterClassW(&description) }, 0, "the test class");

        // SAFETY: the class just registered; no parent, no menu, no parameter; hidden.
        let window = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE(0),
                PCWSTR(name.as_ptr()),
                None,
                WS_POPUP,
                0,
                0,
                10,
                10,
                None,
                None,
                Some(instance.into()),
                None,
            )
        }
        .expect("a window of the test class");

        match live {
            Some(live) => {
                std::thread::sleep(live);
                // SAFETY: the window this thread made, destroyed once by this thread.
                unsafe { DestroyWindow(window) }.expect("the window goes");
            }
            // Lives as long as the test binary: a thread that is never joined.
            None => loop {
                std::thread::sleep(std::time::Duration::from_secs(3600));
            },
        }
    })
}

/// **Phase two counts the program as raised only when its window appeared and lived** — решение
/// 189.34, Q-189-1: round 6 of Э104 showed a «program» that never ran counted by its process. The
/// helper `/window` waits up to sixty seconds for the window of the class and then ten seconds of
/// its life; here the same function under a class of the test's own and shorter times.
#[test]
fn the_program_counts_as_raised_only_when_its_window_appears_and_lives() {
    use std::time::Duration;

    let second = Duration::from_secs(1);

    // No window at all — the fake «does not start», the loader's «System Error» box.
    assert!(
        !update::window_lives("LangSwitcher.TestE105.None", second, second),
        "no window is not a running program"
    );

    // A window that comes after a moment and stays.
    let _stays = window_of_class_on_a_thread(
        "LangSwitcher.TestE105.Stays",
        Duration::from_millis(700),
        None,
    );
    assert!(
        update::window_lives("LangSwitcher.TestE105.Stays", 3 * second, 2 * second),
        "a window that appears within the wait and lives is a running program"
    );

    // A window that goes a moment after it came — a crash, the WER box, an exit.
    let goes = window_of_class_on_a_thread(
        "LangSwitcher.TestE105.Goes",
        Duration::ZERO,
        Some(Duration::from_millis(800)),
    );
    assert!(
        !update::window_lives("LangSwitcher.TestE105.Goes", 3 * second, 3 * second),
        "a window that does not live its time is not a running program"
    );
    goes.join().expect("the thread of the short window");

    // The numbers of the decision, written as numbers (урок Э104: a loop up to the constant passes
    // a mutant of the constant).
    assert_eq!(update::RAISE_WAIT_SECONDS, 60);
    assert_eq!(update::WINDOW_LIFE_SECONDS, 10);
    assert_eq!(update::WINDOW_SWITCH, "/window");
    assert_eq!(update::code::NO_WINDOW, 8);
}

/// **Phase two asks the helper about every session, and no process counts by itself** — the
/// order of the code of [`update::raise_and_check`]: the old criterion («a process of the program
/// is in the session») must not be its answer any more.
#[test]
fn raise_and_check_answers_by_the_window_helper_and_not_by_the_process() {
    let source = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/update.rs"),
    )
    .expect("src/update.rs must be readable");

    let start = source
        .find("pub fn raise_and_check(")
        .expect("raise_and_check");
    let body = &source[start..start + source[start..].find("\n}").expect("its end")];

    assert!(
        body.contains("start_system_in_session(session, me, WINDOW_SWITCH)"),
        "the window helper is started in each awaited session"
    );
    assert!(
        body.contains("exit_code_within(patience) != Some(u32::from(code::DONE))"),
        "the helper's exit code is the answer"
    );
    assert!(
        !body.contains("retain(|&session| !running_in(session, program))"),
        "a process in the session is not the answer any more"
    );
}

/// **The updater's version resource carries the version, and 0 in the fourth field** — решения
/// 192.1 и 192.3 (stage Э105): `[package] version` in the first three fields from build.rs, the
/// fourth always 0 (the number of the delivery is an internal mark only), and the updater compares
/// the feed with the three (§0 п. 15, посылка П4). Read off the file cargo really built; raised
/// **by hand** with every version, the honest red before a bump, like the reader of `app.rc` in
/// `tests\tray.rs`.
#[test]
fn the_updater_carries_the_version_in_its_resource() {
    use windows::Win32::Storage::FileSystem::{GetFileVersionInfoSizeW, GetFileVersionInfoW};
    use windows::core::PCWSTR;

    let path = std::path::Path::new(env!("CARGO_BIN_EXE_LangSwitcherUpdate"));
    let name: Vec<u16> = std::os::windows::ffi::OsStrExt::encode_wide(path.as_os_str())
        .chain(Some(0))
        .collect();

    // SAFETY: a NUL-terminated name of this frame; no handle is asked for.
    let size = unsafe { GetFileVersionInfoSizeW(PCWSTR(name.as_ptr()), None) };
    assert!(size > 0, "the updater has a version resource");

    let mut bytes = vec![0u8; size as usize];
    // SAFETY: `bytes` holds `size` bytes and lives for this frame.
    unsafe { GetFileVersionInfoW(PCWSTR(name.as_ptr()), None, size, bytes.as_mut_ptr().cast()) }
        .expect("the resource is read");

    assert_eq!(
        lang_switcher::tray::parse_fixed_file_version(&bytes),
        Some((1, 4, 0, 0)),
        "the updater must carry 1.4.0.0 (решение 192.3)"
    );
    assert_eq!(
        update::version_of_file(path).as_deref(),
        Some("1.4.0"),
        "the updater compares three fields"
    );
}

/// **Phase two closes the new version that did not start before it rolls back** — Q-189-1, stage
/// Э105: the build that lacks a DLL stands in the loader's «System Error» box with a process and
/// no window, the window helper answers «no window», and the rollback's installer would find the
/// file held. The order of the code lines of phase two: the failure marked, every session closed,
/// and only then the rollback started.
#[test]
fn phase_two_closes_the_program_that_did_not_start_before_the_rollback() {
    let source = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/update.rs"),
    )
    .expect("src/update.rs must be readable");

    let start = source.find("fn phase_two_of(").expect("phase two");
    let body = &source[start..start + source[start..].find("\nfn ").expect("its end")];
    let at = |needle: &str| {
        body.find(needle)
            .unwrap_or_else(|| panic!("phase two has no «{needle}»"))
    };

    assert!(at("state.mark_failed(&pending.version);") < at("close_program_in(session"));
    assert!(
        at("close_program_in(session") < at("if start_setup(&rollback"),
        "the program that did not start must be closed BEFORE the rollback is installed"
    );
}
