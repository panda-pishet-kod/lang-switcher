; ============================================================================
;  Lang Switcher -- Inno Setup script.  SPEC.md section 8.4, task T-09-2.
;
;  WHAT THIS SCRIPT PACKAGES
;
;    Two ALREADY SIGNED images of one build, produced by tools\release.ps1 and handed
;    to this script by tools\build-installer.ps1 -- not freshly compiled binaries:
;      /DSourceExe=      LangSwitcher.exe, the FULL image (uiAccess);
;      /DSourceExeBase=  LangSwitcher-base.exe, the BASE image (no uiAccess), the
;                        same body (tools\verify-one-body.ps1);
;    and installer\local-trust.ps1, the helper, with /DAuthorThumbprint= -- the
;    certificate the full image is signed with.  Section 8.2 of SPEC.md: a
;    uiAccess="true" image needs BOTH an Authenticode signature the machine trusts
;    AND a location under %ProgramFiles%.  Packaging an unsigned binary produces an
;    installer whose product cannot start, and the defect then looks like an
;    installer defect, which it is not.
;
;  THE FULL IMAGE BY DEFAULT, THE BASE ONE AS THE FALLBACK (stage E91, question 155)
;
;    The base image becomes the program file first: it starts on any machine.  The
;    full one is put beside it under a temporary name, and after the files are
;    copied [Code] runs the helper, which answers 0 -- the author's signature is
;    trusted here, the full image may stay as it is -- or 1 -- it made a certificate
;    on this machine, signed the full image with it and destroyed the key -- and only
;    then does the full image replace the base one.  Any other answer, /NOLOCALCERT
;    included, leaves the base image, which does not reach the windows of programs
;    run as administrator; the program starts whatever the outcome.  No page and no
;    question (155.6); one line on the Ready page says what will happen.  The
;    uninstaller removes the local certificate; the author's certificate is never
;    touched.
;
;  WHY THIS FILE IS PURE ASCII, AND IN WHICH LANGUAGE THE WIZARD SPEAKS
;
;    This script stays ASCII (decision R-05, the rule of the .ps1 files): it is
;    read by more tools than the compiler, and ASCII is the one encoding none of
;    them can misread.  Its words for the user are not in it.  Since stage E91
;    (decisions 155.10...155.12) the wizard speaks the language of Windows: the
;    official translations of Inno Setup for the languages of the program, and,
;    for the messages that are this installer's own, the files lang\<Language>.isl,
;    UTF-8 without a byte order mark -- read as UTF-8 by this compiler since Inno
;    Setup 6.3.0, measured on 6.7.3 (scratchpad-E91, probe P12: MATCH).  English is
;    the first language and the fallback; Greek has no official translation, so a
;    Greek Windows gets English (155.11).  No language dialog: ShowLanguageDialog=no,
;    and /LANG=<name> picks one by hand.
;
;  WHY THE INSTALL DIRECTORY IS NOT A CHOICE
;
;    The scheduled task LangSw-Uninstall is hardwired to
;    C:\Program Files\Lang_Switcher\unins000.exe and cannot be edited from inside
;    this project.  The destination is therefore fixed from outside, the directory
;    page is disabled, and the uninstaller keeps Inno's ordinary unins000.exe name.
; ============================================================================

; --- Autostart: the product is the one writer of the Run value ----------------
;
; FR-93, decision 120.4 of DECISIONS.md (stage E55).  This script does NOT write
; HKCU\...\Run.  The product makes that value agree with general.autostart of its
; own configuration at every start -- default true, SPEC.md section 7 -- and the
; settings dialog and the tray menu change it from there.  One writer, on purpose:
; a second one here would write from an elevated Setup, whose HKCU is not
; necessarily the hive of the person who signs in, and could only ever agree with
; the product by accident.  The uninstaller still removes the value, and does so
; unconditionally -- CurUninstallStepChanged below.
;
; Until E55 an optional [Tasks] entry wrote the value, unticked by default behind
; a compile-time knob AUTOSTART_DEFAULT_ON, and the shipping default was OFF.
; Decision 120.4 turned autostart on by default and made the program carry it
; out, so the task, its [Registry] entry and the knob are gone together.

; One name, the name of the program, everywhere: the wizard, the properties of the installer file,
; the Start menu folder, the entry in Apps & features, the shortcuts -- the heading of the program's
; notifications is the name of its shortcut -- and the autostart value below (decision 173.1 of the
; owner, stage E101).  From e90 to e100 the installer added the platform to the name where it stood
; alone (decision 149.2, done by 153.4); 173.1 cancelled that.  AppId is not touched, so an update
; is still the same application.
#define AppName        "Lang Switcher"
; The name of the Start menu folder of e90...e100, with the platform: kept only for the cleanup in
; [InstallDelete].  tests\installer_name.rs allows the old name on this line and on no other.
#define GroupNameUntilE100 "Lang Switcher for Windows"
#define AppVersion     "1.3.0"
#define AppPublisher   "Panda Koder"
#define AppExeName     "LangSwitcher.exe"
#define AppCopyright   "Copyright (C) 2026 Panda Koder"

; The signed shipping artifact, always given on the command line:
; tools\build-installer.ps1 runs ISCC /DSourceExe=<signed image>.  There is no
; default (stage E89, decision 152.3): a default would be a path of one machine,
; and a compile without the define would package whatever happened to lie there.
#ifndef SourceExe
  #error SourceExe is not defined: run tools\build-installer.ps1, it passes /DSourceExe=<signed image>
#endif
; Stage E91: the base image and the certificate of the full one come the same way, and for the
; same reason have no defaults.
#ifndef SourceExeBase
  #error SourceExeBase is not defined: run tools\build-installer.ps1, it passes /DSourceExeBase=<signed base image>
#endif
#ifndef AuthorThumbprint
  #error AuthorThumbprint is not defined: run tools\build-installer.ps1, it passes /DAuthorThumbprint=<SHA-1>
#endif
; Stage E104 (question 189, TZ E104 p. 13): the updater, signed like the program, one image (no
; uiAccess), beside the program file. No default, for the reason SourceExe has none.
#ifndef SourceUpdater
  #error SourceUpdater is not defined: run tools\build-installer.ps1, it passes /DSourceUpdater=<signed updater>
#endif

; The updater and what it needs on the machine (TZ E104 p. 14-18). THESE MUST MATCH
; src\update.rs LITERALLY: TASK_NAME, DATA_DIR + UPDATE_DIR, INSTALLED_SETUP, SETUP_MUTEX and
; PHASE_TWO_SWITCH; tests\installer_updater.rs reads both files and compares them.
#define UpdaterExeName "LangSwitcherUpdate.exe"
#define UpdateTask     "Lang Switcher Update"
#define UpdateDir      "Lang_Switcher\update"
#define InstalledSetup "installed-setup.exe"
#define SetupMutexName "Global\Lang_Switcher.Setup"
#define PhaseTwo       "/phase2"

; The temporary name of the full image beside the program file, and the public part of the
; certificate the helper makes on this machine (installer\local-trust.ps1 writes it).
#define FullTempName   "LangSwitcher-full.tmp"
#define LocalCertFile  "LangSwitcher-local.cer"

; res\langswitcher-active.ico -- the tray icon of the running product, reused for
; the installer, the Start menu shortcut and the entry in Apps & features.
#define IconFile "..\res\langswitcher-active.ico"

; The name and the key FR-93 uses.  BOTH MUST MATCH src\settings.rs LITERALLY:
;   RUN_KEY_PATH        = r"Software\Microsoft\Windows\CurrentVersion\Run"
;   AUTOSTART_VALUE_NAME = "Lang Switcher"
; This script no longer writes the value (see the head of the file); the
; uninstaller deletes it by this name.  If the name drifted from the one the
; product writes, uninstalling would leave the product's value behind, pointing
; at a deleted file.
#define RunKey     "Software\Microsoft\Windows\CurrentVersion\Run"
#define RunValue   "Lang Switcher"

; The mutex the running product holds (FR-82).  MUST MATCH src\app.rs LITERALLY:
;   MUTEX_NAME = w!(r"Local\Lang_Switcher.SingleInstance")
; The uninstaller looks for it to refuse removing a running program (decision
; 155.24); under another name the check would never fire, and nothing but
; tests\installer_uninstall.rs would say so.
#define ProductMutex "Local\Lang_Switcher.SingleInstance"

; The class of the product's one top-level window, the hidden UI window that owns
; the tray icon.  MUST MATCH src\app.rs LITERALLY:
;   WINDOW_CLASS_NAME = w!("LangSwitcher.Hidden")
; The uninstaller posts WM_ENDSESSION to it to close the running program (decision
; 155.25); the two other windows of that class are message-only and are neither
; found nor asked.  tests\installer_uninstall.rs keeps the two names together.
#define ProductWindowClass "LangSwitcher.Hidden"

; The files the program writes into %APPDATA%\Lang_Switcher, by the names the uninstaller removes
; them (decisions 177.1 and 177.20.1, the end of this script).  EACH MUST MATCH THE PRODUCT
; LITERALLY -- tests\installer_uninstall.rs reads both sides:
;   DataFolder    src\lib.rs       CONFIG_DIR_NAME   = "Lang_Switcher"
;   ConfigName    src\settings.rs  CONFIG_FILE_NAME  = "config.toml"
;   BadSuffix     src\settings.rs  QUARANTINE_SUFFIX = ".bad"
;   JournalName   src\diag.rs      LOG_FILE_NAME     = "diag.log"
;   AppealStem,   src\letters.rs   report::file_name: an appeal saved from the wizard "Write to the
;   AppealStemRu                   author" is <stem>-<date>.txt, its stem "report", or the Russian
;                                  word for it when the interface is Russian -- written here as
;                                  Pascal character codes, so that this file stays ASCII.
#define DataFolder   "Lang_Switcher"
#define ConfigName   "config.toml"
#define BadSuffix    ".bad"
#define JournalName  "diag.log"
#define AppealStem   "report"
#define AppealStemRu "#$043E#$0431#$0440#$0430#$0449#$0435#$043D#$0438#$0435"

[Setup]
; AppId is the identity Windows matches an upgrade against.  It is a constant and
; must never change: a new AppId turns every future update into a second parallel
; installation with its own uninstall entry.
AppId={{1A22BDD8-916A-490E-8269-66869A611388}
AppName={#AppName}
AppVersion={#AppVersion}
AppVerName={#AppName} {#AppVersion}
AppPublisher={#AppPublisher}
AppCopyright={#AppCopyright}
VersionInfoVersion=1.3.0.0
VersionInfoCompany={#AppPublisher}
VersionInfoProductName={#AppName}
VersionInfoProductVersion=1.3.0.0
VersionInfoDescription={#AppName} Setup
VersionInfoCopyright={#AppCopyright}

; Section 8.4: install into %ProgramFiles% with a single elevation.  admin asks
; for it once, at the start, and nothing later needs to ask again.
PrivilegesRequired=admin
PrivilegesRequiredOverridesAllowed=
; The per-user areas this script touches are the ones the uninstaller cleans up for the account
; it runs as -- the Run value and the temporaries under {userappdata}\Lang_Switcher -- and each
; carries that caveat where it is used.  Acknowledged here instead of repeated in every build log.
UsedUserAreasWarning=no

; x64 only.  The shipping binary is PE machine 0x8664, and on 64-bit Windows a
; 32-bit install would resolve {commonpf} to "Program Files (x86)" -- the wrong
; directory, and not the one LangSw-Uninstall is hardwired to.
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible

; Exactly C:\Program Files\Lang_Switcher, see the header.  {commonpf64} is the
; 64-bit Program Files even in a 32-bit install mode, so the path cannot drift.
DefaultDirName={commonpf64}\Lang_Switcher
DisableDirPage=yes
DefaultGroupName={#AppName}
; Inno Setup would otherwise reuse the Start menu folder of the installed version on an update,
; and the folder of e90...e100 (GroupNameUntilE100) would keep its name.  The old folder itself is
; removed by [InstallDelete] below.
UsePreviousGroup=no
DisableProgramGroupPage=yes
AllowNoIcons=yes

; Section 8.4: the running instance is closed before its file is replaced.
; CloseApplications=yes uses the Restart Manager, which identifies the holder of
; a file by HANDLE, not by image name -- it cannot mistake an unrelated process
; that happens to be called LangSwitcher.exe for ours, which a taskkill /IM would.
; It also asks the process to close politely first, so the product gets to unhook
; its global hook and drop its tray icon instead of leaving a ghost.
; RestartApplications=no: after an update the product stays down.  Starting it is
; a separate, deliberate step, never a side effect of installing (task item 9).
CloseApplications=yes
CloseApplicationsFilter=*.exe
RestartApplications=no
; Stage E104: one Setup at a time on the machine, and the name phase two of the updater waits on
; -- it raises the program again, or starts a rollback, only once this Setup has ended.
SetupMutex={#SetupMutexName}

Uninstallable=yes
UninstallDisplayName={#AppName}
UninstallDisplayIcon={app}\{#AppExeName}
SetupIconFile={#IconFile}

; A silent install shows nothing and answers nothing. SetupLogging=yes leaves a log
; in %TEMP% on every run, which is the only way a /VERYSILENT failure can be
; diagnosed after the fact. It earned its place here immediately: the first silent
; install of this script reported success while quietly skipping the autostart
; registry entry, and the log is what found it.
SetupLogging=yes

OutputDir=Output
OutputBaseFilename=LangSwitcher-setup

; The language of the wizard is the language of Windows (decision 155.12): the UI language,
; matched first exactly, then by the primary language, then the first entry -- English.
; UsePreviousLanguage=no: by default Inno takes the language of the installed version, and every
; version before e91 was installed in English for want of any other -- an update would speak
; English on every Windows.
ShowLanguageDialog=no
LanguageDetectionMethod=uilanguage
UsePreviousLanguage=no
Compression=lzma2/max
SolidCompression=yes
; The look of the site panda-pishet-kod.dev (stage E99, decision 167.1: variant B of question 167,
; chosen by the owner on the mock-up).  The background is the "paper" of the site in the light
; theme of Windows and its "ink" in the dark one, with the bamboo of its margins at the left and
; the right edge; no picture of Inno's own.  dynamic: the wizard follows the theme of Windows,
; looked at once, at the start -- without it the ...DynamicDark lines do nothing and the wizard is
; always light.  windows11 is the light counterpart of the dark style of Inno, excludelightcontrols
; keeps the native controls of Windows in the light theme, hidebevels takes the lines away.  The
; uninstaller gets the colour alone: Inno shows a background picture in the wizard only.
; The two empty ...DynamicDark lines of the pictures are not decoration (decision 167.20): in the
; dark theme Inno reads them instead of WizardImageFile and WizardSmallImageFile, and without them
; it puts its own dark picture back into the corner.
; The pictures are drawn by tools\make-installer-art.ps1, one per scale of the table of the Inno
; help -- Inno takes the nearest; tests\installer_art.rs holds them to their sizes and
; tests\installer_languages.rs holds these lines.
WizardStyle=modern dynamic windows11 excludelightcontrols hidebevels
WizardBackColor=#FAF9F6
WizardBackColorDynamicDark=#23272F
WizardBackImageFile=art\back-light-100.png,art\back-light-125.png,art\back-light-150.png,art\back-light-175.png,art\back-light-200.png
WizardBackImageFileDynamicDark=art\back-dark-100.png,art\back-dark-125.png,art\back-dark-150.png,art\back-dark-175.png,art\back-dark-200.png
WizardImageFile=
WizardImageFileDynamicDark=
WizardSmallImageFile=
WizardSmallImageFileDynamicDark=
; The installer is signed afterwards by tools\build-installer.ps1, not by an Inno
; SignTool= directive: the private key is non-exportable and lives in
; Cert:\CurrentUser\My, so signing happens in the same user session, once, over
; the finished file (TOOLCHAIN.md section 5).

[Languages]
; The messages of this installer that are its own live in lang\<Language>.isl, the second file
; of each entry (stage E91): the line of the Ready page, the window of a failed certificate and
; the question about the settings file. English first: it is the fallback. Then the twelve
; official translations for the languages of the program, and Brazilian Portuguese with the
; same messages as Portuguese (155.12). Greek: no official file -- English (155.11).
; tests\installer_languages.rs holds every entry to its file and every file to English.isl.
; Stage E108 (decision 202.2): the languages of group A follow, in the order of the registry
; res\lang\languages.toml. Where Inno has no translation of its own (inno_official = false) the
; first file is Default.isl and lang\<Language>.isl carries all of Inno's messages too, translated
; by the same window as the program; where it has one (ta, th, bg, hy) it is the official file.
Name: "english"; MessagesFile: "compiler:Default.isl,lang\English.isl"
Name: "russian"; MessagesFile: "compiler:Languages\Russian.isl,lang\Russian.isl"
Name: "ukrainian"; MessagesFile: "compiler:Languages\Ukrainian.isl,lang\Ukrainian.isl"
Name: "german"; MessagesFile: "compiler:Languages\German.isl,lang\German.isl"
Name: "french"; MessagesFile: "compiler:Languages\French.isl,lang\French.isl"
Name: "spanish"; MessagesFile: "compiler:Languages\Spanish.isl,lang\Spanish.isl"
Name: "portuguese"; MessagesFile: "compiler:Languages\Portuguese.isl,lang\Portuguese.isl"
Name: "italian"; MessagesFile: "compiler:Languages\Italian.isl,lang\Italian.isl"
Name: "polish"; MessagesFile: "compiler:Languages\Polish.isl,lang\Polish.isl"
Name: "czech"; MessagesFile: "compiler:Languages\Czech.isl,lang\Czech.isl"
Name: "turkish"; MessagesFile: "compiler:Languages\Turkish.isl,lang\Turkish.isl"
Name: "hebrew"; MessagesFile: "compiler:Languages\Hebrew.isl,lang\Hebrew.isl"
Name: "arabic"; MessagesFile: "compiler:Languages\Arabic.isl,lang\Arabic.isl"
Name: "brazilianportuguese"; MessagesFile: "compiler:Languages\BrazilianPortuguese.isl,lang\BrazilianPortuguese.isl"
Name: "hindi"; MessagesFile: "compiler:Default.isl,lang\Hindi.isl"
Name: "bengali"; MessagesFile: "compiler:Default.isl,lang\Bengali.isl"
Name: "urdu"; MessagesFile: "compiler:Default.isl,lang\Urdu.isl"
Name: "punjabi"; MessagesFile: "compiler:Default.isl,lang\Punjabi.isl"
Name: "marathi"; MessagesFile: "compiler:Default.isl,lang\Marathi.isl"
Name: "telugu"; MessagesFile: "compiler:Default.isl,lang\Telugu.isl"
Name: "tamil"; MessagesFile: "compiler:Languages\Tamil.isl,lang\Tamil.isl"
Name: "persian"; MessagesFile: "compiler:Default.isl,lang\Persian.isl"
Name: "gujarati"; MessagesFile: "compiler:Default.isl,lang\Gujarati.isl"
Name: "thai"; MessagesFile: "compiler:Languages\Thai.isl,lang\Thai.isl"
Name: "kannada"; MessagesFile: "compiler:Default.isl,lang\Kannada.isl"
Name: "odia"; MessagesFile: "compiler:Default.isl,lang\Odia.isl"
Name: "malayalam"; MessagesFile: "compiler:Default.isl,lang\Malayalam.isl"
Name: "uzbek"; MessagesFile: "compiler:Default.isl,lang\Uzbek.isl"
Name: "uzbekcyrillic"; MessagesFile: "compiler:Default.isl,lang\UzbekCyrillic.isl"
Name: "nepali"; MessagesFile: "compiler:Default.isl,lang\Nepali.isl"
Name: "assamese"; MessagesFile: "compiler:Default.isl,lang\Assamese.isl"
Name: "azerbaijani"; MessagesFile: "compiler:Default.isl,lang\Azerbaijani.isl"
Name: "khmer"; MessagesFile: "compiler:Default.isl,lang\Khmer.isl"
Name: "kazakh"; MessagesFile: "compiler:Default.isl,lang\Kazakh.isl"
Name: "serbiancyrillic"; MessagesFile: "compiler:Default.isl,lang\SerbianCyrillic.isl"
Name: "serbianlatin"; MessagesFile: "compiler:Default.isl,lang\SerbianLatin.isl"
Name: "uyghur"; MessagesFile: "compiler:Default.isl,lang\Uyghur.isl"
Name: "bulgarian"; MessagesFile: "compiler:Languages\Bulgarian.isl,lang\Bulgarian.isl"
Name: "armenian"; MessagesFile: "compiler:Languages\Armenian.isl,lang\Armenian.isl"
Name: "lao"; MessagesFile: "compiler:Default.isl,lang\Lao.isl"
Name: "belarusian"; MessagesFile: "compiler:Default.isl,lang\Belarusian.isl"
Name: "tatar"; MessagesFile: "compiler:Default.isl,lang\Tatar.isl"
Name: "georgian"; MessagesFile: "compiler:Default.isl,lang\Georgian.isl"
Name: "macedonian"; MessagesFile: "compiler:Default.isl,lang\Macedonian.isl"

[Files]
; ignoreversion: the product is replaced whenever this installer runs, including a
; reinstall of the identical version.  Version-number comparison would silently
; skip the copy and leave a stale binary that still passes every file listing.
;
; Stage E91: the BASE image becomes the program file first, so that the program starts
; whatever the helper answers; the FULL image lies beside it under a temporary name until
; [Code] makes it the program file (helper 0 or 1) or deletes it (anything else).  The helper
; itself is never copied: [Code] extracts it into {tmp}.
Source: "{#SourceExeBase}"; DestDir: "{app}"; DestName: "{#AppExeName}"; Flags: ignoreversion
Source: "{#SourceExe}"; DestDir: "{app}"; DestName: "{#FullTempName}"; Flags: ignoreversion
; Stage E104: the updater, one image, signed by the author like the program; it needs no
; uiAccess and no local certificate (TZ E104 p. 13).
Source: "{#SourceUpdater}"; DestDir: "{app}"; DestName: "{#UpdaterExeName}"; Flags: ignoreversion
Source: "local-trust.ps1"; Flags: dontcopy
; Stage E99, decision 167.1 (B1): the logo of the signature of the author, one per theme and scale
; (tools\make-installer-art.ps1); [Code] extracts the one it needs into {tmp}.
Source: "art\logo-light-100.png"; Flags: dontcopy
Source: "art\logo-light-125.png"; Flags: dontcopy
Source: "art\logo-light-150.png"; Flags: dontcopy
Source: "art\logo-light-175.png"; Flags: dontcopy
Source: "art\logo-light-200.png"; Flags: dontcopy
Source: "art\logo-dark-100.png"; Flags: dontcopy
Source: "art\logo-dark-125.png"; Flags: dontcopy
Source: "art\logo-dark-150.png"; Flags: dontcopy
Source: "art\logo-dark-175.png"; Flags: dontcopy
Source: "art\logo-dark-200.png"; Flags: dontcopy

[InstallDelete]
; The Start menu folder has had two names: "Lang Switcher" (AppName) before e90 and again since
; e101 (decision 173.1), and the name with the platform, GroupNameUntilE100, from e90 to e100.
; Both folders go here, whichever version is installed: without the first line an update from
; e90...e100 would leave the folder of the old name and its two shortcuts behind, beside the new
; ones.  The second line removes the folder this installer is about to fill -- on purpose: Inno
; processes [InstallDelete] as the first step of the installation, before [Files] and [Icons] (the
; help of Inno Setup 6.7, "Installation Order"), so [Icons] puts the folder back with its two
; shortcuts, and an update from any version comes to one folder of the one name.
; {autoprograms} is the all-users Start menu here, because Setup runs elevated
; (PrivilegesRequired=admin).
Type: filesandordirs; Name: "{autoprograms}\{#GroupNameUntilE100}"
Type: filesandordirs; Name: "{autoprograms}\{#AppName}"

[Icons]
Name: "{group}\{#AppName}"; Filename: "{app}\{#AppExeName}"; IconFilename: "{app}\{#AppExeName}"
Name: "{group}\Uninstall {#AppName}"; Filename: "{uninstallexe}"

[UninstallDelete]
; Task T-55-5, finding T8: a temporary of the atomic write of config.toml that an interrupted
; write left behind -- config.toml.<pid>.tmp.  The mask the product sweeps at every start
; (settings.rs, remove_abandoned_temporaries), with one difference: an Inno mask cannot say
; "digits", so here the middle is any text -- nothing but this program writes names of that
; shape into its own folder.  config.toml.bad, the kept copy of an unreadable configuration,
; does not match: it goes, with config.toml, only on an explicit yes, and the journal goes always
; (the [Code] at the end of this script).
; {userappdata} is the profile of the account the uninstaller runs as -- the same caveat as the
; HKCU value in CurUninstallStepChanged.
Type: files; Name: "{userappdata}\Lang_Switcher\config.toml.*.tmp"
; Stage E104 (TZ E104 p. 17, SPEC section 13 p. 7): the folder of the updater -- its state, its
; journal, the copies of the installer -- goes whole, and the data folder with it when nothing
; else is in it. The task of the scheduler goes in [Code] (CurUninstallUpdater), before the files.
Type: filesandordirs; Name: "{commonappdata}\{#UpdateDir}"
Type: dirifempty; Name: "{commonappdata}\Lang_Switcher"

[Dirs]
; Stage E104 (TZ E104 p. 15, premise P10): the folder of the updater. Its list of rights is not
; Inno's to give -- Permissions only adds, and %ProgramData% lets every user create files below
; it -- so [Code] (SecureUpdateFolders) takes the owner and cuts the inheritance with icacls.
Name: "{commonappdata}\Lang_Switcher"
Name: "{commonappdata}\{#UpdateDir}"

[Run]
; skipifsilent is the whole point of this entry being safe: a silent install --
; which is how the scheduled task runs it -- never starts the product.
; runasoriginaluser matters for section 8.1: Setup is elevated, and a child of an
; elevated process inherits high integrity.  The product is meant to run at MEDIUM
; integrity with uiAccess, not as an administrator, so it is launched back as the
; user who started Setup.
; shellexec (stage E91, defect D2 of decision 155v): without it Setup starts the program
; through CreateProcess, and CreateProcess refuses an image that asks for uiAccess with
; error 740 in ANY folder (T-09-2) -- the "CreateProcess failed; code 740" window the owner
; saw on a clean machine.  ShellExecute goes through the AppInfo service, which is what grants
; uiAccess; the base image starts through it just the same.
; The caption is Inno's own LaunchProgram, translated in every official .isl (155.12).
Filename: "{app}\{#AppExeName}"; Description: "{cm:LaunchProgram,{#AppName}}"; \
    Flags: nowait postinstall skipifsilent runasoriginaluser shellexec

[Code]

#ifdef DIAG
{ Temporary probe, compiled in only with ISCC /DDIAG. Answers one question that
  cannot be answered from outside: WHICH user hive HKEY_CURRENT_USER refers to
  inside an installer launched by a scheduled task, and whether a value written
  there can be read back by the installer itself one instruction later. }
function InitializeSetup(): Boolean;
var
  ReadBack: String;
begin
  Result := True;
  { State BEFORE this run writes anything. On a SECOND run this answers whether
    the value written by the FIRST run survived on the machine at all. }
  if RegQueryStringValue(HKEY_CURRENT_USER, '{#RunKey}', '{#RunValue}', ReadBack) then
    Log('DIAG pre-install HKCU = [' + ReadBack + ']')
  else
    Log('DIAG pre-install HKCU = *** NOT FOUND ***');
end;

procedure CurStepChanged(CurStep: TSetupStep);
var
  ReadBack: String;
begin
  if CurStep = ssPostInstall then
  begin
    Log('DIAG username      = ' + ExpandConstant('{username}'));
    Log('DIAG userappdata   = ' + ExpandConstant('{userappdata}'));
    Log('DIAG localappdata  = ' + ExpandConstant('{localappdata}'));
    Log('DIAG userprofile   = ' + ExpandConstant('{%USERPROFILE|<unset>}'));
    Log('DIAG IsAdminInstallMode = ' + IntToStr(Integer(IsAdminInstallMode())));
    if RegQueryStringValue(HKEY_CURRENT_USER, '{#RunKey}', '{#RunValue}', ReadBack) then
      Log('DIAG readback HKCU = [' + ReadBack + ']')
    else
      Log('DIAG readback HKCU = *** NOT FOUND ***');
    if RegQueryStringValue(HKEY_CURRENT_USER, '{#RunKey}',
         'MicrosoftEdgeAutoLaunch_C46CFC0629905CC775E70B50EA8A519C', ReadBack) then
      Log('DIAG edge value    = present (this is the human user hive)')
    else
      Log('DIAG edge value    = ABSENT (this is NOT the human user hive)');
  end;
end;
#endif

(* ---------------------------------------------------------------------------
  The full image and the local certificate (stage E91, question 155).
  A comment of this kind and not in braces: the constants below carry braces.

  The Files section has put the BASE image down as the program file and the FULL
  image beside it as {#FullTempName}.  After the files, installer\local-trust.ps1 is
  run -- extracted into {tmp}, through ExecAndLogOutput, so that every line it
  prints lands in this log -- and its exit code decides:
    0  the author's signature is trusted here: the full image replaces the base
    1  a certificate was made on this machine and the full image signed with it:
       the full image replaces the base one
    2  /NOLOCALCERT: the base image stays, and nothing is said
    anything else, or no answer: the base image stays; one window says so, or,
       in a silent run, one line of this log
  The replacement is one MoveFileEx over the program file, checked by SHA-256,
  and the temporary file is gone at the end whatever happened.  The program
  starts in every case.

  The line on the Ready page (155.8) is shown when a certificate is going to be
  made: not under /NOLOCALCERT, and not when the author's certificate is already
  in a Root store of this machine -- that is read from the registry, a forecast
  for one line of text; the helper decides by the signature itself.
  --------------------------------------------------------------------------- *)
const
  MOVEFILE_REPLACE_EXISTING = $00000001;
  MOVEFILE_WRITE_THROUGH = $00000008;

var
  NoLocalCert: Boolean;
  AuthorTrustedHere: Boolean;

function MoveFileEx(lpExistingFileName, lpNewFileName: String; dwFlags: Cardinal): Boolean;
  external 'MoveFileExW@kernel32.dll stdcall';

function YesNo(const Value: Boolean): String;
begin
  if Value then Result := 'yes' else Result := 'no';
end;

function HasSwitch(const Name: String): Boolean;
var
  I: Integer;
begin
  Result := False;
  for I := 1 to ParamCount do
    if CompareText(ParamStr(I), Name) = 0 then
      Result := True;
end;

function IsInMachineRoot(const Thumbprint: String): Boolean;
begin
  Result := RegKeyExists(HKLM, 'SOFTWARE\Microsoft\SystemCertificates\ROOT\Certificates\' + Thumbprint) or
            RegKeyExists(HKLM, 'SOFTWARE\Policies\Microsoft\SystemCertificates\Root\Certificates\' + Thumbprint) or
            RegKeyExists(HKLM, 'SOFTWARE\Microsoft\EnterpriseCertificates\Root\Certificates\' + Thumbprint);
end;

<event('InitializeSetup')>
function InitializeLocalTrust(): Boolean;
begin
  NoLocalCert := HasSwitch('/NOLOCALCERT');
  AuthorTrustedHere := IsInMachineRoot('{#AuthorThumbprint}');
  Log('Lang Switcher: /NOLOCALCERT ' + YesNo(NoLocalCert) +
      '; the author''s certificate {#AuthorThumbprint} is in a machine Root store: ' + YesNo(AuthorTrustedHere));
  Result := True;
end;

function UpdateReadyMemo(Space, NewLine, MemoUserInfoInfo, MemoDirInfo, MemoTypeInfo,
  MemoComponentsInfo, MemoGroupInfo, MemoTasksInfo: String): String;
begin
  Result := '';
  if MemoUserInfoInfo <> '' then Result := Result + MemoUserInfoInfo + NewLine + NewLine;
  if MemoDirInfo <> '' then Result := Result + MemoDirInfo + NewLine + NewLine;
  if MemoTypeInfo <> '' then Result := Result + MemoTypeInfo + NewLine + NewLine;
  if MemoComponentsInfo <> '' then Result := Result + MemoComponentsInfo + NewLine + NewLine;
  if MemoGroupInfo <> '' then Result := Result + MemoGroupInfo + NewLine + NewLine;
  if MemoTasksInfo <> '' then Result := Result + MemoTasksInfo + NewLine + NewLine;
  if (not NoLocalCert) and (not AuthorTrustedHere) then
    Result := Result + CustomMessage('LocalCertReady');
end;

(* ---------------------------------------------------------------------------
  The field of the Ready page wraps (stage E98, finding 165.3 of the owner).
  The Ready page is the first the wizard shows -- the pages of the folder and
  of the group are off -- and the line above is one sentence, longer than the
  field in every language.  The field does not wrap unless told to, and the
  sentence ran off the right edge of the window.  Wrapping by words, and a
  vertical scroll bar for a field whose lines grow longer than it is.  The
  texts of LocalCertReady stay as they are: the field breaks them, not a hand.
  tests\installer_languages.rs holds both statements to an event of the wizard.
  --------------------------------------------------------------------------- *)
<event('InitializeWizard')>
procedure InitializeWizardReadyMemo();
begin
  WizardForm.ReadyMemo.WordWrap := True;
  WizardForm.ReadyMemo.ScrollBars := ssVertical;
end;

(* ---------------------------------------------------------------------------
  The line above the field of the Ready page (stage E99, decision 167.5).
  Inno puts ReadyLabel2a there -- "click Install to continue, or click Back
  if you want to review or change any settings" -- although the Ready page
  is the first page of this wizard and has no Back button (measured on the
  live window, decision 167.20).  ReadyLabel2b is the same sentence without
  the Back button, in every official translation; no text of Inno is
  rewritten.  The label becomes a line shorter, and the field moves up and
  grows by as much -- what Inno does with its own label when it lays the page
  out, so the page keeps its spacing.
  Not in InitializeWizard: Inno sets the label again, and lays the field out
  under it, each time it shows the Ready page -- set there, the line came back
  with Back on the live window (decision 167.21).  CurPageChanged comes after.
  --------------------------------------------------------------------------- *)
<event('CurPageChanged')>
procedure CurPageChangedReadyLabel(CurPageID: Integer);
var
  Delta: Integer;
begin
  if CurPageID <> wpReady then
    Exit;
  WizardForm.ReadyLabel.Caption := SetupMessage(msgReadyLabel2b);
  Delta := WizardForm.ReadyLabel.AdjustHeight;
  WizardForm.ReadyMemo.Top := WizardForm.ReadyMemo.Top + Delta;
  WizardForm.ReadyMemo.Height := WizardForm.ReadyMemo.Height - Delta;
end;

(* ---------------------------------------------------------------------------
  The signature of the author on every page of the wizard (stage E99,
  decision 167.1, signature B1 of question 167, chosen by the owner on the
  mock-up): at the bottom left, beside the buttons, the logo of the site, the
  name -- bold -- with the slogan, and the address of the site on the second
  line.  A click on the address opens the site.
  * The colours are those of the site (tools\make-installer-art.ps1 names
    where they come from): the name --ink, the slogan --ink-2, the address
    --accent, of the light or the dark theme as the wizard is.  Under a
    style of Inno a colour set here is dropped unless seFont is taken out of
    StyleElements (measured both ways, decision 167.20).
  * The name and the address are not translated; the slogan is AuthorSlogan
    of lang\<Language>.isl.  The font is the font of the wizard.
  * The place follows the buttons, not numbers of its own: the margin at the
    left is the margin the buttons keep at the right, the space after the
    logo is the space between two buttons, and the block stands in the
    middle of the height of the buttons.  In Hebrew and Arabic the wizard
    mirrors it together with everything else.
  * The address opens with ShellExecAsOriginalUser: Setup runs as an
    administrator, and the site must open in the browser of the person who
    started it, without those rights.  The installer itself never goes to
    the network.  The address is a label, not a button: like on the mock-up,
    the keyboard does not reach it.
  * The logo is a PNG for the theme and the scale -- ScaleX(28) at the five
    scales is 28, 37, 47, 56 and 60 -- the nearest one, stretched to the
    exact size.
  Not in the uninstaller: its windows carry the colour of the wizard alone.
  tests\installer_languages.rs holds the address, the call and the event.
  --------------------------------------------------------------------------- *)
var
  AuthorLogo: TBitmapImage;
  AuthorName: TNewStaticText;
  AuthorSlogan: TNewStaticText;
  AuthorSite: TNewStaticText;

procedure AuthorSiteClick(Sender: TObject);
var
  ErrorCode: Integer;
begin
  if not ShellExecAsOriginalUser('open', 'https://panda-pishet-kod.dev/', '', '', SW_SHOWNORMAL, ewNoWait, ErrorCode) then
    Log('Lang Switcher: the site of the author did not open, code ' + IntToStr(ErrorCode));
end;

function AuthorText(const Caption, Light, Dark: String): TNewStaticText;
begin
  Result := TNewStaticText.Create(WizardForm);
  Result.Caption := Caption;
  Result.Parent := WizardForm;
  Result.StyleElements := Result.StyleElements - [seFont];
  if IsDarkInstallMode then
    Result.Font.Color := StrToColor(Dark)
  else
    Result.Font.Color := StrToColor(Light);
end;

<event('InitializeWizard')>
procedure InitializeWizardAuthorSignature();
var
  Theme, Scale, LogoFile: String;
  Size, Gap, Margin, Middle, TextHeight, TextLeft: Integer;
begin
  if IsDarkInstallMode then Theme := 'dark' else Theme := 'light';
  Size := ScaleX(28);
  if Size < 33 then Scale := '100'
  else if Size < 42 then Scale := '125'
  else if Size < 52 then Scale := '150'
  else if Size < 58 then Scale := '175'
  else Scale := '200';
  LogoFile := 'logo-' + Theme + '-' + Scale + '.png';
  ExtractTemporaryFile(LogoFile);

  Margin := WizardForm.ClientWidth - (WizardForm.CancelButton.Left + WizardForm.CancelButton.Width);
  Gap := WizardForm.CancelButton.Left - (WizardForm.NextButton.Left + WizardForm.NextButton.Width);
  Middle := WizardForm.NextButton.Top + WizardForm.NextButton.Height div 2;

  AuthorLogo := TBitmapImage.Create(WizardForm);
  AuthorLogo.BackColor := clNone;
  AuthorLogo.Stretch := True;
  AuthorLogo.PngImage.LoadFromFile(ExpandConstant('{tmp}\') + LogoFile);
  AuthorLogo.SetBounds(Margin, Middle - Size div 2, Size, Size);
  AuthorLogo.Parent := WizardForm;

  AuthorName := AuthorText('{#AppPublisher}', '#1C1B18', '#D6DAE0');
  AuthorName.Font.Style := [fsBold];
  AuthorSlogan := AuthorText(' ' + #$00B7 + ' ' + CustomMessage('AuthorSlogan'), '#5B5955', '#A2A8B3');
  AuthorSite := AuthorText('panda-pishet-kod.dev', '#1F7A4A', '#6CC894');
  AuthorSite.Cursor := crHand;
  AuthorSite.OnClick := @AuthorSiteClick;

  TextLeft := Margin + Size + Gap;
  TextHeight := AuthorName.Height + AuthorSite.Height;
  AuthorName.Left := TextLeft;
  AuthorName.Top := Middle - TextHeight div 2;
  AuthorSlogan.Left := AuthorName.Left + AuthorName.Width;
  AuthorSlogan.Top := AuthorName.Top;
  AuthorSite.Left := TextLeft;
  AuthorSite.Top := AuthorName.Top + AuthorName.Height;
end;

<event('CurStepChanged')>
procedure CurStepLocalTrust(CurStep: TSetupStep);
var
  AppDir, ExeFile, FullFile, Helper, PowerShell, Params, FullHash: String;
  Code: Integer;
  Replaced: Boolean;
begin
  if CurStep <> ssPostInstall then
    Exit;
  AppDir := ExpandConstant('{app}');
  ExeFile := AppDir + '\{#AppExeName}';
  FullFile := AppDir + '\{#FullTempName}';
  (* In 64-bit install mode Exec and its kin turn WOW64 file system redirection off,
     so the sys constant here is the 64-bit System32 and this is the 64-bit Windows
     PowerShell; the helper prints which one it is. *)
  PowerShell := ExpandConstant('{sys}\WindowsPowerShell\v1.0\powershell.exe');
  Code := -1;
  Replaced := False;
  try
    ExtractTemporaryFile('local-trust.ps1');
    Helper := ExpandConstant('{tmp}\local-trust.ps1');
    Params := '-NoProfile -NonInteractive -ExecutionPolicy Bypass -File "' + Helper +
              '" -FullImage "' + FullFile + '" -AppDir "' + AppDir +
              '" -AuthorThumbprint {#AuthorThumbprint}';
    if NoLocalCert then
      Params := Params + ' -NoLocalCert';
    Log('Lang Switcher: running the helper: "' + PowerShell + '" ' + Params);
    if not ExecAndLogOutput(PowerShell, Params, ExpandConstant('{tmp}'), SW_SHOWNORMAL,
                            ewWaitUntilTerminated, Code, nil) then
    begin
      Log('Lang Switcher: the helper could not be started: ' + SysErrorMessage(Code));
      Code := -1;
    end;
  except
    Log('Lang Switcher: the helper failed to run: ' + GetExceptionMessage);
    Code := -1;
  end;
  Log('Lang Switcher: the helper returned ' + IntToStr(Code));

  if (Code = 0) or (Code = 1) then
  begin
    try
      FullHash := GetSHA256OfFile(FullFile);
      if MoveFileEx(FullFile, ExeFile, MOVEFILE_REPLACE_EXISTING or MOVEFILE_WRITE_THROUGH) then
      begin
        if GetSHA256OfFile(ExeFile) = FullHash then
        begin
          Replaced := True;
          if Code = 0 then
            Log('Lang Switcher: branch 0 -- the program file is the FULL image, signed by the author; no certificate was made')
          else
            Log('Lang Switcher: branch 1 -- the program file is the FULL image, signed with the certificate made on this computer');
        end
        else
          Log('Lang Switcher: after the move the program file is not the full image (SHA-256 differs)');
      end
      else
        Log('Lang Switcher: replacing the program file failed: ' + SysErrorMessage(DLLGetLastError()));
    except
      Log('Lang Switcher: replacing the program file failed: ' + GetExceptionMessage);
    end;
  end;

  if FileExists(FullFile) then
    if not DeleteFile(FullFile) then
      Log('Lang Switcher: could not delete ' + FullFile);

  if not Replaced then
  begin
    Log('Lang Switcher: the program file is the BASE image -- it does not work in windows of programs run as administrator');
    if Code <> 2 then
    begin
      if WizardSilent() then
        Log('Lang Switcher: ' + CustomMessage('LocalCertFailed'))
      else
        MsgBox(CustomMessage('LocalCertFailed'), mbInformation, MB_OK);
    end;
  end;
end;

{ Uninstall, before anything is removed: the program must not be running
  (decisions 155.23, 155.24, 155.25).  Inno's uninstaller does not close programs
  -- the Restart Manager of CloseApplications serves Setup only -- and a running
  LangSwitcher.exe keeps its file locked: the uninstaller would skip the file, the
  folder would stay and the program would go on living in the tray.

  1. The product holds the mutex ProductMutex while it runs.  If it exists, the
     program is asked to close the way Windows asks at sign-out and the Restart
     Manager asks on an update: WM_ENDSESSION, wParam TRUE, lParam
     ENDSESSION_CLOSEAPP, posted to its one top-level window, ProductWindowClass.
     The product answers it with its one cleanup path, the same as "Exit" in its
     menu (FR-83): the hook goes, the tray icon goes, the configuration is saved,
     the process ends.  Posted, not sent: a program that hangs cannot hang the
     uninstaller.  Nothing is asked of the person.
  2. The uninstaller waits until the mutex is gone AND the program file can be
     opened for writing -- the image of a process that has just ended is released
     a moment after its mutex -- for up to LangSwCloseWaitMs.
  3. Only a program still there after that is left to the person: the request to
     exit it and click OK, in Inno's own words (UninstallAppRunningError, translated
     in every language of this installer).  Cancel aborts the uninstall before a
     single file, value or certificate is touched; /SUPPRESSMSGBOXES gets the
     default answer, Cancel, and a non-zero exit code.

  The mutex lives in the session namespace (Local\) and FindWindowByClassName looks
  at one desktop: a copy running in ANOTHER user's session is neither closed nor
  seen, and its file stays.  The [Setup] directive AppMutex is not used: it would
  stop Setup at startup too, and an update closes the product by itself
  (CloseApplications above). }
const
  LangSwEndSession = $0016;       { WM_ENDSESSION }
  LangSwCloseApp = $00000001;     { ENDSESSION_CLOSEAPP }
  LangSwOpenReadWrite = $0002;    { fmOpenReadWrite }
  LangSwShareExclusive = $0010;   { fmShareExclusive }
  LangSwCloseWaitMs = 10000;
  LangSwAfterOkWaitMs = 5000;
  LangSwPollMs = 250;

function ProgramFileIsFree(const FileName: String): Boolean;
var
  Stream: TFileStream;
begin
  Result := True;
  if not FileExists(FileName) then
    Exit;
  try
    Stream := TFileStream.Create(FileName, LangSwOpenReadWrite or LangSwShareExclusive);
    Stream.Free;
  except
    Result := False;
  end;
end;

function ProgramIsGone(const FileName: String; const TimeoutMs: Integer): Boolean;
var
  Waited: Integer;
begin
  Waited := 0;
  while (CheckForMutexes('{#ProductMutex}') or not ProgramFileIsFree(FileName)) and (Waited < TimeoutMs) do
  begin
    Sleep(LangSwPollMs);
    Waited := Waited + LangSwPollMs;
  end;
  Result := (not CheckForMutexes('{#ProductMutex}')) and ProgramFileIsFree(FileName);
  Log(Format('Lang Switcher: waited %d ms; mutex gone and file free: %s', [Waited, YesNo(Result)]));
end;

<event('InitializeUninstall')>
function InitializeUninstallProgramRunning(): Boolean;
var
  ExeFile: String;
  Wnd: HWND;
begin
  Result := True;
  if not CheckForMutexes('{#ProductMutex}') then
    Exit;
  ExeFile := ExpandConstant('{app}\{#AppExeName}');
  Wnd := FindWindowByClassName('{#ProductWindowClass}');
  if Wnd = 0 then
    Log('Lang Switcher: the program is running, but its window {#ProductWindowClass} is not found')
  else if PostMessage(Wnd, LangSwEndSession, 1, LangSwCloseApp) then
    Log('Lang Switcher: the program is running; asked to close as at sign-out (WM_ENDSESSION)')
  else
    Log('Lang Switcher: the program is running; WM_ENDSESSION could not be posted');
  if ProgramIsGone(ExeFile, LangSwCloseWaitMs) then
  begin
    Log('Lang Switcher: the program has closed by itself; the uninstall goes on');
    Exit;
  end;
  Log('Lang Switcher: the program did not close by itself; the person is asked');
  while CheckForMutexes('{#ProductMutex}') do
  begin
    if SuppressibleMsgBox(FmtMessage(SetupMessage(msgUninstallAppRunningError), ['{#AppName}']),
         mbError, MB_OKCANCEL, IDCANCEL) <> IDOK then
    begin
      Log('Lang Switcher: the uninstall is cancelled while the program runs; nothing was removed');
      Result := False;
      Exit;
    end;
  end;
  if not ProgramIsGone(ExeFile, LangSwAfterOkWaitMs) then
    Log('Lang Switcher: the program file is still busy; the uninstaller may have to leave it');
end;

{ Uninstall: the certificate this computer got from the helper leaves LocalMachine\Root
  before the files go.  It is named by its public part in the program folder -- the
  SHA-1 of a DER certificate is its thumbprint -- and removed by certutil, the tool
  every Windows has; the file goes only once the certificate is gone, so that a
  failure leaves the trace in place.  The author's certificate is never touched. }
<event('CurUninstallStepChanged')>
procedure CurUninstallLocalTrust(CurUninstallStep: TUninstallStep);
var
  CerFile, Thumb, CertUtil: String;
  Code: Integer;
begin
  if CurUninstallStep <> usUninstall then
    Exit;
  CerFile := ExpandConstant('{app}\{#LocalCertFile}');
  if not FileExists(CerFile) then
  begin
    Log('Lang Switcher: no local certificate on this computer (' + CerFile + ' is absent)');
    Exit;
  end;
  try
    Thumb := Uppercase(GetSHA1OfFile(CerFile));
  except
    Log('Lang Switcher: the local certificate file cannot be read: ' + GetExceptionMessage);
    Exit;
  end;
  if CompareText(Thumb, '{#AuthorThumbprint}') = 0 then
  begin
    Log('Lang Switcher: ' + CerFile + ' names the author''s certificate, which is never removed here');
    Exit;
  end;
  CertUtil := ExpandConstant('{sys}\certutil.exe');
  Log('Lang Switcher: removing the local certificate ' + Thumb + ' from LocalMachine\Root');
  if not ExecAndLogOutput(CertUtil, '-delstore Root ' + Thumb, '', SW_SHOWNORMAL, ewWaitUntilTerminated, Code, nil) then
  begin
    Log('Lang Switcher: certutil could not be started: ' + SysErrorMessage(Code) + '; the file is kept');
    Exit;
  end;
  Log('Lang Switcher: certutil -delstore returned ' + IntToStr(Code));
  if not ExecAndLogOutput(CertUtil, '-store Root ' + Thumb, '', SW_SHOWNORMAL, ewWaitUntilTerminated, Code, nil) then
  begin
    Log('Lang Switcher: certutil could not be started for the check; the file is kept');
    Exit;
  end;
  if Code = 0 then
    Log('Lang Switcher: the local certificate ' + Thumb + ' is STILL in LocalMachine\Root; the file is kept as its trace')
  else
  begin
    if DeleteFile(CerFile) then
      Log('Lang Switcher: the local certificate ' + Thumb + ' is gone from LocalMachine\Root; its file is deleted')
    else
      Log('Lang Switcher: the local certificate is gone, but ' + CerFile + ' could not be deleted');
  end;
end;

(* ---------------------------------------------------------------------------
  Uninstall: the files of the program in its folder of the profile (decisions
  177.1 and 177.20.1 of the owner, backlog Q-171-1, stage E101).  A comment of
  this kind and not in braces: the constants below carry braces.

  The journal goes ALWAYS -- it is not a setting, nobody is asked about it, and a
  silent uninstall removes it too -- and with it the temporaries of its write,
  JournalName.<pid>.tmp, which a write interrupted before its rename leaves
  behind and nothing else sweeps.  The settings, the kept copy of unreadable
  settings and the appeals saved from the wizard "Write to the author" go only
  on an explicit yes (CurUninstallStepChanged below), and then the folder goes
  if nothing is left in it.  Until e100 the yes removed ConfigName alone, and
  the folder stayed with the journal in it.

  An implementation with an event attribute: Inno calls it BEFORE the main
  CurUninstallStepChanged (the help, "Event Attributes"), so the journal is gone
  by the time the main one asks about the settings and removes the folder.
  --------------------------------------------------------------------------- *)
procedure DeleteDataFile(const FileName, What: String);
begin
  if not FileExists(FileName) then
    Log('Lang Switcher: ' + What + ': none (' + FileName + ')')
  else if DeleteFile(FileName) then
    Log('Lang Switcher: ' + What + ': deleted (' + FileName + ')')
  else
    Log('Lang Switcher: ' + What + ': could NOT be deleted (' + FileName + ')');
end;

procedure DeleteDataFiles(const Folder, Mask, What: String);
begin
  if DelTree(Folder + '\' + Mask, False, True, False) then
    Log('Lang Switcher: ' + What + ' (' + Mask + '): every match deleted, if there was any')
  else
    Log('Lang Switcher: ' + What + ' (' + Mask + '): NOT every match could be deleted');
end;

<event('CurUninstallStepChanged')>
procedure CurUninstallJournal(CurUninstallStep: TUninstallStep);
var
  Folder: String;
begin
  if CurUninstallStep <> usPostUninstall then
    Exit;
  Folder := ExpandConstant('{userappdata}\{#DataFolder}');
  DeleteDataFile(Folder + '\{#JournalName}', 'the journal');
  DeleteDataFiles(Folder, '{#JournalName}.*.tmp', 'the temporaries of the journal');
end;

{ ---------------------------------------------------------------------------
  Uninstall: leave nothing behind EXCEPT the user's settings.

  1. The autostart value goes unconditionally.  Its one writer is the product
     itself (decision 120.4): every start makes HKCU\...\Run agree with the
     configuration, and the settings dialog and the tray menu change it, so this
     script has no [Registry] entry of its own to undo and this call is the only
     thing that removes the value.  Without it, uninstalling would leave a Run
     value pointing at a deleted file.  RegDeleteValue on a missing value is not
     an error.  An uninstaller elevated as another account -- or through a
     scheduled task -- sees another HKCU and leaves the person's value alone;
     unticking autostart in the product before uninstalling removes it then.

  2. The settings in %APPDATA%\Lang_Switcher -- config.toml, the kept copy of
     an unreadable one, config.toml.bad, and the appeals saved from the wizard
     "Write to the author" -- are DELETED ONLY ON AN EXPLICIT YES, AND ONLY WHEN
     A HUMAN CAN ANSWER (decision 177.20.1 adds the copy and the appeals).
     Section 8.4 says "removed on confirmation"; /VERYSILENT /SUPPRESSMSGBOXES
     cannot obtain confirmation, and the absence of an answer is not consent.
     Two independent guards, on purpose:
       - UninstallSilent returns True for /SILENT and /VERYSILENT, and the question
         is not even asked;
       - SuppressibleMsgBox returns its DEFAULT answer, IDNO here, whenever
         /SUPPRESSMSGBOXES is in force.
     Either guard alone keeps the files.  It takes a human clicking Yes to remove
     them, which is what section 8.4 asked for.  The question is asked when
     either settings file is there; the field shows the one that is.

  3. Then the folder goes, if nothing is left in it: the journal is already gone
     (CurUninstallJournal above), so after a yes, or with no settings at all,
     nothing of the program stays in the profile.
  --------------------------------------------------------------------------- }
procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
var
  Folder, ConfigFile, BadFile, Shown: String;
#ifdef DIAG
  Probe: String;
#endif
begin
  if CurUninstallStep = usPostUninstall then
  begin
    RegDeleteValue(HKEY_CURRENT_USER, '{#RunKey}', '{#RunValue}');
    Log('Autostart value removed from HKCU (absent is not an error).');
#ifdef DIAG
    if RegQueryStringValue(HKEY_CURRENT_USER, '{#RunKey}', '{#RunValue}', Probe) then
      Log('DIAG post-uninstall HKCU = *** STILL THERE: ' + Probe + ' ***')
    else
      Log('DIAG post-uninstall HKCU = gone');
#endif

    Folder := ExpandConstant('{userappdata}\{#DataFolder}');
    ConfigFile := Folder + '\{#ConfigName}';
    BadFile := ConfigFile + '{#BadSuffix}';
    if FileExists(ConfigFile) then
      Shown := ConfigFile
    else
      Shown := BadFile;
    if not (FileExists(ConfigFile) or FileExists(BadFile)) then
      Log('No settings file present, nothing to ask about: ' + ConfigFile)
    else if UninstallSilent() then
      Log('Silent uninstall: the settings are KEPT. Silence is not consent.')
    else if SuppressibleMsgBox(
         CustomMessage('RemoveSettingsQuestion') + #13#10 + #13#10 +
         Shown + #13#10 + #13#10 +
         CustomMessage('RemoveSettingsKeep'),
         mbConfirmation, MB_YESNO, IDNO) = IDYES then
    begin
      DeleteDataFile(ConfigFile, 'the settings');
      DeleteDataFile(BadFile, 'the kept copy of unreadable settings');
      DeleteDataFiles(Folder, '{#AppealStem}-*.txt', 'the saved appeals');
      DeleteDataFiles(Folder, {#AppealStemRu} + '-*.txt', 'the saved appeals in Russian');
      Log('The user confirmed: the settings and the saved appeals were deleted.');
    end
    else
      Log('The user declined (or msgboxes are suppressed): the settings and the saved appeals are KEPT.');

    if not DirExists(Folder) then
      Log('Lang Switcher: no folder ' + Folder)
    else if RemoveDir(Folder) then
      Log('Lang Switcher: the folder ' + Folder + ' was empty and is removed')
    else
      Log('Lang Switcher: the folder ' + Folder + ' is kept: something is left in it');
  end;
end;

(* ---------------------------------------------------------------------------
  The updater (stage E104, question 189; TZ E104 p. 13-18).

  After the files:
    1. the folders %ProgramData%\Lang_Switcher and its update -- owner Administrators,
       inheritance cut, full control for SYSTEM and Administrators and for nobody else
       (premise P10); what was in them takes the list of the folder.  icacls of System32,
       by its full path: %ProgramData% lets every user create files below it, and a folder a
       person made there before this Setup ran would otherwise stay that person's;
    2. the copy of this very installer, the srcexe constant, under installed-setup.exe in
       that folder -- what a rollback installs (TZ E104 p. 17, premise P9);
    3. the task "Lang Switcher Update" of the scheduler, from an XML of this script:
       the system account, once a day and at the sign-in of any user five minutes late,
       only when the computer has been idle ten minutes, a missed start at the first chance,
       thirty minutes at most, no second instance (TZ E104 p. 14).  schtasks of System32.
  At the very end of a Setup started with /UPDATE -- phase one of the updater starts it so --
  the updater just installed is started with /phase2 (TZ E104 p. 18).  In DeinitializeSetup
  and not in [Run], on purpose: [Run] runs only after a successful installation, and phase two
  is just as needed after a failed one -- it raises the program the update closed.  It waits
  for SetupMutex to go before it does anything.
  Before the files of a Setup started by a person, a running instance of the task is ended:
  the updater must not hold its file while it is replaced.
  The uninstaller ends and deletes the task before the files; [UninstallDelete] takes the
  folder.  Every step writes one line in the log of Setup; none of them stops it.
  --------------------------------------------------------------------------- *)
const
  LangSwUpdateTask = '{#UpdateTask}';

function UpdateFolder(): String;
begin
  Result := ExpandConstant('{commonappdata}\{#UpdateDir}');
end;

function RunLogged(const Exe, Params: String): Integer;
var
  Code: Integer;
begin
  Code := -1;
  Log('Lang Switcher: running "' + Exe + '" ' + Params);
  try
    if not ExecAndLogOutput(Exe, Params, '', SW_HIDE, ewWaitUntilTerminated, Code, nil) then
    begin
      Log('Lang Switcher: could not start it: ' + SysErrorMessage(Code));
      Code := -1;
    end;
  except
    Log('Lang Switcher: it failed to run: ' + GetExceptionMessage);
    Code := -1;
  end;
  Log('Lang Switcher: it returned ' + IntToStr(Code));
  Result := Code;
end;

procedure SecureUpdateFolders();
var
  Icacls, Data: String;
begin
  Icacls := ExpandConstant('{sys}\icacls.exe');
  Data := ExpandConstant('{commonappdata}\Lang_Switcher');
  RunLogged(Icacls, '"' + Data + '" /setowner *S-1-5-32-544 /T /C /Q');
  RunLogged(Icacls, '"' + Data + '" /inheritance:r /grant:r *S-1-5-18:(OI)(CI)F *S-1-5-32-544:(OI)(CI)F /C /Q');
  RunLogged(Icacls, '"' + Data + '\*" /reset /T /C /Q');
end;

procedure CopySetupForRollback();
var
  Source, Target: String;
begin
  Source := ExpandConstant('{srcexe}');
  Target := UpdateFolder() + '\{#InstalledSetup}';
  if CompareText(Source, Target) = 0 then
    Log('Lang Switcher: this Setup is its own kept copy; nothing to copy')
  else if CopyFile(Source, Target, False) then
    Log('Lang Switcher: the copy of this Setup is kept for a rollback: ' + Target)
  else
    Log('Lang Switcher: the copy of this Setup could not be kept: ' + SysErrorMessage(DLLGetLastError()));
end;

function UpdateTaskXml(): String;
var
  N: String;
begin
  N := #13#10;
  (* NO XML declaration, on purpose (finding of round 1 of T-104-9 on Test_VM): schtasks reads
     the file into a wide string before it parses it, and a declaration of UTF-8 there is refused
     with "(1,40) unable to switch the encoding" -- the task was not registered at all.  The text
     is ASCII, so it needs no declaration. *)
  Result :=
    '<Task version="1.4" xmlns="http://schemas.microsoft.com/windows/2004/02/mit/task">' + N +
    '  <RegistrationInfo>' + N +
    '    <Author>{#AppPublisher}</Author>' + N +
    '    <Description>Lang Switcher: reads the author''s feed once a day and installs a new version while the computer is idle.</Description>' + N +
    '  </RegistrationInfo>' + N +
    '  <Triggers>' + N +
    '    <CalendarTrigger>' + N +
    '      <StartBoundary>2026-01-01T12:00:00</StartBoundary>' + N +
    '      <Enabled>true</Enabled>' + N +
    '      <ScheduleByDay><DaysInterval>1</DaysInterval></ScheduleByDay>' + N +
    '    </CalendarTrigger>' + N +
    '    <LogonTrigger>' + N +
    '      <Enabled>true</Enabled>' + N +
    '      <Delay>PT5M</Delay>' + N +
    '    </LogonTrigger>' + N +
    '  </Triggers>' + N +
    '  <Principals>' + N +
    '    <Principal id="System">' + N +
    '      <UserId>S-1-5-18</UserId>' + N +
    '      <RunLevel>HighestAvailable</RunLevel>' + N +
    '    </Principal>' + N +
    '  </Principals>' + N +
    '  <Settings>' + N +
    '    <MultipleInstancesPolicy>IgnoreNew</MultipleInstancesPolicy>' + N +
    '    <DisallowStartIfOnBatteries>false</DisallowStartIfOnBatteries>' + N +
    '    <StopIfGoingOnBatteries>false</StopIfGoingOnBatteries>' + N +
    '    <AllowHardTerminate>true</AllowHardTerminate>' + N +
    '    <StartWhenAvailable>true</StartWhenAvailable>' + N +
    '    <RunOnlyIfNetworkAvailable>false</RunOnlyIfNetworkAvailable>' + N +
    '    <IdleSettings>' + N +
    '      <Duration>PT10M</Duration>' + N +
    '      <WaitTimeout>PT23H</WaitTimeout>' + N +
    '      <StopOnIdleEnd>false</StopOnIdleEnd>' + N +
    '      <RestartOnIdle>false</RestartOnIdle>' + N +
    '    </IdleSettings>' + N +
    '    <AllowStartOnDemand>true</AllowStartOnDemand>' + N +
    '    <Enabled>true</Enabled>' + N +
    '    <Hidden>false</Hidden>' + N +
    '    <RunOnlyIfIdle>true</RunOnlyIfIdle>' + N +
    '    <WakeToRun>false</WakeToRun>' + N +
    '    <ExecutionTimeLimit>PT30M</ExecutionTimeLimit>' + N +
    '    <Priority>7</Priority>' + N +
    '  </Settings>' + N +
    '  <Actions Context="System">' + N +
    '    <Exec>' + N +
    '      <Command>"' + ExpandConstant('{app}\{#UpdaterExeName}') + '"</Command>' + N +
    '    </Exec>' + N +
    '  </Actions>' + N +
    '</Task>' + N;
end;

procedure CreateUpdateTask();
var
  Xml: String;
begin
  Xml := ExpandConstant('{tmp}\langsw-update-task.xml');
  if not SaveStringToFile(Xml, UpdateTaskXml(), False) then
  begin
    Log('Lang Switcher: the XML of the task could not be written: ' + Xml);
    Exit;
  end;
  if RunLogged(ExpandConstant('{sys}\schtasks.exe'), '/Create /TN "' + LangSwUpdateTask + '" /XML "' + Xml + '" /F') = 0 then
    Log('Lang Switcher: the task "' + LangSwUpdateTask + '" is registered')
  else
    Log('Lang Switcher: the task "' + LangSwUpdateTask + '" could NOT be registered');
end;

<event('CurStepChanged')>
procedure CurStepUpdater(CurStep: TSetupStep);
begin
  if CurStep <> ssPostInstall then
    Exit;
  SecureUpdateFolders();
  CopySetupForRollback();
  CreateUpdateTask();
end;

<event('PrepareToInstall')>
function PrepareToInstallUpdater(var NeedsRestart: Boolean): String;
begin
  Result := '';
  if not HasSwitch('/UPDATE') then
    RunLogged(ExpandConstant('{sys}\schtasks.exe'), '/End /TN "' + LangSwUpdateTask + '"');
end;

<event('DeinitializeSetup')>
procedure DeinitializeUpdater();
var
  Updater: String;
  Code: Integer;
begin
  if not HasSwitch('/UPDATE') then
    Exit;
  try
    Updater := ExpandConstant('{app}\{#UpdaterExeName}');
  except
    Updater := ExpandConstant('{commonpf64}\Lang_Switcher\{#UpdaterExeName}');
  end;
  if Exec(Updater, '{#PhaseTwo}', '', SW_HIDE, ewNoWait, Code) then
    Log('Lang Switcher: phase two of the updater is started')
  else
    Log('Lang Switcher: phase two of the updater could not be started: ' + SysErrorMessage(Code));
end;

<event('CurUninstallStepChanged')>
procedure CurUninstallUpdater(CurUninstallStep: TUninstallStep);
var
  SchTasks: String;
begin
  if CurUninstallStep <> usUninstall then
    Exit;
  SchTasks := ExpandConstant('{sys}\schtasks.exe');
  RunLogged(SchTasks, '/End /TN "' + LangSwUpdateTask + '"');
  if RunLogged(SchTasks, '/Delete /TN "' + LangSwUpdateTask + '" /F') = 0 then
    Log('Lang Switcher: the task "' + LangSwUpdateTask + '" is deleted')
  else
    Log('Lang Switcher: the task "' + LangSwUpdateTask + '" could not be deleted (absent is not an error)');
end;
