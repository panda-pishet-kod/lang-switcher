<#
    make-letter-stats.ps1 -- the trigram table of a language for src\letter_stats.rs (stage E105, decision 196.1;
    the languages besides English -- stage E106, task T-106-5, decision 198.7).

    Reads plain-text books -- DATA only: nothing in their folder is run, they are read as bytes by this
    script from its own folder -- strips the Project Gutenberg header and licence (everything outside
    the "*** START OF" and "*** END OF" lines), splits the text into words of the letters of the
    language (any other character ends a word), and counts the trigrams of symbols of every word framed
    by boundaries: 0 0 w1 ... wn 0 (0 = the boundary, 1..N = the classes of the alphabet, case folded).
    That is exactly what Reading of src\letter_stats.rs feeds itself for a candidate.

    The alphabet is the line of -Language in res\letter-stats-alphabets.txt -- the same file the
    program reads, so the two cannot count by different letters. A class is the lower-case code points
    of one symbol; a letter with a sign above or below folds into its base letter.

    The table is S x S x S bytes for S = N + 1 symbols, at (a * S + b) * S + c: the cost of c after
    a b, round(-log2 P(c | a b) * 4) capped at 255, with P = (count(a b c) + 1) / (count(a b .) + S) --
    add-one smoothing, so no trigram is impossible. Lower is more probable.

    The books of the table of English, stage E105 (decision 197.10a of the owner): Project Gutenberg,
    public domain in the USA -- https://www.gutenberg.org/cache/epub/<N>/pg<N>.txt for N = 1342
    (Pride and Prejudice), 1661 (The Adventures of Sherlock Holmes), 98 (A Tale of Two Cities),
    84 (Frankenstein), 11 (Alice's Adventures in Wonderland). The books of French, Spanish and
    Russian -- stage E106, decisions 198.3, 198.7: see the report ITOG-E106. Their sizes and SHA-256
    are in the reports of the stages; the books themselves are not in this tree and not in the image.

    Stage E107, task T-107-5 (decisions 198.22a, 198.22g): the six target languages -- Arabic, Persian,
    Indonesian, Hindi, Tamil, Sinhala -- out of the Leipzig Corpora Collection (Universitaet Leipzig,
    CC BY; the Sinhala corpus is of Wikipedia, CC BY-SA): -Format leipzig reads a file
    <name>-sentences.txt of a corpus, one sentence a line after its number and a tab, every line a
    text of its own (its end ends a word); there is no frame to strip. The corpora: ara_news_2005-2009_300K,
    fas_news_2019_300K, ind_mixed_2013_300K, hin_mixed_2019_300K, tam_newscrawl_2011_300K,
    sin_wikipedia_2021_100K of https://downloads.wortschatz-leipzig.de/corpora/ -- their sizes and
    SHA-256 are in the report ITOG-E107. A token '~...' of the alphabet lists the marks read as
    nothing: skipped, the word goes on across them.

    Written for Windows PowerShell 5.1 without a Cyrillic character (decision R-05).

    Usage:
      .\make-letter-stats.ps1 -Language en -Books <folder>\pg1342.txt,<folder>\pg1661.txt,... -Out res\letter-stats-en.bin
      .\make-letter-stats.ps1 -Language si -Format leipzig -Books <folder>\sin_wikipedia_2021_100K-sentences.txt -Out res\letter-stats-si.bin
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$Language,
    [Parameter(Mandatory = $true)][string[]]$Books,
    [Parameter(Mandatory = $true)][string]$Out,
    [ValidateSet('gutenberg', 'leipzig')][string]$Format = 'gutenberg',
    [string]$Alphabets = (Join-Path $PSScriptRoot '..\res\letter-stats-alphabets.txt')
)

$ErrorActionPreference = 'Stop'

# The classes of the language: code point -> symbol (1..N).
$line = Get-Content $Alphabets | Where-Object { $_ -notmatch '^\s*#' -and $_.Split(' ')[0] -eq $Language } | Select-Object -First 1
if (-not $line) {
    Write-Host ("FAIL: no alphabet for '{0}' in {1}" -f $Language, $Alphabets)
    exit 1
}
$tokens = @($line.Trim().Split(' ', [StringSplitOptions]::RemoveEmptyEntries) | Select-Object -Skip 2)
# The marks read as nothing -- the tokens '~...' (stage E107); every other token is a class.
$skip = New-Object 'System.Collections.Generic.HashSet[int]'
foreach ($token in @($tokens | Where-Object { $_.StartsWith('~') })) {
    foreach ($hex in $token.Substring(1).Split('+')) { [void]$skip.Add([Convert]::ToInt32($hex, 16)) }
}
$classes = @($tokens | Where-Object { -not $_.StartsWith('~') })
$symbolOf = New-Object 'System.Collections.Generic.Dictionary[int,int]'
for ($i = 0; $i -lt $classes.Count; $i++) {
    foreach ($hex in $classes[$i].Split('+')) { $symbolOf.Add([Convert]::ToInt32($hex, 16), $i + 1) }
}

$symbols = $classes.Count + 1
$counts = New-Object 'int[]' ($symbols * $symbols * $symbols)
$words = 0
$letters = 0

foreach ($book in $Books) {
    if ($Format -eq 'leipzig') {
        # One sentence a line, after its number and a tab; the line breaks stay between them, so the
        # end of every sentence ends its last word.
        $sentences = New-Object System.Text.StringBuilder
        foreach ($row in [IO.File]::ReadLines($book, [Text.Encoding]::UTF8)) {
            $tab = $row.IndexOf("`t")
            [void]$sentences.Append($row.Substring($tab + 1)).Append("`n")
        }
        $body = $sentences.ToString()
        $sentences = $null
    } else {
        $text = [IO.File]::ReadAllText($book, [Text.Encoding]::UTF8)
        $start = $text.IndexOf('*** START OF')
        $end = $text.IndexOf('*** END OF')
        if ($start -lt 0 -or $end -le $start) {
            Write-Host ("FAIL: {0} has no Project Gutenberg frame (*** START OF / *** END OF)" -f $book)
            exit 1
        }
        $start = $text.IndexOf("`n", $start) + 1
        $body = $text.Substring($start, $end - $start)
    }

    $a = 0
    $b = 0
    $inWord = $false
    $s = 0
    foreach ($ch in $body.ToCharArray()) {
        $c = [int][char]::ToLowerInvariant($ch)
        if ($skip.Contains($c)) { continue }
        if ($symbolOf.TryGetValue($c, [ref]$s)) {
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
Write-Host ("table {0} ({1}, {2} symbols): {3} bytes, sha256 {4}; {5} words, {6} letters" -f $Out, $Language, $symbols, $table.Length, $sha, $words, $letters)
