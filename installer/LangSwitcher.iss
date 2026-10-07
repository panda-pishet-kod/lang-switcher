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

#define AppName        "Lang Switcher"
; The name with the platform, where the name stands alone and could be taken for a namesake
; (decision 149.2, backlog Q-149-1, done in stage E90 by decision 153.4): the wizard, the
; properties of the installer file, the Start menu folder and the entry in Apps & features.
; AppName itself stays the short name of the PROGRAM: the shortcuts are named by it -- and the
; heading of the program's notifications is the name of its shortcut -- and so is the autostart
; value below.  AppId is not touched, so an update is still the same application.
#define AppTitle       "Lang Switcher for Windows"
#define AppVersion     "0.95.0"
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

[Setup]
; AppId is the identity Windows matches an upgrade against.  It is a constant and
; must never change: a new AppId turns every future update into a second parallel
; installation with its own uninstall entry.
AppId={{1A22BDD8-916A-490E-8269-66869A611388}
AppName={#AppTitle}
AppVersion={#AppVersion}
AppVerName={#AppTitle} {#AppVersion}
AppPublisher={#AppPublisher}
AppCopyright={#AppCopyright}
VersionInfoVersion=0.95.0.0
VersionInfoCompany={#AppPublisher}
VersionInfoProductName={#AppTitle}
VersionInfoProductVersion=0.95.0.0
VersionInfoDescription={#AppTitle} Setup
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
DefaultGroupName={#AppTitle}
; Inno Setup would otherwise reuse the Start menu folder of the installed version on an update,
; and the folder "Lang Switcher" of every installation before e90 would never get the new name.
; The old folder itself is removed by [InstallDelete] below.
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

Uninstallable=yes
UninstallDisplayName={#AppTitle}
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
WizardStyle=modern
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
Source: "local-trust.ps1"; Flags: dontcopy

[InstallDelete]
; The Start menu folder of the versions before e90 was named "Lang Switcher" (the short name);
; since e90 it is "Lang Switcher for Windows".  Without this line an update would leave the old
; folder and its two shortcuts behind, beside the new ones.  {autoprograms} is the all-users Start
; menu here, because Setup runs elevated (PrivilegesRequired=admin).
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
; does not match and stays; config.toml itself goes only on an explicit yes (the [Code] below).
; {userappdata} is the profile of the account the uninstaller runs as -- the same caveat as the
; HKCU value in CurUninstallStepChanged.
Type: files; Name: "{userappdata}\Lang_Switcher\config.toml.*.tmp"

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
    if SuppressibleMsgBox(FmtMessage(SetupMessage(msgUninstallAppRunningError), ['{#AppTitle}']),
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

{ ---------------------------------------------------------------------------
  Uninstall: leave nothing behind EXCEPT the user's configuration.

  1. The autostart value goes unconditionally.  Its one writer is the product
     itself (decision 120.4): every start makes HKCU\...\Run agree with the
     configuration, and the settings dialog and the tray menu change it, so this
     script has no [Registry] entry of its own to undo and this call is the only
     thing that removes the value.  Without it, uninstalling would leave a Run
     value pointing at a deleted file.  RegDeleteValue on a missing value is not
     an error.  An uninstaller elevated as another account -- or through a
     scheduled task -- sees another HKCU and leaves the person's value alone;
     unticking autostart in the product before uninstalling removes it then.

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
             CustomMessage('RemoveSettingsQuestion') + #13#10 + #13#10 +
             ConfigFile + #13#10 + #13#10 +
             CustomMessage('RemoveSettingsKeep'),
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
