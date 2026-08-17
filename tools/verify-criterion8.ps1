<#
    verify-criterion8.ps1 -- acceptance criterion 8 of section 13 of SPEC, measured.

    The criterion, word for word:

      "The Release binary carries no trace of the SEC-04a debug channel: the build was made
       without the `testing` feature, a search over the strings of the binary does not find
       the name of the named channel, and `Cargo.lock` and `cargo tree --features testing`
       are checked against each other for differences in the set of dependencies."

    WHAT THIS SCRIPT DOES, and why each part is not decoration.

    1. It REBUILDS EACH CONFIGURATION IMMEDIATELY BEFORE ITS OWN MEASUREMENT. Both
       configurations write the same file, <dev>\cache\target\release\LangSwitcher.exe, so a
       measurement taken without a build in front of it measures the leftover of the previous
       one. The controller was caught by exactly this while checking task T-03-4-2: the first
       "release without testing" measurement found the channel name, because what lay on disk
       was the build WITH the feature. Fact 8 of section 9 of STATE.md.

    2. It runs a POSITIVE CONTROL over every one of the five strings. Absence on its own
       proves nothing at all: the Release profile has `strip = true`, and a single typo in a
       search pattern gives the same answer as a string that is genuinely not there. Each
       string must be FOUND in the build with the feature and MISSING from the build without
       it. Decision R-25, question 29.

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

    THE LIST OF FIVE STRINGS IS CLOSED by decisions R-28 and R-53. Do not shorten it and do not
    extend it: if a sixth appears, that is a decision for the controller, not for this file.

    Written without a single Cyrillic character on purpose. The tooling that writes files here
    puts them in UTF-8 with NO byte order mark, Windows PowerShell 5.1 then reads such a .ps1
    in the system ANSI code page, and the parse dies on the first Russian word. Decision R-05.
    Windows PowerShell 5.1, not 7.x: no `&&`, no `??`, no ternary operator, no -AsHashtable.

    Exit code: 0 only if all five strings are absent without the feature, all five are present
    with it, and the dependency sets agree. Otherwise non-zero, with the reasons listed.

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

if (-not $ProjectDir) { $ProjectDir = Split-Path -Parent $PSScriptRoot }
$ProjectDir = (Resolve-Path $ProjectDir).Path

# --- Environment --------------------------------------------------------------------------
# Set only what is not already set. A caller building in an isolated target directory
# (<dev>\cache\target-<id>, decision R-27) must keep it: overriding CARGO_TARGET_DIR here
# would send that caller's artifacts into the shared tree behind its back.
if (-not $env:CARGO_HOME)       { $env:CARGO_HOME       = '<dev>\tools\cargo' }
if (-not $env:RUSTUP_HOME)      { $env:RUSTUP_HOME      = '<dev>\tools\rustup' }
if (-not $env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR = '<dev>\cache\target' }
if ($env:PATH -notlike '*<dev>\tools\cargo\bin*') {
    $env:PATH = "<dev>\tools\cargo\bin;$env:PATH"
}

$ReleaseExe = Join-Path $env:CARGO_TARGET_DIR 'release\LangSwitcher.exe'

# --- The five strings -------------------------------------------------------------------
# Position 1 is copied character for character from `PIPE_NAME_PREFIX` in src\control.rs,
# which declares it as a raw literal r"\\.\pipe\Lang_Switcher.control." -- the backslashes are
# part of the value. A single-quoted PowerShell string keeps them as they are; a
# double-quoted one would too, but single quotes make that impossible to get wrong later.
$Strings = @(
    @{ Name = 'SEC-04a channel name';        Value = '\\.\pipe\Lang_Switcher.control.'; Source = 'src\control.rs, PIPE_NAME_PREFIX' },
    @{ Name = 'LANGSW_TESTING_REPORT';           Value = 'LANGSW_TESTING_REPORT';           Source = 'src\app.rs' },
    @{ Name = 'LANGSW_TESTING_PANIC_ON_HANDOFF'; Value = 'LANGSW_TESTING_PANIC_ON_HANDOFF'; Source = 'src\hook.rs' },
    @{ Name = 'LANGSW_TESTING_PANIC_ON_VK';      Value = 'LANGSW_TESTING_PANIC_ON_VK';      Source = 'src\hook.rs' },
    @{ Name = 'LANGSW_TESTING_DROP_HOOK_MS';     Value = 'LANGSW_TESTING_DROP_HOOK_MS';     Source = 'src\watchdog.rs' }
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
Write-Host ("Target dir     {0}" -f $env:CARGO_TARGET_DIR)
Write-Host ("Release binary {0}" -f $ReleaseExe)

# ==========================================================================================
# Measurement A -- the shipping configuration. All five strings must be ABSENT.
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
Write-Host 'Expected: FOUND in at least one encoding.'
foreach ($s in $Strings) {
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
        Write-Host '  the five strings were measured against.'
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
    Write-Host ' All five strings are absent from the shipping build, all five are present in'
    Write-Host ' the build with the feature, and the dependency sets agree.'
    Write-Host '======================================================================'
    exit 0
}

Write-Host (' RESULT: FAIL -- {0} reason(s)' -f $Failures.Count)
foreach ($f in $Failures) { Write-Host ("  * " + $f) }
Write-Host '======================================================================'
exit 1
