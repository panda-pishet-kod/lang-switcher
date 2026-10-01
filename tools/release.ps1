<#
    release.ps1 -- build the shipping configuration, prove criterion 8 over it, then sign it.

    THE ORDER MATTERS AND IS THE POINT OF THIS FILE.

        verify-version.ps1 passes                          (task T-22-11)
          -> rebuild the shipping configuration
            -> verify-criterion8.ps1 passes
              -> copy to <artifact folder>\LangSwitcher.exe
                -> sign the copy
                  -> signtool verify /pa
                    -> verify-perimeter.ps1 passes         (task T-22-11)

    The two gates task T-22-11 added stand where the thing they are about exists. The version
    check is FIRST, because a tree whose nine version values disagree must not be compiled at
    all; the criterion 4 gate is LAST together with the NFR-07 measurement, because NFR-07 is
    about the SIGNED file and the signature is appended to it. Before that task neither was
    checked by anything: the size was printed as information beside the artifact, and the
    dependency list was compared only by its line count.

    Since decision 106.2 (2026-09-04, task T-45-0) NFR-07 has NO ceiling: the size of the
    signed artifact is measured and printed at every delivery, and the growth over the previous
    delivery goes into the report. So the last step no longer refuses a build for being large;
    it refuses one whose artifact cannot be measured at all, and one that fails criterion 4 or
    the SEC-03 address gate.

    Both build configurations write the same LangSwitcher.exe (fact 8 of section 9 of
    STATE.md), so "sign whatever is on disk" is a way to ship the build WITH the `testing`
    feature and never find out. verify-criterion8.ps1 finishes by rebuilding the shipping
    configuration precisely so that the file this script then copies is the right one.

    Task T-09-2 needs this: the Inno Setup installer packages an ALREADY SIGNED product, and
    rebuilding by hand before every installer build is the direct route to packaging an
    unsigned one.

    WHERE THE ARTIFACT GOES (stage E89, decision 152.3). -Artifact names the file. Without it
    the file is LangSwitcher.exe in the folder the LANGSW_ARTIFACTS environment variable names,
    and without that variable in <repository>\dist\, created when the copy is made. The
    variable is read after tools\local.ps1, the untracked file of this machine's own folders
    (tools\local.example.ps1 shows its form); that file fills only what is empty, so a caller
    that sets a variable itself keeps its own value. cargo comes from PATH, and its folders
    are cargo's own defaults unless CARGO_HOME, RUSTUP_HOME and CARGO_TARGET_DIR are set: this
    script no longer supplies the author's folders for them.

    SIGNTOOL (stage E90). LANGSW_SIGNTOOL names it; without it signtool.exe on PATH; without that
    the newest Windows Kits 10 x64 signtool. Not found and signing asked: the script refuses
    before it builds anything (exit 10). It used to put the folder of one SDK version on PATH,
    and a machine with another version had no signtool at all.

    THE SIGNATURE. The certificate is CN=Panda_Pishet_Kod, thumbprint below, private key in
    Cert:\CurrentUser\My and NOT exportable, so there is no .pfx and none can be made. The
    command needs no administrator rights -- established by the setup stage, TOOLCHAIN.md
    section 5.

    The timestamp countersignature needs the network. If the service cannot be reached the
    signature is still worth having locally, so this script falls back to signing without
    /tr and /td, says so loudly, and reports it through -TimestampMissing in the summary.
    That fallback is a DEVIATION and has to be written up as one, not passed over: without a
    countersignature the signature stops verifying the day the certificate expires.

    Written without a single Cyrillic character on purpose: the tooling writes files in UTF-8
    with NO byte order mark, Windows PowerShell 5.1 reads such a .ps1 in the system ANSI code
    page, and the parse dies on the first Russian word. Decision R-05.
    Windows PowerShell 5.1, not 7.x: no `&&`, no `??`, no ternary operator.

    Exit code: 0 only if the nine version values agree, the build succeeded, criterion 8
    passed, the copy was made, -- unless -SkipSign was given -- the signature was applied and
    verified, the artifact could be measured for NFR-07, and criterion 4 holds. 8 is the
    version gate, 9 the perimeter gate; 1 to 7 are as they were; 10 -- signtool was not found
    and signing was asked.

    Examples:
      .\release.ps1
      .\release.ps1 -SkipSign
      .\release.ps1 -Thumbprint <other thumbprint>
#>
[CmdletBinding()]
param(
    # SHA-1 thumbprint of the code signing certificate. TOOLCHAIN.md section 5.
    [string]$Thumbprint = '8F038C7D00DACFCE34EC742EC97CCEBFD8465CAC',
    # Build, verify and copy, but do not sign. For a dry run of everything before the signature.
    [switch]$SkipSign,
    # Where the shipping artifact goes. Empty means LangSwitcher.exe in the artifact folder --
    # LANGSW_ARTIFACTS, or <repository>\dist\ without it; see the header. The folder may hold
    # other artifacts of the machine; this script touches this one file and nothing else in it.
    [string]$Artifact = '',
    [string]$TimestampUrl = 'http://timestamp.digicert.com',
    # Size in bytes of the artifact of the PREVIOUS delivery. Passed straight to
    # verify-perimeter.ps1, which turns it into the growth line decision 106.2 asks every
    # delivery report to carry. Zero means "not given".
    [long]$PreviousSize = 0
)

$ErrorActionPreference = 'Stop'

# --- This machine's own folders (stage E89) ------------------------------------------------
# First, so that everything below reads the LANGSW_* variables it sets. See the header.
if (Test-Path "$PSScriptRoot\local.ps1") { . "$PSScriptRoot\local.ps1" }

$ScriptDir = $PSScriptRoot
$ProjectDir = Split-Path -Parent $ScriptDir

# --- Environment --------------------------------------------------------------------------
# cargo from PATH, with cargo's own defaults for whatever CARGO_* is not set (stage E89).

# signtool: LANGSW_SIGNTOOL, else PATH, else the newest Windows Kits 10 x64 one. See the header.
$SignTool = $env:LANGSW_SIGNTOOL
$SignToolFrom = 'LANGSW_SIGNTOOL'
if (-not $SignTool) {
    $onPath = @(Get-Command 'signtool.exe' -CommandType Application -ErrorAction SilentlyContinue)
    if ($onPath.Count -gt 0) {
        $SignTool = $onPath[0].Path
        $SignToolFrom = 'PATH'
    }
}
if ((-not $SignTool) -and ${env:ProgramFiles(x86)}) {
    $kits = Join-Path ${env:ProgramFiles(x86)} 'Windows Kits\10\bin'
    if (Test-Path -LiteralPath $kits) {
        $versions = @(Get-ChildItem -LiteralPath $kits -Directory |
            Where-Object { $_.Name -match '^\d+(\.\d+){3}$' } |
            Sort-Object -Property @{ Expression = { [version]$_.Name } } -Descending)
        foreach ($v in $versions) {
            $candidate = Join-Path $v.FullName 'x64\signtool.exe'
            if (Test-Path -LiteralPath $candidate) {
                $SignTool = $candidate
                $SignToolFrom = 'Windows Kits ' + $v.Name
                break
            }
        }
    }
}
if (-not $SignTool) {
    $SignToolFrom = 'not found'
    if (-not $SkipSign) {
        Write-Host ''
        Write-Host 'FAIL: signtool.exe was not found: not named by LANGSW_SIGNTOOL, not on PATH, not under'
        Write-Host '      Windows Kits\10\bin. Install the Windows SDK, or set LANGSW_SIGNTOOL in'
        Write-Host '      tools\local.ps1 (tools\local.example.ps1 shows the form), or pass -SkipSign.'
        exit 10
    }
}

# Where cargo builds: CARGO_TARGET_DIR when a caller set it -- an isolated acceptance run keeps
# its own -- and cargo's default <repository>\target otherwise.
$TargetDir = $env:CARGO_TARGET_DIR
if (-not $TargetDir) { $TargetDir = Join-Path $ProjectDir 'target' }
$ReleaseExe = Join-Path $TargetDir 'release\LangSwitcher.exe'

# --- The artifact --------------------------------------------------------------------------
# -Artifact, else LANGSW_ARTIFACTS, else <repository>\dist\ -- the one folder this script may
# create, at the copy, because nobody named it.
$ArtifactFrom = '-Artifact'
$ArtifactDirIsDefault = $false
if (-not $Artifact) {
    if ($env:LANGSW_ARTIFACTS) {
        $Artifact = Join-Path $env:LANGSW_ARTIFACTS 'LangSwitcher.exe'
        $ArtifactFrom = 'LANGSW_ARTIFACTS'
    } else {
        $Artifact = Join-Path (Join-Path $ProjectDir 'dist') 'LangSwitcher.exe'
        $ArtifactFrom = 'default <repository>\dist'
        $ArtifactDirIsDefault = $true
    }
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
Write-Host ' Lang_Switcher -- shipping build, criterion 8, signature'
Write-Host '======================================================================'
Write-Host ("Project    {0}" -f $ProjectDir)
Write-Host ("Artifact   {0}  [{1}]" -f $Artifact, $ArtifactFrom)
Write-Host ("Target dir {0}" -f $TargetDir)
Write-Host ("Thumbprint {0}" -f $Thumbprint)
Write-Host ("signtool   {0}  [{1}]" -f $SignTool, $SignToolFrom)
if ($SkipSign) { Write-Host 'Signing     SKIPPED by -SkipSign' }

# --- 0. One version, nine places -------------------------------------------------------------
# Task T-22-11, finding 18. In front of the build and not behind it: a tree whose VERSIONINFO,
# installer and assembly identity disagree must not be compiled at all, because everything
# after this point is a file that would have to be thrown away. The About window reads its
# number out of the VERSIONINFO resource, so a disagreement is something the user is shown.
Write-Section 'Step 0: one version, nine places'
& (Join-Path $ScriptDir 'verify-version.ps1')
$versions = $LASTEXITCODE
if ($versions -ne 0) {
    Write-Host ''
    Write-Host ("FAIL: verify-version.ps1 returned {0}. Nothing was built." -f $versions)
    exit 8
}

# --- 1. Build the shipping configuration ---------------------------------------------------
Write-Section 'Step 1: build the shipping configuration'
Push-Location $ProjectDir
try {
    & cargo build --release
    $code = $LASTEXITCODE
} finally {
    Pop-Location
}
if ($code -ne 0) {
    Write-Host ("FAIL: cargo build --release returned {0}" -f $code)
    exit 1
}
Write-Host '  ok'

# --- 2. Criterion 8 ------------------------------------------------------------------------
# This also rebuilds both configurations around its own measurements and finishes by
# rebuilding the shipping one, so what is left on disk afterwards is the file to copy.
Write-Section 'Step 2: acceptance criterion 8, section 13 of SPEC'
& (Join-Path $ScriptDir 'verify-criterion8.ps1')
$criterion = $LASTEXITCODE
if ($criterion -ne 0) {
    Write-Host ''
    Write-Host ("FAIL: verify-criterion8.ps1 returned {0}. Nothing was copied and nothing was signed." -f $criterion)
    exit 2
}

if (-not (Test-Path $ReleaseExe)) {
    Write-Host ("FAIL: the shipping binary is missing: {0}" -f $ReleaseExe)
    exit 3
}

# --- 3. Copy to the artifact directory ------------------------------------------------------
Write-Section 'Step 3: copy to the artifact directory'
$artifactDir = Split-Path -Parent $Artifact
if ((-not (Test-Path $artifactDir)) -and $ArtifactDirIsDefault) {
    New-Item -ItemType Directory -Path $artifactDir | Out-Null
    Write-Host ("  created the default artifact directory {0}" -f $artifactDir)
}
if (-not (Test-Path $artifactDir)) {
    Write-Host ("FAIL: the artifact directory does not exist: {0}" -f $artifactDir)
    exit 4
}
Copy-Item -Path $ReleaseExe -Destination $Artifact -Force
$builtHash = Show-Binary -Path $ReleaseExe -Label 'built (unsigned):'
$copyHash = Show-Binary -Path $Artifact -Label 'copy in the artifact directory:'
if ($builtHash -ne $copyHash) {
    Write-Host 'FAIL: the copy does not match the built binary.'
    exit 5
}
Write-Host '  the copy is byte for byte the binary criterion 8 was measured over'

# --- 4. Sign --------------------------------------------------------------------------------
$timestamped = $false
if ($SkipSign) {
    Write-Section 'Step 4: signature -- SKIPPED'
    Write-Host '  -SkipSign was given. The artifact is UNSIGNED and will not run from'
    Write-Host '  %ProgramFiles%: a uiAccess binary needs both a signature and that location.'
} else {
    Write-Section 'Step 4: sign the copy'
    Write-Host ("  signtool sign /sha1 {0} /fd SHA256 /td SHA256 /tr {1} <artifact>" -f $Thumbprint, $TimestampUrl)
    & $SignTool sign /sha1 $Thumbprint /fd SHA256 /td SHA256 /tr $TimestampUrl $Artifact
    $signCode = $LASTEXITCODE

    if ($signCode -eq 0) {
        $timestamped = $true
    } else {
        Write-Host ''
        Write-Host ("  signtool returned {0}. The usual cause is that the timestamp service" -f $signCode)
        Write-Host '  cannot be reached. Retrying WITHOUT a countersignature.'
        Write-Host '  *** THIS IS A DEVIATION AND MUST BE WRITTEN UP AS ONE ***'
        Write-Host ("  signtool sign /sha1 {0} /fd SHA256 <artifact>" -f $Thumbprint)
        & $SignTool sign /sha1 $Thumbprint /fd SHA256 $Artifact
        $signCode = $LASTEXITCODE
        if ($signCode -ne 0) {
            Write-Host ("FAIL: signing failed even without a timestamp, exit code {0}" -f $signCode)
            exit 6
        }
    }

    # --- 5. Verify the signature ------------------------------------------------------------
    Write-Section 'Step 5: verify the signature'
    Write-Host '  signtool verify /pa <artifact>'
    & $SignTool verify /pa $Artifact
    $verifyCode = $LASTEXITCODE
    if ($verifyCode -ne 0) {
        Write-Host ("FAIL: signtool verify /pa returned {0}" -f $verifyCode)
        exit 7
    }
}

# --- 6. NFR-07 and criterion 4 ----------------------------------------------------------------
# Task T-22-11, finding 19. Behind the signature and not in front of it: NFR-07 is about the
# file the user runs, a signature is appended to that file, and the unsigned build is therefore
# the easier case. Under -SkipSign the artifact is unsigned and the gate says so rather than
# pretending the measurement was the right one.
# Task T-45-0, decision 106.2: the NFR-07 half of this step is a MEASUREMENT, not a limit --
# it prints the size and, with -PreviousSize, the growth over the previous delivery.
Write-Section 'Step 6: NFR-07 measurement and criterion 4 of section 13'
if ($SkipSign) {
    Write-Host '  NOTE: -SkipSign was given, so the file measured below is UNSIGNED and is'
    Write-Host '  smaller than the product will be. The size below is not the shipping one.'
}
& (Join-Path $ScriptDir 'verify-perimeter.ps1') -Artifact $Artifact -Previous $PreviousSize
$perimeter = $LASTEXITCODE
if ($perimeter -ne 0) {
    Write-Host ''
    Write-Host ("FAIL: verify-perimeter.ps1 returned {0}. The artifact exists and is signed," -f $perimeter)
    Write-Host '      and it must not be shipped: see the gate that failed above.'
    exit 9
}

# --- 7. Summary -----------------------------------------------------------------------------
Write-Section 'Result'
$finalHash = Show-Binary -Path $Artifact -Label 'shipping artifact:'
Write-Host ''
Write-Host ("  unsigned build SHA-256  {0}" -f $builtHash)
Write-Host ("  artifact SHA-256        {0}" -f $finalHash)
if (-not $SkipSign) {
    Write-Host '  (the two differ by design: a signature is appended to the file)'
    if ($timestamped) {
        Write-Host '  timestamp: PRESENT'
    } else {
        Write-Host '  timestamp: ABSENT -- DEVIATION, record it in the report'
    }
}
Write-Host ''
Write-Host '======================================================================'
Write-Host ' RESULT: PASS'
Write-Host ("  {0}" -f $Artifact)
Write-Host '======================================================================'
exit 0
