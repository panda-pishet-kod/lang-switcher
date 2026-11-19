<#
    export-tokens.ps1 -- prints the style table of the interface shell, src\ui\tokens.rs, as the CSS
    variables of the mock-ups (tokens.css) and as the tables of the passport (PASSPORT-tables.md).

    tokens.rs is the one source of the look. The mock-ups and the passport are brought up to date
    from these two files by the controller -- this script writes nothing but them.

    USAGE
        powershell -NoProfile -ExecutionPolicy Bypass -File tools\export-tokens.ps1 -OutDir <folder>

        -OutDir   the folder for the two files; created when missing
        -Tokens   the table to read; by default src\ui\tokens.rs of this tree

    THE FORM IT READS is the contract written at the top of tokens.rs: one entry per line, the ///
    line right above an entry says what it is, // §N. lines open the sections. An entry in a form
    this script does not know stops it with the line number: a table that quietly drops a value
    is worse than no table.

    Numbers are printed as Rust prints them ({} of an f32: 8.0 is 8, 0.20 is 0.2), so a value of
    tokens.css is byte for byte the value of tokens.rs (tests\ui_tokens.rs holds this); the
    passport tables write a decimal comma, as the passport does.
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string] $OutDir,
    [string] $Tokens = ''
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version 2.0

# Windows PowerShell leaves $PSScriptRoot empty inside the param block, so the default is set here.
if ($Tokens -eq '') { $Tokens = Join-Path $PSScriptRoot '..\src\ui\tokens.rs' }

$source = [IO.File]::ReadAllLines((Resolve-Path -LiteralPath $Tokens).Path, [Text.Encoding]::UTF8)

function Stop-Export([int] $Index, [string] $Why) {
    throw "tokens.rs line $($Index + 1): $Why"
}

# A number as Rust's {} prints the f32 it was written as: no trailing zeros, no trailing point.
function Format-Plain([string] $Number) {
    if ($Number.Contains('.')) {
        $Number = $Number.TrimEnd('0').TrimEnd('.')
    }
    $Number
}

# The same number as the passport writes it, with a decimal comma.
function Format-Comma([string] $Number) {
    (Format-Plain $Number).Replace('.', ',')
}

# The lines of the function whose header matches $Header, up to its closing brace.
function Get-Body([string] $Header) {
    for ($i = 0; $i -lt $source.Count; $i++) {
        if ($source[$i] -match $Header) {
            $indent = ([regex]::Match($source[$i], '^\s*')).Value
            $body = @()
            for ($k = $i + 1; $k -lt $source.Count; $k++) {
                if ($source[$k] -eq "$indent}") { return , $body }
                $body += , @($k, $source[$k])
            }
        }
    }
    throw "tokens.rs: no function matching '$Header'"
}

# The variants of an enum and the /// line above each.
function Get-Variants([string] $Enum) {
    $found = [ordered]@{}
    $body = Get-Body "^pub enum $Enum \{"
    $doc = $null
    foreach ($entry in $body) {
        $text = $entry[1]
        if ($text -match '^\s*///\s?(.*)$') {
            $doc = $Matches[1]
        } elseif ($text -match '^\s*([A-Z]\w*),$') {
            if ($null -eq $doc) { Stop-Export $entry[0] "variant $($Matches[1]) of $Enum has no /// line" }
            $found[$Matches[1]] = $doc
            $doc = $null
        }
    }
    $found
}

# ---- §1. Colours -------------------------------------------------------------------------------

$colourDoc = Get-Variants 'Color'
$colourName = @{}
foreach ($entry in (Get-Body '^\s+pub const fn name\(self\) -> &''static str \{' )) {
    if ($entry[1] -match '^\s+Color::(\w+) => "([a-z0-9_]+)",$') { $colourName[$Matches[1]] = $Matches[2] }
}
$colourValue = @{}
foreach ($entry in (Get-Body '^\s+pub const fn pair\(self\) -> Pair \{')) {
    $text = $entry[1]
    if ($text -match '^\s+Color::(\w+) => Pair::new\(0x([0-9A-F]{6}), 0x([0-9A-F]{6})\),$') {
        $colourValue[$Matches[1]] = @("#$($Matches[2])", "#$($Matches[3])")
    } elseif ($text -match '^\s+Color::(\w+) => Color::(\w+)\.pair\(\),$') {
        $colourValue[$Matches[1]] = @("=$($Matches[2])")
    } elseif ($text -match '^\s+Color::') {
        Stop-Export $entry[0] 'a colour in a form this script does not know'
    }
}

$colours = @()
foreach ($variant in $colourDoc.Keys) {
    if (-not $colourName.ContainsKey($variant)) { throw "tokens.rs: colour $variant has no name" }
    if (-not $colourValue.ContainsKey($variant)) { throw "tokens.rs: colour $variant has no value" }
    $colours += , @($variant, $colourName[$variant], $colourValue[$variant], $colourDoc[$variant])
}

function Get-CssName([string] $Name) { '--w-' + $Name.ToLowerInvariant().Replace('_', '-') }

function Get-ColourName([string] $Variant) {
    if (-not $colourName.ContainsKey($Variant)) { throw "tokens.rs: no colour $Variant" }
    $colourName[$Variant]
}

# ---- §2. Roles ---------------------------------------------------------------------------------

$roleDoc = Get-Variants 'Role'
$roleName = @{}
# The name() of Role is the second name() of the file; its match arms are told by the prefix.
foreach ($line in $source) {
    if ($line -match '^\s+Role::(\w+) => "([a-z0-9_]+)",$') { $roleName[$Matches[1]] = $Matches[2] }
}
$roleStyle = @{}
foreach ($entry in (Get-Body '^\s+pub const fn style\(self\) -> RoleStyle \{')) {
    $text = $entry[1]
    $m = [regex]::Match($text, '^\s+Role::(\w+) => RoleStyle::(main|mono)\(([\d.]+), ([\d.]+), (\d+), (None|Some\(Color::(\w+)\))\)(\.caps\(([\d.]+)\))?,$')
    if ($m.Success) {
        $colour = if ($m.Groups[7].Success) { Get-ColourName $m.Groups[7].Value } else { $null }
        $tracking = if ($m.Groups[9].Success) { $m.Groups[9].Value } else { $null }
        $roleStyle[$m.Groups[1].Value] = @{
            Face = $m.Groups[2].Value; Size = $m.Groups[3].Value; Line = $m.Groups[4].Value
            Weight = $m.Groups[5].Value; Colour = $colour; Tracking = $tracking
        }
    } elseif ($text -match '^\s+Role::') {
        Stop-Export $entry[0] 'a role in a form this script does not know'
    }
}

# ---- Constants, by section ---------------------------------------------------------------------

$constants = @()          # section, name, doc, md value, css value
$raw = @{}                # name -> the number it holds, for the formula of B
$section = 0
$sectionTitle = @{}
$doc = @()

for ($i = 0; $i -lt $source.Count; $i++) {
    $text = $source[$i]

    if ($text -match '^// §(\d)\. (.+)$') {
        $section = [int] $Matches[1]
        $sectionTitle[$section] = $Matches[2]
        $doc = @()
        continue
    }
    if ($text -match '^///\s?(.*)$') { $doc += $Matches[1]; continue }

    $m = [regex]::Match($text, '^pub const ([A-Z][A-Z0-9_]*): ([^=]+?) = (.+);$')
    if (-not $m.Success) { $doc = @(); continue }

    $name = $m.Groups[1].Value
    $value = $m.Groups[3].Value
    if ($section -eq 0) { Stop-Export $i "constant $name before the first // §N. line" }
    if ($doc.Count -eq 0) { Stop-Export $i "constant $name has no /// line" }

    $n = '([\d.]+)'
    $b = 'var(--w-b)'
    if ($value -match "^Pt\($n\)$") {
        $mdValue = "$(Format-Comma $Matches[1]) т."; $cssValue = "$(Format-Plain $Matches[1])px"; $raw[$name] = $Matches[1]
    } elseif ($value -match "^Em\($n\)$") {
        $mdValue = "$(Format-Comma $Matches[1]) B"; $cssValue = "calc($(Format-Plain $Matches[1]) * $b)"
    } elseif ($value -match "^Pad::em\($n, $n\)$") {
        $mdValue = "$(Format-Comma $Matches[1]) × $(Format-Comma $Matches[2]) B"
        $cssValue = "calc($(Format-Plain $Matches[1]) * $b) calc($(Format-Plain $Matches[2]) * $b)"
    } elseif ($value -match "^SizePt::pt\($n, $n\)$") {
        $mdValue = "$(Format-Comma $Matches[1]) × $(Format-Comma $Matches[2]) т."
        $cssValue = "$(Format-Plain $Matches[1])px $(Format-Plain $Matches[2])px"
    } elseif ($value -match "^SizeEm::em\($n, $n\)$") {
        $mdValue = "$(Format-Comma $Matches[1]) × $(Format-Comma $Matches[2]) B"
        $cssValue = "calc($(Format-Plain $Matches[1]) * $b) calc($(Format-Plain $Matches[2]) * $b)"
    } elseif ($value -match "^Sides::pt\($n, $n, $n, $n\)$") {
        $mdValue = "$(Format-Comma $Matches[1]) / $(Format-Comma $Matches[2]) / $(Format-Comma $Matches[3]) / $(Format-Comma $Matches[4]) т."
        $cssValue = "$(Format-Plain $Matches[1])px $(Format-Plain $Matches[2])px $(Format-Plain $Matches[3])px $(Format-Plain $Matches[4])px"
    } elseif ($value -match "^Share\($n / $n\)$") {
        $mdValue = "$(Format-Plain $Matches[1])/$(Format-Plain $Matches[2])"; $cssValue = "calc($(Format-Plain $Matches[1]) / $(Format-Plain $Matches[2]))"
    } elseif ($value -match "^Share\($n\)$") {
        $mdValue = Format-Comma $Matches[1]; $cssValue = Format-Plain $Matches[1]
    } elseif ($value -match "^Clamp::new\(Pt\($n\), Em\($n\), Pt\($n\)\)$") {
        $mdValue = "clamp($(Format-Comma $Matches[1]) т.; $(Format-Comma $Matches[2]) B; $(Format-Comma $Matches[3]) т.)"
        $cssValue = "clamp($(Format-Plain $Matches[1])px, calc($(Format-Plain $Matches[2]) * $b), $(Format-Plain $Matches[3])px)"
    } elseif ($value -match "^Circle::new\($n, $n, Pt\($n\)\)$") {
        $mdValue = "min($(Format-Comma $Matches[1])·W; $(Format-Comma $Matches[2])·H; $(Format-Comma $Matches[3]) т.)"
        $cssValue = "min(calc($(Format-Plain $Matches[1]) * var(--w-w)), calc($(Format-Plain $Matches[2]) * var(--w-h)), $(Format-Plain $Matches[3])px)"
    } elseif ($value -match '^Color::(\w+)$') {
        $colour = Get-ColourName $Matches[1]
        $mdValue = "``$colour``"; $cssValue = "var($(Get-CssName $colour))"
    } elseif ($value -match "^$n$") {
        $mdValue = Format-Comma $Matches[1]; $cssValue = Format-Plain $Matches[1]; $raw[$name] = $Matches[1]
    } else {
        Stop-Export $i "constant $name in a form this script does not know: $value"
    }

    $constants += , @($section, $name, ($doc -join ' '), $mdValue, $cssValue)
    $doc = @()
}

# ---- tokens.css --------------------------------------------------------------------------------

$css = New-Object Collections.Generic.List[string]
$css.Add('/* Generated by tools\export-tokens.ps1 from src\ui\tokens.rs -- do not edit by hand.')
$css.Add('   --w-b is the type size B of the window, --w-w and --w-h its client area: the page sets them. */')
$css.Add('')

$light = @(); $dark = @()
foreach ($c in $colours) {
    $property = Get-CssName $c[1]
    if ($c[2].Count -eq 1) {
        $light += "  ${property}: var($(Get-CssName (Get-ColourName $c[2][0].Substring(1))));"
    } else {
        $light += "  ${property}: $($c[2][0]);"
        $dark += "  ${property}: $($c[2][1]);"
    }
}
$css.Add(':root, :root[data-theme="light"] {'); $light | ForEach-Object { $css.Add($_) }; $css.Add('}')
$css.Add('@media (prefers-color-scheme: dark) {'); $css.Add('  :root:not([data-theme="light"]) {')
$dark | ForEach-Object { $css.Add("  $_") }; $css.Add('  }'); $css.Add('}')
$css.Add(':root[data-theme="dark"] {'); $dark | ForEach-Object { $css.Add($_) }; $css.Add('}')
$css.Add('')
$css.Add(':root {')
foreach ($k in $constants) { $css.Add("  $(Get-CssName $k[1]): $($k[4]);") }
$floor = Format-Plain $raw['TYPE_FLOOR']
foreach ($variant in $roleDoc.Keys) {
    $role = $roleName[$variant]
    if ($null -eq $role) { throw "tokens.rs: role $variant has no name" }
    $style = $roleStyle[$variant]
    if ($null -eq $style) { throw "tokens.rs: role $variant has no style" }
    $prefix = "  --w-role-$($role.Replace('_', '-'))"
    if ($variant -eq 'Brand') {
        $size = "clamp($(Format-Plain $raw['BRAND_MIN'])px, calc($(Format-Plain $raw['BRAND_BY_HEIGHT']) * var(--w-h)), $(Format-Plain $raw['BRAND_MAX'])px)"
    } else {
        $size = "max(${floor}px, calc($(Format-Plain $style.Size) * var(--w-b)))"
    }
    $css.Add("${prefix}-size: $size;")
    $css.Add("${prefix}-line: $(Format-Plain $style.Line);")
    $css.Add("${prefix}-weight: $($style.Weight);")
    $css.Add("${prefix}-face: $($style.Face);")
    if ($null -ne $style.Tracking) { $css.Add("${prefix}-tracking: $(Format-Plain $style.Tracking)em;") }
    if ($null -ne $style.Colour) { $css.Add("${prefix}-color: var($(Get-CssName $style.Colour));") }
}
$css.Add('}')

# ---- PASSPORT-tables.md ------------------------------------------------------------------------

$md = New-Object Collections.Generic.List[string]
$md.Add('# Таблицы паспорта из `src\ui\tokens.rs`')
$md.Add('')
$md.Add('Напечатано `tools\export-tokens.ps1`. Рукой не править: правится `tokens.rs`, таблицы печатаются заново. ◆ — значение ждёт слова владельца по стенду элементов.')

$md.Add(''); $md.Add("## 1. $($sectionTitle[1])"); $md.Add('')
$md.Add('| Имя | Светлая | Тёмная | Где |'); $md.Add('|---|---|---|---|')
foreach ($c in $colours) {
    $where = $c[3]; $mark = ''
    if ($where.StartsWith('◆')) { $mark = ' ◆'; $where = $where.Substring(1).Trim() }
    if ($c[2].Count -eq 1) {
        $alias = Get-ColourName $c[2][0].Substring(1)
        $md.Add("| ``$($c[1])``$mark | = ``$alias`` | = ``$alias`` | $where |")
    } else {
        $md.Add("| ``$($c[1])``$mark | ``$($c[2][0])`` | ``$($c[2][1])`` | $where |")
    }
}

$md.Add(''); $md.Add("## 2. $($sectionTitle[2])"); $md.Add('')
$f = @{}; foreach ($key in @('TYPE_FLOOR', 'B_MIN', 'B_MAX', 'B_BY_WIDTH_BASE', 'B_BY_WIDTH_SLOPE', 'B_BY_HEIGHT_BASE', 'B_BY_HEIGHT_SLOPE', 'BRAND_MIN', 'BRAND_MAX', 'BRAND_BY_HEIGHT')) {
    if (-not $raw.ContainsKey($key)) { throw "tokens.rs: no constant $key" }
    $f[$key] = Format-Comma $raw[$key]
}
$md.Add("**Кегль окна:** ``B = max($($f.TYPE_FLOOR); k × clamp($($f.B_MIN); min($($f.B_BY_WIDTH_BASE) + $($f.B_BY_WIDTH_SLOPE)·W; $($f.B_BY_HEIGHT_BASE) + $($f.B_BY_HEIGHT_SLOPE)·H); $($f.B_MAX)))``. Пол $($f.TYPE_FLOOR) — внутри функции размера роли.")
$md.Add('')
$md.Add('| Роль | Размер | Высота строки | Начертание | Цвет |'); $md.Add('|---|---|---|---|---|')
foreach ($variant in $roleDoc.Keys) {
    $style = $roleStyle[$variant]
    if ($variant -eq 'Brand') {
        $size = "clamp($($f.BRAND_MIN); $($f.BRAND_BY_HEIGHT)·H; $($f.BRAND_MAX))"
    } else {
        $size = "$(Format-Comma $style.Size) B"
    }
    if ($style.Face -eq 'mono') { $size += ', моноширинный' }
    if ($null -ne $style.Tracking) { $size += ", капс, разрядка $(Format-Comma $style.Tracking)" }
    $colour = if ($null -ne $style.Colour) { "``$($style.Colour)``" } else { '—' }
    $md.Add("| ``$($roleName[$variant])`` — $($roleDoc[$variant]) | $size | $(Format-Comma $style.Line) | $($style.Weight) | $colour |")
}

foreach ($s in @(2, 3, 4, 5)) {
    if ($s -ne 2) { $md.Add(''); $md.Add("## $s. $($sectionTitle[$s])") }
    $md.Add('')
    $md.Add('| Что | Значение | Имя |'); $md.Add('|---|---|---|')
    foreach ($k in $constants) {
        if ($k[0] -eq $s) { $md.Add("| $($k[2]) | $($k[3]) | ``$($k[1])`` |") }
    }
}

# ---- Write -------------------------------------------------------------------------------------

if (-not (Test-Path -LiteralPath $OutDir)) { New-Item -ItemType Directory -Path $OutDir | Out-Null }
$utf8 = New-Object Text.UTF8Encoding $false
[IO.File]::WriteAllText((Join-Path $OutDir 'tokens.css'), (($css -join "`r`n") + "`r`n"), $utf8)
[IO.File]::WriteAllText((Join-Path $OutDir 'PASSPORT-tables.md'), (($md -join "`r`n") + "`r`n"), $utf8)
Write-Output "tokens.css: $($colours.Count) colours, $($constants.Count) constants, $($roleDoc.Count) roles; PASSPORT-tables.md written to $OutDir"
