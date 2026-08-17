; ============================================================================
;  Lang Switcher -- Inno Setup script.  SPEC.md section 8.4, task T-09-2.
;
;  WHAT THIS SCRIPT PACKAGES
;
;    <dev>\artifacts\LangSwitcher.exe -- the ALREADY SIGNED shipping build
;    produced by tools\release.ps1.  Not a freshly compiled binary.  Section 8.2
;    of SPEC.md: a uiAccess="true" image needs BOTH an Authenticode signature the
;    machine trusts AND a location under %ProgramFiles%.  Packaging an unsigned
;    binary produces an installer whose product cannot start, and the defect then
;    looks like an installer defect, which it is not.
;
;  WHY THIS FILE IS PURE ASCII
;
;    Inno Setup 6 reads an .iss without a byte order mark in the system ANSI code
;    page.  The tooling that writes files here does not emit a BOM (decision R-05,
;    the same trap that governs .ps1 files).  Cyrillic in this file would therefore
;    be mangled at compile time in a way that is invisible until a user sees it.
;    Consequence, stated plainly: the installer user interface is English only,
;    while the product itself is bilingual (FR-94).  Section 8.4 asks for no
;    installer localisation, so this is a limitation, not an unmet requirement.
;
;  WHY THE INSTALL DIRECTORY IS NOT A CHOICE
;
;    The scheduled task LangSw-Uninstall is hardwired to
;    C:\Program Files\Lang_Switcher\unins000.exe and cannot be edited from inside
;    this project.  The destination is therefore fixed from outside, the directory
;    page is disabled, and the uninstaller keeps Inno's ordinary unins000.exe name.
; ============================================================================

; --- Compile-time knob, and the only one --------------------------------------
;
; AUTOSTART_DEFAULT_ON selects whether the optional autostart task starts out
; ticked.  It exists for one reason: FR-93 has to be measured in BOTH states, and
; on this machine elevation is available only through `schtasks /run`, whose
; command line is hardwired to /VERYSILENT /NORESTART /SUPPRESSMSGBOXES and cannot
; carry /TASKS=.  A silent install therefore always takes the compiled-in default,
; so the only way to exercise the other branch is to compile it.
;
; The shipping default is OFF.  A silent install must not register autostart that
; nobody asked for -- section 8.4 says "by the user's choice", and a choice that
; is made by default is not a choice.
#ifndef AUTOSTART_DEFAULT_ON
  #define AUTOSTART_DEFAULT_ON 0
#endif

#define AppName        "Lang Switcher"
#define AppVersion     "0.1.0"
#define AppPublisher   "Panda_Pishet_Kod"
#define AppExeName     "LangSwitcher.exe"
#define AppCopyright   "Copyright (C) 2026 Panda_Pishet_Kod"

; The signed shipping artifact.  Overridable with ISCC /DSourceExe=... so that a
; dry run can point at something else; the default is what release.ps1 writes.
#ifndef SourceExe
  #define SourceExe "<dev>\artifacts\LangSwitcher.exe"
#endif

; res\langswitcher-active.ico -- the tray icon of the running product, reused for
; the installer, the Start menu shortcut and the entry in Apps & features.
#define IconFile "..\res\langswitcher-active.ico"

; The name and the key FR-93 uses.  BOTH MUST MATCH src\settings.rs LITERALLY:
;   RUN_KEY_PATH        = r"Software\Microsoft\Windows\CurrentVersion\Run"
;   AUTOSTART_VALUE_NAME = "Lang Switcher"
; If the installer wrote a different value name, the settings dialog of FR-92
; would read one key while the installer wrote another, the tick would disagree
; with reality, and turning autostart off in the product would leave the
; installer's value behind forever.
#define RunKey     "Software\Microsoft\Windows\CurrentVersion\Run"
#define RunValue   "Lang Switcher"

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
VersionInfoVersion=0.1.0.0
VersionInfoCompany={#AppPublisher}
VersionInfoProductName={#AppName}
VersionInfoProductVersion=0.1.0.0
VersionInfoDescription={#AppName} Setup
VersionInfoCopyright={#AppCopyright}

; Section 8.4: install into %ProgramFiles% with a single elevation.  admin asks
; for it once, at the start, and nothing later needs to ask again.
PrivilegesRequired=admin
PrivilegesRequiredOverridesAllowed=

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
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
; The installer is signed afterwards by tools\build-installer.ps1, not by an Inno
; SignTool= directive: the private key is non-exportable and lives in
; Cert:\CurrentUser\My, so signing happens in the same user session, once, over
; the finished file (TOOLCHAIN.md section 5).

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
; A separate Inno task, so that FR-93 is a tick the user can see and clear.
#if AUTOSTART_DEFAULT_ON
Name: "autostart"; Description: "Start {#AppName} automatically when I sign in"; GroupDescription: "Startup:"
#else
Name: "autostart"; Description: "Start {#AppName} automatically when I sign in"; GroupDescription: "Startup:"; Flags: unchecked
#endif

[Files]
; ignoreversion: the product is replaced whenever this installer runs, including a
; reinstall of the identical version.  Version-number comparison would silently
; skip the copy and leave a stale binary that still passes every file listing.
Source: "{#SourceExe}"; DestDir: "{app}"; DestName: "{#AppExeName}"; Flags: ignoreversion

[Icons]
Name: "{group}\{#AppName}"; Filename: "{app}\{#AppExeName}"; IconFilename: "{app}\{#AppExeName}"
Name: "{group}\Uninstall {#AppName}"; Filename: "{uninstallexe}"

[Registry]
; FR-93.  The command is the quoted full path, byte for byte the shape
; settings.rs::autostart_command builds -- format!("\"{}\"", exe.display()) -- so
; that the value the installer writes and the value the product writes are the
; same string and neither side sees the other's as foreign.
; uninsdeletevalue removes it again on uninstall.  It only covers the value THIS
; installer wrote; a value the product wrote for itself is handled unconditionally
; in CurUninstallStepChanged below.
Root: HKCU; Subkey: "{#RunKey}"; ValueType: string; ValueName: "{#RunValue}"; \
    ValueData: """{app}\{#AppExeName}"""; Flags: uninsdeletevalue; Tasks: autostart

[Run]
; skipifsilent is the whole point of this entry being safe: a silent install --
; which is how the scheduled task runs it -- never starts the product.
; runasoriginaluser matters for section 8.1: Setup is elevated, and a child of an
; elevated process inherits high integrity.  The product is meant to run at MEDIUM
; integrity with uiAccess, not as an administrator, so it is launched back as the
; user who started Setup.
Filename: "{app}\{#AppExeName}"; Description: "Start {#AppName} now"; \
    Flags: nowait postinstall skipifsilent runasoriginaluser

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

{ ---------------------------------------------------------------------------
  Uninstall: leave nothing behind EXCEPT the user's configuration.

  1. The autostart value goes unconditionally.  The [Registry] entry above only
     records an uninstall action when the autostart task was ticked at install
     time, but the product writes the very same value itself from the settings
     dialog (FR-92 / FR-93).  Without this, uninstalling a product whose autostart
     was switched on from inside the program would leave a Run value pointing at a
     deleted file.  RegDeleteValue on a missing value is not an error.

  2. %APPDATA%\Lang_Switcher\config.toml is DELETED ONLY ON AN EXPLICIT YES, AND
     ONLY WHEN A HUMAN CAN ANSWER.  Section 8.4 says "removed on confirmation";
     /VERYSILENT /SUPPRESSMSGBOXES cannot obtain confirmation, and the absence of
     an answer is not consent.  Two independent guards, on purpose:
       - UninstallSilent returns True for /SILENT and /VERYSILENT, and the question
         is not even asked;
       - SuppressibleMsgBox returns its DEFAULT answer, IDNO here, whenever
         /SUPPRESSMSGBOXES is in force.
     Either guard alone keeps the file.  It takes a human clicking Yes to remove
     it, which is what section 8.4 asked for.
  --------------------------------------------------------------------------- }
procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
var
  ConfigFile: String;
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

    ConfigFile := ExpandConstant('{userappdata}\Lang_Switcher\config.toml');
    if not FileExists(ConfigFile) then
      Log('Configuration file not present, nothing to ask about: ' + ConfigFile)
    else
    begin
      if UninstallSilent() then
        Log('Silent uninstall: the configuration file is KEPT. Silence is not consent.')
      else
      begin
        if SuppressibleMsgBox(
             'Remove the Lang Switcher settings file as well?' + #13#10 + #13#10 +
             ConfigFile + #13#10 + #13#10 +
             'Choose No to keep your settings for a later reinstall.',
             mbConfirmation, MB_YESNO, IDNO) = IDYES then
        begin
          DeleteFile(ConfigFile);
          RemoveDir(ExpandConstant('{userappdata}\Lang_Switcher'));
          Log('The user confirmed: the configuration file was deleted.');
        end
        else
          Log('The user declined (or msgboxes are suppressed): the configuration file is KEPT.');
      end;
    end;
  end;
end;
