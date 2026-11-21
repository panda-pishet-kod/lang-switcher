<#
    sign-news.ps1 -- checks and signs the author's feed (FR-102, task T-32-6).

    WHAT IT REFUSES TO SIGN, and why the checks are here rather than in the program. The program
    can only drop a bad file silently; the author is the one who can fix it, and the moment to
    tell them is before the file is published, not after. So this script is the strict half of
    FR-102:

      * more than three `news` entries        -- the program keeps three and shows three
      * more than one `update` entry          -- the program keeps one
      * a missing id, type, or per-language table
      * an entry without BOTH `ru` and `en`   -- FR-102 makes the two obligatory
      * identifiers that repeat or descend    -- «прочитано» is remembered BY NUMBER
      * a body over 192 KB                    -- the program reads at most 256 KB
    and since stage E104 (TZ E104 p. 16), about the two tables only the updater reads:
      * a [download] without the update entry of the same version, or more than one [download]
      * a [download] url not under https://panda-pishet-kod.dev/langswitcher/files/
      * a sha256 that is not 64 hexadecimal digits; a size that is not 1 byte to 16 MB
      * a field of [download] or [stop] missing, given twice or unknown; [[download]], [[stop]]
      * a [stop] in a file not signed with -Reserve -- the updater honours it only from the reserve
    and since stage E120 (decision 208.194), about the length of a text:
      * a text longer than the cap of its language and of the kind of its entry in
        tools\feed-caps.tsv (tag, update, news) -- the whole text, in code points, line breaks not
        counted; the caps are printed by the instrument of fitting (tests\ui_fit.rs,
        print_the_caps_of_the_feed with LANGSW_FEED_CAPS_WRITE), and an update at its cap fits the
        letter "Obnovlenie", a news item at its cap the letter "Novost", without lowering the type
        size of the program
      * a language whose cap of that kind is 0 -- the letter does not fit even without a text: a
        person of that language sees the en table instead
      * a language the caps file does not name

    A refusal writes nothing: the file on disk is left exactly as it was, and the exit code is
    non-zero.

    WHAT IT WRITES. The same document with one line in front of it:

        signature = "<base64 of r||s, 64 bytes>"

    The body is every byte AFTER that first newline, verbatim, and the signature is ECDSA P-256
    over its SHA-256. Line endings are forced to LF: a signature is over bytes, and a file that
    travels through a tool that helpfully rewrites CRLF would stop verifying with no visible
    change.

    THE KEYS (stage E89, decision 152.3). The working key lives in the Windows key store under
    -Container and needs no folder. The reserve key is read from reserve.p8 in the folder -KeyDir
    names, or LANGSW_NEWS_KEYS without it; the variable is read after tools\local.ps1, the
    untracked file of this machine's own folders. Keys have no default place: -Reserve without
    either is refused.

    Usage:
      pwsh -File tools\sign-news.ps1 <site folder>\news.toml
      pwsh -File tools\sign-news.ps1 <file> -Reserve         # sign with the reserve key
      pwsh -File tools\sign-news.ps1 <file> -Check           # check only, write nothing
#>

[CmdletBinding()]
param(
    [Parameter(Mandatory = $true, Position = 0)]
    [string]$Path,
    [string]$Container = 'LangSwitcherNewsWorking',
    # The folder of reserve.p8. Empty means LANGSW_NEWS_KEYS; see the header.
    [string]$KeyDir = '',
    [switch]$Reserve,
    [switch]$Check
)

$ErrorActionPreference = 'Stop'

# This machine's own folders (stage E89): first, so that the LANGSW_NEWS_KEYS it sets is read
# below. tools\local.example.ps1 shows the form of the file.
if (Test-Path "$PSScriptRoot\local.ps1") { . "$PSScriptRoot\local.ps1" }
if (-not $KeyDir) { $KeyDir = $env:LANGSW_NEWS_KEYS }

$BODY_CAP = 192 * 1024
$NEWS_KEPT = 3

function Fail([string]$why) {
    Write-Host ''
    Write-Host ("  REFUSED: {0}" -f $why)
    Write-Host '  Nothing was written.'
    exit 1
}

# STAGE E108 (decision 202.13): the longest first line of a text, in code points -- see the check in
# the parse below for why and where the number was measured.
$EXCERPT_CAP = 150

function Test-FirstLine($entry, [string]$tag, [string]$line) {
    $points = @($line.Trim().EnumerateRunes()).Count
    if ($points -gt $EXCERPT_CAP) {
        $why = "entry {0} [{1}]: the first line of the text is {2} characters, the limit is {3} -- " +
            "'Recent letters' shows it as the excerpt; break the text into lines"
        Fail ($why -f $entry.Id, $tag, $points, $EXCERPT_CAP)
    }
}

# STAGE E120 (decision 208.194; word of the owner on the kinds, 2026-11-20): the cap of the WHOLE text
# of an entry, by language AND by the kind of the entry. The instrument of fitting prints it into
# tools\feed-caps.tsv ("tag<TAB>update<TAB>news", '#' lines explain): an update at its cap still fits
# the letter "Obnovlenie" of its language, a news item at its cap the letter "Novost" -- narrow and
# ordinary, at the type size of the table. Those two letters are the only windows that show the whole
# text ("Recent letters" shows the first line, held to $EXCERPT_CAP above). The feed is ours, and a
# letter must not lower the type size of the whole program. The file is never edited by hand; a
# missing file or a malformed line is a refusal.
$CapsFile = Join-Path $PSScriptRoot 'feed-caps.tsv'
$Caps = @{}

function Read-Caps {
    if (-not (Test-Path -LiteralPath $CapsFile)) {
        $why = "no caps of the feed at {0} -- they are printed by tests\ui_fit.rs " +
            "(print_the_caps_of_the_feed with LANGSW_FEED_CAPS_WRITE)"
        Fail ($why -f $CapsFile)
    }
    $number = 0
    foreach ($raw in [System.IO.File]::ReadAllLines($CapsFile, [System.Text.UTF8Encoding]::new($false))) {
        $number++
        $row = $raw.Trim()
        if ($row -eq '' -or $row.StartsWith('#')) { continue }
        if ($row -notmatch '^([A-Za-z]{2,3}(?:-[A-Za-z]{4})?)\t(\d+)\t(\d+)$') {
            Fail ("{0}, line {1}: not 'tag<TAB>update<TAB>news': {2}" -f $CapsFile, $number, $raw)
        }
        $Caps[$Matches[1]] = [pscustomobject]@{ update = [int]$Matches[2]; news = [int]$Matches[3]; Line = $number }
    }
}

# The cap of the kind of `entry` in `tag`: an entry of another type has no cap and is refused.
function Cap-Of($entry, [string]$tag) {
    if (@('update', 'news') -notcontains $entry.Type) {
        Fail ("entry {0} [{1}]: the type must come before the languages and be news or update, found '{2}'" -f $entry.Id, $tag, $entry.Type)
    }
    return $Caps[$tag].($entry.Type)
}

# A language table of an entry: its language must be in the caps file, with a cap of its kind above 0.
function Test-Language($entry, [string]$tag) {
    if (-not $Caps.ContainsKey($tag)) {
        $why = "entry {0} [{1}]: the language has no cap in {2} -- print the caps again with the " +
            "instrument of fitting; nothing is signed with a text nobody measured"
        Fail ($why -f $entry.Id, $tag, $CapsFile)
    }
    if ((Cap-Of $entry $tag) -eq 0) {
        $why = "entry {0} ({1}) [{2}]: the letter does not fit even without a text in this language " +
            "(cap 0, {3}, line {4}) -- remove this language from the entry; a person of it will see en"
        Fail ($why -f $entry.Id, $entry.Type, $tag, $CapsFile, $Caps[$tag].Line)
    }
}

# A whole text: its code points, line breaks not counted and the ends trimmed, against the cap of
# its kind.
function Test-Text($entry, [string]$tag, [string]$body) {
    $points = @((($body -replace "[`r`n]", '').Trim()).EnumerateRunes()).Count
    $cap = Cap-Of $entry $tag
    if ($points -gt $cap) {
        $letter = if ($entry.Type -eq 'update') { 'Obnovlenie' } else { 'Novost' }
        $why = "entry {0} ({1}) [{2}]: the text is {3} characters (code points, line breaks not counted), " +
            "the cap of {1} is {4} ({5}, line {6}) -- a longer text makes the letter '{7}' lower the " +
            "type size of the program; shorten the text"
        Fail ($why -f $entry.Id, $entry.Type, $tag, $points, $cap, $CapsFile, $Caps[$tag].Line, $letter)
    }
    $script:TextsChecked++
}

Write-Host '======================================================================'
Write-Host ' Lang_Switcher -- check and sign the author feed (FR-102)'
Write-Host '======================================================================'
Write-Host ("File      {0}" -f $Path)
Write-Host ("Key       {0}" -f $(if ($Reserve) { 'RESERVE' } else { 'working' }))

if (-not (Test-Path -LiteralPath $Path)) { Fail ("no such file: {0}" -f $Path) }

# --- 1. The body, in the bytes it will be signed in --------------------------------------------
$text = [System.IO.File]::ReadAllText($Path, [System.Text.UTF8Encoding]::new($false))

# A file that already carries a signature is signed again from its body: publishing twice must
# not need an unsigned copy kept somewhere.
if ($text -match '^\s*signature\s*=') {
    $newline = $text.IndexOf("`n")
    if ($newline -lt 0) { Fail 'the file is one line and that line is a signature' }
    $text = $text.Substring($newline + 1)
    Write-Host '  (the previous signature line was dropped and will be replaced)'
}

# LF, always. See the header.
$text = $text -replace "`r`n", "`n"
$body = [System.Text.UTF8Encoding]::new($false).GetBytes($text)

Write-Host ("Body      {0} bytes" -f $body.Length)

if ($body.Length -gt $BODY_CAP) {
    Fail ("the body is {0} bytes, the limit is {1}" -f $body.Length, $BODY_CAP)
}

# --- 2. The checks FR-102 makes the author's business --------------------------------------------
Write-Host ''
Write-Host '--- checks ---------------------------------------------------------'

# The parse is deliberately shallow and by hand: this script must run on a machine with nothing
# installed but PowerShell, and a TOML parser is not something PowerShell has. What it needs to
# know is the shape, and the shape is flat: `[[item]]` starts an entry, `[item.xx]` starts a
# language, `key = value` at the top of an entry is a field of it.
#
# STAGE E104 (TZ E104 p. 16): two more top-level tables, read by the updater only -- [download] (the
# installer of the version of the update entry: version, url, sha256, size) and [stop] (versions
# no updater installs, and the word that the working key is revoked). So the parse now knows WHICH
# table a line is in: until E104 a key after any other header still went to the last entry, and a
# `version =` of [download] written after the entries would have become the entry's version.
$lines = $text -split "`n"
$items = @()
$current = $null
$language = $null
$schema = $null
$section = 'top'
$downloads = 0
$stops = 0
$download = [ordered]@{}
$stop = [ordered]@{}

Read-Caps
$TextsChecked = 0
$collecting = $false
$awaitFirst = $false
$gathered = New-Object System.Collections.Generic.List[string]

foreach ($line in $lines) {
    # STAGE E120 (decision 208.194): a text of several lines is gathered WHOLE before anything else
    # reads the line -- inside it a blank line, a '#' or a '[' is text, not a comment or a table --
    # and measured against the cap of its language when its closing quotes come. Until E120 only
    # its first line was read (the excerpt below) and the rest was skipped line by line.
    if ($collecting) {
        $close = $line.IndexOf('"""')
        $part = if ($close -ge 0) { $line.Substring(0, $close) } else { $line }
        if ($awaitFirst) {
            $awaitFirst = $false
            if ($close -lt 0 -or $part.Trim() -ne '') { Test-FirstLine $current $language $part.Trim() }
        }
        if ($close -ge 0) {
            if ($part -ne '') { $gathered.Add($part) }
            $collecting = $false
            Test-Text $current $language ($gathered -join "`n")
        } else {
            $gathered.Add($line)
        }
        continue
    }

    $trimmed = $line.Trim()

    if ($trimmed -eq '' -or $trimmed.StartsWith('#')) { continue }

    if ($trimmed -eq '[[item]]') {
        if ($current) { $items += $current }
        $current = [pscustomobject]@{ Id = $null; Type = $null; Version = $null; Languages = @() }
        $language = $null
        $section = 'item'
        continue
    }

    # STAGE E108 (T-108-2): a tag is two or three letters and, for the second Serbian and the second
    # Uzbek of decision 200b.1, a script after a hyphen -- [item.sr-Latn], [item.uz-Cyrl]. Until
    # E108 such a header fell through to "another table" below and its language was left out of
    # the count without a word.
    if ($trimmed -match '^\[item\.([A-Za-z]{2,3}(?:-[A-Za-z]{4})?)\]$') {
        if (-not $current) { Fail 'a language table before the first [[item]]' }
        $language = $Matches[1]
        $current.Languages += $language
        $section = 'language'
        Test-Language $current $language
        continue
    }

    if ($trimmed -match '^\[\[\s*(download|stop)\s*\]\]') {
        Fail ("[[{0}]] is an array of tables -- the format has one [{0}] table and no more" -f $Matches[1])
    }
    if ($trimmed -match '^\[\s*download\s*\]') {
        $downloads++
        if ($downloads -gt 1) { Fail ("{0} [download] tables -- the format has one" -f $downloads) }
        $section = 'download'; $language = $null; continue
    }
    if ($trimmed -match '^\[\s*stop\s*\]') {
        $stops++
        if ($stops -gt 1) { Fail ("{0} [stop] tables -- the format has one" -f $stops) }
        $section = 'stop'; $language = $null; continue
    }

    if ($trimmed -match '^\[') { $language = $null; $section = 'other'; continue }

    if ($section -eq 'download' -or $section -eq 'stop') {
        if ($trimmed -match '^([A-Za-z_][A-Za-z0-9_]*)\s*=\s*(.+?)\s*$') {
            $table = $download
            if ($section -eq 'stop') { $table = $stop }
            if ($table.Contains($Matches[1])) { Fail ("[{0}] gives {1} twice" -f $section, $Matches[1]) }
            $table[$Matches[1]] = $Matches[2]
        }
        continue
    }

    # STAGE E108 (decision 202.13): the first line of a text is the excerpt the window "Recent
    # letters" shows under the title, wrapped to the width of the row; a first line long enough
    # makes that window the tallest of the program, and the font step of every screen is counted
    # from the tallest window. The limit is measured, not chosen (scratchpad-E108,
    # exp-threshold.out): a Russian first line of 150 characters leaves the tallest window as it
    # is and one of 160 makes it grow; an Urdu one of 150 does not. Published entries of 1.2.0 are
    # 116 at most.
    if ($section -eq 'language') {
        if ($trimmed -match '^text\s*=\s*"""(.*)$') {
            $rest = $Matches[1]
            $close = $rest.IndexOf('"""')
            if ($close -ge 0) {
                # The whole text on the line of its key.
                $one = $rest.Substring(0, $close)
                Test-FirstLine $current $language $one.Trim()
                Test-Text $current $language $one
            } else {
                # TOML trims the line break right after the opening quotes: the text starts on the
                # next line, and that line is the excerpt; otherwise the rest of this one is.
                $gathered.Clear()
                $collecting = $true
                if ($rest.Trim() -eq '') { $awaitFirst = $true }
                else { $awaitFirst = $false; Test-FirstLine $current $language $rest.Trim(); $gathered.Add($rest) }
            }
            continue
        }
        if ($trimmed -match '^text\s*=\s*"(.*)"$') {
            Test-FirstLine $current $language $Matches[1]
            Test-Text $current $language $Matches[1]
            continue
        }
    }

    if ($section -eq 'language' -or $section -eq 'other') { continue }

    if ($section -eq 'top' -and $trimmed -match '^schema\s*=\s*(\d+)') { $schema = [int]$Matches[1]; continue }

    if ($section -eq 'item' -and $trimmed -match '^id\s*=\s*(\d+)') { $current.Id = [int]$Matches[1]; continue }
    if ($section -eq 'item' -and $trimmed -match '^type\s*=\s*"([a-z]+)"') { $current.Type = $Matches[1]; continue }
    if ($section -eq 'item' -and $trimmed -match '^version\s*=\s*"([^"]+)"') { $current.Version = $Matches[1]; continue }
}

if ($current) { $items += $current }

if ($collecting) { Fail ("entry {0} [{1}]: a text opened with three quotes is never closed" -f $current.Id, $language) }

if ($schema -ne 1) { Fail ("schema must be 1, found {0}" -f $schema) }
Write-Host ("  schema = 1                    ok")
Write-Host ("  {0} texts within the caps of {1}  ok" -f $TextsChecked, $CapsFile)

$news = @($items | Where-Object { $_.Type -eq 'news' })
$updates = @($items | Where-Object { $_.Type -eq 'update' })

Write-Host ("  entries: {0} news, {1} update  {2}" -f $news.Count, $updates.Count,
    $(if ($news.Count -le $NEWS_KEPT -and $updates.Count -le 1) { 'ok' } else { 'REFUSED' }))

if ($news.Count -gt $NEWS_KEPT) { Fail ("{0} news entries, the program keeps {1}" -f $news.Count, $NEWS_KEPT) }
if ($updates.Count -gt 1) { Fail ("{0} update entries, the program keeps one" -f $updates.Count) }
if ($items.Count -eq 0) { Fail 'no entries at all' }

$seen = @{}
$previous = -1

foreach ($item in $items) {
    if ($null -eq $item.Id) { Fail 'an entry with no id' }
    if ($null -eq $item.Type) { Fail ("entry {0} has no type" -f $item.Id) }
    if ($item.Type -ne 'news' -and $item.Type -ne 'update') { Fail ("entry {0}: type must be news or update" -f $item.Id) }
    if ($item.Type -eq 'update' -and -not $item.Version) { Fail ("entry {0} is an update with no version" -f $item.Id) }
    if ($seen.ContainsKey($item.Id)) { Fail ("id {0} appears twice" -f $item.Id) }
    if ($item.Id -le $previous) { Fail ("id {0} does not follow {1}: identifiers must ascend" -f $item.Id, $previous) }

    $seen[$item.Id] = $true
    $previous = $item.Id

    foreach ($needed in @('ru', 'en')) {
        if ($item.Languages -notcontains $needed) {
            Fail ("entry {0} has no [{1}] text -- FR-102 makes ru and en obligatory" -f $item.Id, $needed)
        }
    }

    Write-Host ("  entry {0,-4} {1,-7} languages: {2}" -f $item.Id, $item.Type, ($item.Languages -join ' '))
}

Write-Host '  every entry has ru and en     ok'
Write-Host '  identifiers unique, ascending ok'

# --- 2b. The tables of the updater (stage E104, TZ E104 p. 16) --------------------------------------
# What the updater installs is what this script lets through, so every rule of p. 16 is a refusal
# here, before the file is signed: the updater refuses the same things, but only after the file has
# reached every machine.
$FilesPrefix = 'https://panda-pishet-kod.dev/langswitcher/files/'
$MaxInstaller = 16 * 1024 * 1024

if ($downloads -gt 1) { Fail ("{0} [download] tables -- the format has one" -f $downloads) }
if ($downloads -eq 1) {
    foreach ($field in @('version', 'url', 'sha256', 'size')) {
        if (-not $download.Contains($field)) { Fail ("[download] has no {0}" -f $field) }
    }
    foreach ($field in $download.Keys) {
        if (@('version', 'url', 'sha256', 'size') -notcontains $field) { Fail ("[download] has an unknown field {0}" -f $field) }
    }
    if ($download['version'] -notmatch '^"(\d+\.\d+\.\d+)"$') { Fail ("[download] version must be a quoted X.Y.Z, found {0}" -f $download['version']) }
    $downloadVersion = $Matches[1]
    if (($updates.Count -ne 1) -or ($updates[0].Version -ne $downloadVersion)) {
        Fail ("[download] is for {0}, and the file has no update entry of that version -- the two go together" -f $downloadVersion)
    }
    if ($download['url'] -notmatch '^"([^"]*)"$') { Fail '[download] url must be a quoted string' }
    $url = $Matches[1]
    if (-not $url.StartsWith($FilesPrefix, [System.StringComparison]::Ordinal)) {
        Fail ("[download] url must start with {0} -- the updater takes the installer from that folder of that host and from nowhere else; found {1}" -f $FilesPrefix, $url)
    }
    $rest = $url.Substring($FilesPrefix.Length)
    if (($rest -notmatch '^[A-Za-z0-9._-]+(/[A-Za-z0-9._-]+)*$') -or (@($rest -split '/') -contains '..') -or (@($rest -split '/') -contains '.')) {
        Fail ("[download] url must name a file under {0} by plain path segments; found {1}" -f $FilesPrefix, $url)
    }
    if ($download['sha256'] -notmatch '^"([0-9A-Fa-f]{64})"$') { Fail ("[download] sha256 must be 64 hexadecimal digits in quotes, found {0}" -f $download['sha256']) }
    if ($download['size'] -notmatch '^\d+$') { Fail ("[download] size must be a whole number of bytes, found {0}" -f $download['size']) }
    $size = [long]$download['size']
    if (($size -le 0) -or ($size -gt $MaxInstaller)) { Fail ("[download] size {0} is out of 1...{1} bytes -- the updater refuses an installer over 16 MB" -f $size, $MaxInstaller) }
    Write-Host ("  [download] {0}, {1} bytes  ok" -f $downloadVersion, $size)
}

if ($stops -gt 1) { Fail ("{0} [stop] tables -- the format has one" -f $stops) }
if ($stops -eq 1) {
    if (-not $Reserve) {
        Fail '[stop] counts only in a feed signed with the RESERVE key -- sign this file with -Reserve'
    }
    if ($stop.Contains('versions') -and ($stop['versions'] -notmatch '^\[\s*("\d+\.\d+\.\d+"\s*(,\s*"\d+\.\d+\.\d+"\s*)*,?\s*)?\]$')) {
        Fail ("[stop] versions must be a list of quoted X.Y.Z, found {0}" -f $stop['versions'])
    }
    if ($stop.Contains('revoke_working') -and ($stop['revoke_working'] -notmatch '^(true|false)$')) {
        Fail ("[stop] revoke_working must be true or false, found {0}" -f $stop['revoke_working'])
    }
    foreach ($field in $stop.Keys) {
        if (@('versions', 'revoke_working') -notcontains $field) { Fail ("[stop] has an unknown field {0}" -f $field) }
    }
    Write-Host '  [stop] signed with the reserve key  ok'
}

if ($Check) {
    Write-Host ''
    Write-Host '======================================================================'
    Write-Host ' RESULT: PASS -- the document is signable (-Check, nothing written)'
    Write-Host '======================================================================'
    exit 0
}

# --- 3. The signature ----------------------------------------------------------------------------
Write-Host ''
Write-Host '--- signature ------------------------------------------------------'

if ($Reserve) {
    # The reserve key does not live on this machine: it is brought back from its encrypted copy
    # for this one signature, and the password is asked for rather than stored.
    if (-not $KeyDir) {
        Fail 'no key folder for the reserve key: name it with -KeyDir <folder> or set LANGSW_NEWS_KEYS (tools\local.example.ps1 shows the form)'
    }
    $file = Join-Path $KeyDir 'reserve.p8'
    if (-not (Test-Path $file)) { Fail ("no reserve key at {0}" -f $file) }

    $secure = Read-Host -AsSecureString 'password for the reserve key'
    $plain = [System.Net.NetworkCredential]::new('', $secure).Password

    $key = [System.Security.Cryptography.ECDsa]::Create()
    $key.ImportEncryptedPkcs8PrivateKey($plain, [System.IO.File]::ReadAllBytes($file), [ref]$null)
    Write-Host '  the reserve key was opened from its encrypted copy'
} else {
    if (-not [System.Security.Cryptography.CngKey]::Exists($Container)) {
        Fail ("the working key container {0} does not exist -- run tools\make-news-key.ps1" -f $Container)
    }

    $key = New-Object System.Security.Cryptography.ECDsaCng(
        [System.Security.Cryptography.CngKey]::Open($Container))
    Write-Host ("  the working key was opened from the container {0}" -f $Container)
}

# `SignData` with SHA-256 answers the raw r||s pair -- exactly what BCryptVerifySignature wants,
# and NOT a DER structure.
$signature = $key.SignData($body, [System.Security.Cryptography.HashAlgorithmName]::SHA256)
$key.Dispose()

if ($signature.Length -ne 64) { Fail ("the signature is {0} bytes, P-256 must give 64" -f $signature.Length) }

$encoded = [Convert]::ToBase64String($signature)
Write-Host ("  signature: 64 bytes, base64 {0}…{1}" -f $encoded.Substring(0, 8), $encoded.Substring($encoded.Length - 4))

# --- 4. The file ----------------------------------------------------------------------------------
$out = [System.Text.UTF8Encoding]::new($false).GetBytes(('signature = "{0}"' -f $encoded) + "`n")
$whole = New-Object byte[] ($out.Length + $body.Length)
[Array]::Copy($out, 0, $whole, 0, $out.Length)
[Array]::Copy($body, 0, $whole, $out.Length, $body.Length)

[System.IO.File]::WriteAllBytes($Path, $whole)

$hash = (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash

Write-Host ''
Write-Host '--- result ---------------------------------------------------------'
Write-Host ("  file      {0}" -f $Path)
Write-Host ("  size      {0} bytes ({1} of them the body)" -f $whole.Length, $body.Length)
Write-Host ("  SHA-256   {0}" -f $hash)
Write-Host ''
Write-Host '======================================================================'
Write-Host ' RESULT: PASS -- signed'
Write-Host '======================================================================'
