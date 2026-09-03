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

    A refusal writes nothing: the file on disk is left exactly as it was, and the exit code is
    non-zero.

    WHAT IT WRITES. The same document with one line in front of it:

        signature = "<base64 of r||s, 64 bytes>"

    The body is every byte AFTER that first newline, verbatim, and the signature is ECDSA P-256
    over its SHA-256. Line endings are forced to LF: a signature is over bytes, and a file that
    travels through a tool that helpfully rewrites CRLF would stop verifying with no visible
    change.

    Usage:
      pwsh -File tools\sign-news.ps1 <dev>\artifacts\news-site\news.toml
      pwsh -File tools\sign-news.ps1 <file> -Reserve         # sign with the reserve key
      pwsh -File tools\sign-news.ps1 <file> -Check           # check only, write nothing
#>

[CmdletBinding()]
param(
    [Parameter(Mandatory = $true, Position = 0)]
    [string]$Path,
    [string]$Container = 'LangSwitcherNewsWorking',
    [string]$KeyDir = '<dev>\tools\admin\news-keys',
    [switch]$Reserve,
    [switch]$Check
)

$ErrorActionPreference = 'Stop'

$BODY_CAP = 192 * 1024
$NEWS_KEPT = 3

function Fail([string]$why) {
    Write-Host ''
    Write-Host ("  REFUSED: {0}" -f $why)
    Write-Host '  Nothing was written.'
    exit 1
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
$lines = $text -split "`n"
$items = @()
$current = $null
$language = $null
$schema = $null

foreach ($line in $lines) {
    $trimmed = $line.Trim()

    if ($trimmed -eq '' -or $trimmed.StartsWith('#')) { continue }

    if ($trimmed -eq '[[item]]') {
        if ($current) { $items += $current }
        $current = [pscustomobject]@{ Id = $null; Type = $null; Version = $null; Languages = @() }
        $language = $null
        continue
    }

    if ($trimmed -match '^\[item\.([A-Za-z]+)\]$') {
        if (-not $current) { Fail 'a language table before the first [[item]]' }
        $language = $Matches[1]
        $current.Languages += $language
        continue
    }

    if ($trimmed -match '^\[') { $language = $null; continue }

    if ($language) { continue }

    if ($trimmed -match '^schema\s*=\s*(\d+)') { $schema = [int]$Matches[1]; continue }

    if ($current -and $trimmed -match '^id\s*=\s*(\d+)') { $current.Id = [int]$Matches[1]; continue }
    if ($current -and $trimmed -match '^type\s*=\s*"([a-z]+)"') { $current.Type = $Matches[1]; continue }
    if ($current -and $trimmed -match '^version\s*=\s*"([^"]+)"') { $current.Version = $Matches[1]; continue }
}

if ($current) { $items += $current }

if ($schema -ne 1) { Fail ("schema must be 1, found {0}" -f $schema) }
Write-Host ("  schema = 1                    ok")

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
