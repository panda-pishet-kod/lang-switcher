<#
    make-letter-stats.ps1 -- the trigram table of a language for src\letter_stats.rs (stage E105, decision 196.1).

    Reads plain-text books -- DATA only: nothing in their folder is run, they are read as bytes by this
    script from its own folder -- strips the Project Gutenberg header and licence (everything outside
    the "*** START OF" and "*** END OF" lines), splits the text into words of Latin letters (any other
    character ends a word), and counts the trigrams of symbols of every word framed by boundaries:
    0 0 w1 ... wn 0 (0 = the boundary, 1..26 = a..z, case folded). That is exactly what Reading of
    src\letter_stats.rs feeds itself for a candidate.

    The table is 27 x 27 x 27 bytes, at (a * 27 + b) * 27 + c: the cost of c after a b,
    round(-log2 P(c | a b) * 4) capped at 255, with P = (count(a b c) + 1) / (count(a b .) + 27) --
    add-one smoothing, so no trigram is impossible. Lower is more probable.

    The books of the table of English, stage E105 (decision 197.10a of the owner): Project Gutenberg,
    public domain in the USA -- https://www.gutenberg.org/cache/epub/<N>/pg<N>.txt for N = 1342
    (Pride and Prejudice), 1661 (The Adventures of Sherlock Holmes), 98 (A Tale of Two Cities),
    84 (Frankenstein), 11 (Alice's Adventures in Wonderland). Their sizes and SHA-256 are in the
    report of the stage; the books themselves are not in this tree and not in the image.

    Written for Windows PowerShell 5.1 without a Cyrillic character (decision R-05).

    Usage:
      .\make-letter-stats.ps1 -Books <folder>\pg1342.txt,<folder>\pg1661.txt,... -Out res\letter-stats-en.bin
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string[]]$Books,
    [Parameter(Mandatory = $true)][string]$Out
)

$ErrorActionPreference = 'Stop'

$symbols = 27
$counts = New-Object 'int[]' ($symbols * $symbols * $symbols)
$words = 0
$letters = 0

foreach ($book in $Books) {
    $text = [IO.File]::ReadAllText($book, [Text.Encoding]::UTF8)
    $start = $text.IndexOf('*** START OF')
    $end = $text.IndexOf('*** END OF')
    if ($start -lt 0 -or $end -le $start) {
        Write-Host ("FAIL: {0} has no Project Gutenberg frame (*** START OF / *** END OF)" -f $book)
        exit 1
    }
    $start = $text.IndexOf("`n", $start) + 1
    $body = $text.Substring($start, $end - $start)

    $a = 0
    $b = 0
    $inWord = $false
    foreach ($ch in $body.ToCharArray()) {
        $c = [int][char]::ToLowerInvariant($ch)
        if ($c -ge 97 -and $c -le 122) {
            $s = $c - 96
            if (-not $inWord) { $a = 0; $b = 0; $inWord = $true; $words = $words + 1 }
            $counts[($a * $symbols + $b) * $symbols + $s] = $counts[($a * $symbols + $b) * $symbols + $s] + 1
            $a = $b
            $b = $s
            $letters = $letters + 1
        } elseif ($inWord) {
            $counts[($a * $symbols + $b) * $symbols] = $counts[($a * $symbols + $b) * $symbols] + 1
            $inWord = $false
        }
    }
    if ($inWord) { $counts[($a * $symbols + $b) * $symbols] = $counts[($a * $symbols + $b) * $symbols] + 1 }
}

$table = New-Object 'byte[]' ($symbols * $symbols * $symbols)
for ($context = 0; $context -lt $symbols * $symbols; $context++) {
    $total = 0
    for ($s = 0; $s -lt $symbols; $s++) { $total = $total + $counts[$context * $symbols + $s] }
    for ($s = 0; $s -lt $symbols; $s++) {
        $p = ($counts[$context * $symbols + $s] + 1) / ($total + $symbols)
        $cost = [Math]::Round(-[Math]::Log($p, 2) * 4)
        if ($cost -gt 255) { $cost = 255 }
        $table[$context * $symbols + $s] = [byte]$cost
    }
}

[IO.File]::WriteAllBytes($Out, $table)
$sha = (Get-FileHash $Out -Algorithm SHA256).Hash
Write-Host ("table {0}: {1} bytes, sha256 {2}; {3} words, {4} letters" -f $Out, $table.Length, $sha, $words, $letters)
