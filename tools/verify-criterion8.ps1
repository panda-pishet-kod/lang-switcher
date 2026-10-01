<#
    verify-criterion8.ps1 -- acceptance criterion 8 of section 13 of SPEC, measured.

    The criterion, word for word:

      "The Release binary carries no trace of the SEC-04a debug channel: the build was made
       without the `testing` feature, a search over the strings of the binary does not find
       the name of the named channel, and `Cargo.lock` and `cargo tree --features testing`
       are checked against each other for differences in the set of dependencies."

    WHAT THIS SCRIPT DOES, and why each part is not decoration.

    1. It REBUILDS EACH CONFIGURATION IMMEDIATELY BEFORE ITS OWN MEASUREMENT. Both
       configurations write the same file, <target>\release\LangSwitcher.exe, so a
       measurement taken without a build in front of it measures the leftover of the previous
       one. The controller was caught by exactly this while checking task T-03-4-2: the first
       "release without testing" measurement found the channel name, because what lay on disk
       was the build WITH the feature. Fact 8 of section 9 of STATE.md.

    2. It runs a POSITIVE CONTROL over every one of the seven strings. Absence on its own
       proves nothing at all: the Release profile has `strip = true`, and a single typo in a
       search pattern gives the same answer as a string that is genuinely not there. Each
       string must be MISSING from the shipping build and FOUND in the build that is supposed
       to carry it. Decision R-25, question 29.

       There are TWO such builds, because the seven strings are switched off by two different
       mechanisms. Five are behind the `testing` feature and are controlled by measurement B.
       The sixth and the seventh, LANGSW_DEBUG_TIMEOUT_SEC and LANGSW_DEBUG_FONT_STEP, are
       behind `#[cfg(debug_assertions)]` and are controlled by measurement B2 -- a release
       built with debug assertions forced on. Giving them the feature's control would have been
       a control that cannot fail: the strings are rightly absent from the `testing` build too.

    3. It searches in UTF-8 AND in UTF-16LE. Rust puts string literals in the binary as UTF-8,
       but a literal that went into a Win32 call through PCWSTR can be sitting there wide. A
       string that was looked for in the wrong encoding and not found is a false pass.

    4. It compares `cargo tree` against `cargo tree --features testing` WITH EVERY NON-ASCII
       CHARACTER DISCARDED. The tree is drawn with box-drawing characters, the console mangles
       them, and comparing the raw text reports differences that are an artefact of the
       terminal rather than of the dependency graph. Section 8.4 of STATE.md.

    5. It FINISHES BY REBUILDING THE SHIPPING CONFIGURATION, so that what is left on disk is
       the binary without the feature and not the one with it. The next step after this script
       is the signature, and signing the wrong file is not an error anything downstream would
       catch.

    THE LIST OF STRINGS IS CLOSED by decisions R-28 and R-53. Do not shorten it and do not
    extend it: a new entry is a decision for the owner, not for this file.

    The SIXTH entry was added by task T-41-12 under decision 125.2 -- finding H134 of the audit
    of 2026-09-04. The Release profile turns off two mechanisms of the program and used to write
    down only one of them: `panic = "abort"` for the panic guard of FR-99 was there, and
    `debug-assertions = false` for the self-termination of FR-97 was not, standing on cargo's
    default instead. Turn debug assertions on in the release profile to chase a bug, forget to
    turn them off, and a resident program ships that quietly exits after ten minutes -- and no
    instrument would have noticed, because the acceptance read the mechanism out of the SOURCE
    rather than out of the built file. It is read out of the built file now.

    The SEVENTH entry was added by the owner's word of 2026-09-25 (decision 143d, debt E83-B-2
    of stage E83). LANGSW_DEBUG_FONT_STEP is the debug substitution of the step of the windows'
    font (task T-83-3, 6 to 15 points), built the way FR-97's deadline is: behind
    `#[cfg(debug_assertions)]`, absent from the Release build rather than disabled in it. The
    deliveries e83 and e84 measured it with a separate instrument of their own, because this
    list was closed; the list carries it now, so every delivery measures it without anybody
    having to remember a second script.

    Written without a single Cyrillic character on purpose. The tooling that writes files here
    puts them in UTF-8 with NO byte order mark, Windows PowerShell 5.1 then reads such a .ps1
    in the system ANSI code page, and the parse dies on the first Russian word. Decision R-05.
    Windows PowerShell 5.1, not 7.x: no `&&`, no `??`, no ternary operator, no -AsHashtable.

    Exit code: 0 only if all seven strings are absent from the shipping build, the five of the
    `testing` feature are present in the build with it, the sixth and the seventh are present in
    the build with debug assertions on, and the dependency sets agree. Otherwise non-zero,
    reasons listed.

    Example:
      .\verify-criterion8.ps1
      .\verify-criterion8.ps1 -SkipFinalRebuild
#>
[CmdletBinding()]
param(
    # The project directory. Defaults to the parent of the directory this script sits in.
    [string]$ProjectDir,
    # Leaves the build with the `testing` feature on disk. For diagnosis only: the shipping
    # binary is the point of the final rebuild, so this switch must not be used before signing.
    [switch]$SkipFinalRebuild
)

$ErrorActionPreference = 'Stop'

# --- This machine's own folders (stage E89) ------------------------------------------------
# tools\local.ps1 is the untracked file of this machine's own values (tools\local.example.ps1
# shows its form). Loaded first, so that a CARGO_TARGET_DIR it sets is the one read below; it
# fills only what is empty, so a caller's own value stays.
if (Test-Path "$PSScriptRoot\local.ps1") { . "$PSScriptRoot\local.ps1" }

if (-not $ProjectDir) { $ProjectDir = Split-Path -Parent $PSScriptRoot }
$ProjectDir = (Resolve-Path $ProjectDir).Path

# --- Environment --------------------------------------------------------------------------
# Nothing is set here any more (stage E89): cargo comes from PATH, and CARGO_HOME, RUSTUP_HOME
# and CARGO_TARGET_DIR keep cargo's own defaults unless a caller set them. A caller building in
# an isolated target directory (a target-<id> folder of its own, decision R-27) keeps it: this
# script reads CARGO_TARGET_DIR and never writes it, so that caller's artifacts are not sent
# into a shared tree behind its back.
$TargetDir = $env:CARGO_TARGET_DIR
if (-not $TargetDir) { $TargetDir = Join-Path $ProjectDir 'target' }

$ReleaseExe = Join-Path $TargetDir 'release\LangSwitcher.exe'

# --- The five strings -------------------------------------------------------------------
# Position 1 is copied character for character from `PIPE_NAME_PREFIX` in src\control.rs,
# which declares it as a raw literal r"\\.\pipe\Lang_Switcher.control." -- the backslashes are
# part of the value. A single-quoted PowerShell string keeps them as they are; a
# double-quoted one would too, but single quotes make that impossible to get wrong later.
#
# --- The sixth string, and why its control is a different one ----------------------------
# Added by task T-41-12 (finding H134, decision 125.2). LANGSW_DEBUG_TIMEOUT_SEC is the
# environment variable of the self-termination of FR-97, and it is NOT behind the `testing`
# feature at all: it is behind `#[cfg(debug_assertions)]`. So the positive control of
# measurement B cannot speak for it -- a release build with the feature still has debug
# assertions off, and the string is rightly missing there too. Its control is measurement B2
# instead: a release built with debug assertions forced ON, where it MUST be present.
#
# --- The seventh string, under the same control ------------------------------------------
# Added by the owner's word of 2026-09-25 (decision 143d, debt E83-B-2). LANGSW_DEBUG_FONT_STEP
# is the environment variable of the debug substitution of the font step (task T-83-3), in
# `mod debug_font_step` of src\settings.rs -- behind `#[cfg(debug_assertions)]` exactly like the
# sixth, so its control is measurement B2 as well.
#
# The `Control` field says which of the two halves proves the absence measured in A.
$Strings = @(
    @{ Name = 'SEC-04a channel name';        Value = '\\.\pipe\Lang_Switcher.control.'; Source = 'src\control.rs, PIPE_NAME_PREFIX'; Control = 'testing' },
    @{ Name = 'LANGSW_TESTING_REPORT';           Value = 'LANGSW_TESTING_REPORT';           Source = 'src\app.rs';      Control = 'testing' },
    @{ Name = 'LANGSW_TESTING_PANIC_ON_HANDOFF'; Value = 'LANGSW_TESTING_PANIC_ON_HANDOFF'; Source = 'src\hook.rs';     Control = 'testing' },
    @{ Name = 'LANGSW_TESTING_PANIC_ON_VK';      Value = 'LANGSW_TESTING_PANIC_ON_VK';      Source = 'src\hook.rs';     Control = 'testing' },
    @{ Name = 'LANGSW_TESTING_DROP_HOOK_MS';     Value = 'LANGSW_TESTING_DROP_HOOK_MS';     Source = 'src\watchdog.rs'; Control = 'testing' },
    @{ Name = 'LANGSW_DEBUG_TIMEOUT_SEC';        Value = 'LANGSW_DEBUG_TIMEOUT_SEC';        Source = 'src\app.rs, mod debug_timeout'; Control = 'debug-assertions' },
    @{ Name = 'LANGSW_DEBUG_FONT_STEP';          Value = 'LANGSW_DEBUG_FONT_STEP';          Source = 'src\settings.rs, mod debug_font_step'; Control = 'debug-assertions' }
)

$Failures = New-Object System.Collections.Generic.List[string]

# --- Byte search ----------------------------------------------------------------------------
# The file is turned into a string of one character per byte through ISO-8859-1, which is the
# one encoding that maps all 256 byte values onto 256 distinct characters and back without
# loss. The needle goes through the same mapping, and IndexOf then does an exact byte search
# at the speed of the framework rather than of a PowerShell loop.
$Latin1 = [System.Text.Encoding]::GetEncoding(28591)

function ConvertTo-ByteString {
    param([byte[]]$Bytes)
    return $Latin1.GetString($Bytes)
}

function Test-BinaryHasString {
    param(
        [string]$Haystack,   # the file, one character per byte
        [string]$Value,
        [string]$Encoding    # 'UTF-8' or 'UTF-16LE'
    )
    if ($Encoding -eq 'UTF-8') {
        $needle = ConvertTo-ByteString ([System.Text.Encoding]::UTF8.GetBytes($Value))
    } else {
        $needle = ConvertTo-ByteString ([System.Text.Encoding]::Unicode.GetBytes($Value))
    }
    return ($Haystack.IndexOf($needle, [System.StringComparison]::Ordinal) -ge 0)
}

function Invoke-Cargo {
    param([string[]]$CargoArgs, [string]$What)
    Write-Host ("  cargo " + ($CargoArgs -join ' '))
    Push-Location $ProjectDir
    try {
        & cargo @CargoArgs
        $code = $LASTEXITCODE
    } finally {
        Pop-Location
    }
    if ($code -ne 0) {
        throw ("{0} failed with exit code {1}" -f $What, $code)
    }
}

function Measure-Binary {
    param([string]$Path, [string]$Label)
    $item = Get-Item $Path
    $hash = (Get-FileHash -Algorithm SHA256 -Path $Path).Hash
    Write-Host ''
    Write-Host ("{0}" -f $Label)
    Write-Host ("  path    {0}" -f $item.FullName)
    Write-Host ("  size    {0} bytes ({1} KB)" -f $item.Length, [math]::Round($item.Length / 1KB, 1))
    Write-Host ("  SHA-256 {0}" -f $hash)
    return $hash
}

Write-Host ''
Write-Host '======================================================================'
Write-Host ' Acceptance criterion 8, section 13 of SPEC -- SEC-04a traces'
Write-Host '======================================================================'
Write-Host ("Project        {0}" -f $ProjectDir)
Write-Host ("Target dir     {0}" -f $TargetDir)
Write-Host ("Release binary {0}" -f $ReleaseExe)

# ==========================================================================================
# Measurement A -- the shipping configuration. All seven strings must be ABSENT.
# ==========================================================================================
Write-Host ''
Write-Host '--- A. Release WITHOUT the `testing` feature ---------------------------'
Write-Host 'Rebuilt immediately before the measurement (fact 8 of section 9 of STATE.md).'
Invoke-Cargo -CargoArgs @('build', '--release') -What 'cargo build --release'
$hashShipping = Measure-Binary -Path $ReleaseExe -Label 'Release, no feature:'
$plainBytes = [System.IO.File]::ReadAllBytes($ReleaseExe)
$plain = ConvertTo-ByteString $plainBytes

$absent = @{}
Write-Host ''
Write-Host 'Expected: NOT FOUND in either encoding.'
foreach ($s in $Strings) {
    $u8 = Test-BinaryHasString -Haystack $plain -Value $s.Value -Encoding 'UTF-8'
    $u16 = Test-BinaryHasString -Haystack $plain -Value $s.Value -Encoding 'UTF-16LE'
    $found = ($u8 -or $u16)
    $absent[$s.Name] = (-not $found)
    $verdict = 'ABSENT'
    if ($found) { $verdict = 'PRESENT' }
    Write-Host ("  {0,-8} {1,-38} UTF-8={2,-5} UTF-16LE={3,-5}  [{4}]" -f $verdict, $s.Name, $u8, $u16, $s.Source)
    if ($found) {
        $Failures.Add(("the shipping build carries `"{0}`" (UTF-8={1}, UTF-16LE={2})" -f $s.Name, $u8, $u16))
    }
}

# ==========================================================================================
# Measurement B -- the positive control. All five strings must be PRESENT.
# ==========================================================================================
Write-Host ''
Write-Host '--- B. Release WITH the `testing` feature -- POSITIVE CONTROL ----------'
Write-Host 'Without this half, measurement A proves nothing: `strip = true` and a typo in a'
Write-Host 'search pattern give exactly the same answer as a string that is truly absent.'
Invoke-Cargo -CargoArgs @('build', '--release', '--features', 'testing') -What 'cargo build --release --features testing'
$hashTesting = Measure-Binary -Path $ReleaseExe -Label 'Release, --features testing:'
$testingBytes = [System.IO.File]::ReadAllBytes($ReleaseExe)
$testing = ConvertTo-ByteString $testingBytes

Write-Host ''
Write-Host 'Expected: FOUND in at least one encoding. Only the strings whose control this is.'
foreach ($s in $Strings) {
    if ($s.Control -ne 'testing') {
        Write-Host ("  {0,-8} {1,-38} control is B2, not this one" -f 'SKIPPED', $s.Name)
        continue
    }
    $u8 = Test-BinaryHasString -Haystack $testing -Value $s.Value -Encoding 'UTF-8'
    $u16 = Test-BinaryHasString -Haystack $testing -Value $s.Value -Encoding 'UTF-16LE'
    $found = ($u8 -or $u16)
    $where = 'none'
    if ($u8 -and $u16) { $where = 'UTF-8 + UTF-16LE' }
    elseif ($u8)       { $where = 'UTF-8' }
    elseif ($u16)      { $where = 'UTF-16LE' }
    $verdict = 'PRESENT'
    if (-not $found) { $verdict = 'MISSING' }
    Write-Host ("  {0,-8} {1,-38} found in: {2}" -f $verdict, $s.Name, $where)
    if (-not $found) {
        $Failures.Add(("the positive control failed for `"{0}`": the build WITH the feature does not carry it either, so the absence measured in A means nothing" -f $s.Name))
    }
}

if ($hashShipping -eq $hashTesting) {
    $Failures.Add('the two builds hashed identically, which means the second build did not replace the first on disk')
}

# ==========================================================================================
# Measurement B2 -- the positive control of the sixth and the seventh strings. Task T-41-12,
# finding H134; the seventh by decision 143d.
#
# The self-termination of FR-97 is behind `#[cfg(debug_assertions)]`, and so is the debug
# substitution of the font step (task T-83-3); the Release profile
# now says `debug-assertions = false` out loud. A mechanism switched off by a default nobody
# wrote down was the finding; a mechanism switched off by a line in the profile is a decision.
#
# This measurement is what makes the absence in A mean something: built with debug assertions
# forced ON through RUSTFLAGS, which overrides the profile key (measured: the string appears),
# the binary MUST carry the variable. Without this half, a typo in the needle would read exactly
# like a mechanism that is not there.
# ==========================================================================================
$debugControlled = @($Strings | Where-Object { $_.Control -eq 'debug-assertions' })
if ($debugControlled.Count -gt 0) {
    Write-Host ''
    Write-Host '--- B2. Release with debug assertions forced ON -- POSITIVE CONTROL ----'
    Write-Host 'RUSTFLAGS overrides the profile key, so this is the build a person chasing a bug'
    Write-Host 'would have made -- and the one that used to pass this criterion in silence.'
    $savedFlags = $env:RUSTFLAGS
    try {
        $env:RUSTFLAGS = '-C debug-assertions=on'
        Invoke-Cargo -CargoArgs @('build', '--release') -What 'cargo build --release (debug assertions on)'
    } finally {
        if ($null -eq $savedFlags) {
            Remove-Item Env:\RUSTFLAGS -ErrorAction SilentlyContinue
        } else {
            $env:RUSTFLAGS = $savedFlags
        }
    }
    $hashAsserted = Measure-Binary -Path $ReleaseExe -Label 'Release, debug assertions on:'
    $assertedBytes = [System.IO.File]::ReadAllBytes($ReleaseExe)
    $asserted = ConvertTo-ByteString $assertedBytes

    Write-Host ''
    Write-Host 'Expected: FOUND in at least one encoding.'
    foreach ($s in $debugControlled) {
        $u8 = Test-BinaryHasString -Haystack $asserted -Value $s.Value -Encoding 'UTF-8'
        $u16 = Test-BinaryHasString -Haystack $asserted -Value $s.Value -Encoding 'UTF-16LE'
        $found = ($u8 -or $u16)
        $where = 'none'
        if ($u8 -and $u16) { $where = 'UTF-8 + UTF-16LE' }
        elseif ($u8)       { $where = 'UTF-8' }
        elseif ($u16)      { $where = 'UTF-16LE' }
        $verdict = 'PRESENT'
        if (-not $found) { $verdict = 'MISSING' }
        Write-Host ("  {0,-8} {1,-38} found in: {2}" -f $verdict, $s.Name, $where)
        if (-not $found) {
            $Failures.Add(("the positive control failed for `"{0}`": the build with debug assertions ON does not carry it either, so the absence measured in A means nothing" -f $s.Name))
        }
    }

    if ($hashAsserted -eq $hashShipping) {
        $Failures.Add('the build with debug assertions on hashed the same as the shipping one, which means it did not replace it on disk')
    }
}

# ==========================================================================================
# The third part of the criterion -- the set of dependencies.
# ==========================================================================================
Write-Host ''
Write-Host '--- C. Dependency set: cargo tree vs cargo tree --features testing -----'
Push-Location $ProjectDir
try {
    $treePlain = & cargo tree
    $treePlainCode = $LASTEXITCODE
    $treeTesting = & cargo tree --features testing
    $treeTestingCode = $LASTEXITCODE
} finally {
    Pop-Location
}
if ($treePlainCode -ne 0)   { throw "cargo tree failed with exit code $treePlainCode" }
if ($treeTestingCode -ne 0) { throw "cargo tree --features testing failed with exit code $treeTestingCode" }

# Every non-ASCII character is discarded before the comparison: the tree is drawn with box
# characters, the console mangles them, and comparing the raw text reports differences that
# belong to the terminal and not to the dependency graph. Section 8.4 of STATE.md.
function Get-AsciiSkeleton {
    param([string[]]$Lines)
    $out = New-Object System.Collections.Generic.List[string]
    foreach ($line in $Lines) {
        $clean = ($line -replace '[^\x20-\x7E]', '').Trim()
        if ($clean -ne '') { $out.Add($clean) }
    }
    return $out.ToArray()
}

$skeletonPlain   = Get-AsciiSkeleton -Lines $treePlain
$skeletonTesting = Get-AsciiSkeleton -Lines $treeTesting

Write-Host ("  cargo tree                    {0} lines (non-ASCII discarded)" -f $skeletonPlain.Count)
Write-Host ("  cargo tree --features testing {0} lines (non-ASCII discarded)" -f $skeletonTesting.Count)

$treeDiff = Compare-Object -ReferenceObject $skeletonPlain -DifferenceObject $skeletonTesting -SyncWindow 0
if ($treeDiff) {
    Write-Host '  DIFFERENT -- the two dependency sets do not agree:'
    foreach ($d in $treeDiff) {
        Write-Host ("    {0} {1}" -f $d.SideIndicator, $d.InputObject)
    }
    $Failures.Add('cargo tree and cargo tree --features testing differ in the set of dependencies')
} else {
    Write-Host '  IDENTICAL -- the `testing` feature pulls in no crate of its own.'
}

# ==========================================================================================
# The shipping configuration is left on disk. The next step is the signature.
# ==========================================================================================
Write-Host ''
Write-Host '--- D. Final rebuild of the shipping configuration ---------------------'
if ($SkipFinalRebuild) {
    Write-Host '  SKIPPED by -SkipFinalRebuild. The binary on disk CARRIES the `testing` feature.'
    Write-Host '  Do not sign it and do not package it.'
} else {
    Invoke-Cargo -CargoArgs @('build', '--release') -What 'cargo build --release'
    $hashFinal = Measure-Binary -Path $ReleaseExe -Label 'Release on disk after the final rebuild:'
    if ($hashFinal -ne $hashShipping) {
        Write-Host ''
        Write-Host '  NOTE: this hash differs from the one measured in A. That is a statement about'
        Write-Host '  build reproducibility (SEC-08), not about criterion 8, and it is reported'
        Write-Host '  rather than failed here.'
    } else {
        Write-Host ''
        Write-Host '  Same SHA-256 as measurement A: the shipping binary is byte for byte the one'
        Write-Host '  the seven strings were measured against.'
    }
    if ($hashFinal -eq $hashTesting) {
        $Failures.Add('after the final rebuild the binary on disk is still the one with the `testing` feature')
    }
}

# ==========================================================================================
Write-Host ''
Write-Host '======================================================================'
if ($Failures.Count -eq 0) {
    Write-Host ' RESULT: PASS'
    Write-Host ' All seven strings are absent from the shipping build; the five of the `testing`'
    Write-Host ' feature are present in the build with it, the sixth and the seventh are present in'
    Write-Host ' the build with debug assertions on, and the dependency sets agree.'
    Write-Host '======================================================================'
    exit 0
}

Write-Host (' RESULT: FAIL -- {0} reason(s)' -f $Failures.Count)
foreach ($f in $Failures) { Write-Host ("  * " + $f) }
Write-Host '======================================================================'
exit 1
