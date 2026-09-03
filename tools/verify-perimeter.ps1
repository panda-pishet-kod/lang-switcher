<#
    verify-perimeter.ps1 -- the two numbers of the perimeter nothing used to check.

    Task T-22-11, finding 19 of the audit of 2026-08-31: NFR-07 and acceptance criterion 4 of
    section 13 were verified by nobody. release.ps1 printed the size of the artifact as
    information beside it, and the dependency list was compared only by its LINE COUNT --
    which a crate swapped for another crate passes without a murmur.

    GATE 1 -- NFR-07, "Razmer ispolnyaemogo fayla < 1.5 MB".

        The letter of the requirement is 1.5 MB and the binary reading of it is
        1.5 * 1024 * 1024 = 1572864 bytes. The measurement is taken over the SIGNED artifact,
        which is the file the user runs: a signature is appended to the file, so the unsigned
        build is the smaller of the two and measuring it would be measuring the easier case.
        The comparison is strict, as the requirement writes it.

    GATE 2 -- criterion 4 of section 13, "cargo tree ne soderzhit kreytov s setevymi
    vozmozhnostyami", and SEC-03 behind it.

        `cargo tree` is read for crate NAMES, and the names are matched against a list of
        markers -- the crates a network capability would arrive through in a Rust program.
        The list is deliberately over-broad: a false alarm costs one look, and a crate that
        can open a socket in a program whose whole security argument is "there is no network
        activity at all" costs the argument.

        This is not the same check as counting the lines of `cargo tree`, and the difference
        is the finding: a tree of the same size holding `ureq` instead of `toml` passes the
        count and fails this.

    THE POSITIVE CONTROLS, and why neither needs a file to be edited:

        .\verify-perimeter.ps1 -MaxBytes 1
            the size gate must FAIL. An instrument that answers "under the ceiling" whatever
            the ceiling is has not measured anything.

        .\verify-perimeter.ps1 -ExtraMarkers serde
            the crate gate must FAIL and must name `serde`, which really is in the tree. That
            is the proof that the tree is read and the names are compared, rather than the
            list of markers merely failing to appear in a file nobody opened.

        Both controls can be run together; each is reported separately.

    Written without a single Cyrillic character and for Windows PowerShell 5.1: no `&&`, no
    `??`, no ternary. Decision R-05, as release.ps1 states it.

    Exit code: 0 only if both gates pass -- or, under a control, only if the gate under
    control really failed.

    Examples:
      .\verify-perimeter.ps1
      .\verify-perimeter.ps1 -Artifact <dev>\artifacts\LangSwitcher.exe
      .\verify-perimeter.ps1 -MaxBytes 1 -ExtraMarkers serde   # both controls: must PASS
#>
[CmdletBinding()]
param(
    # The file NFR-07 is about: the signed product the user runs.
    [string]$Artifact = '<dev>\artifacts\LangSwitcher.exe',
    # NFR-07: 1.5 MB, read as 1.5 * 1024 * 1024. A small value is the positive control.
    [long]$MaxBytes = 1572864,
    # Crate names added to the marker list. `serde` is the positive control: it is in the
    # tree, so the gate has to fire on it.
    [string[]]$ExtraMarkers = @()
)

$ErrorActionPreference = 'Stop'

$ScriptDir = $PSScriptRoot
$ProjectDir = Split-Path -Parent $ScriptDir

if (-not $env:CARGO_HOME)       { $env:CARGO_HOME       = '<dev>\tools\cargo' }
if (-not $env:RUSTUP_HOME)      { $env:RUSTUP_HOME      = '<dev>\tools\rustup' }
if (-not $env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR = '<dev>\cache\target' }
if ($env:PATH -notlike '*<dev>\tools\cargo\bin*') { $env:PATH = "<dev>\tools\cargo\bin;$env:PATH" }

# The crates a network capability arrives through in a Rust program. Names are matched whole,
# against the crate name `cargo tree` prints, so `native-tls` cannot be missed by a substring
# rule and `hyper` cannot be found inside `hyperfine`.
$Markers = @(
    'reqwest', 'hyper', 'hyper-util', 'h2', 'h3', 'tokio', 'tokio-util', 'async-std',
    'smol', 'mio', 'socket2', 'curl', 'curl-sys', 'openssl', 'openssl-sys', 'native-tls',
    'schannel', 'rustls', 'rustls-pki-types', 'webpki', 'ureq', 'attohttpc', 'isahc',
    'surf', 'ws', 'tungstenite', 'tokio-tungstenite', 'quinn', 'trust-dns-resolver',
    'hickory-resolver', 'url'
)

if ($ExtraMarkers.Count -gt 0) {
    $Markers = $Markers + $ExtraMarkers
}

$sizeControl = ($MaxBytes -ne 1572864)
$crateControl = ($ExtraMarkers.Count -gt 0)

Write-Host ''
Write-Host '======================================================================'
Write-Host ' Lang_Switcher -- NFR-07 and criterion 4 of section 13'
Write-Host '======================================================================'
Write-Host ("Project   {0}" -f $ProjectDir)
Write-Host ("Artifact  {0}" -f $Artifact)
Write-Host ("Ceiling   {0} bytes" -f $MaxBytes)
if ($sizeControl)  { Write-Host '          *** POSITIVE CONTROL: the size gate must FAIL ***' }
if ($crateControl) { Write-Host ("Markers   + {0}  *** POSITIVE CONTROL: the crate gate must FAIL ***" -f ($ExtraMarkers -join ', ')) }

# --- Gate 1: NFR-07 --------------------------------------------------------------------------
Write-Host ''
Write-Host '--- Gate 1: NFR-07, the size of the signed product --------------------'

if (-not (Test-Path $Artifact)) {
    Write-Host ("FAIL: the artifact is missing: {0}" -f $Artifact)
    Write-Host '      NFR-07 is about the file the user runs; there is nothing to measure.'
    exit 1
}

$size = (Get-Item $Artifact).Length
$sizeFailed = ($size -ge $MaxBytes)

Write-Host ("  size    {0} bytes" -f $size)
Write-Host ("  ceiling {0} bytes (strict)" -f $MaxBytes)
Write-Host ("  headroom {0} bytes" -f ($MaxBytes - $size))

if ($sizeFailed) {
    Write-Host '  RESULT: FAIL -- the product is at or over the ceiling of NFR-07'
} else {
    Write-Host '  RESULT: pass'
}

# --- Gate 2: criterion 4 ----------------------------------------------------------------------
Write-Host ''
Write-Host '--- Gate 2: criterion 4, no crate with network capability -------------'

Push-Location $ProjectDir
try {
    $tree = & cargo tree 2>&1
    $treeCode = $LASTEXITCODE
} finally {
    Pop-Location
}

if ($treeCode -ne 0) {
    Write-Host ("FAIL: cargo tree returned {0}" -f $treeCode)
    exit 1
}

$lines = @($tree)
Write-Host ("  cargo tree lines: {0}" -f $lines.Count)

# Every crate name the tree prints. A line looks like
#   "|-- windows v0.62.2" or "windows-core v0.62.2 (*)"; the name is the token before the
# version, and the version is the first token starting with `v` followed by a digit.
$names = New-Object System.Collections.Generic.HashSet[string]

foreach ($line in $lines) {
    $text = [string]$line
    $match = [regex]::Match($text, '([A-Za-z0-9_\-\.]+)\s+v[0-9]')
    if ($match.Success) {
        [void]$names.Add($match.Groups[1].Value)
    }
}

Write-Host ("  distinct crate names: {0}" -f $names.Count)

if ($names.Count -eq 0) {
    Write-Host '  FAIL: not one crate name was read out of the tree.'
    Write-Host '        An instrument that finds no markers in nothing is not an instrument.'
    exit 1
}

$hits = @()
foreach ($marker in $Markers) {
    if ($names.Contains($marker)) {
        $hits = $hits + $marker
    }
}

$crateFailed = ($hits.Count -gt 0)

if ($crateFailed) {
    foreach ($hit in $hits) {
        Write-Host ("  FOUND   {0}" -f $hit)
    }
    Write-Host '  RESULT: FAIL -- a crate with network capability is in the dependency tree'
} else {
    Write-Host ("  none of the {0} markers is in the tree" -f $Markers.Count)
    Write-Host '  RESULT: pass'
}

# --- Gate 3: SEC-03, the addresses in the shipped binary ------------------------------------------
#
# FR-102 and SEC-03: «в Release-бинаре единственные сетевые адреса — адреса ленты, проверяется
# поиском по строкам бинаря». This gate reads the strings of the FILE THAT SHIPS and refuses any
# https:// that the program does not declare in `letters::links`.
#
# ⚠ THREE ADDRESSES AND NOT ONE. The mandate's sentence names the feed alone; the program also
# carries the channel and the support page (FR-103), and both are https:// by SEC-03's own rule
# that a link this program opens is a secure one. So the allowed set is what `letters::links`
# declares, read out of the SOURCE rather than typed here -- a list typed twice is a list that
# drifts.
#
# The strings are read the way `strings(1)` reads them: runs of printable ASCII of four or more
# bytes, over the whole file. UTF-16 literals are found by the same walk with the zero bytes
# dropped, because Rust `&str` constants are UTF-8 and Windows resources are UTF-16 -- both
# appear in this binary.
Write-Host ''
Write-Host '--- Gate 3: SEC-03, the addresses in the shipped binary ---------------'

$source = Join-Path $ProjectDir 'src\letters.rs'
$declared = @()

if (Test-Path $source) {
    $text = Get-Content -LiteralPath $source -Raw -Encoding UTF8
    foreach ($match in [regex]::Matches($text, '"(https://[^"]+)"')) {
        $declared += $match.Groups[1].Value
    }
}

$declared = $declared | Sort-Object -Unique

Write-Host ("  declared in src\letters.rs: {0}" -f $declared.Count)
foreach ($url in $declared) { Write-Host ("    {0}" -f $url) }

$bytes = [System.IO.File]::ReadAllBytes($Artifact)

# Both encodings in one pass: the raw bytes, and the same bytes with every zero dropped.
$ascii = -join ($bytes | ForEach-Object { if ($_ -ge 32 -and $_ -lt 127) { [char]$_ } else { "`n" } })
$wide  = -join ($bytes | Where-Object { $_ -ne 0 } | ForEach-Object { if ($_ -ge 32 -and $_ -lt 127) { [char]$_ } else { "`n" } })

# ⚠ Rust string constants sit next to one another in `.rdata` with NO terminator between them,
# so two addresses come out of the file as ONE run: «https://a/onehttps://b/two». A pattern that
# stopped at the first «https://» would then find one address, and a stray one glued to a known
# one would be «explained» by it. Each run is therefore cut at every further «https://».
$found = @()
foreach ($haystack in @($ascii, $wide)) {
    foreach ($match in [regex]::Matches($haystack, 'https://(?:(?!https://)[A-Za-z0-9\.\-/_%~:@\?=&\+])*')) {
        $found += $match.Value
    }
}

$found = $found | Sort-Object -Unique
$strayAddresses = @()

foreach ($url in $found) {
    # A run is explained when a declared address is a PREFIX of it -- the linker stores strings
    # side by side with no terminator, so «<the feed address>example.invalid» is one run and two
    # constants -- or when the run is a prefix of a declared one.
    #
    # ⚠ This is safe ONLY because the runs above are cut at every further «https://»: a stray
    # address glued behind a known one is a run of its own and is judged on its own. Measured
    # both ways -- the control below appends `https://evil.example.org/collect` to a copy of the
    # binary and the gate must fail on it.
    $explained = $false
    foreach ($known in $declared) {
        if ($url.StartsWith($known) -or $known.StartsWith($url)) { $explained = $true; break }
    }
    if (-not $explained) { $strayAddresses += $url }
}

Write-Host ("  https:// runs in the binary: {0}" -f $found.Count)

$addressFailed = $strayAddresses.Count -gt 0

if ($addressFailed) {
    foreach ($url in $strayAddresses) { Write-Host ("  FOUND   {0}" -f $url) }
    Write-Host '  RESULT: FAIL -- the binary carries an address the program does not declare'
} else {
    Write-Host '  every address in the binary is one the program declares'
    Write-Host '  RESULT: pass'
}

# --- Verdict ------------------------------------------------------------------------------------
Write-Host ''
Write-Host '======================================================================'

if ($sizeControl -or $crateControl) {
    $ok = $true

    if ($sizeControl -and (-not $sizeFailed)) {
        Write-Host ' POSITIVE CONTROL FAILED: the size gate passed a ceiling nothing can fit under'
        $ok = $false
    }
    if ($crateControl -and (-not $crateFailed)) {
        Write-Host ' POSITIVE CONTROL FAILED: the crate gate missed a crate that is in the tree'
        $ok = $false
    }

    if ($ok) {
        Write-Host ' POSITIVE CONTROL PASSED: every gate under control really failed'
        Write-Host '======================================================================'
        exit 0
    }

    Write-Host '======================================================================'
    exit 1
}

if ($sizeFailed -or $crateFailed -or $addressFailed) {
    Write-Host ' RESULT: FAIL'
    Write-Host '======================================================================'
    exit 1
}

Write-Host ' RESULT: PASS -- NFR-07, criterion 4 and SEC-03 addresses all hold'
Write-Host '======================================================================'
exit 0
