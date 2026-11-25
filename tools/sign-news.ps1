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
    and since 1.8.1 (decision 208.242, the finding of the window of the author's cards), about the
    shape of an entry. Every released version reads an [[item]] into id, type, date, version, link
    and its language tables, each with a title and a text, and refuses the WHOLE feed over anything
    else -- no news and no update reach anybody -- so this script refuses it first:
      * a field of an entry other than id, type, date, version and link, or one given twice
      * a line of an entry that is not "key = value"
      * a table under an entry that is not a language -- [item.image], [[item.x]], [item.ru.x]
      * a language table given twice, or without its title or its text
    and since 1.9.0 (stage E121, decision 208.244), about the author's cards -- [[card]], a top-level
    array every released version skips (tests\feed_cards.rs reads it with the frozen parse of 1.8.0):
      * a field of a card other than id, from, to, view, side, link, image, sha256 and size, one
        given twice, a line that is not "key = value", or a table under a card that is not a language
      * no id, identifiers that repeat or descend, more than three cards -- the program keeps three
      * a term that is not two dates YYYY-MM-DD, `from` after `to`, or a term already over
      * a view other than panel (B: the picture in place of the panel) or full (C1: the whole
        window); a side other than start or end; a side of a full card
      * no image, an image not under https://panda-pishet-kod.dev/langswitcher/cards/ by plain path
        segments, or no file of its name in the folder cards\ beside this file
      * a picture that is not a PNG or a JPEG by its signature, not exactly 1000 x 1440 (panel) or
        2560 x 1280 (full) pixels by its header, or over 1 MB
      * a card without ru or en, a language table without its title or its text, a language with
        no cap; a text over the cap of its view -- panel: the column news of tools\feed-caps.tsv
        (the text stands where a news item's does), full: the column card
    THE SUM AND THE SIZE of a card's picture are this script's own: it reads the file in cards\,
    writes `sha256 = "<64 digits>"` and `size = <bytes>` right under the card's `image` line --
    those the author wrote are replaced -- and signs that.
    THE ORDER OF PUBLISHING: the pictures (the folder cards\) first, the feed after them. A feed that
    names a picture not yet on the site leaves the card in the plain template until the next read.

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

# 1.8.1 (decision 208.242): the fields of an entry a released version knows -- RawItem of src\feed.rs.
# Every other key of an entry goes to its `#[serde(flatten)] languages`, must be a table with a title
# and a text, and refuses the whole feed when it is not.
$ItemFields = @('id', 'type', 'date', 'version', 'link')
$WholeFeed = 'every released version refuses the whole feed over it: no news and no update reach anybody'

# 1.9.0 (stage E121, decision 208.244): the author's cards -- the fields of [[card]] (RawCard of
# src\feed.rs), how many the program keeps (cards::CARDS_KEPT), the one folder of the pictures
# (links::CARDS_URL), the cap of a picture (cards::IMAGE_CAP) and its size in pixels by the view
# (cards::PANEL_PIXELS, cards::FULL_PIXELS).
$CardFields = @('id', 'from', 'to', 'view', 'side', 'link', 'image', 'sha256', 'size')
$CARDS_KEPT = 3
$CardsPrefix = 'https://panda-pishet-kod.dev/langswitcher/cards/'
$CARD_IMAGE_CAP = 1024 * 1024
$CardPixels = @{ panel = @(1000, 1440); full = @(2560, 1280) }

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
        # 1.9.0 (stage E121): a fourth column, card -- the cap of the words of a card of the view
        # full (C1), measured by the same instrument; a file printed before it has three.
        if ($row -notmatch '^([A-Za-z]{2,3}(?:-[A-Za-z]{4})?)\t(\d+)\t(\d+)(?:\t(\d+))?$') {
            Fail ("{0}, line {1}: not 'tag<TAB>update<TAB>news<TAB>card': {2}" -f $CapsFile, $number, $raw)
        }
        $card = $null
        if ($Matches[4]) { $card = [int]$Matches[4] }
        $Caps[$Matches[1]] = [pscustomobject]@{ update = [int]$Matches[2]; news = [int]$Matches[3]; card = $card; Line = $number }
    }
}

# The cap of the kind of `entry` in `tag`: an entry of another type has no cap and is refused.
function Cap-Of($entry, [string]$tag) {
    # 1.9.0 (stage E121): a card's text is held to the cap of its view -- panel (B) stands where a
    # news item's text does, full (C1) is the strip under the picture, the column card.
    if ($entry.Type -eq 'card') {
        if (@('panel', 'full') -cnotcontains $entry.View) {
            Fail ("card {0} [{1}]: the view must come before the languages and be panel or full, found '{2}'" -f $entry.Id, $tag, $entry.View)
        }
        if ($entry.View -eq 'panel') { return $Caps[$tag].news }
        if ($null -eq $Caps[$tag].card) {
            $why = "card {0} [{1}]: {2} has no column card -- print the caps again with the instrument " +
                "of fitting (print_the_caps_of_the_feed with LANGSW_FEED_CAPS_WRITE)"
            Fail ($why -f $entry.Id, $tag, $CapsFile)
        }
        return $Caps[$tag].card
    }
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
        $letter = if ($entry.Type -eq 'update') { 'Obnovlenie' } elseif ($entry.Type -eq 'card') { 'Pismo ot avtora (card)' } else { 'Novost' }
        $why = "entry {0} ({1}) [{2}]: the text is {3} characters (code points, line breaks not counted), " +
            "the cap of {1} is {4} ({5}, line {6}) -- a longer text makes the letter '{7}' lower the " +
            "type size of the program; shorten the text"
        Fail ($why -f $entry.Id, $entry.Type, $tag, $points, $cap, $CapsFile, $Caps[$tag].Line, $letter)
    }
    $script:TextsChecked++
}

# 1.9.0 (stage E121): the format and the size in pixels of a picture out of its header, the way the
# program reads them (cards::sniff): PNG -- the signature and IHDR; JPEG -- the markers up to the
# first frame header (SOF0..SOF15 but DHT, JPG, DAC), every segment skipped by its length. $null for
# anything else.
function Get-PictureSize([byte[]]$b) {
    $png = @(0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A)
    $isPng = $b.Length -ge 24
    for ($i = 0; $isPng -and $i -lt 8; $i++) { if ($b[$i] -ne $png[$i]) { $isPng = $false } }
    if ($isPng) {
        if ([System.Text.Encoding]::ASCII.GetString($b, 12, 4) -cne 'IHDR') { return $null }
        $w = ([long]$b[16] -shl 24) -bor ([long]$b[17] -shl 16) -bor ([long]$b[18] -shl 8) -bor [long]$b[19]
        $h = ([long]$b[20] -shl 24) -bor ([long]$b[21] -shl 16) -bor ([long]$b[22] -shl 8) -bor [long]$b[23]
        return [pscustomobject]@{ Format = 'PNG'; W = $w; H = $h }
    }
    if ($b.Length -ge 4 -and $b[0] -eq 0xFF -and $b[1] -eq 0xD8) {
        $at = 2
        while ($at + 3 -lt $b.Length) {
            while ($at + 1 -lt $b.Length -and $b[$at] -eq 0xFF -and $b[$at + 1] -eq 0xFF) { $at++ }
            if ($b[$at] -ne 0xFF) { return $null }
            $marker = [int]$b[$at + 1]
            if (($marker -ge 0xD0 -and $marker -le 0xD7) -or $marker -eq 0x01) { $at += 2; continue }
            if ($marker -eq 0xD9 -or $marker -eq 0xDA) { return $null }
            $length = ([int]$b[$at + 2] -shl 8) -bor [int]$b[$at + 3]
            if ($length -lt 2) { return $null }
            if ($marker -ge 0xC0 -and $marker -le 0xCF -and @(0xC4, 0xC8, 0xCC) -notcontains $marker) {
                if ($at + 8 -ge $b.Length) { return $null }
                $h = ([int]$b[$at + 5] -shl 8) -bor [int]$b[$at + 6]
                $w = ([int]$b[$at + 7] -shl 8) -bor [int]$b[$at + 8]
                return [pscustomobject]@{ Format = 'JPEG'; W = $w; H = $h }
            }
            $at += 2 + $length
        }
    }
    return $null
}

# 1.9.0 (stage E121): the picture of a card -- the address in the one folder, the file of its name in
# cards\ beside the feed, its signature of a format, its size in pixels for the view, at most 1 MB.
# Answers the sum and the size this script writes into the card.
function Test-CardPicture($card, [string]$value) {
    if ($value -notmatch '^"([^"]*)"$') { Fail ("card {0}: image must be a quoted address, found {1}" -f $card.Id, $value) }
    $url = $Matches[1]
    if (-not $url.StartsWith($CardsPrefix, [System.StringComparison]::Ordinal)) {
        Fail ("card {0}: image must start with {1} -- the program takes a picture from that folder of that host and from nowhere else; found {2}" -f $card.Id, $CardsPrefix, $url)
    }
    $rest = $url.Substring($CardsPrefix.Length)
    if (($rest -notmatch '^[A-Za-z0-9._-]+(/[A-Za-z0-9._-]+)*$') -or (@($rest -split '/') -contains '..') -or (@($rest -split '/') -contains '.') -or $rest.Length -gt 200) {
        Fail ("card {0}: image must name a file under {1} by plain path segments; found {2}" -f $card.Id, $CardsPrefix, $url)
    }
    if (@('panel', 'full') -cnotcontains $card.View) {
        Fail ("card {0}: the view must come before the image and be panel or full, found '{1}'" -f $card.Id, $card.View)
    }
    $file = Join-Path $SiteDir ('cards\' + ($rest -replace '/', '\'))
    if (-not (Test-Path -LiteralPath $file -PathType Leaf)) {
        Fail ("card {0}: no picture at {1} -- put the file in the folder cards\ beside the feed; it is published first, the feed after it" -f $card.Id, $file)
    }
    $bytes = [System.IO.File]::ReadAllBytes($file)
    if ($bytes.Length -eq 0 -or $bytes.Length -gt $CARD_IMAGE_CAP) {
        Fail ("card {0}: the picture is {1} bytes, the limit is {2} (1 MB)" -f $card.Id, $bytes.Length, $CARD_IMAGE_CAP)
    }
    $picture = Get-PictureSize $bytes
    if ($null -eq $picture) { Fail ("card {0}: {1} is not a PNG or a JPEG by its signature and header" -f $card.Id, $file) }
    $want = $CardPixels[$card.View]
    if ($picture.W -ne $want[0] -or $picture.H -ne $want[1]) {
        $why = "card {0}: the picture of a {1} card is {2} x {3} pixels, found {4} x {5} -- twice the largest " +
            "window; the program makes it smaller below 200 %"
        Fail ($why -f $card.Id, $card.View, $want[0], $want[1], $picture.W, $picture.H)
    }
    $sum = [Convert]::ToHexString([System.Security.Cryptography.SHA256]::HashData($bytes)).ToLowerInvariant()
    $card.Image = $url
    Write-Host ("  card {0,-4} picture {1}, {2} x {3}, {4} bytes  ok" -f $card.Id, $picture.Format, $picture.W, $picture.H, $bytes.Length)
    return @($sum, $bytes.Length)
}

Write-Host '======================================================================'
Write-Host ' Lang_Switcher -- check and sign the author feed (FR-102)'
Write-Host '======================================================================'
Write-Host ("File      {0}" -f $Path)
Write-Host ("Key       {0}" -f $(if ($Reserve) { 'RESERVE' } else { 'working' }))

if (-not (Test-Path -LiteralPath $Path)) { Fail ("no such file: {0}" -f $Path) }
# 1.9.0 (stage E121): the folder of the feed -- the pictures of the cards lie in cards\ beside it.
$SiteDir = Split-Path -Parent ([System.IO.Path]::GetFullPath($Path))

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
# 1.9.0 (stage E121): the cards, and the lines of the document as they will be signed -- the same
# lines, but for the sum and the size of each card's picture, which this script writes itself.
$cards = @()
$out = New-Object System.Collections.Generic.List[string]

foreach ($line in $lines) {
    $out.Add($line)

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
        if ($current) { if ($current.IsCard) { $cards += $current } else { $items += $current } }
        $current = [pscustomobject]@{ Id = $null; Type = $null; Version = $null; Languages = @(); Fields = @{}; Texts = @{}; IsCard = $false }
        $language = $null
        $section = 'item'
        continue
    }

    # 1.9.0 (stage E121): a card of the author -- a top-level array of its own (decision 208.244 p. 5).
    if ($trimmed -eq '[[card]]') {
        if ($current) { if ($current.IsCard) { $cards += $current } else { $items += $current } }
        $current = [pscustomobject]@{ Id = $null; Type = 'card'; View = $null; Side = $null; From = $null; To = $null; Image = $null; Languages = @(); Fields = @{}; Texts = @{}; IsCard = $true }
        $language = $null
        $section = 'card'
        continue
    }
    if ($trimmed -match '^\[card\.([A-Za-z]{2,3}(?:-[A-Za-z]{4})?)\]$') {
        if (-not $current -or -not $current.IsCard) { Fail ("[card.{0}] outside a card" -f $Matches[1]) }
        $language = $Matches[1]
        if ($current.Languages -ccontains $language) { Fail ("card {0}: [card.{1}] is given twice" -f $current.Id, $language) }
        $current.Languages += $language
        $current.Texts[$language] = [pscustomobject]@{ Title = $false; Text = $false }
        $section = 'language'
        Test-Language $current $language
        continue
    }
    if ($trimmed -match '^\[\[?\s*card\s*\.') {
        Fail ("card {0}: {1} is not a language table -- a card has the fields {2} and [card.<tag>] with a title and a text" -f $current.Id, $trimmed, ($CardFields -join ', '))
    }

    # STAGE E108 (T-108-2): a tag is two or three letters and, for the second Serbian and the second
    # Uzbek of decision 200b.1, a script after a hyphen -- [item.sr-Latn], [item.uz-Cyrl]. Until
    # E108 such a header fell through to "another table" below and its language was left out of
    # the count without a word.
    if ($trimmed -match '^\[item\.([A-Za-z]{2,3}(?:-[A-Za-z]{4})?)\]$') {
        if (-not $current) { Fail 'a language table before the first [[item]]' }
        if ($current.IsCard) { Fail ("card {0}: [item.{1}] under a card -- a card's languages are [card.<tag>]" -f $current.Id, $Matches[1]) }
        $language = $Matches[1]
        if ($current.Languages -ccontains $language) {
            Fail ("entry {0}: [item.{1}] is given twice -- {2}" -f $current.Id, $language, $WholeFeed)
        }
        $current.Languages += $language
        $current.Texts[$language] = [pscustomobject]@{ Title = $false; Text = $false }
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

    # 1.8.1 (decision 208.242): any other table under an entry. A released version reads it as a
    # language with a title and a text, and the feed with it as no feed at all. Until 1.8.1 it fell
    # to "another table" below and was signed without a word.
    if ($trimmed -match '^\[\[?\s*item\s*\.') {
        $why = "entry {0}: {1} is not a language table -- an entry has id, type, date, version, link " +
            "and [item.<tag>] with a title and a text; {2}"
        Fail ($why -f $current.Id, $trimmed, $WholeFeed)
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
        # 1.8.1: a released version refuses the whole feed over a language table without its title
        # or its text, so both are noted here and asked for after the parse.
        if ($trimmed -cmatch '^title\s*=') { $current.Texts[$language].Title = $true; continue }
        if ($trimmed -cmatch '^text\s*=') { $current.Texts[$language].Text = $true }
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

    # 1.9.0 (stage E121): a field of a card. The sum and the size are this script's: the lines the
    # author wrote are dropped, and the ones this script reads off the picture go under `image`.
    if ($section -eq 'card') {
        if ($trimmed -notmatch '^([A-Za-z_][A-Za-z0-9_]*)\s*=\s*(.+?)\s*$') {
            Fail ("card {0}: the line '{1}' is not 'key = value'" -f $current.Id, $trimmed)
        }
        $key = $Matches[1]
        $value = $Matches[2]
        if ($CardFields -cnotcontains $key) {
            Fail ("card {0}: unknown field '{1}' -- a card has {2} and its language tables [card.<tag>]" -f $current.Id, $key, ($CardFields -join ', '))
        }
        if ($current.Fields.ContainsKey($key)) { Fail ("card {0} gives {1} twice" -f $current.Id, $key) }
        $current.Fields[$key] = $true
        switch -CaseSensitive ($key) {
            'id' { if ($value -match '^\d+$') { $current.Id = [long]$value } else { Fail ("card: id must be a whole number, found {0}" -f $value) } }
            'from' { $current.From = $value }
            'to' { $current.To = $value }
            'view' { if ($value -match '^"([a-z]+)"$') { $current.View = $Matches[1] } else { Fail ("card {0}: view must be a quoted word, found {1}" -f $current.Id, $value) } }
            'side' { if ($value -match '^"([a-z]+)"$') { $current.Side = $Matches[1] } else { Fail ("card {0}: side must be a quoted word, found {1}" -f $current.Id, $value) } }
        }
        if ($key -eq 'sha256' -or $key -eq 'size') {
            $out.RemoveAt($out.Count - 1)
        }
        if ($key -eq 'image') {
            $picture = Test-CardPicture $current $value
            $out.Add(('sha256 = "{0}"' -f $picture[0]))
            $out.Add(('size = {0}' -f $picture[1]))
        }
        continue
    }

    # 1.8.1 (decision 208.242): a field of an entry is one of the five a released version knows. Any
    # other key -- a typo such as `url =`, a new one such as `layout =` -- is read by it as a language
    # table and the whole feed is refused; until 1.8.1 this loop let it through without a word.
    if ($section -eq 'item') {
        if ($trimmed -notmatch '^([^=]+?)\s*=') {
            Fail ("entry {0}: the line '{1}' is not 'key = value' -- {2}" -f $current.Id, $trimmed, $WholeFeed)
        }
        $key = $Matches[1]
        if ($ItemFields -cnotcontains $key) {
            $why = "entry {0}: unknown field '{1}' -- an entry has {2} and its language tables " +
                "[item.<tag>]; {3}"
            Fail ($why -f $current.Id, $key, ($ItemFields -join ', '), $WholeFeed)
        }
        if ($current.Fields.ContainsKey($key)) {
            Fail ("entry {0} gives {1} twice -- {2}" -f $current.Id, $key, $WholeFeed)
        }
        $current.Fields[$key] = $true
        if ($trimmed -match '^id\s*=\s*(\d+)') { $current.Id = [int]$Matches[1] }
        elseif ($trimmed -match '^type\s*=\s*"([a-z]+)"') { $current.Type = $Matches[1] }
        elseif ($trimmed -match '^version\s*=\s*"([^"]+)"') { $current.Version = $Matches[1] }
        continue
    }
}

if ($current) { if ($current.IsCard) { $cards += $current } else { $items += $current } }

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

    foreach ($tag in $item.Languages) {
        foreach ($part in @('Title', 'Text')) {
            if (-not $item.Texts[$tag].$part) {
                Fail ("entry {0} [{1}] has no {2} -- {3}" -f $item.Id, $tag, $part.ToLowerInvariant(), $WholeFeed)
            }
        }
    }

    Write-Host ("  entry {0,-4} {1,-7} languages: {2}" -f $item.Id, $item.Type, ($item.Languages -join ' '))
}

Write-Host '  every entry has ru and en     ok'
Write-Host '  identifiers unique, ascending ok'

# --- 2a. The author's cards (1.9.0, stage E121, decision 208.244) -----------------------------------
if ($cards.Count -gt $CARDS_KEPT) { Fail ("{0} cards, the program keeps {1}" -f $cards.Count, $CARDS_KEPT) }

$seenCards = @{}
$previousCard = -1
$day = {
    param($card, [string]$name, [string]$value)
    if ($value -notmatch '^"?(\d{4}-\d{2}-\d{2})"?$') { Fail ("card {0}: {1} must be a date YYYY-MM-DD, found '{2}'" -f $card.Id, $name, $value) }
    $parsed = [datetime]::MinValue
    if (-not [datetime]::TryParseExact($Matches[1], 'yyyy-MM-dd', [System.Globalization.CultureInfo]::InvariantCulture, [System.Globalization.DateTimeStyles]::None, [ref]$parsed)) {
        Fail ("card {0}: {1} {2} is no day of the calendar" -f $card.Id, $name, $value)
    }
    return $parsed
}

foreach ($card in $cards) {
    if ($null -eq $card.Id) { Fail 'a card with no id' }
    if ($seenCards.ContainsKey($card.Id)) { Fail ("card id {0} appears twice" -f $card.Id) }
    if ($card.Id -le $previousCard) { Fail ("card id {0} does not follow {1}: identifiers must ascend" -f $card.Id, $previousCard) }
    $seenCards[$card.Id] = $true
    $previousCard = $card.Id

    if ($null -eq $card.From -or $null -eq $card.To) { Fail ("card {0} has no term -- from and to" -f $card.Id) }
    $from = & $day $card 'from' $card.From
    $to = & $day $card 'to' $card.To
    if ($from -gt $to) { Fail ("card {0}: the term runs backwards, {1} after {2}" -f $card.Id, $card.From, $card.To) }
    if ($to -lt [datetime]::Today) { Fail ("card {0}: the term is over -- it ended on {1}" -f $card.Id, $card.To) }

    if (@('panel', 'full') -cnotcontains $card.View) { Fail ("card {0}: view must be panel or full, found '{1}'" -f $card.Id, $card.View) }
    if ($card.View -eq 'full' -and $null -ne $card.Side) { Fail ("card {0}: a full card has no side" -f $card.Id) }
    if ($null -ne $card.Side -and @('start', 'end') -cnotcontains $card.Side) { Fail ("card {0}: side must be start or end, found '{1}'" -f $card.Id, $card.Side) }
    if ($null -eq $card.Image) { Fail ("card {0} has no image -- the views panel and full are its picture" -f $card.Id) }

    foreach ($needed in @('ru', 'en')) {
        if ($card.Languages -notcontains $needed) { Fail ("card {0} has no [{1}] text -- ru and en are obligatory" -f $card.Id, $needed) }
    }
    foreach ($tag in $card.Languages) {
        foreach ($part in @('Title', 'Text')) {
            if (-not $card.Texts[$tag].$part) { Fail ("card {0} [{1}] has no {2}" -f $card.Id, $tag, $part.ToLowerInvariant()) }
        }
    }

    Write-Host ("  card  {0,-4} {1,-5} {2} -- {3}  languages: {4}" -f $card.Id, $card.View, $card.From, $card.To, ($card.Languages -join ' '))
}

if ($cards.Count -gt 0) {
    # The document as it is signed: the same lines, the sum and the size of each picture this
    # script's own. The cap of the body is asked again of it.
    $text = $out -join "`n"
    $body = [System.Text.UTF8Encoding]::new($false).GetBytes($text)
    if ($body.Length -gt $BODY_CAP) { Fail ("the body is {0} bytes, the limit is {1}" -f $body.Length, $BODY_CAP) }
    # One sum and one size per card in what is signed -- the ones read off the pictures.
    $sums = @(); $sizes = 0; $others = 0; $inCard = $false
    foreach ($signed in $out) {
        $t = $signed.Trim()
        if ($t -eq '[[card]]') { $inCard = $true; continue }
        if ($t.StartsWith('[')) { $inCard = $false; continue }
        if (-not $inCard) { continue }
        if ($t -cmatch '^sha256 = "([0-9a-f]{64})"$') { $sums += $Matches[1] }
        elseif ($t -cmatch '^size = \d+$') { $sizes++ }
        elseif ($t -match '^(sha256|size)\s*=') { $others++ }
    }
    if ($sums.Count -ne $cards.Count -or $sizes -ne $cards.Count -or $others -ne 0) {
        Fail ("the signed body has {0} sums and {1} sizes of its own for {2} card(s), {3} of the author's" -f $sums.Count, $sizes, $cards.Count, $others)
    }
    foreach ($sum in $sums) { Write-Host ("  signed: sha256 = {0}" -f $sum) }
    Write-Host ("  {0} card(s): the sum and the size written from cards\  ok" -f $cards.Count)
}

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
