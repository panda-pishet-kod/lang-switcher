<#
    verify-version.ps1 -- one version, two sources, ten places, and a gate in front of the build.

    Task T-22-11, finding 18 of the audit of 2026-08-31: "0.1.0" is written out not in three
    files but in FIVE, as NINE values, and not one instrument of the perimeter compared them.
    A release built from a tree where they disagree ships a binary whose VERSIONINFO, whose
    installer and whose assembly identity say three different things -- and the About window
    reads its number from the VERSIONINFO resource, so the user is told the wrong one.

    TWO SOURCES (stage E105, decision 192.1). Cargo.toml holds both, and everything else is
    compared against them:

        Cargo.toml  [package] version = "X.Y.Z"                          <- source 1
        Cargo.toml  [package.metadata.langswitcher] delivery = D         <- source 2
        app.rc                        FILEVERSION     X,Y,Z,D
        app.rc                        PRODUCTVERSION  X,Y,Z,D
        app.rc                        VALUE "FileVersion",    "X.Y.Z.D"
        app.rc                        VALUE "ProductVersion", "X.Y.Z.D"
        installer\LangSwitcher.iss    #define AppVersion "X.Y.Z"
        installer\LangSwitcher.iss    VersionInfoVersion=X.Y.Z.D
        installer\LangSwitcher.iss    VersionInfoProductVersion=X.Y.Z.D
        app.manifest                  assemblyIdentity version="X.Y.Z.D"
        app-dev.manifest              assemblyIdentity version="X.Y.Z.D"
        updater.manifest              assemblyIdentity version="X.Y.Z.D"

    updater.rc is not a place: build.rs passes it both sources as macros. updater.manifest IS
    one since E105: until then it said a constant "1.0.0.0", which from 1.0.0 on would read like
    a version and be the wrong one at 1.0.1 (decision 197.11).

    A place that is MISSING is a failure and not a pass. An instrument that answers "all
    agree" because its pattern stopped matching is the kind of instrument this project has
    already paid for three times; every row below has to be found before it can be compared.

    WHAT THE NUMBER ITSELF HAS TO BE -- decision 192.1, the owner's word of 2026-10-27.

        X.Y.Z IS AN ORDINARY VERSION from 1.0.0 (the delivery tagged `e105`): X moves for a big
        change (the default behaviour, a break of the settings), Y for new abilities, Z for
        repairs. D IS THE NUMBER OF THE DELIVERY TAG: `e105` is 1.0.0.105. A second delivery
        under one tag raises Z and keeps D. Stage names and tags go on in order (E105, `e105`).
        Up to `e104` (0.104.0) the rule of 2026-09-03 held -- MINOR was the number of the tag,
        and D was 0 -- so a tree whose X is 0 must carry D = 0, and one whose X is 1 or more a D
        of 105 or more: a D the tag cannot have is refused here, in front of the build.

    WHY THE TAG STAYS IN THE NUMBER. The question this project kept answering by hand, with
    SHA-256 sums, at every acceptance is "WHICH DELIVERY IS THIS ONE?"; the fourth field answers
    it, as MINOR did before, while the three fields people see answer the ordinary question.
    "0." said "early development" of a program public since 0.90.0, and the middle digit carried
    the stage number outside ("the hundred and fourth version" after 14 public ones) -- the two
    defects decision 192.1 removed.

    `tests\tray.rs` holds the reader of the number that parses the bytes `rc.exe` really
    produced, and `tests\update.rs` the one that reads the updater's; both are edited by hand
    beside the places below, and that is what makes them the honest red before a bump.

    ONE OF THE PATTERNS ALREADY CAUGHT ITSELF DOING THE OTHER THING. The first draft looked
    for `version="..."` in the manifests, and the first `version=` in an XML file is the
    declaration on line 1 -- `<?xml version="1.0" ...?>`. The script reported the two
    manifests as holding "1.0" and failed a tree in which they were perfectly correct. The
    pattern is anchored to <assemblyIdentity> for that reason, and the episode is why the
    -Reference control below reports every place separately instead of one verdict.

    THE TWO POSITIVE CONTROLS, and why they need no file to be edited:

        .\verify-version.ps1 -Reference 9.9.9
        .\verify-version.ps1 -Delivery 9999

    The first compares the same ten places against a version none of them holds: every one of
    them must be reported as a mismatch -- the proof that every one is really read, and not
    merely that the script found nothing to complain about. The second keeps X.Y.Z and takes a
    fourth field none of them holds: every place that carries a fourth field (nine of the ten;
    AppVersion has three) must be reported, so a gate that compared only three fields would
    fail it (stage E105, decision 192.1).

    Written without a single Cyrillic character and for Windows PowerShell 5.1: no `&&`, no
    `??`, no ternary. Decision R-05, as release.ps1 states it.

    Exit code: 0 if every place agrees with the reference, 1 otherwise.

    Examples:
      .\verify-version.ps1
      .\verify-version.ps1 -Reference 9.9.9      # positive control: must FAIL everywhere
      .\verify-version.ps1 -Delivery 9999        # positive control: every fourth field must FAIL
#>
[CmdletBinding()]
param(
    # The version every place is compared against. Empty means "read it from Cargo.toml",
    # which is what the build does; a value is the first positive control.
    [string]$Reference = '',

    # The fourth field every place is compared against. -1 means "read it from Cargo.toml";
    # a value is the second positive control (decision 192.1).
    [int]$Delivery = -1
)

$ErrorActionPreference = 'Stop'

$ScriptDir = $PSScriptRoot
$ProjectDir = Split-Path -Parent $ScriptDir

function Read-Text { param([string]$Relative)
    $path = Join-Path $ProjectDir $Relative
    if (-not (Test-Path $path)) {
        Write-Host ("FAIL: the file is missing: {0}" -f $path)
        exit 1
    }
    return (Get-Content -Raw -Path $path)
}

# --- The reference --------------------------------------------------------------------------
$cargo = Read-Text 'Cargo.toml'

# The first `version = "..."` of the file, which is the one in [package]: the dependency
# versions below it are `name = { version = ... }` on one line and do not match at line start.
$match = [regex]::Match($cargo, '(?m)^version\s*=\s*"([0-9]+\.[0-9]+\.[0-9]+)"')
if (-not $match.Success) {
    Write-Host 'FAIL: Cargo.toml has no [package] version to take as the reference.'
    exit 1
}

$fromCargo = $match.Groups[1].Value

# Source 2 -- the delivery, inside its own table and nowhere else (decision 192.1).
$table = [regex]::Match($cargo, '(?ms)^\[package\.metadata\.langswitcher\]\s*$(.*?)(?=^\[|\z)')
$deliveryMatch = $null
if ($table.Success) {
    $deliveryMatch = [regex]::Match($table.Groups[1].Value, '(?m)^delivery\s*=\s*([0-9]+)\s*$')
}
if (($null -eq $deliveryMatch) -or (-not $deliveryMatch.Success)) {
    Write-Host 'FAIL: Cargo.toml has no delivery in [package.metadata.langswitcher] (decision 192.1).'
    exit 1
}

$deliveryFromCargo = [int]$deliveryMatch.Groups[1].Value

# The rule itself, on the sources: 0.x carries D = 0 (the old rule), 1.0.0 and later the tag.
$major = [int]$fromCargo.Split('.')[0]
if (($major -eq 0) -and ($deliveryFromCargo -ne 0)) {
    Write-Host ('FAIL: version {0} is of the old rule, its fourth field is 0, not {1}.' -f $fromCargo, $deliveryFromCargo)
    exit 1
}
if (($major -ge 1) -and ($deliveryFromCargo -lt 105)) {
    Write-Host ('FAIL: version {0} carries the delivery tag in its fourth field, 105 or more, not {1}.' -f $fromCargo, $deliveryFromCargo)
    exit 1
}

$control = $false
$controlFourth = $false

if ($Reference -eq '') {
    $version = $fromCargo
} else {
    $version = $Reference
    $control = $true
}

if ($Delivery -lt 0) {
    $fourth = $deliveryFromCargo
} else {
    $fourth = $Delivery
    $controlFourth = $true
}

$parts = $version.Split('.')
$three = $version
$four = $version + '.' + $fourth
$commas = ($parts -join ',') + ',' + $fourth

Write-Host ''
Write-Host '======================================================================'
Write-Host ' Lang_Switcher -- one version, two sources, ten places'
Write-Host '======================================================================'
Write-Host ("Project     {0}" -f $ProjectDir)
Write-Host ("Cargo.toml  {0}, delivery {1}" -f $fromCargo, $deliveryFromCargo)
if ($control) {
    Write-Host ("Reference   {0}  *** POSITIVE CONTROL: every place must MISMATCH ***" -f $version)
} else {
    Write-Host ("Reference   {0}  (from Cargo.toml)" -f $version)
}
if ($controlFourth) {
    Write-Host ("Delivery    {0}  *** POSITIVE CONTROL: every fourth field must MISMATCH ***" -f $fourth)
} else {
    Write-Host ("Delivery    {0}  (from Cargo.toml)" -f $fourth)
}
Write-Host ("Forms       three {0} | four {1} | commas {2}" -f $three, $four, $commas)
Write-Host ''

# --- The ten places -------------------------------------------------------------------------
# Each row: the file, a human name, the regex that FINDS the place, and the string its capture
# has to equal. The regex is what makes a missing place a failure: a pattern that matches
# nothing is reported as "not found", never as "agrees".
$rc = Read-Text 'app.rc'
$iss = Read-Text 'installer\LangSwitcher.iss'
$manifest = Read-Text 'app.manifest'
$devManifest = Read-Text 'app-dev.manifest'
$updaterManifest = Read-Text 'updater.manifest'

$places = @(
    @{ File = 'app.rc';   Name = 'FILEVERSION';               Text = $rc;          Pattern = '(?m)^\s*FILEVERSION\s+([0-9,]+)\s*$';                Want = $commas },
    @{ File = 'app.rc';   Name = 'PRODUCTVERSION';            Text = $rc;          Pattern = '(?m)^\s*PRODUCTVERSION\s+([0-9,]+)\s*$';             Want = $commas },
    @{ File = 'app.rc';   Name = 'VALUE FileVersion';         Text = $rc;          Pattern = 'VALUE\s+"FileVersion",\s*"([0-9.]+)"';               Want = $four },
    @{ File = 'app.rc';   Name = 'VALUE ProductVersion';      Text = $rc;          Pattern = 'VALUE\s+"ProductVersion",\s*"([0-9.]+)"';            Want = $four },
    @{ File = 'installer\LangSwitcher.iss'; Name = 'AppVersion';                 Text = $iss; Pattern = '(?m)^#define\s+AppVersion\s+"([0-9.]+)"';                 Want = $three },
    @{ File = 'installer\LangSwitcher.iss'; Name = 'VersionInfoVersion';         Text = $iss; Pattern = '(?m)^VersionInfoVersion=([0-9.]+)\s*$';                   Want = $four },
    @{ File = 'installer\LangSwitcher.iss'; Name = 'VersionInfoProductVersion';  Text = $iss; Pattern = '(?m)^VersionInfoProductVersion=([0-9.]+)\s*$';            Want = $four },
    @{ File = 'app.manifest';     Name = 'assemblyIdentity';  Text = $manifest;    Pattern = '(?s)<assemblyIdentity\b.*?\sversion="([0-9.]+)"';                                Want = $four },
    @{ File = 'app-dev.manifest'; Name = 'assemblyIdentity';  Text = $devManifest; Pattern = '(?s)<assemblyIdentity\b.*?\sversion="([0-9.]+)"';                                Want = $four },
    @{ File = 'updater.manifest'; Name = 'assemblyIdentity';  Text = $updaterManifest; Pattern = '(?s)<assemblyIdentity\b.*?\sversion="([0-9.]+)"';                            Want = $four }
)

# The places that carry a fourth field -- what the second positive control must see fail.
$withFourth = @($places | Where-Object { $_.Want -ne $three }).Count

$bad = 0
$checked = 0

foreach ($place in $places) {
    $found = [regex]::Match($place.Text, $place.Pattern)

    if (-not $found.Success) {
        Write-Host ("  NOT FOUND  {0,-28} {1,-26} <- the pattern matched nothing" -f $place.File, $place.Name)
        $bad = $bad + 1
        continue
    }

    $checked = $checked + 1
    $got = $found.Groups[1].Value

    if ($got -eq $place.Want) {
        Write-Host ("  ok         {0,-28} {1,-26} {2}" -f $place.File, $place.Name, $got)
    } else {
        Write-Host ("  MISMATCH   {0,-28} {1,-26} {2}  (expected {3})" -f $place.File, $place.Name, $got, $place.Want)
        $bad = $bad + 1
    }
}

Write-Host ''
Write-Host ("  places found and compared: {0} of {1}" -f $checked, $places.Count)
Write-Host ("  disagreements:             {0}" -f $bad)
Write-Host ''

if ($controlFourth -and (-not $control)) {
    # The second control passes when every place with a fourth field FAILS and the three-field
    # one agrees: a gate blind to the fourth field would answer "all agree" here.
    if (($bad -eq $withFourth) -and ($withFourth -eq ($places.Count - 1))) {
        Write-Host '======================================================================'
        Write-Host (' POSITIVE CONTROL PASSED: all {0} fourth fields disagreed, as they must' -f $withFourth)
        Write-Host '======================================================================'
        exit 0
    }

    Write-Host '======================================================================'
    Write-Host (' POSITIVE CONTROL FAILED: only {0} of {1} fourth fields noticed' -f $bad, $withFourth)
    Write-Host ' The instrument cannot be trusted to catch a wrong delivery number.'
    Write-Host '======================================================================'
    exit 1
}

if ($control) {
    # The control passes when the instrument FAILS everywhere. An instrument that answers
    # "all agree" against a version nothing holds is an instrument that reads nothing.
    if ($bad -eq $places.Count) {
        Write-Host '======================================================================'
        Write-Host (' POSITIVE CONTROL PASSED: all {0} places disagreed, as they must' -f $places.Count)
        Write-Host '======================================================================'
        exit 0
    }

    Write-Host '======================================================================'
    Write-Host (' POSITIVE CONTROL FAILED: only {0} of {1} places noticed' -f $bad, $places.Count)
    Write-Host ' The instrument cannot be trusted to catch a real disagreement.'
    Write-Host '======================================================================'
    exit 1
}

if ($bad -ne 0) {
    Write-Host '======================================================================'
    Write-Host (' RESULT: FAIL -- {0} place(s) disagree with Cargo.toml' -f $bad)
    Write-Host '======================================================================'
    exit 1
}

Write-Host '======================================================================'
Write-Host (' RESULT: PASS -- all {0} places say {1}' -f $places.Count, $version)
Write-Host '======================================================================'
exit 0
