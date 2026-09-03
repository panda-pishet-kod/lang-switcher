<#
    verify-version.ps1 -- one version, nine places, and a gate in front of the build.

    Task T-22-11, finding 18 of the audit of 2026-08-31: "0.1.0" is written out not in three
    files but in FIVE, as NINE values, and not one instrument of the perimeter compared them.
    A release built from a tree where they disagree ships a binary whose VERSIONINFO, whose
    installer and whose assembly identity say three different things -- and the About window
    reads its number from the VERSIONINFO resource, so the user is told the wrong one.

    Cargo.toml is the reference. Everything else is compared against it:

        Cargo.toml                    version = "X.Y.Z"            <- the reference
        app.rc                        FILEVERSION     X,Y,Z,0
        app.rc                        PRODUCTVERSION  X,Y,Z,0
        app.rc                        VALUE "FileVersion",    "X.Y.Z.0"
        app.rc                        VALUE "ProductVersion", "X.Y.Z.0"
        installer\LangSwitcher.iss    #define AppVersion "X.Y.Z"
        installer\LangSwitcher.iss    VersionInfoVersion=X.Y.Z.0
        installer\LangSwitcher.iss    VersionInfoProductVersion=X.Y.Z.0
        app.manifest                  assemblyIdentity version="X.Y.Z.0"
        app-dev.manifest              assemblyIdentity version="X.Y.Z.0"

    A place that is MISSING is a failure and not a pass. An instrument that answers "all
    agree" because its pattern stopped matching is the kind of instrument this project has
    already paid for three times; every row below has to be found before it can be compared.

    WHAT THE NUMBER ITSELF HAS TO BE -- the rule adopted on 2026-09-03, by the user.

        MINOR IS THE NUMBER OF THE DELIVERY TAG. The build tagged `e36` is 0.36.0, `e37` is
        0.37.0, and so on. PATCH exists for a second delivery made under one tag, which is
        not expected and is the escape hatch rather than the road. MAJOR stays 0 until the
        public release, which is a decision of its own and will bring its own rule.

    WHY THIS RULE AND NOT SemVer's "raise it when something breaks". This product has no API
    to break: what a user can see is one executable and one configuration file. The question a
    version has to answer here is therefore "WHICH DELIVERY IS THIS ONE?" -- the question this
    project has had to answer by hand, with SHA-256 sums, at every acceptance -- and tying the
    number to the tag answers it outright. The number is also unfakeable: `git tag` measures
    it, so nobody has to judge whether a change "deserves" a bump, and the version cannot
    quietly stand still for thirty-two deliveries the way 0.1.0 did between 2026-08-14 and
    2026-09-03.

    Решение 8 named 0.1.0 the STARTING version, so this rule continues that decision rather
    than reopening it. `tests\tray.rs` holds the tenth reader of the number -- it parses the
    bytes `rc.exe` really produced -- and it is the one place that has to be edited by hand
    beside the ten below.

    ONE OF THE PATTERNS ALREADY CAUGHT ITSELF DOING THE OTHER THING. The first draft looked
    for `version="..."` in the manifests, and the first `version=` in an XML file is the
    declaration on line 1 -- `<?xml version="1.0" ...?>`. The script reported the two
    manifests as holding "1.0" and failed a tree in which they were perfectly correct. The
    pattern is anchored to <assemblyIdentity> for that reason, and the episode is why the
    -Reference control below reports every place separately instead of one verdict.

    THE POSITIVE CONTROL, and why it needs no file to be edited:

        .\verify-version.ps1 -Reference 9.9.9

    compares the same nine places against a version none of them holds. Every one of them
    must be reported as a mismatch -- which is the proof that every one is really read, and
    not merely that the script found nothing to complain about.

    Written without a single Cyrillic character and for Windows PowerShell 5.1: no `&&`, no
    `??`, no ternary. Decision R-05, as release.ps1 states it.

    Exit code: 0 if every place agrees with the reference, 1 otherwise.

    Examples:
      .\verify-version.ps1
      .\verify-version.ps1 -Reference 9.9.9      # positive control: must FAIL
#>
[CmdletBinding()]
param(
    # The version every place is compared against. Empty means "read it from Cargo.toml",
    # which is what the build does; a value is the positive control.
    [string]$Reference = ''
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

if ($Reference -eq '') {
    $version = $fromCargo
    $control = $false
} else {
    $version = $Reference
    $control = $true
}

$parts = $version.Split('.')
$three = $version
$four = $version + '.0'
$commas = ($parts -join ',') + ',0'

Write-Host ''
Write-Host '======================================================================'
Write-Host ' Lang_Switcher -- one version, nine places'
Write-Host '======================================================================'
Write-Host ("Project     {0}" -f $ProjectDir)
Write-Host ("Cargo.toml  {0}" -f $fromCargo)
if ($control) {
    Write-Host ("Reference   {0}  *** POSITIVE CONTROL: every place must MISMATCH ***" -f $version)
} else {
    Write-Host ("Reference   {0}  (from Cargo.toml)" -f $version)
}
Write-Host ("Forms       three {0} | four {1} | commas {2}" -f $three, $four, $commas)
Write-Host ''

# --- The nine places ------------------------------------------------------------------------
# Each row: the file, a human name, the regex that FINDS the place, and the string its capture
# has to equal. The regex is what makes a missing place a failure: a pattern that matches
# nothing is reported as "not found", never as "agrees".
$rc = Read-Text 'app.rc'
$iss = Read-Text 'installer\LangSwitcher.iss'
$manifest = Read-Text 'app.manifest'
$devManifest = Read-Text 'app-dev.manifest'

$places = @(
    @{ File = 'app.rc';   Name = 'FILEVERSION';               Text = $rc;          Pattern = '(?m)^\s*FILEVERSION\s+([0-9,]+)\s*$';                Want = $commas },
    @{ File = 'app.rc';   Name = 'PRODUCTVERSION';            Text = $rc;          Pattern = '(?m)^\s*PRODUCTVERSION\s+([0-9,]+)\s*$';             Want = $commas },
    @{ File = 'app.rc';   Name = 'VALUE FileVersion';         Text = $rc;          Pattern = 'VALUE\s+"FileVersion",\s*"([0-9.]+)"';               Want = $four },
    @{ File = 'app.rc';   Name = 'VALUE ProductVersion';      Text = $rc;          Pattern = 'VALUE\s+"ProductVersion",\s*"([0-9.]+)"';            Want = $four },
    @{ File = 'installer\LangSwitcher.iss'; Name = 'AppVersion';                 Text = $iss; Pattern = '(?m)^#define\s+AppVersion\s+"([0-9.]+)"';                 Want = $three },
    @{ File = 'installer\LangSwitcher.iss'; Name = 'VersionInfoVersion';         Text = $iss; Pattern = '(?m)^VersionInfoVersion=([0-9.]+)\s*$';                   Want = $four },
    @{ File = 'installer\LangSwitcher.iss'; Name = 'VersionInfoProductVersion';  Text = $iss; Pattern = '(?m)^VersionInfoProductVersion=([0-9.]+)\s*$';            Want = $four },
    @{ File = 'app.manifest';     Name = 'assemblyIdentity';  Text = $manifest;    Pattern = '(?s)<assemblyIdentity\b.*?\sversion="([0-9.]+)"';                                Want = $four },
    @{ File = 'app-dev.manifest'; Name = 'assemblyIdentity';  Text = $devManifest; Pattern = '(?s)<assemblyIdentity\b.*?\sversion="([0-9.]+)"';                                Want = $four }
)

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
