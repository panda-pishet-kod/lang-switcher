<#
    build-installer.ps1 -- build the product, compile the Inno Setup installer,
    sign the installer, verify the signature, report the path and the SHA-256.

    THE ORDER MATTERS, FOR THE SAME REASON IT MATTERS IN release.ps1.

        tools\release.ps1                    (build, criterion 8, sign the product)
          -> ISCC installer\LangSwitcher.iss (package the SIGNED product)
            -> copy to <dev>\artifacts\LangSwitcher-setup.exe
              -> sign the copy
                -> signtool verify /pa

    WHY release.ps1 RUNS FIRST AND NOT "USE WHATEVER IS ON DISK". Section 8.2 of
    SPEC.md: an image with uiAccess="true" starts only when it is BOTH signed with
    a certificate the machine trusts AND located under %ProgramFiles%. Package an
    unsigned or stale product and the installed program fails to start with
    CreateProcess error 740, which looks exactly like an installer defect and is
    not one. release.ps1 also re-proves acceptance criterion 8 of section 13, so
    the packaged build is the one without the `testing` feature.

    Skip it with -SkipProductBuild only when <dev>\artifacts\LangSwitcher.exe is
    already known good; the script still refuses to run if that file is missing.

    THE SIGNATURE. Certificate CN=Panda_Pishet_Kod, thumbprint below, private key
    in Cert:\CurrentUser\My and NOT exportable. The command needs no administrator
    rights (TOOLCHAIN.md section 5). If the RFC3161 timestamp service cannot be
    reached the script retries without a countersignature, says so loudly, and
    reports it in the summary. That fallback is a DEVIATION and belongs in the
    report: without a countersignature the signature stops verifying the day the
    certificate expires.

    -AutostartDefaultOn compiles the installer with the optional autostart task
    pre-ticked. It exists to measure FR-93 in both states: elevation on this
    machine is available only through `schtasks /run`, whose command line is
    hardwired and cannot carry /TASKS=, so a silent install always takes the
    compiled-in default. THE SHIPPING DEFAULT IS OFF; do not pass this switch for
    a build that is meant to be kept.

    Written without a single Cyrillic character on purpose: the tooling writes
    files in UTF-8 with NO byte order mark, Windows PowerShell 5.1 reads such a
    .ps1 in the system ANSI code page, and the parse dies on the first Russian
    word. Decision R-05.
    Windows PowerShell 5.1, not 7.x: no `&&`, no `??`, no ternary operator.

    Exit codes -- non-zero on every refusal:
      0  everything below succeeded
      1  tools\release.ps1 failed
      2  the signed product artifact is missing
      3  installer\LangSwitcher.iss is missing
      4  ISCC failed to compile the script
      5  ISCC reported success but produced no file
      6  the artifact directory does not exist
      7  the copy does not match the compiled installer
      8  signing failed, even without a timestamp
      9  signtool verify /pa failed

    Examples:
      .\build-installer.ps1
      .\build-installer.ps1 -SkipProductBuild
      .\build-installer.ps1 -AutostartDefaultOn      # measurement build only
#>
[CmdletBinding()]
param(
    # SHA-1 thumbprint of the code signing certificate. TOOLCHAIN.md section 5.
    [string]$Thumbprint = '8F038C7D00DACFCE34EC742EC97CCEBFD8465CAC',
    # The signed product the installer packages. Produced by tools\release.ps1.
    [string]$ProductArtifact = '<dev>\artifacts\LangSwitcher.exe',
    # Where the finished installer goes. This exact path is hardwired into the
    # LangSw-Install scheduled task and cannot be renamed.
    [string]$SetupArtifact = '<dev>\artifacts\LangSwitcher-setup.exe',
    [string]$TimestampUrl = 'http://timestamp.digicert.com',
    [string]$Iscc = '<dev>\tools\InnoSetup\ISCC.exe',
    # Do not rebuild and re-sign the product first. See the header.
    [switch]$SkipProductBuild,
    # Compile with the autostart task pre-ticked. Measurement builds only.
    [switch]$AutostartDefaultOn,
    # Compile and copy, but do not sign. For a dry run of everything before the
    # signature.
    [switch]$SkipSign
)

$ErrorActionPreference = 'Stop'

$ScriptDir  = $PSScriptRoot
$ProjectDir = Split-Path -Parent $ScriptDir
$IssFile    = Join-Path $ProjectDir 'installer\LangSwitcher.iss'
$IssOutput  = Join-Path $ProjectDir 'installer\Output\LangSwitcher-setup.exe'

# --- Environment ----------------------------------------------------------------------------
# Only what is not already set, so a caller with an isolated target directory keeps it.
if (-not $env:CARGO_HOME)       { $env:CARGO_HOME       = '<dev>\tools\cargo' }
if (-not $env:RUSTUP_HOME)      { $env:RUSTUP_HOME      = '<dev>\tools\rustup' }
if (-not $env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR = '<dev>\cache\target' }
$SdkBin  = 'C:\Program Files (x86)\Windows Kits\10\bin\10.0.26100.0\x64'
$InnoBin = '<dev>\tools\InnoSetup'
if ($env:PATH -notlike '*<dev>\tools\cargo\bin*') { $env:PATH = "<dev>\tools\cargo\bin;$env:PATH" }
if ($env:PATH -notlike "*$SdkBin*")                { $env:PATH = "$SdkBin;$env:PATH" }
if ($env:PATH -notlike "*$InnoBin*")               { $env:PATH = "$InnoBin;$env:PATH" }

function Write-Section { param([string]$Text)
    Write-Host ''
    Write-Host ('--- ' + $Text + ' ' + ('-' * [Math]::Max(0, 66 - $Text.Length)))
}

function Show-Binary { param([string]$Path, [string]$Label)
    $item = Get-Item $Path
    $hash = (Get-FileHash -Algorithm SHA256 -Path $Path).Hash
    Write-Host ("  {0}" -f $Label)
    Write-Host ("    path    {0}" -f $item.FullName)
    Write-Host ("    size    {0} bytes" -f $item.Length)
    Write-Host ("    SHA-256 {0}" -f $hash)
    return $hash
}

Write-Host ''
Write-Host '======================================================================'
Write-Host ' Lang_Switcher -- Inno Setup installer, build and signature'
Write-Host '======================================================================'
Write-Host ("Project    {0}" -f $ProjectDir)
Write-Host ("Script     {0}" -f $IssFile)
Write-Host ("Product    {0}" -f $ProductArtifact)
Write-Host ("Installer  {0}" -f $SetupArtifact)
Write-Host ("Thumbprint {0}" -f $Thumbprint)
if ($AutostartDefaultOn) {
    Write-Host 'Autostart  DEFAULT ON -- MEASUREMENT BUILD, NOT FOR KEEPING'
} else {
    Write-Host 'Autostart  default off (shipping default)'
}
if ($SkipProductBuild) { Write-Host 'Product    NOT rebuilt, -SkipProductBuild was given' }
if ($SkipSign)         { Write-Host 'Signing    SKIPPED by -SkipSign' }

# --- 1. The product ---------------------------------------------------------------------------
Write-Section 'Step 1: the signed product'
if ($SkipProductBuild) {
    Write-Host '  skipped by -SkipProductBuild'
} else {
    & (Join-Path $ScriptDir 'release.ps1')
    $releaseCode = $LASTEXITCODE
    if ($releaseCode -ne 0) {
        Write-Host ''
        Write-Host ("FAIL: tools\release.ps1 returned {0}. Nothing was packaged." -f $releaseCode)
        exit 1
    }
}
if (-not (Test-Path $ProductArtifact)) {
    Write-Host ("FAIL: the signed product is missing: {0}" -f $ProductArtifact)
    exit 2
}
$productHash = Show-Binary -Path $ProductArtifact -Label 'product to be packaged:'

# The packaged product must already be signed, or the installed copy will not run
# from %ProgramFiles% (section 8.2). Checked, not assumed.
Write-Host '  signtool verify /pa <product>'
& signtool verify /pa $ProductArtifact
if ($LASTEXITCODE -ne 0) {
    Write-Host ''
    Write-Host '  WARNING: the product does not carry a valid signature.'
    Write-Host '  The installed program will fail to start with CreateProcess error 740.'
    Write-Host '  Continuing, because a deliberately unsigned dry run is a legitimate use.'
}

# --- 2. Compile the script ----------------------------------------------------------------------
Write-Section 'Step 2: compile the Inno Setup script'
if (-not (Test-Path $IssFile)) {
    Write-Host ("FAIL: the script is missing: {0}" -f $IssFile)
    exit 3
}
if (Test-Path $IssOutput) { Remove-Item -Path $IssOutput -Force }

$isccArgs = @()
if ($AutostartDefaultOn) { $isccArgs += '/DAUTOSTART_DEFAULT_ON=1' }
$isccArgs += ('/DSourceExe=' + $ProductArtifact)
$isccArgs += $IssFile

Write-Host ("  {0} {1}" -f $Iscc, ($isccArgs -join ' '))
& $Iscc $isccArgs
$isccCode = $LASTEXITCODE
if ($isccCode -ne 0) {
    Write-Host ("FAIL: ISCC returned {0}" -f $isccCode)
    exit 4
}
if (-not (Test-Path $IssOutput)) {
    Write-Host ("FAIL: ISCC succeeded but produced no file at {0}" -f $IssOutput)
    exit 5
}
$compiledHash = Show-Binary -Path $IssOutput -Label 'compiled installer:'

# --- 3. Copy to the artifact directory ------------------------------------------------------------
# The path below is hardwired into the LangSw-Install scheduled task. This script
# touches this one file and nothing else in that directory: the rest belongs to
# the setup stage.
Write-Section 'Step 3: copy to the artifact directory'
$artifactDir = Split-Path -Parent $SetupArtifact
if (-not (Test-Path $artifactDir)) {
    Write-Host ("FAIL: the artifact directory does not exist: {0}" -f $artifactDir)
    exit 6
}
Copy-Item -Path $IssOutput -Destination $SetupArtifact -Force
$copyHash = (Get-FileHash -Algorithm SHA256 -Path $SetupArtifact).Hash
if ($compiledHash -ne $copyHash) {
    Write-Host 'FAIL: the copy does not match the compiled installer.'
    exit 7
}
Write-Host '  the copy is byte for byte what ISCC produced'

# --- 4. Sign --------------------------------------------------------------------------------------
$timestamped = $false
if ($SkipSign) {
    Write-Section 'Step 4: signature -- SKIPPED'
    Write-Host '  -SkipSign was given. The installer is UNSIGNED.'
} else {
    Write-Section 'Step 4: sign the installer'
    Write-Host ("  signtool sign /sha1 {0} /fd SHA256 /td SHA256 /tr {1} <installer>" -f $Thumbprint, $TimestampUrl)
    & signtool sign /sha1 $Thumbprint /fd SHA256 /td SHA256 /tr $TimestampUrl $SetupArtifact
    $signCode = $LASTEXITCODE

    if ($signCode -eq 0) {
        $timestamped = $true
    } else {
        Write-Host ''
        Write-Host ("  signtool returned {0}. The usual cause is that the timestamp service" -f $signCode)
        Write-Host '  cannot be reached. Retrying WITHOUT a countersignature.'
        Write-Host '  *** THIS IS A DEVIATION AND MUST BE WRITTEN UP AS ONE ***'
        Write-Host ("  signtool sign /sha1 {0} /fd SHA256 <installer>" -f $Thumbprint)
        & signtool sign /sha1 $Thumbprint /fd SHA256 $SetupArtifact
        $signCode = $LASTEXITCODE
        if ($signCode -ne 0) {
            Write-Host ("FAIL: signing failed even without a timestamp, exit code {0}" -f $signCode)
            exit 8
        }
    }

    # --- 5. Verify the signature ------------------------------------------------------------------
    Write-Section 'Step 5: verify the signature'
    Write-Host '  signtool verify /pa <installer>'
    & signtool verify /pa $SetupArtifact
    $verifyCode = $LASTEXITCODE
    if ($verifyCode -ne 0) {
        Write-Host ("FAIL: signtool verify /pa returned {0}" -f $verifyCode)
        exit 9
    }
}

# --- 6. Summary -------------------------------------------------------------------------------------
Write-Section 'Result'
$finalHash = Show-Binary -Path $SetupArtifact -Label 'installer artifact:'
Write-Host ''
Write-Host ("  packaged product SHA-256  {0}" -f $productHash)
Write-Host ("  installer SHA-256         {0}" -f $finalHash)
if (-not $SkipSign) {
    if ($timestamped) {
        Write-Host '  timestamp: PRESENT'
    } else {
        Write-Host '  timestamp: ABSENT -- DEVIATION, record it in the report'
    }
}
if ($AutostartDefaultOn) {
    Write-Host ''
    Write-Host '  *** MEASUREMENT BUILD: the autostart task is pre-ticked. ***'
    Write-Host '  *** Rebuild without -AutostartDefaultOn before leaving   ***'
    Write-Host '  *** this installer in place.                            ***'
}
Write-Host ''
Write-Host '======================================================================'
Write-Host ' RESULT: PASS'
Write-Host ("  {0}" -f $SetupArtifact)
Write-Host '======================================================================'
exit 0
