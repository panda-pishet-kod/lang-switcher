# Architecture

How Lang Switcher is built: the two executables, the threads of the program and why they are split,
the path of one key press, the clipboard, the settings, the feed and the updates, the installer.
The specification is `SPEC.md` (in Russian); section numbers below are its sections, `FR-…`,
`NFR-…` and `SEC-…` its requirements. Security is summed up in [SECURITY.md](SECURITY.md).

## Two executables of one crate

| File | What it is | Rights |
|---|---|---|
| `LangSwitcher.exe` — [`src/main.rs`](src/main.rs), [`src/app.rs`](src/app.rs) | The program: the keyboard hook, the conversion, the tray, the windows | The full image has `uiAccess` in its manifest ([`app.manifest`](app.manifest)), the base image does not ([`app-dev.manifest`](app-dev.manifest)); [`build.rs`](build.rs) picks the manifest by the build profile and `LANGSW_NO_UIACCESS` |
| `LangSwitcherUpdate.exe` — [`src/update_main.rs`](src/update_main.rs), [`src/update.rs`](src/update.rs) | The updater: no window, run by the scheduled task "Lang Switcher Update" under the system account | No `uiAccess`, `asInvoker` ([`updater.manifest`](updater.manifest)) |

They never talk to each other (SEC-04). What the updater knows of the program is its file, the
version resource of that file and whether its process runs. The feed — the read, the signature, the
parse, the one `GET` — is one body in [`src/feed.rs`](src/feed.rs) (`feed`; the old path `letters::feed` stays), used by
both. Everything is in the library ([`src/lib.rs`](src/lib.rs)), so the integration tests in
[`tests/`](tests) link against the same code.

## Threads of the program

A low-level keyboard hook is called on the thread that installed it, and Windows removes it
silently if that thread does not answer in time (`LowLevelHooksTimeout`, at most a second since
Windows 10 1709). So nothing that can block — a menu, a dialog, file I/O, COM — shares a thread
with the hook (§6.1, FR-80). Each window is created by the thread that serves it, because a message
queue belongs to the thread that made the window. [`src/app.rs`](src/app.rs) starts the threads and
brings them down in order.

| Thread | Does | Does not |
|---|---|---|
| **Input** (`app.rs`, message-only window) | Owns `WH_KEYBOARD_LL` ([`src/hook.rs`](src/hook.rs)), the typing buffer ([`src/buffer.rs`](src/buffer.rs)), the layout cache ([`src/layouts.rs`](src/layouts.rs)); runs every `SendInput` ([`src/inject.rs`](src/inject.rs)) and the layout switch ([`src/switch.rs`](src/switch.rs)); the watchdog timer of the hook ([`src/watchdog.rs`](src/watchdog.rs)) | UI, file I/O, COM |
| **UI** (`app.rs`, hidden top-level window) | The tray icon and menu ([`src/tray.rs`](src/tray.rs)), the settings ([`src/settings.rs`](src/settings.rs); the configuration file in [`src/config.rs`](src/config.rs), the capture of the hotkey in [`src/hotkey_capture.rs`](src/hotkey_capture.rs), both children of `settings` since T-120-23; the window itself in [`src/ui/windows/settings.rs`](src/ui/windows/settings.rs)), the theme and drawing ([`src/theme.rs`](src/theme.rs), [`src/widgets.rs`](src/widgets.rs)), the letters and the "From the author" window ([`src/letters.rs`](src/letters.rs)) | The hook |
| **Watcher** (`app.rs`, message-only window, COM STA) | `SetWinEventHook` for the foreground window, the focus and the desktop switch; the password-field check through UI Automation and the exclusion list ([`src/guard.rs`](src/guard.rs)) | The hook, input |
| **Focus events** (`app.rs`, COM MTA, since 1.5.0) | UI Automation focus-changed handler; it only posts a message to the watcher window (FR-71а) | The hook, input, UI, file I/O |
| **Feed** (`feed.rs`, short-lived) | One WinHTTP request, the TOML parse, the signature check; puts the result into a mailbox and posts "look into the mailbox" to the UI thread | UI, input, the hook |
| **Appeal** (`appeal.rs`, short-lived) | The one `POST` of the "Write to the author…" wizard, after **Send** or **Retry** | UI, input, the hook |

A small thread of [`src/hook.rs`](src/hook.rs) waits for the emergency chord `Ctrl+Alt+Shift+F12`
(FR-96): the callback returns at once, and that thread removes the hooks and ends the process.

**Budgets of the hook callback** (§5): p99 under 100 µs (NFR-01), never over 1 ms (NFR-02), no
allocation (NFR-03), no blocking lock (NFR-04), no I/O or logging (NFR-05). From the start of the
program to the installed hook — under 50 ms (NFR-08); from the hotkey to a finished replacement on
the buffer path — under 30 ms (NFR-09); idle memory under 8 MB (NFR-06).

**How state crosses threads** (§6.3). The buffer and the layout cache are written and read by the
input thread only. The password flag goes from the watcher to the input thread as an `AtomicBool`.
Compound state the input thread must read whole — the exclusion list of `guard`, for one — goes
through a seqlock. The feed result goes through a mailbox under a mutex that is never on the hook's
path; the message to the UI thread carries no data, so a forged message of the same number only
makes it look into an empty mailbox (SEC-05). No mutex on the hook path.

## The path of one key press

1. **The callback** ([`hook.rs`](src/hook.rs), `keyboard_hook_proc`) sees every key. The program's
   own injected input is told by a signature in `dwExtraInfo`, not by the "injected" flag, which
   would also drop other programs' input (FR-03). An ordinary key goes into the ring buffer
   ([`buffer.rs`](src/buffer.rs), 256 keystrokes, FR-07) as a key, not as text; word boundaries,
   clicks, focus changes and the like reset it, and every reset overwrites it with zeros (FR-10,
   SEC-02). In a password field nothing is buffered (FR-70).
2. **The hotkey** is recognised inside the callback, not through `RegisterHotKey`, so it keeps its
   place among the other keys (FR-02). The callback swallows it, posts `WM_APP_HOTKEY` to the input
   window and returns at once: `SendInput` from inside the hook is forbidden (FR-40, steps 1–2).
3. **The input thread** takes the message ([`inject.rs`](src/inject.rs), `on_hotkey`) and chooses the
   source: a selection, else the typed word, else nothing — and then it only switches the layout
   (FR-60).
4. **The conversion** is pure logic with no Win32 in it ([`src/convert.rs`](src/convert.rs)): the
   keys are read again through the layout cache — every virtual key × eight modifier states × every
   installed layout, through `ToUnicodeEx`, built at start and rebuilt when the layouts change
   (FR-20, [`layouts.rs`](src/layouts.rs)). The target layout is the other one of the pair or the
   next one of the cycle (§4.4). A second press converts again from the same keys, so the original
   comes back byte for byte (FR-32).
5. **The replacement** ([`inject.rs`](src/inject.rs)): held modifiers are released first (FR-40,
   step 3); then one `SendInput` call carries `N` × `Backspace` and the new text as
   `KEYEVENTF_UNICODE` characters (FR-41). Where a field needs it, the word is selected instead of
   erased (FR-42; the Notepad of Windows 11), or the same keys are typed again instead of
   characters (FR-42а; the classic console).
6. **The layout switch** ([`switch.rs`](src/switch.rs)) comes after the replacement and only if the
   whole packet was accepted (FR-43, FR-45): one method, `PostMessage(WM_INPUTLANGCHANGEREQUEST)` to
   the window with the keyboard focus (FR-50). The result is read back from that window's thread
   (FR-52); on a press with no word the program watches for up to half a second
   (`LATE_VERDICT_MS`) before it answers with a sound — the double click for a switch, the knock for
   a refusal.

Windows also drops a low-level hook silently on a switch to the secure desktop, a change of session
and a resume from sleep. The watchdog ([`watchdog.rs`](src/watchdog.rs)) listens for those three,
checks every 30 seconds that the hook is alive, and sets it again when it is gone (FR-80).

## Selected text and the clipboard

[`src/selection.rs`](src/selection.rs), §4.7. The program saves every format of the clipboard (up to
4 MB in all; above that only `CF_UNICODETEXT`, FR-64), sends `Ctrl+C` and waits for the clipboard
sequence number to change (300 ms by default). No change means no selection, and the typed word is
used instead. The text is converted word by word, each word by its own script, written back and
pasted with `Ctrl+V`; the layout switches to the target of the whole text. After 200 ms the saved
clipboard is put back — unless another program has changed the clipboard in the meantime, so a new
copy of yours is never overwritten (FR-61). Every clipboard call is retried 10 times, 20 ms apart
(FR-62). In console windows the selection path is not used: `Ctrl+C` there interrupts a running
command.

## Settings and `schema_version`

`%APPDATA%\Lang_Switcher\config.toml` (§7), read and written by the UI thread
([`src/config.rs`](src/config.rs), re-exported by `settings`) and published to the input thread. The current schema is
10 (`CURRENT_SCHEMA_VERSION`). Unknown keys are ignored and missing ones take their defaults, so a
new key alone does not raise the schema. The schema is raised when an older build would destroy
something by rewriting the file — a new field it does not know, a new value of a closed list: a
file of a newer schema is read but not saved until the end of the session, so its unknown fields
survive a step back. A damaged file is kept byte for byte as `config.toml.bad` before the first
write; a file that cannot be read (locked, no rights) is read once more and then left untouched
(§10, row 12). The program folder in `Program Files` is not writable by the user, and no settings
live there.

## The feed and updates

The feed is a small signed TOML file on the author's site (FR-102). The program reads it at most
once a day on the feed thread, if `[letters] feed` is on; the letters and the news it carries are
shown by [`letters.rs`](src/letters.rs). A file is accepted only with a valid signature of one of
the author's two keys and only up to a size limit (SEC-03).

The updater ([`update.rs`](src/update.rs), SEC-10) reads the same feed at most once a day, while the
computer is idle, whatever the feed switch says. Its checks, in order: the signature; a
`[download]` table; a version strictly newer than the version resource of the installed
`LangSwitcher.exe`; not stopped and not marked failed; an address under
`https://panda-pishet-kod.dev/langswitcher/files/`; the download into
`%ProgramData%\Lang_Switcher\update\` (writable by SYSTEM and Administrators only, checked on every
run); the size the feed named, at most 16 MB; the SHA-256 the feed named. Then the silent
installation; if the new version does not start, the previous one is put back. Any refusal installs
nothing and leaves the next attempt to the next day.

## The installer and `uiAccess`

`uiAccess` lets the program's hook and `SendInput` pass UIPI, the wall Windows puts between a
program and the windows of a higher integrity level (§8.1). Windows grants it only to a file signed
with a certificate the computer trusts and lying in a protected folder such as `Program Files`
(§8.2). Hence two images of one build: the full one with `uiAccess` and the base one without; they
differ only in the manifest and the fields the linker derives from the contents
([`tools/verify-one-body.ps1`](tools/verify-one-body.ps1)).

The installer ([`installer/LangSwitcher.iss`](installer/LangSwitcher.iss), Inno Setup, §8.4) puts the
base image first and replaces it with the full one only after the full one's signature has been
checked as valid on this computer: the author's signature where it is trusted, or a local
certificate made by [`installer/local-trust.ps1`](installer/local-trust.ps1) — code signing only,
its private key destroyed right after signing, removed on uninstall (SEC-09). With
`/NOLOCALCERT`, or if anything fails, the base image stays. The program is started through
`ShellExecute`, because `CreateProcess` refuses an image with `uiAccess` (error 740). The installer
also places the updater, its folder under `%ProgramData%` and its scheduled task. The release
scripts are in [`tools/`](tools); they are described in [README.ru.md](README.ru.md) (in Russian).
