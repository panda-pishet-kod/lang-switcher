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

    GATE 3 (task T-22-11; split by decision 181.37, stage E103) -- SEC-03, every https:// in the
    file is one it may carry. The file is TWO texts: the image the compiler made, and the
    certificate table signtool appended (the PE security directory). The image is judged against
    the addresses src\letters.rs declares; the certificate table against the one address the
    signature names, /du of signtool. Until stage E103 the whole file was judged against
    letters.rs alone, and the /du address passed only because it happened to be the program's own
    page (links::PROGRAM_URL); when that address took its mark ?lang=ru (decision 181.35), the
    signature's address was left unexplained and the gate failed on a string the program does
    not carry. Positive control: -SignUrlControl 'https://example.invalid/' -- the signature is
    then judged against an address it does not name, so on a signed file the gate must FAIL.

    GATE 5 (stage E104, question 190) -- the C runtime of MSVC is inside the image. Until 0.103.0
    every image imported VCRUNTIME140.dll and five api-ms-win-crt-* libraries and did not start on
    a Windows without the Visual C++ package (finding 188.25). The gate reads the import table and
    the delay-import table of the PE header itself and refuses VCRUNTIME*.dll, MSVCP*.dll and
    api-ms-win-crt-*.dll. Positive control: -ExtraImportNeedles KERNEL32.dll -- every image
    imports it, so the gate must FAIL.
    GATE 6 (stage E104, TZ E104 p. 22) -- the test doors of the updater (a feed on the loopback, a
    key and a folder of the stand) compile only under the `testing` feature; the gate refuses an
    image that names localhost, 127.0.0.1, update-doors.toml or LANGSW_UPDATE_, in ASCII and
    UTF-16LE. Positive control: -ExtraDoorNeedles LangSwitcher -- every image names itself.
    Exit code: 0 only if the artifact could be measured and gates 2 to 6 pass -- or, under
    a control, only if the gate under control really failed.

    Examples:
      .\verify-perimeter.ps1
      .\verify-perimeter.ps1 -Artifact ..\dist\LangSwitcher.exe
      .\verify-perimeter.ps1 -Previous 1413240              # prints the growth over e44
      .\verify-perimeter.ps1 -ExtraMarkers serde            # crate control: must PASS
      .\verify-perimeter.ps1 -ExtraPathNeedles '/rustc/'    # path control: must PASS
      .\verify-perimeter.ps1 -SignUrlControl 'https://example.invalid/'   # signature control: must PASS
      .\verify-perimeter.ps1 -ExtraImportNeedles KERNEL32.dll   # import control: must PASS
      .\verify-perimeter.ps1 -ExtraDoorNeedles LangSwitcher     # door control: must PASS
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
    [string[]]$ExtraPathNeedles = @(),
    # The address the signature carries (/du of signtool) -- the one https:// gate 3 accepts in
    # the certificate table (decision 181.37). Empty means the default of release.ps1, read out
    # of that script rather than typed here a second time.
    [string]$SignUrl = '',
    # Gate 3's control of the certificate table: the signature is judged against THIS address
    # instead, so the real /du of a signed file becomes stray and the gate has to FAIL.
    [string]$SignUrlControl = '',
    # Names added to the libraries gate 5 refuses. 'KERNEL32.dll' is the positive control: every
    # image imports it, so the gate has to fire on it.
    [string[]]$ExtraImportNeedles = @(),
    # Strings added to the test doors gate 6 looks for. 'LangSwitcher' is the positive control:
    # every image names itself, so the gate has to fire on it.
    [string[]]$ExtraDoorNeedles = @()
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

# The address of the signature (decision 181.37): given, or the default of release.ps1, which
# is the script that signs this file.
if (-not $SignUrl) {
    $releaseScript = Join-Path $ScriptDir 'release.ps1'
    if (Test-Path $releaseScript) {
        $releaseText = Get-Content -LiteralPath $releaseScript -Raw
        $signMatch = [regex]::Match($releaseText, '\[string\]\$SignUrl\s*=\s*''(https://[^'']+)''')
        if ($signMatch.Success) { $SignUrl = $signMatch.Groups[1].Value }
    }
}
$signControl = ($SignUrlControl -ne '')
$signJudge = $SignUrl
if ($signControl) { $signJudge = $SignUrlControl }

# The certificate table: the PE security directory, the fifth of the data directories -- a FILE
# offset and a length (unlike the other directories, which are addresses in memory). Zero
# length means an unsigned file: then the whole file is the image.
$certOffset = 0
$certLength = 0
$peOffset = [BitConverter]::ToInt32($bytes, 0x3C)
$optionalHeader = $peOffset + 24
$directories = $optionalHeader + 96
if ([BitConverter]::ToUInt16($bytes, $optionalHeader) -eq 0x20B) { $directories = $optionalHeader + 112 }
$certOffset = [long][BitConverter]::ToUInt32($bytes, $directories + 32)
$certLength = [long][BitConverter]::ToUInt32($bytes, $directories + 36)
$tableFailed = $false
if ($certLength -gt 0 -and ($certOffset -le 0 -or $certOffset + $certLength -gt $bytes.Length)) {
    Write-Host ("  FAIL: the certificate table ({0} bytes at {1}) lies outside the file of {2} bytes" -f $certLength, $certOffset, $bytes.Length)
    $tableFailed = $true
    $certLength = 0
}

$imageBytes = $bytes
$signatureBytes = New-Object byte[] 0
if ($certLength -gt 0) {
    $signatureBytes = New-Object byte[] ([int]$certLength)
    [Array]::Copy($bytes, $certOffset, $signatureBytes, 0, $certLength)
    $after = $bytes.Length - ($certOffset + $certLength)
    $imageBytes = New-Object byte[] ([int]($certOffset + $after))
    [Array]::Copy($bytes, 0, $imageBytes, 0, $certOffset)
    if ($after -gt 0) { [Array]::Copy($bytes, $certOffset + $certLength, $imageBytes, $certOffset, $after) }
}

# The https:// runs of a span of bytes, in both encodings in one pass: the raw bytes, and the
# same bytes with every zero dropped.
#
# ⚠ Rust string constants sit next to one another in `.rdata` with NO terminator between them,
# so two addresses come out of the file as ONE run: «https://a/onehttps://b/two». A pattern that
# stopped at the first «https://» would then find one address, and a stray one glued to a known
# one would be «explained» by it. Each run is therefore cut at every further «https://».
function Get-HttpsRuns([byte[]]$span) {
    $ascii = -join ($span | ForEach-Object { if ($_ -ge 32 -and $_ -lt 127) { [char]$_ } else { "`n" } })
    $wide  = -join ($span | Where-Object { $_ -ne 0 } | ForEach-Object { if ($_ -ge 32 -and $_ -lt 127) { [char]$_ } else { "`n" } })
    $runs = @()
    foreach ($haystack in @($ascii, $wide)) {
        foreach ($match in [regex]::Matches($haystack, 'https://(?:(?!https://)[A-Za-z0-9\.\-/_%~:@\?=&\+])*')) {
            $runs += $match.Value
        }
    }
    return @($runs | Sort-Object -Unique)
}

# The runs of `$runs` that no address of `$known` explains. A run is explained when a known
# address is a PREFIX of it -- the linker stores strings side by side with no terminator, so
# «<the feed address>example.invalid» is one run and two constants, and in the certificate table
# the ASN.1 tag after the /du address (0x30) reads as a '0' glued to it -- or when the run is a
# prefix of a known one.
#
# ⚠ This is safe ONLY because the runs are cut at every further «https://»: a stray address glued
# behind a known one is a run of its own and is judged on its own.
function Get-StrayRuns($runs, $known) {
    $stray = @()
    foreach ($url in $runs) {
        $explained = $false
        foreach ($address in $known) {
            if ($address -and ($url.StartsWith($address) -or $address.StartsWith($url))) { $explained = $true; break }
        }
        if (-not $explained) { $stray += $url }
    }
    return @($stray)
}

$imageRuns = Get-HttpsRuns $imageBytes
$imageStray = Get-StrayRuns $imageRuns $declared
Write-Host ("  the image ({0} bytes): https:// runs {1}, against src\letters.rs" -f $imageBytes.Length, $imageRuns.Count)
foreach ($url in $imageStray) { Write-Host ("  FOUND   {0}  (in the image)" -f $url) }

$signatureStray = @()
if ($signatureBytes.Length -gt 0) {
    $signatureRuns = Get-HttpsRuns $signatureBytes
    if ($signControl) { Write-Host ("  *** POSITIVE CONTROL: the signature is judged against {0}; the gate must FAIL ***" -f $signJudge) }
    if (-not $signJudge) {
        Write-Host '  FAIL: the address of the signature is not known -- neither -SignUrl nor release.ps1 gives it'
        $tableFailed = $true
    }
    $signatureStray = Get-StrayRuns $signatureRuns @($signJudge)
    Write-Host ("  the certificate table ({0} bytes at {1}): https:// runs {2}, against the signature's {3}" -f $signatureBytes.Length, $certOffset, $signatureRuns.Count, $signJudge)
    foreach ($url in $signatureStray) { Write-Host ("  FOUND   {0}  (in the certificate table)" -f $url) }
} else {
    Write-Host '  the file carries no certificate table: the whole file is the image'
}

$signatureFailed = ($signatureStray.Count -gt 0)
$addressFailed = ($imageStray.Count -gt 0) -or $signatureFailed -or $tableFailed

if ($addressFailed) {
    Write-Host '  RESULT: FAIL -- the file carries an address neither the program nor its signature declares'
} else {
    Write-Host '  every address in the image is one the program declares, and the signature names only its own'
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

# --- Gate 5: the C runtime is inside the image (stage E104, question 190) -----------------------
#
# Until 0.103.0 every image imported VCRUNTIME140.dll and five api-ms-win-crt-* libraries, and on a
# Windows without the Visual C++ package the program did not start at all (finding 188.25). Since
# T-104-2 the runtime is linked in (+crt-static, .cargo\config.toml and release.ps1), and this gate
# reads the IMPORT TABLE of the file -- the import directory and the delay-import directory of the
# PE header, the names the loader will look for -- and refuses any name of the C runtime of MSVC.
# The table is read by this script, not by dumpbin: the gate must mean the same on a machine
# without Build Tools.
Write-Host ''
Write-Host '--- Gate 5: the C runtime of MSVC is inside the image (no CRT imports) ---'

$importControl = ($ExtraImportNeedles.Count -gt 0)
$importNeedles = @('VCRUNTIME*.dll', 'MSVCP*.dll', 'api-ms-win-crt-*.dll') + $ExtraImportNeedles
if ($importControl) { Write-Host ("  + {0}  *** POSITIVE CONTROL: the import gate must FAIL ***" -f ($ExtraImportNeedles -join ', ')) }

function Get-PeSectionOffset([byte[]]$b, [int]$secBase, [int]$count, [uint32]$rva) {
    for ($i = 0; $i -lt $count; $i++) {
        $s = $secBase + 40 * $i
        $vsize = [BitConverter]::ToUInt32($b, $s + 8)
        $va = [BitConverter]::ToUInt32($b, $s + 12)
        $rawSize = [BitConverter]::ToUInt32($b, $s + 16)
        $raw = [BitConverter]::ToUInt32($b, $s + 20)
        $span = [Math]::Max($vsize, $rawSize)
        if (($rva -ge $va) -and ($rva -lt ($va + $span))) { return [int]($raw + ($rva - $va)) }
    }
    return -1
}

function Get-PeAsciiz([byte[]]$b, [int]$at) {
    $end = $at
    while (($end -lt $b.Length) -and ($b[$end] -ne 0)) { $end++ }
    return [System.Text.Encoding]::ASCII.GetString($b, $at, $end - $at)
}

# The names of the DLLs the file imports, plain and delayed. Throws on a file that is not a PE32+
# image or whose directories do not resolve: a table that cannot be read is not an empty table.
function Get-PeImports([byte[]]$b) {
    $e = [BitConverter]::ToInt32($b, 0x3C)
    if ([BitConverter]::ToUInt32($b, $e) -ne 0x00004550) { throw 'not a PE file' }
    $coff = $e + 4
    $count = [BitConverter]::ToUInt16($b, $coff + 2)
    $optSize = [BitConverter]::ToUInt16($b, $coff + 16)
    $opt = $coff + 20
    if ([BitConverter]::ToUInt16($b, $opt) -ne 0x20B) { throw 'not a PE32+ image' }
    $dd = $opt + 112
    $secBase = $opt + $optSize
    $names = @()
    # Data directory 1: IMAGE_IMPORT_DESCRIPTOR, 20 bytes each, Name at +12, ended by zeros.
    $impRva = [BitConverter]::ToUInt32($b, $dd + 8)
    if ($impRva -ne 0) {
        $at = Get-PeSectionOffset $b $secBase $count $impRva
        if ($at -lt 0) { throw 'the import directory does not resolve' }
        while ($true) {
            $nameRva = [BitConverter]::ToUInt32($b, $at + 12)
            $thunk = [BitConverter]::ToUInt32($b, $at + 16)
            if (($nameRva -eq 0) -and ($thunk -eq 0)) { break }
            $nameAt = Get-PeSectionOffset $b $secBase $count $nameRva
            if ($nameAt -lt 0) { throw 'an import name does not resolve' }
            $names += Get-PeAsciiz $b $nameAt
            $at += 20
        }
    }
    # Data directory 13: the delay-load descriptors, 32 bytes each, DllNameRVA at +4.
    $delayRva = [BitConverter]::ToUInt32($b, $dd + 8 * 13)
    if ($delayRva -ne 0) {
        $at = Get-PeSectionOffset $b $secBase $count $delayRva
        if ($at -lt 0) { throw 'the delay-import directory does not resolve' }
        while ($true) {
            $nameRva = [BitConverter]::ToUInt32($b, $at + 4)
            if ($nameRva -eq 0) { break }
            $nameAt = Get-PeSectionOffset $b $secBase $count $nameRva
            if ($nameAt -lt 0) { throw 'a delay-import name does not resolve' }
            $names += Get-PeAsciiz $b $nameAt
            $at += 32
        }
    }
    return @($names)
}

$peBytes = [System.IO.File]::ReadAllBytes($Artifact)
$importFailed = $false
try {
    $imports = @(Get-PeImports $peBytes)
} catch {
    Write-Host ("  FAIL: the import table could not be read: {0}" -f $_.Exception.Message)
    $imports = @()
    $importFailed = $true
}
if ((-not $importFailed) -and ($imports.Count -eq 0)) {
    Write-Host '  FAIL: not one import was read -- every Windows image imports something; the gate measured nothing.'
    $importFailed = $true
}
$crtHits = @()
foreach ($dll in $imports) {
    foreach ($needle in $importNeedles) {
        if ($dll -like $needle) { $crtHits += $dll; break }
    }
}
if ($imports.Count -gt 0) {
    Write-Host ("  imports read: {0} ({1})" -f $imports.Count, (($imports | Sort-Object -Unique) -join ', '))
    foreach ($dll in $crtHits) { Write-Host ("  FOUND   {0}" -f $dll) }
    if ($crtHits.Count -gt 0) {
        $importFailed = $true
        Write-Host ("  RESULT: FAIL -- the image imports {0} library(ies) of the C runtime" -f $crtHits.Count)
    } elseif (-not $importFailed) {
        Write-Host '  no library of the C runtime of MSVC is imported'
        Write-Host '  RESULT: pass'
    }
}

# --- Gate 6: no test door of the updater is in the image (stage E104, TZ E104 p. 22) ------------
#
# The updater has doors for the stand -- a feed on the guest's own loopback, a key of the stand and
# a folder of its own -- and they compile only under the `testing` feature. This gate reads the
# whole file, byte for byte, in ASCII and UTF-16LE, without regard to case, and refuses it if it
# names one of them. Positive control: -ExtraDoorNeedles 'LangSwitcher' -- every image names itself.
Write-Host ''
Write-Host '--- Gate 6: the test doors of the updater are not in the image --------'

$doorControl = ($ExtraDoorNeedles.Count -gt 0)
$doorNeedles = @('localhost', '127.0.0.1', 'update-doors.toml', 'LANGSW_UPDATE_') + $ExtraDoorNeedles
if ($doorControl) { Write-Host ("  + {0}  *** POSITIVE CONTROL: the door gate must FAIL ***" -f ($ExtraDoorNeedles -join ', ')) }
$folded = [System.Text.Encoding]::GetEncoding(28591).GetString($peBytes).ToLowerInvariant()
$doorHits = 0
foreach ($needle in $doorNeedles) {
    $low = $needle.ToLowerInvariant()
    $wide = (($low.ToCharArray() | ForEach-Object { [string]$_ + [char]0 }) -join '')
    $a = ([regex]::Matches($folded, [regex]::Escape($low))).Count
    $w = ([regex]::Matches($folded, [regex]::Escape($wide))).Count
    $verdict = 'absent'
    if (($a + $w) -gt 0) { $verdict = 'FOUND'; $doorHits += $a + $w }
    Write-Host ("  {0,-7} {1,-30} ASCII {2,3}  UTF-16LE {3,3}" -f $verdict, $needle, $a, $w)
}
$doorFailed = ($doorHits -gt 0)
if ($doorFailed) {
    Write-Host ("  RESULT: FAIL -- the image names a test door {0} time(s)" -f $doorHits)
} else {
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

if ($signControl) {
    if ($signatureFailed) {
        Write-Host ' POSITIVE CONTROL PASSED: the signature gate really failed'
        Write-Host '======================================================================'
        exit 0
    }

    Write-Host ' POSITIVE CONTROL FAILED: the signature gate did not fail -- the file has no certificate table, or its /du passed under a foreign address'
    Write-Host '======================================================================'
    exit 1
}

if ($importControl) {
    if ($importFailed) {
        Write-Host ' POSITIVE CONTROL PASSED: the import gate really failed'
        Write-Host '======================================================================'
        exit 0
    }

    Write-Host ' POSITIVE CONTROL FAILED: the import gate missed a library the image imports'
    Write-Host '======================================================================'
    exit 1
}

if ($doorControl) {
    if ($doorFailed) {
        Write-Host ' POSITIVE CONTROL PASSED: the door gate really failed'
        Write-Host '======================================================================'
        exit 0
    }

    Write-Host ' POSITIVE CONTROL FAILED: the door gate missed a string that is in every image'
    Write-Host '======================================================================'
    exit 1
}

if ($crateFailed -or $addressFailed -or $pathFailed -or $importFailed -or $doorFailed) {
    Write-Host ' RESULT: FAIL'
    Write-Host '======================================================================'
    exit 1
}

Write-Host ' RESULT: PASS -- criterion 4, SEC-03 addresses, the build folders, the C runtime and the test doors hold; NFR-07 measured above'
Write-Host '======================================================================'
exit 0
