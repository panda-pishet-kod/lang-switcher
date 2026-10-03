<#
    release.ps1 -- build the shipping configuration, prove criterion 8 over it, then sign it;
    then build the base image from the same tree, prove it is the same body, and sign it too.

    THE ORDER MATTERS AND IS THE POINT OF THIS FILE.

        verify-version.ps1 passes                          (task T-22-11)
          -> rebuild the shipping configuration
            -> verify-criterion8.ps1 passes
              -> copy to <artifact folder>\LangSwitcher.exe
                -> sign the copy
                  -> signtool verify /pa
                    -> verify-perimeter.ps1 passes         (task T-22-11)
                      -> build the base image (LANGSW_NO_UIACCESS=1)          (stage E91)
                        -> verify-one-body.ps1 passes over the two unsigned builds
                          -> copy to <artifact folder>\LangSwitcher-base.exe
                            -> sign the copy, signtool verify /pa
                              -> verify-perimeter.ps1 passes over it

    TWO IMAGES (stage E91, question 155). LangSwitcher.exe is the FULL image -- app.manifest,
    uiAccess. LangSwitcher-base.exe is the BASE image: the same Release, built with
    LANGSW_NO_UIACCESS=1, which build.rs answers with app-dev.manifest. Windows starts the full
    image only where the machine trusts its signature; the base one starts anywhere and does not
    reach the windows of programs run as administrator. The installer puts the base image down
    first and replaces it with the full one where it can (installer\local-trust.ps1). The two
    are one body, and that is proved, not assumed: tools\verify-one-body.ps1 compares the two
    UNSIGNED builds -- kept under <target dir>\one-body\ -- and refuses any difference but the
    manifest and the fields the linker derives from the hash of the content. Criterion 8 is
    measured over the full image; the base image, one body with it, gets the gates of
    verify-perimeter.ps1 over its own signed file. A LANGSW_NO_UIACCESS the caller had set is
    removed first: with it every "full" build of this script would be a base one.

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
    The base image is written into that same file, so AFTER THIS SCRIPT <target dir>\release\
    LangSwitcher.exe IS THE BASE IMAGE, unsigned -- the next `cargo build --release` without the
    variable rebuilds the full one (build.rs reruns on its change). The two unsigned builds the
    one-body check compared stay in <target dir>\one-body\.

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

    THE BUILD MACHINE'S FOLDERS ARE KEPT OUT OF THE IMAGE (stage E90). The compiler writes the
    absolute paths of the registry crates (under CARGO_HOME) and of the standard library's
    sources (under the toolchain's sysroot) into the panic locations of the program -- the e89
    image carried forty-seven of them. Every build of this script and of verify-criterion8.ps1
    runs with --remap-path-prefix for both, through CARGO_ENCODED_RUSTFLAGS: the registry
    becomes /cargo, the standard library /rustc/<commit> -- the path a toolchain without the
    rust-src component writes anyway. verify-perimeter.ps1 gate 4 checks the signed image.

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
    verified, the artifact could be measured for NFR-07, and criterion 4 holds -- and all of
    that again for the base image, with the one-body check between. 8 is the version gate, 9
    the perimeter gate; 1 to 7 are as they were; 10 -- signtool was not found and signing was
    asked; 11 -- rustc did not tell its sysroot and commit, so the remap of the build machine's
    folders cannot be made; 12 -- the base image did not build; 13 -- verify-one-body.ps1 failed;
    14 -- the copy of the base image does not match its build; 15 -- signing the base image
    failed; 16 -- its signature does not verify; 17 -- the perimeter gate failed over it.

    Examples:
      .\release.ps1
      .\release.ps1 -SkipSign
      .\release.ps1 -Thumbprint <other thumbprint>
      .\release.ps1 -Artifact <folder>\LangSwitcher.exe     # the base image beside it
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
    # Where the base image goes (stage E91). Empty means <name>-base.exe beside -Artifact:
    # LangSwitcher-base.exe in the artifact folder.
    [string]$BaseArtifact = '',
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

# The variable that turns a Release build into the base image (stage E91, build.rs). Set by a
# caller, it would make every "full" build below a base one; this script sets it itself, for the
# base build only.
$NoUiAccessFromCaller = $env:LANGSW_NO_UIACCESS
Remove-Item Env:LANGSW_NO_UIACCESS -ErrorAction SilentlyContinue

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

# The build machine's folders out of the image -- see the header. Flags already set are kept;
# the two of this script are added once, so a second run in the same process changes nothing.
$cargoHome = $env:CARGO_HOME
if (-not $cargoHome) { $cargoHome = Join-Path $env:USERPROFILE '.cargo' }
$sysroot = (& rustc --print sysroot | Out-String).Trim()
$commit = ([string](@(& rustc -vV) -match '^commit-hash:')) -replace '^commit-hash:\s*', ''
if ((-not $sysroot) -or (-not $commit)) {
    Write-Host ("FAIL: rustc did not tell its sysroot ({0}) and commit ({1})." -f $sysroot, $commit)
    exit 11
}
$Remap = @(
    ('--remap-path-prefix=' + $cargoHome.TrimEnd('\') + '=/cargo'),
    ('--remap-path-prefix=' + (Join-Path $sysroot 'lib\rustlib\src\rust') + '=/rustc/' + $commit.Trim())
)
$unit = [string][char]0x1F
$flags = @()
if ($env:CARGO_ENCODED_RUSTFLAGS) {
    $flags = @($env:CARGO_ENCODED_RUSTFLAGS -split $unit)
} elseif ($env:RUSTFLAGS) {
    $flags = @($env:RUSTFLAGS -split '\s+' | Where-Object { $_ })
}
foreach ($flag in $Remap) {
    if ($flags -notcontains $flag) { $flags += $flag }
}
$env:CARGO_ENCODED_RUSTFLAGS = $flags -join $unit

# Where cargo builds: CARGO_TARGET_DIR when a caller set it -- an isolated acceptance run keeps
# its own -- and cargo's default <repository>\target otherwise.
$TargetDir = $env:CARGO_TARGET_DIR
if (-not $TargetDir) { $TargetDir = Join-Path $ProjectDir 'target' }
$ReleaseExe = Join-Path $TargetDir 'release\LangSwitcher.exe'
# The two UNSIGNED builds the one-body check compares (stage E91): the full one is kept here
# before it is signed, because the base build then overwrites $ReleaseExe.
$OneBodyDir = Join-Path $TargetDir 'one-body'
$OneBodyFull = Join-Path $OneBodyDir 'LangSwitcher-full.exe'
$OneBodyBase = Join-Path $OneBodyDir 'LangSwitcher-base.exe'

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
if (-not $BaseArtifact) {
    $BaseArtifact = Join-Path (Split-Path -Parent $Artifact) ([System.IO.Path]::GetFileNameWithoutExtension($Artifact) + '-base.exe')
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

# Sign one file and verify the signature; the same command for the full and the base image, so
# the two can never be signed differently. Returns $true when the timestamp countersignature is
# there; on a failure it exits with the code the caller names for that image. The output of
# signtool goes to Out-Host: inside a function it would otherwise become part of the value
# returned, and "$stamped" would be an array that is always true.
function Set-ArtifactSignature { param([string]$Path, [int]$SignFailCode, [int]$VerifyFailCode)
    Write-Host ("  signtool sign /sha1 {0} /fd SHA256 /td SHA256 /tr {1} <file>" -f $Thumbprint, $TimestampUrl)
    & $SignTool sign /sha1 $Thumbprint /fd SHA256 /td SHA256 /tr $TimestampUrl $Path | Out-Host
    $signCode = $LASTEXITCODE
    $stamped = $true
    if ($signCode -ne 0) {
        $stamped = $false
        Write-Host ''
        Write-Host ("  signtool returned {0}. The usual cause is that the timestamp service" -f $signCode)
        Write-Host '  cannot be reached. Retrying WITHOUT a countersignature.'
        Write-Host '  *** THIS IS A DEVIATION AND MUST BE WRITTEN UP AS ONE ***'
        Write-Host ("  signtool sign /sha1 {0} /fd SHA256 <file>" -f $Thumbprint)
        & $SignTool sign /sha1 $Thumbprint /fd SHA256 $Path | Out-Host
        $signCode = $LASTEXITCODE
        if ($signCode -ne 0) {
            Write-Host ("FAIL: signing failed even without a timestamp, exit code {0}" -f $signCode)
            exit $SignFailCode
        }
    }
    Write-Host '  signtool verify /pa <file>'
    & $SignTool verify /pa $Path | Out-Host
    $verifyCode = $LASTEXITCODE
    if ($verifyCode -ne 0) {
        Write-Host ("FAIL: signtool verify /pa returned {0}" -f $verifyCode)
        exit $VerifyFailCode
    }
    return $stamped
}

Write-Host ''
Write-Host '======================================================================'
Write-Host ' Lang_Switcher -- shipping build, criterion 8, signature'
Write-Host '======================================================================'
Write-Host ("Project    {0}" -f $ProjectDir)
Write-Host ("Artifact   {0}  [{1}]" -f $Artifact, $ArtifactFrom)
Write-Host ("Base image {0}" -f $BaseArtifact)
Write-Host ("Target dir {0}" -f $TargetDir)
if ($NoUiAccessFromCaller) {
    Write-Host ("LANGSW_NO_UIACCESS was set by the caller ({0}) and is removed: see the header" -f $NoUiAccessFromCaller)
}
Write-Host ("Thumbprint {0}" -f $Thumbprint)
Write-Host ("signtool   {0}  [{1}]" -f $SignTool, $SignToolFrom)
Write-Host ("Remap      {0}" -f ($Remap -join '  '))
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
# The unsigned full image, kept for the one-body check of step 8 (stage E91).
if (-not (Test-Path -LiteralPath $OneBodyDir)) { New-Item -ItemType Directory -Path $OneBodyDir | Out-Null }
Copy-Item -Path $ReleaseExe -Destination $OneBodyFull -Force

# --- 4 and 5. Sign and verify -------------------------------------------------------------
$timestamped = $false
if ($SkipSign) {
    Write-Section 'Steps 4 and 5: signature -- SKIPPED'
    Write-Host '  -SkipSign was given. The artifact is UNSIGNED and will not run from'
    Write-Host '  %ProgramFiles%: a uiAccess binary needs both a signature and that location.'
} else {
    Write-Section 'Steps 4 and 5: sign the copy and verify the signature'
    $timestamped = Set-ArtifactSignature -Path $Artifact -SignFailCode 6 -VerifyFailCode 7
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

# --- 7. The base image ----------------------------------------------------------------------
# Stage E91. The same tree, the same flags, LANGSW_NO_UIACCESS=1: build.rs then embeds
# app-dev.manifest. The variable is set for this one build and removed whatever happens.
Write-Section 'Step 7: build the base image (LANGSW_NO_UIACCESS=1)'
Push-Location $ProjectDir
try {
    $env:LANGSW_NO_UIACCESS = '1'
    & cargo build --release
    $code = $LASTEXITCODE
} finally {
    Remove-Item Env:LANGSW_NO_UIACCESS -ErrorAction SilentlyContinue
    Pop-Location
}
if (($code -ne 0) -or (-not (Test-Path $ReleaseExe))) {
    Write-Host ("FAIL: cargo build --release of the base image returned {0}" -f $code)
    exit 12
}
Copy-Item -Path $ReleaseExe -Destination $OneBodyBase -Force
Write-Host '  ok'

# --- 8. One body -----------------------------------------------------------------------------
Write-Section 'Step 8: the full and the base image are one body'
& (Join-Path $ScriptDir 'verify-one-body.ps1') -Full $OneBodyFull -Base $OneBodyBase -RequireFullAndBase
$oneBody = $LASTEXITCODE
if ($oneBody -ne 0) {
    Write-Host ''
    Write-Host ("FAIL: verify-one-body.ps1 returned {0}. The base image was not copied and not signed." -f $oneBody)
    exit 13
}

# --- 9. Copy, sign and verify the base image --------------------------------------------------
Write-Section 'Step 9: the base image -- copy, sign, verify'
$baseDir = Split-Path -Parent $BaseArtifact
if (-not (Test-Path $baseDir)) {
    Write-Host ("FAIL: the directory of the base image does not exist: {0}" -f $baseDir)
    exit 14
}
Copy-Item -Path $ReleaseExe -Destination $BaseArtifact -Force
$baseBuiltHash = Show-Binary -Path $ReleaseExe -Label 'base image built (unsigned):'
$baseCopyHash = Show-Binary -Path $BaseArtifact -Label 'copy in the artifact directory:'
if ($baseBuiltHash -ne $baseCopyHash) {
    Write-Host 'FAIL: the copy of the base image does not match its build.'
    exit 14
}
$baseTimestamped = $false
if ($SkipSign) {
    Write-Host '  -SkipSign was given: the base image is UNSIGNED.'
} else {
    $baseTimestamped = Set-ArtifactSignature -Path $BaseArtifact -SignFailCode 15 -VerifyFailCode 16
}

# --- 10. The perimeter over the base image ------------------------------------------------------
Write-Section 'Step 10: the perimeter over the base image'
& (Join-Path $ScriptDir 'verify-perimeter.ps1') -Artifact $BaseArtifact
$basePerimeter = $LASTEXITCODE
if ($basePerimeter -ne 0) {
    Write-Host ''
    Write-Host ("FAIL: verify-perimeter.ps1 returned {0} over the base image; it must not be shipped." -f $basePerimeter)
    exit 17
}

# --- 11. Summary ----------------------------------------------------------------------------
Write-Section 'Result'
$finalHash = Show-Binary -Path $Artifact -Label 'shipping artifact, FULL image:'
$baseFinalHash = Show-Binary -Path $BaseArtifact -Label 'shipping artifact, BASE image:'
Write-Host ''
Write-Host ("  full: unsigned build SHA-256  {0}" -f $builtHash)
Write-Host ("        artifact SHA-256        {0}" -f $finalHash)
Write-Host ("  base: unsigned build SHA-256  {0}" -f $baseBuiltHash)
Write-Host ("        artifact SHA-256        {0}" -f $baseFinalHash)
if (-not $SkipSign) {
    Write-Host '  (each pair differs by design: a signature is appended to the file)'
    if ($timestamped -and $baseTimestamped) {
        Write-Host '  timestamp: PRESENT on both'
    } else {
        Write-Host ("  timestamp: full {0}, base {1} -- DEVIATION, record it in the report" -f $timestamped, $baseTimestamped)
    }
}
Write-Host ('  {0} now holds the BASE image (see the header)' -f $ReleaseExe)
Write-Host ''
Write-Host '======================================================================'
Write-Host ' RESULT: PASS'
Write-Host ("  {0}" -f $Artifact)
Write-Host ("  {0}" -f $BaseArtifact)
Write-Host '======================================================================'
exit 0
