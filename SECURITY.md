# Security policy

Lang Switcher sets a global keyboard hook and, in its full version, runs with `uiAccess`. This file
says what that means, what the program keeps, what it sends, and how to report a vulnerability.
Every statement here follows the specification (`SPEC.md`, section 8 «Права доступа и установка»
and section 9 «Безопасность и приватность», in Russian) and the code it names.

## Reporting a vulnerability

- **GitHub's private report.** The repository's
  [Security tab](https://github.com/panda-pishet-kod/lang-switcher/security) → **Report a
  vulnerability**. Only the author sees the report.
- **The program's own wizard.** Tray menu → "Write to the author…": describe the problem, check the
  text, press **Send**. The message goes to the author's site, not to a public page. If the site
  does not answer, the wizard offers to send the same text by email or to save it to a file.
- **Please do not post the details of a vulnerability in a public issue.** If you can use neither
  of the above, open an
  [issue](https://github.com/panda-pishet-kod/lang-switcher/issues) that says only that you have a
  security report, without the details.

There is one line of versions: a fix comes out as the next version, and the program installs new
versions by itself (see [Updates](#what-goes-over-the-network)).

## What the program keeps

- **Keystrokes are never written to disk** — not to the log, not to the settings, not to temporary
  files, not to crash dumps (SEC-01).
- **The buffer of the word you are typing lives in memory only** and is overwritten with zeros at
  every reset, not just marked empty (SEC-02).
- **One exception of a known size** (since 1.0.0): for layouts with an ambiguous ligature key —
  Arabic and the like — up to 64 keystrokes are kept in a separate memory, so that the word you
  just typed converts by the keys you really pressed. It is zeroed when you move to another window
  or field, lock the computer, pause the program, and after 60 seconds without typing. For Russian,
  English and every layout without such a key nothing goes into it (SEC-02).
- **Password fields are not buffered at all** (SEC-06). A password field is recognised through UI
  Automation, a component of Windows that runs inside the program's process (SEC-06a).
- **The diagnostic log**, when it is on, holds only the program's own events — the hook set and
  removed, a layout switch that failed, a refused `SendInput`. Never characters, key codes or
  clipboard contents (SEC-07).
- **Selected text** is converted through the clipboard, and the clipboard is put back as it was,
  every format of it.

## What the program can and cannot do

- **No inter-process channel** in the shipped program: no named pipes, sockets or shared memory
  (SEC-04). The one exception, a read-only debug channel that reports state and never content, is
  compiled only with the `testing` feature, which release builds do not have (SEC-04a).
- **The hidden window of the program ignores every message it does not handle**, and no
  privileged action starts from an incoming window message (SEC-05).
- **The program starts no other program by itself** (SEC-06b). Downloading and running the
  installer of a new version is the job of the separate updater, without `uiAccess` (SEC-10).
- **Emergency exit:** `Ctrl+Alt+Shift+F12` removes every hook and ends the process; it is handled
  first, before any other logic, in every build.

## `uiAccess`, the two versions and the signature

**Why `uiAccess`.** A keyboard hook of an ordinary program gets no keys while the window in front
belongs to a program of a higher integrity level, such as one run as administrator (UIPI). The
manifest of the full version asks for `uiAccess="true"` with `level="asInvoker"`: Windows adds to
the program's token the right to pass UIPI for the hook and for `SendInput`, and only that. It is
not administrator rights. Windows grants it only to a program that is signed with a certificate the
computer trusts and lies in a protected folder such as `Program Files` (SPEC §8.1, §8.2).

**What stays out of reach.** Neither `uiAccess` nor anything else works on the secure desktop: the
UAC prompt, `Ctrl+Alt+Del`, the sign-in screen (SPEC §8.5).

**The full and the base version.** The installer carries two images of one build: the full one with
`uiAccess` and the base one without it. They differ only in the manifest and the fields the linker
derives from the contents of the file (`tools/verify-one-body.ps1`). The base one is installed first;
the full one replaces it only after its signature has been checked as valid on this computer. If
anything fails, the base version stays, and the installation goes on (SPEC §8.4). The base version
works everywhere except in the windows of programs run as administrator.

**The signature, honestly.** The program, the updater and the installer are signed with the
author's own certificate, `CN=Panda Koder`, not one from a certificate authority. On a computer that
does not trust it, the UAC prompt of the installer is yellow and names an unknown publisher, and the
browser or Windows may warn about a rarely downloaded file (README, «If the browser or Windows warns
you»).

**The local certificate.** Where the author's certificate is not trusted, the installer — which
runs with administrator rights after the UAC prompt — makes a certificate of its own on that
computer, `CN=Lang Switcher (local)`:

- good for code signing only, and not a certificate authority: it cannot issue another certificate,
  a website's included (SEC-09);
- it signs the full version with it and destroys the private key at once, so nothing else can ever
  be signed with it;
- its public part goes into the computer's Trusted Root Certification Authorities, with a copy in
  the program folder;
- uninstalling the program removes it.

`LangSwitcher-setup.exe /NOLOCALCERT` installs the base version and makes no certificate. Where the
author's signature is already trusted, no certificate is made either.

## What goes over the network

The program, `LangSwitcher.exe`, does two things over the network and nothing else (SEC-03). The
updater, `LangSwitcherUpdate.exe`, does two more (SEC-10). All of them go over HTTPS with WinHTTP of
Windows, only to the author's site, to addresses fixed in the build.

| What | Who, when | What is sent |
|---|---|---|
| Read the news file `https://panda-pishet-kod.dev/langswitcher/news.toml` | The program, at most once a day; a failed read is tried again not before the next day. Off with `[letters] feed = false` in the settings file | One `GET`. The header `User-Agent: LangSwitcher/X.Y.Z` — the name and the version of the program. Nothing about the computer or the user |
| The same, the first time | The program, once: the first read after an installation, when the program created its settings file itself. An update never sends it | The address gets `?first=1`, with no number. After the site's first answer the mark is never sent again |
| Read the news file for updates | The updater, at most once a day, whether letters and news are on or off | The same `GET` and header, never the mark |
| Download a new version | The updater, only when the news file, signed by the author, names a version newer than the installed one | One `GET` of the installer from `https://panda-pishet-kod.dev/langswitcher/files/`. Its size and SHA-256 must match the signed news file, or it is not run |
| Send a message from the "Write to the author…" wizard | The program, only when you press **Send** or **Retry** | One `POST` to `https://panda-pishet-kod.dev/langswitcher/feedback`: the text exactly as shown in the window, the interface language, the kind of message, the version of the program, your contact if you typed one, and a random number of the request |

A news file is accepted only with a valid signature of the author and only up to a size limit. The
program depends on no crate with network abilities, and the only network addresses in the release
binaries are those of the module `letters::links` (SPEC SEC-03).

## Where the code is

Everything named here is in this repository; every release has its tag (`e116` is 1.6.0). How the
parts fit together is in [ARCHITECTURE.md](ARCHITECTURE.md).
