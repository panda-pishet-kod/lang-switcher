<#
    verify-perimeter.ps1 -- the two numbers of the perimeter nothing used to check.

    Task T-22-11, finding 19 of the audit of 2026-08-31: NFR-07 and acceptance criterion 4 of
    section 13 were verified by nobody. release.ps1 printed the size of the artifact as
    information beside it, and the dependency list was compared only by its LINE COUNT --
    which a crate swapped for another crate passes without a murmur.

    GATE 1 -- NFR-07, "Razmer ispolnyaemogo fayla": INFORMATION ONLY since decision 106.2.

        The ceiling of 1.5 MB was REMOVED by the user on 2026-09-04 -- "snyat potolok, on
        ogranichivaet, a ne pomogaet" (decision 106.2, task T-45-0). The requirement now reads
        "predela net; razmer podpisannogo artefakta izmeryaetsya i pechataetsya pri kazhdoy
        postavke, prirost protiv prezhney postavki -- v otchet".

        So this gate MEASURES and PRINTS, and never fails on the number itself. The size is
        still taken over the SIGNED artifact, which is the file the user runs: a signature is
        appended to the file, so the unsigned build is the smaller of the two and measuring it
        would be measuring the easier case. -Previous <bytes> adds the growth line the report
        asks for; without it the growth is printed as "not given".

        The one thing that still FAILS here is a measurement that could not be taken at all:
        a missing artifact. That is what keeps the gate honest -- an instrument that cannot
        fail under any circumstance has not measured anything.

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

        .\verify-perimeter.ps1 -Artifact .\no-such-file.exe
            gate 1 must FAIL, naming the file it could not measure. This replaced the old
            `-MaxBytes 1` control when the ceiling went away (task T-45-0): the gate no longer
            judges the number, so the only thing left to prove is that it really opens the
            file and really reads its length.

        .\verify-perimeter.ps1 -ExtraMarkers serde
            the crate gate must FAIL and must name `serde`, which really is in the tree. That
            is the proof that the tree is read and the names are compared, rather than the
            list of markers merely failing to appear in a file nobody opened.

        Both controls can be run together; each is reported separately.

    Written without a single Cyrillic character and for Windows PowerShell 5.1: no `&&`, no
    `??`, no ternary. Decision R-05, as release.ps1 states it.

    THE ARTIFACT (stage E89, decision 152.3). -Artifact names it. Without it the file is
    LangSwitcher.exe in the folder the LANGSW_ARTIFACTS environment variable names, and without
    that variable in <repository>\dist\ -- the rule release.ps1 writes by. The variable is read
    after tools\local.ps1, the untracked file of this machine's own folders.

    GATE 4 (stage E90, debt E89-B-2) -- the folders of the build machine are not in the image.
    The compiler writes absolute source paths into the panic locations of a Rust program; the
    e89 image carried forty-seven of them, under CARGO_HOME and under the toolchain's sysroot.
    release.ps1 now remaps both; this gate reads the signed file and refuses it if it still
    names CARGO_HOME, RUSTUP_HOME, the sysroot of rustc, the project folder or USERPROFILE --
    byte for byte, in UTF-8 and UTF-16LE, without regard to case. Positive control:
    -ExtraPathNeedles '/rustc/' -- every Rust image carries that prefix, so the gate must FAIL.

    Exit code: 0 only if the artifact could be measured and gates 2, 3 and 4 pass -- or, under
    a control, only if the gate under control really failed.

    Examples:
      .\verify-perimeter.ps1
      .\verify-perimeter.ps1 -Artifact ..\dist\LangSwitcher.exe
      .\verify-perimeter.ps1 -Previous 1413240              # prints the growth over e44
      .\verify-perimeter.ps1 -ExtraMarkers serde            # crate control: must PASS
      .\verify-perimeter.ps1 -ExtraPathNeedles '/rustc/'    # path control: must PASS
#>
[CmdletBinding()]
param(
    # The file NFR-07 is about: the signed product the user runs. Empty means LangSwitcher.exe
    # in the artifact folder; see the header.
    [string]$Artifact = '',
    # Size of the artifact of the PREVIOUS delivery, in bytes. Given, it turns into the growth
    # line decision 106.2 asks every delivery report to carry. Zero means "not given".
    [long]$Previous = 0,
    # Crate names added to the marker list. `serde` is the positive control: it is in the
    # tree, so the gate has to fire on it.
    [string[]]$ExtraMarkers = @(),
    # Strings added to the folders gate 4 looks for. '/rustc/' is the positive control: every
    # Rust image carries it, so the gate has to fire on it.
    [string[]]$ExtraPathNeedles = @()
)

$ErrorActionPreference = 'Stop'

# --- This machine's own folders (stage E89) ------------------------------------------------
# First, so that the LANGSW_ARTIFACTS it sets is the one read below. See the header.
if (Test-Path "$PSScriptRoot\local.ps1") { . "$PSScriptRoot\local.ps1" }

$ScriptDir = $PSScriptRoot
$ProjectDir = Split-Path -Parent $ScriptDir

# cargo comes from PATH, with cargo's own defaults for whatever CARGO_* is not set (stage E89).
if (-not $Artifact) {
    $artifactDir = $env:LANGSW_ARTIFACTS
    if (-not $artifactDir) { $artifactDir = Join-Path $ProjectDir 'dist' }
    $Artifact = Join-Path $artifactDir 'LangSwitcher.exe'
}

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

$crateControl = ($ExtraMarkers.Count -gt 0)

Write-Host ''
Write-Host '======================================================================'
Write-Host ' Lang_Switcher -- NFR-07 and criterion 4 of section 13'
Write-Host '======================================================================'
Write-Host ("Project   {0}" -f $ProjectDir)
Write-Host ("Artifact  {0}" -f $Artifact)
Write-Host 'Ceiling   none -- NFR-07 has no limit since decision 106.2'
if ($crateControl) { Write-Host ("Markers   + {0}  *** POSITIVE CONTROL: the crate gate must FAIL ***" -f ($ExtraMarkers -join ', ')) }

# --- Gate 1: NFR-07 --------------------------------------------------------------------------
Write-Host ''
Write-Host '--- Gate 1: NFR-07, the size of the signed product (information) ------'

if (-not (Test-Path $Artifact)) {
    Write-Host ("FAIL: the artifact is missing: {0}" -f $Artifact)
    Write-Host '      NFR-07 is about the file the user runs; there is nothing to measure.'
    exit 1
}

$size = (Get-Item $Artifact).Length

Write-Host ("  size    {0} bytes" -f $size)
if ($Previous -gt 0) {
    $growth = $size - $Previous
    $sign = '+'
    if ($growth -lt 0) { $sign = '' }
    Write-Host ("  previous {0} bytes" -f $Previous)
    Write-Host ("  growth   {0}{1} bytes" -f $sign, $growth)
} else {
    Write-Host '  previous not given -- pass -Previous <bytes> to print the growth'
}
Write-Host '  RESULT: information only -- NFR-07 has no ceiling (decision 106.2)'

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
# ⚠ FIVE ADDRESSES AND NOT ONE. The mandate's sentence names the feed alone; the program also
# carries the channel, the support page, the download page and its own page (FR-103, решение
# 139.2), and every one of them is https:// by SEC-03's own rule that a link this program opens
# is a secure one. So the allowed set is what `letters::links` declares, read out of the SOURCE
# rather than typed here -- a list typed twice is a list that drifts.
#
# ⭐ That is why task T-78-1 changed four addresses and NOT ONE LINE of this gate: the two new
# constants and the two new values were picked up by the read below the moment they were written.
# A hand-kept list here would have been the second place to forget them.
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

# --- Gate 4: the build machine's folders are not in the image (stage E90) -----------------------
#
# The needles are the folders of THIS machine, asked of the environment and of rustc at run time --
# never typed here, so the gate means the same on any machine. CARGO_HOME and RUSTUP_HOME fall back
# to cargo's own defaults under USERPROFILE, and USERPROFILE itself is a needle: a user name in a
# shipped image is the commonest leak of a Rust program. The file is read once and turned into one
# character per byte through ISO-8859-1 (the trick of verify-criterion8.ps1); each needle is looked
# for as UTF-8 and as UTF-16LE bytes, both sides folded to lower case the same way.
Write-Host ''
Write-Host '--- Gate 4: the build machine''s folders are not in the image ----------'

$pathNeedles = New-Object System.Collections.Generic.List[string]
$cargoHomeNeedle = $env:CARGO_HOME
if ((-not $cargoHomeNeedle) -and $env:USERPROFILE) { $cargoHomeNeedle = Join-Path $env:USERPROFILE '.cargo' }
$rustupHomeNeedle = $env:RUSTUP_HOME
if ((-not $rustupHomeNeedle) -and $env:USERPROFILE) { $rustupHomeNeedle = Join-Path $env:USERPROFILE '.rustup' }
$sysrootNeedle = ''
try { $sysrootNeedle = (& rustc --print sysroot | Out-String).Trim() } catch { $sysrootNeedle = '' }
foreach ($folder in @($cargoHomeNeedle, $rustupHomeNeedle, $sysrootNeedle, $ProjectDir, $env:USERPROFILE)) {
    if ($folder -and -not $pathNeedles.Contains($folder.TrimEnd('\'))) { $pathNeedles.Add($folder.TrimEnd('\')) }
}
foreach ($extra in $ExtraPathNeedles) {
    if ($extra) { $pathNeedles.Add($extra) }
}
$pathControl = ($ExtraPathNeedles.Count -gt 0)
if ($pathControl) { Write-Host ("  + {0}  *** POSITIVE CONTROL: the path gate must FAIL ***" -f ($ExtraPathNeedles -join ', ')) }

$latin1 = [System.Text.Encoding]::GetEncoding(28591)
$haystack = $latin1.GetString($bytes).ToLowerInvariant()

function Measure-Needle([string]$haystack, [string]$needle) {
    $count = 0
    $at = $haystack.IndexOf($needle, [System.StringComparison]::Ordinal)
    while ($at -ge 0) {
        $count++
        $at = $haystack.IndexOf($needle, $at + 1, [System.StringComparison]::Ordinal)
    }
    return $count
}

$pathHits = 0
foreach ($needle in $pathNeedles) {
    $u8 = Measure-Needle $haystack ($latin1.GetString([System.Text.Encoding]::UTF8.GetBytes($needle)).ToLowerInvariant())
    $u16 = Measure-Needle $haystack ($latin1.GetString([System.Text.Encoding]::Unicode.GetBytes($needle)).ToLowerInvariant())
    $pathHits += $u8 + $u16
    $verdict = 'absent'
    if (($u8 + $u16) -gt 0) { $verdict = 'FOUND' }
    Write-Host ("  {0,-7} {1,-66} UTF-8 {2,3}  UTF-16LE {3,3}" -f $verdict, $needle, $u8, $u16)
}

$pathFailed = ($pathHits -gt 0)
if ($pathNeedles.Count -eq 0) {
    Write-Host '  FAIL: not one folder to look for -- the gate would have measured nothing.'
    $pathFailed = $true
} elseif ($pathFailed) {
    Write-Host ("  RESULT: FAIL -- the image names a folder of the build machine {0} time(s)" -f $pathHits)
} else {
    Write-Host ("  none of the {0} folders is in the image" -f $pathNeedles.Count)
    Write-Host '  RESULT: pass'
}

# --- Verdict ------------------------------------------------------------------------------------
Write-Host ''
Write-Host '======================================================================'

if ($crateControl) {
    if ($crateFailed) {
        Write-Host ' POSITIVE CONTROL PASSED: the gate under control really failed'
        Write-Host '======================================================================'
        exit 0
    }

    Write-Host ' POSITIVE CONTROL FAILED: the crate gate missed a crate that is in the tree'
    Write-Host '======================================================================'
    exit 1
}

if ($pathControl) {
    if ($pathFailed) {
        Write-Host ' POSITIVE CONTROL PASSED: the path gate really failed'
        Write-Host '======================================================================'
        exit 0
    }

    Write-Host ' POSITIVE CONTROL FAILED: the path gate missed a string that is in every Rust image'
    Write-Host '======================================================================'
    exit 1
}

if ($crateFailed -or $addressFailed -or $pathFailed) {
    Write-Host ' RESULT: FAIL'
    Write-Host '======================================================================'
    exit 1
}

Write-Host ' RESULT: PASS -- criterion 4, SEC-03 addresses and the build folders hold; NFR-07 measured above'
Write-Host '======================================================================'
exit 0
