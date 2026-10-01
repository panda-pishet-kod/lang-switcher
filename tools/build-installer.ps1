<#
    build-installer.ps1 -- build the product, compile the Inno Setup installer,
    sign the installer, verify the signature, report the path and the SHA-256.

    THE ORDER MATTERS, FOR THE SAME REASON IT MATTERS IN release.ps1.

        tools\release.ps1                    (build, criterion 8, sign the product)
          -> ISCC installer\LangSwitcher.iss (package the SIGNED product)
            -> copy to <artifact folder>\LangSwitcher-setup.exe
              -> sign the copy
                -> signtool verify /pa

    WHY release.ps1 RUNS FIRST AND NOT "USE WHATEVER IS ON DISK". Section 8.2 of
    SPEC.md: an image with uiAccess="true" starts only when it is BOTH signed with
    a certificate the machine trusts AND located under %ProgramFiles%. Package an
    unsigned or stale product and the installed program fails to start with
    CreateProcess error 740, which looks exactly like an installer defect and is
    not one. release.ps1 also re-proves acceptance criterion 8 of section 13, so
    the packaged build is the one without the `testing` feature.

    Skip it with -SkipProductBuild only when the signed LangSwitcher.exe of the
    artifact folder is already known good; the script still refuses to run if that
    file is missing.

    WHERE THINGS ARE (stage E89, decision 152.3). The artifact folder is the one the
    LANGSW_ARTIFACTS environment variable names, and <repository>\dist\ without it --
    the same rule release.ps1 follows, so the two scripts meet on the same file. The
    Inno Setup compiler is -Iscc, else LANGSW_ISCC, else ISCC.exe on PATH, else
    Inno Setup 6 under Program Files (x86) or Program Files; found nowhere, the script
    refuses and says how to name it. The variables are read after tools\local.ps1,
    the untracked file of this machine's own folders (tools\local.example.ps1 shows
    its form). The installer script has no default product of its own: it is always
    given /DSourceExe=<the signed product> below.

    THE SIGNATURE. Certificate CN=Panda_Pishet_Kod, thumbprint below, private key
    in Cert:\CurrentUser\My and NOT exportable. The command needs no administrator
    rights (TOOLCHAIN.md section 5). If the RFC3161 timestamp service cannot be
    reached the script retries without a countersignature, says so loudly, and
    reports it in the summary. That fallback is a DEVIATION and belongs in the
    report: without a countersignature the signature stops verifying the day the
    certificate expires.

    AUTOSTART IS NOT AN INSTALLER OPTION. Since stage E55 (decision 120.4) the
    product writes HKCU\...\Run itself at every start, following its own
    configuration, and the installer has no autostart task to pre-tick; the
    former -AutostartDefaultOn switch went with it.

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
     10  ISCC.exe was found neither by -Iscc, LANGSW_ISCC, PATH nor Program Files

    Examples:
      .\build-installer.ps1
      .\build-installer.ps1 -SkipProductBuild
#>
[CmdletBinding()]
param(
    # SHA-1 thumbprint of the code signing certificate. TOOLCHAIN.md section 5.
    [string]$Thumbprint = '8F038C7D00DACFCE34EC742EC97CCEBFD8465CAC',
    # The signed product the installer packages. Produced by tools\release.ps1.
    # Empty means LangSwitcher.exe in the artifact folder; see the header.
    [string]$ProductArtifact = '',
    # Where the finished installer goes. Empty means LangSwitcher-setup.exe in the
    # artifact folder. On the author's machine the LangSw-Install scheduled task runs
    # the installer by this name from that folder, so the name cannot be changed.
    [string]$SetupArtifact = '',
    [string]$TimestampUrl = 'http://timestamp.digicert.com',
    # The Inno Setup 6 compiler. Empty means LANGSW_ISCC, PATH, then Program Files.
    [string]$Iscc = '',
    # Do not rebuild and re-sign the product first. See the header.
    [switch]$SkipProductBuild,
    # Compile and copy, but do not sign. For a dry run of everything before the
    # signature.
    [switch]$SkipSign
)

$ErrorActionPreference = 'Stop'

# --- This machine's own folders (stage E89) ------------------------------------------------
# First, so that everything below reads the LANGSW_* variables it sets. See the header.
if (Test-Path "$PSScriptRoot\local.ps1") { . "$PSScriptRoot\local.ps1" }

$ScriptDir  = $PSScriptRoot
$ProjectDir = Split-Path -Parent $ScriptDir
$IssFile    = Join-Path $ProjectDir 'installer\LangSwitcher.iss'
$IssOutput  = Join-Path $ProjectDir 'installer\Output\LangSwitcher-setup.exe'

# --- Environment ----------------------------------------------------------------------------
# cargo from PATH, with cargo's own defaults for whatever CARGO_* is not set (stage E89). The
# SDK folder stays: signtool lives there.
$SdkBin  = 'C:\Program Files (x86)\Windows Kits\10\bin\10.0.26100.0\x64'
if ($env:PATH -notlike "*$SdkBin*") { $env:PATH = "$SdkBin;$env:PATH" }

# --- The artifact folder and the compiler ----------------------------------------------------
# LANGSW_ARTIFACTS, else <repository>\dist\ -- the rule of release.ps1. Only the default folder
# may be created here, at the copy, because nobody named it.
$ArtifactDirIsDefault = $false
$ArtifactDir = $env:LANGSW_ARTIFACTS
if (-not $ArtifactDir) {
    $ArtifactDir = Join-Path $ProjectDir 'dist'
    $ArtifactDirIsDefault = $true
}
if (-not $ProductArtifact) { $ProductArtifact = Join-Path $ArtifactDir 'LangSwitcher.exe' }
if (-not $SetupArtifact)   { $SetupArtifact   = Join-Path $ArtifactDir 'LangSwitcher-setup.exe' }

$IsccFrom = '-Iscc'
if (-not $Iscc -and $env:LANGSW_ISCC) {
    $Iscc = $env:LANGSW_ISCC
    $IsccFrom = 'LANGSW_ISCC'
}
if (-not $Iscc) {
    $onPath = @(Get-Command 'ISCC.exe' -CommandType Application -ErrorAction SilentlyContinue)
    if ($onPath.Count -gt 0) {
        $Iscc = $onPath[0].Path
        $IsccFrom = 'PATH'
    }
}
if (-not $Iscc) {
    foreach ($programs in @(${env:ProgramFiles(x86)}, $env:ProgramFiles)) {
        if (-not $programs) { continue }
        $candidate = Join-Path $programs 'Inno Setup 6\ISCC.exe'
        if (Test-Path -LiteralPath $candidate) {
            $Iscc = $candidate
            $IsccFrom = 'Program Files'
            break
        }
    }
}
if (-not $Iscc) {
    Write-Host ''
    Write-Host 'FAIL: ISCC.exe, the Inno Setup 6 compiler, was not found: not given by -Iscc,'
    Write-Host '      not named by LANGSW_ISCC, not on PATH, not under Program Files.'
    Write-Host '      Name it with -Iscc <full path>, or set LANGSW_ISCC in tools\local.ps1'
    Write-Host '      (tools\local.example.ps1 shows the form).'
    exit 10
}

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
Write-Host ("ISCC       {0}  [{1}]" -f $Iscc, $IsccFrom)
Write-Host ("Thumbprint {0}" -f $Thumbprint)
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
if ((-not (Test-Path $artifactDir)) -and $ArtifactDirIsDefault) {
    New-Item -ItemType Directory -Path $artifactDir | Out-Null
    Write-Host ("  created the default artifact directory {0}" -f $artifactDir)
}
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
Write-Host ''
Write-Host '======================================================================'
Write-Host ' RESULT: PASS'
Write-Host ("  {0}" -f $SetupArtifact)
Write-Host '======================================================================'
exit 0
