<#
    verify-one-body.ps1 -- the full image and the base image are ONE BODY (stage E91, T-91-1).

    Since stage E91 one Release build of the tree ships twice (question 155):

        LangSwitcher.exe        the FULL image -- app.manifest, uiAccess
        LangSwitcher-base.exe   the BASE image -- the same Release with LANGSW_NO_UIACCESS=1,
                                which build.rs answers with app-dev.manifest

    The installer puts the base image down first and replaces it with the full one when the
    machine can trust the full one's signature. That is honest only if the two are the same
    program, and this script proves it from the two UNSIGNED builds by parsing their PE
    structures rather than by trusting the build script:

      1. THE RESOURCE SECTION. Take the full image's resource section, put the base image's
         manifest in place of its own, move every byte that follows the manifest's data by
         the measured shift, add the shift to the address of every resource whose data moved,
         write the base manifest's size into the manifest's data entry -- and the result must
         be the base image's resource section BYTE FOR BYTE. Every other resource is then the
         same bytes in both images, only moved. Inside the manifest, once its XML comments and
         its layout are put aside, only the value of uiAccess may differ.

      2. EVERYTHING ELSE -- the headers and every other section -- is the same in both files
         byte for byte, EXCEPT the fields of the list below.

    THE LIST, by name and by place in its structure. Measured on the pair stage E91 builds
    (scratchpad-E91\t91-1-one-body-measure.log); the absolute offsets are found by parsing at
    every run and printed, because the code grows and an offset typed here would be the offset
    of one build:

      derived from the hash of the content -- /Brepro, SEC-08, build.rs:
        IMAGE_FILE_HEADER.TimeDateStamp            PE header + 8, 4 bytes
        IMAGE_OPTIONAL_HEADER64.CheckSum           optional header + 64, 4 bytes (0 in both
                                                   as built; listed because it is a sum of
                                                   the file)
        IMAGE_DEBUG_DIRECTORY[i].TimeDateStamp     entry + 4, 4 bytes, every entry
        CodeView RSDS GUID and age                 data of the type 2 entry + 4, 16 + 4 bytes
        REPRO hash                                 data of the type 16 entry + 4, as many
                                                   bytes as the DWORD at its start says (32)

      moved by the shift of the manifest -- each by EXACTLY the shift, no other amount:
        POGO record: size of .rsrc$02              data of the type 13 entry, the size of the
                                                   contribution named .rsrc$02
        .rsrc VirtualSize                          section header + 8
        DataDirectory[RESOURCE].Size               optional header + 112 + 2 * 8 + 4
        and, only if the shift moves the end of the section past its file or memory
        alignment: .rsrc SizeOfRawData, SizeOfInitializedData, SizeOfImage, and the file
        offset, the address and the data directories of every section after .rsrc.

    Measured on the pairs of stage E91: the base manifest is several hundred bytes longer than
    the full one, its comments being longer, and the data of the 217 resources that follow it
    in app.rc moves by that length rounded up to 8 bytes. With the manifests of the base commit
    the shift was +312 and nothing crossed an alignment; with longer comments it was +800, the
    section grew past its file and memory alignment, and .rsrc SizeOfRawData and
    SizeOfInitializedData moved by +1024, SizeOfImage, the address of .reloc and its data
    directory by +4096 -- every one of them by exactly that, which is what this script checks.
    The list above covers both cases; the numbers of each delivery are in its release log.

    THE CONTROLS (task T-91-1):
        positive  two builds of the full image in a row -- zero differences, PASS (SEC-08);
        red       the full Release against the Release with --features testing -- another
                  body, FAIL, naming runs in the code and data sections;
        mutant    one line of code changed between the two builds -- FAIL; changed back.

    Exit code: 0 PASS; 1 FAIL -- a difference outside the list, or, under -RequireFullAndBase,
    manifests that are not one full and one base; 2 an input is missing or is not a PE32+ image.

    ASCII only, Windows PowerShell 5.1 (decision R-05).

    Examples:
      .\verify-one-body.ps1 -Full .\full.exe -Base .\base.exe -RequireFullAndBase
      .\verify-one-body.ps1 -Full .\full-1.exe -Base .\full-2.exe
#>
[CmdletBinding()]
param(
    # The unsigned full image.
    [Parameter(Mandatory = $true)][string]$Full,
    # The unsigned base image -- or, for the positive control, a second build of the full one.
    [Parameter(Mandatory = $true)][string]$Base,
    # Require the manifest of -Full to ask for uiAccess="true" and that of -Base for "false".
    [switch]$RequireFullAndBase,
    # How many differing runs to print in full; the count is always printed.
    [int]$ShowRuns = 40
)

$ErrorActionPreference = 'Stop'

function U16([byte[]]$b, [long]$at) { return [long][BitConverter]::ToUInt16($b, [int]$at) }
function U32([byte[]]$b, [long]$at) { return [long][BitConverter]::ToUInt32($b, [int]$at) }
function Set-U32([byte[]]$b, [long]$at, [long]$value) {
    [Array]::Copy([BitConverter]::GetBytes([uint32]$value), 0, $b, [int]$at, 4)
}

function Read-Pe([string]$Path) {
    if (-not (Test-Path -LiteralPath $Path)) { throw ("missing: {0}" -f $Path) }
    $b = [System.IO.File]::ReadAllBytes((Resolve-Path -LiteralPath $Path).Path)
    if ($b.Length -lt 512 -or $b[0] -ne 0x4D -or $b[1] -ne 0x5A) { throw ("not an MZ image: {0}" -f $Path) }
    $pe = U32 $b 0x3C
    if ((U32 $b $pe) -ne 0x00004550) { throw ("no PE signature: {0}" -f $Path) }
    $count = U16 $b ($pe + 6)
    $optSize = U16 $b ($pe + 20)
    $opt = $pe + 24
    if ((U16 $b $opt) -ne 0x20B) { throw ("not PE32+: {0}" -f $Path) }
    $dirs = @()
    for ($i = 0; $i -lt (U32 $b ($opt + 108)); $i++) {
        $at = $opt + 112 + 8 * $i
        $dirs += @{ Index = $i; At = $at; Rva = (U32 $b $at); Size = (U32 $b ($at + 4)) }
    }
    $table = $opt + $optSize
    $list = @()
    for ($i = 0; $i -lt $count; $i++) {
        $at = $table + 40 * $i
        $name = ([System.Text.Encoding]::ASCII.GetString($b, [int]$at, 8)).TrimEnd([char]0)
        $list += @{ Index = $i; Name = $name; At = $at; VirtualSize = (U32 $b ($at + 8));
                    VirtualAddress = (U32 $b ($at + 12)); RawSize = (U32 $b ($at + 16)); RawAt = (U32 $b ($at + 20)) }
    }
    return @{ Path = $Path; Bytes = $b; Pe = $pe; Opt = $opt; Dirs = $dirs; Sections = $list;
              SizeOfHeaders = (U32 $b ($opt + 60)); SectionAlignment = (U32 $b ($opt + 32));
              FileAlignment = (U32 $b ($opt + 36)) }
}

function Get-SectionOfRva($img, [long]$rva) {
    foreach ($s in $img.Sections) {
        if ($rva -ge $s.VirtualAddress -and $rva -lt $s.VirtualAddress + [Math]::Max($s.VirtualSize, $s.RawSize)) { return $s }
    }
    return $null
}

function Get-OffsetOfRva($img, [long]$rva) {
    $s = Get-SectionOfRva $img $rva
    if ($null -eq $s) { return -1 }
    return $s.RawAt + ($rva - $s.VirtualAddress)
}

function Get-Section($img, [string]$name) {
    foreach ($s in $img.Sections) { if ($s.Name -eq $name) { return $s } }
    return $null
}

# The debug directory: one record per entry, with the file offsets of what this script reads.
function Get-DebugEntries($img) {
    $b = $img.Bytes
    $out = New-Object System.Collections.ArrayList
    $dir = $img.Dirs | Where-Object { $_.Index -eq 6 }
    if (-not $dir -or $dir.Rva -eq 0) { return ,$out }
    $at = Get-OffsetOfRva $img $dir.Rva
    for ($i = 0; $i -lt [int]($dir.Size / 28); $i++) {
        $e = $at + 28 * $i
        [void]$out.Add(@{ Index = $i; At = $e; Type = (U32 $b ($e + 12)); Size = (U32 $b ($e + 16)); Data = (U32 $b ($e + 24)) })
    }
    return ,$out
}

# The fields derived from the hash of the content, as file offsets of one image.
function Get-HashFields($img) {
    $b = $img.Bytes
    $out = New-Object System.Collections.ArrayList
    [void]$out.Add(@{ Start = $img.Pe + 8; Length = 4; Name = 'IMAGE_FILE_HEADER.TimeDateStamp' })
    [void]$out.Add(@{ Start = $img.Opt + 64; Length = 4; Name = 'IMAGE_OPTIONAL_HEADER64.CheckSum' })
    foreach ($e in (Get-DebugEntries $img)) {
        [void]$out.Add(@{ Start = $e.At + 4; Length = 4; Name = ('IMAGE_DEBUG_DIRECTORY[{0}].TimeDateStamp (type {1})' -f $e.Index, $e.Type) })
        if ($e.Type -eq 2 -and $e.Size -ge 24 -and (U32 $b $e.Data) -eq 0x53445352) {
            [void]$out.Add(@{ Start = $e.Data + 4; Length = 16; Name = 'CodeView RSDS GUID' })
            [void]$out.Add(@{ Start = $e.Data + 20; Length = 4; Name = 'CodeView RSDS age' })
        }
        if ($e.Type -eq 16 -and $e.Size -ge 4) {
            $n = U32 $b $e.Data
            [void]$out.Add(@{ Start = $e.Data + 4; Length = $n; Name = ('REPRO hash, {0} bytes' -f $n) })
        }
    }
    return ,$out
}

# The POGO record of the debug directory (type 13): after a 4-byte signature, entries of
# { RVA, size, name } with the name zero-terminated and padded to 4 bytes. Returns the file
# offset of the size of the contribution named $name, or -1.
function Find-PogoSize($img, [string]$name) {
    $b = $img.Bytes
    foreach ($e in (Get-DebugEntries $img)) {
        if ($e.Type -ne 13) { continue }
        $p = $e.Data + 4
        $end = $e.Data + $e.Size
        while ($p + 8 -lt $end) {
            $q = $p + 8
            $sb = New-Object System.Text.StringBuilder
            while ($q -lt $end -and $b[$q] -ne 0) { [void]$sb.Append([char]$b[$q]); $q++ }
            if ($sb.ToString() -eq $name) { return $p + 4 }
            $p = $p + 8 + [Math]::Ceiling(($sb.Length + 1) / 4) * 4
        }
    }
    return -1
}

# Every data entry of the resource tree: its path, the section offset of the entry, its data.
function Get-ResourceEntries($img) {
    $b = $img.Bytes
    $out = New-Object System.Collections.ArrayList
    $dir = $img.Dirs | Where-Object { $_.Index -eq 2 }
    if (-not $dir -or $dir.Rva -eq 0) { return ,$out }
    $root = Get-OffsetOfRva $img $dir.Rva
    $stack = New-Object System.Collections.Stack
    $stack.Push(@{ At = $root; Path = '' })
    while ($stack.Count -gt 0) {
        $node = $stack.Pop()
        $n = (U16 $b ($node.At + 12)) + (U16 $b ($node.At + 14))
        for ($i = $n - 1; $i -ge 0; $i--) {
            $e = $node.At + 16 + 8 * $i
            $id = U32 $b $e
            $to = U32 $b ($e + 4)
            if (($id -band 0x80000000) -ne 0) { $key = 'name@' + ($id -band 0x7FFFFFFF) } else { $key = [string]$id }
            $path = $node.Path + '/' + $key
            if (($to -band 0x80000000) -ne 0) {
                $stack.Push(@{ At = $root + ($to -band 0x7FFFFFFF); Path = $path })
            } else {
                $entry = $root + $to
                [void]$out.Add(@{ Path = $path; EntryAt = $entry; Rva = (U32 $b $entry); Size = (U32 $b ($entry + 4)) })
            }
        }
    }
    return ,$out
}

function Get-UiAccess([string]$xml) {
    $m = [regex]::Match($xml, 'uiAccess="(true|false)"')
    if ($m.Success) { return $m.Groups[1].Value }
    return '(none)'
}

function Get-ManifestSkeleton([string]$xml) {
    $t = [regex]::Replace($xml, '<!--.*?-->', '', [System.Text.RegularExpressions.RegexOptions]::Singleline)
    $t = [regex]::Replace($t, 'uiAccess="(true|false)"', 'uiAccess="?"')
    return ([regex]::Replace($t, '\s+', ' ')).Trim()
}

# Runs of differing bytes between two spans: a list of @{ X; Y; Length }.
function Get-DiffRuns([byte[]]$x, [long]$xStart, [byte[]]$y, [long]$yStart, [long]$length) {
    $out = New-Object System.Collections.ArrayList
    $i = 0
    $open = -1
    while ($i -lt $length) {
        $px = $xStart + $i; $py = $yStart + $i
        $same = ($px -lt $x.Length) -and ($py -lt $y.Length) -and ($x[$px] -eq $y[$py])
        if ($same) {
            if ($open -ge 0) { [void]$out.Add(@{ X = $xStart + $open; Y = $yStart + $open; Length = $i - $open }); $open = -1 }
            while (($i + 8) -le $length -and ($xStart + $i + 8) -le $x.Length -and ($yStart + $i + 8) -le $y.Length -and
                   [BitConverter]::ToUInt64($x, [int]($xStart + $i)) -eq [BitConverter]::ToUInt64($y, [int]($yStart + $i))) { $i += 8 }
            if ($i -lt $length) {
                $px = $xStart + $i; $py = $yStart + $i
                if (($px -lt $x.Length) -and ($py -lt $y.Length) -and ($x[$px] -eq $y[$py])) { $i++ }
            }
            continue
        }
        if ($open -lt 0) { $open = $i }
        $i++
    }
    if ($open -ge 0) { [void]$out.Add(@{ X = $xStart + $open; Y = $yStart + $open; Length = $length - $open }) }
    return ,$out
}

function Find-Place($img, [long]$at) {
    if ($at -lt $img.SizeOfHeaders) { return ('headers+0x{0:X}' -f $at) }
    foreach ($s in $img.Sections) {
        if ($at -ge $s.RawAt -and $at -lt $s.RawAt + $s.RawSize) { return ('{0}+0x{1:X}' -f $s.Name, ($at - $s.RawAt)) }
    }
    return ('overlay+0x{0:X}' -f $at)
}

Write-Host ''
Write-Host '======================================================================'
Write-Host ' Lang_Switcher -- one body: the full and the base image (T-91-1)'
Write-Host '======================================================================'

try {
    $imgFull = Read-Pe $Full
    $imgBase = Read-Pe $Base
} catch {
    Write-Host ("FAIL: {0}" -f $_.Exception.Message)
    exit 2
}

foreach ($img in @($imgFull, $imgBase)) {
    Write-Host ("{0}" -f $img.Path)
    Write-Host ("    size {0} bytes, SHA-256 {1}" -f $img.Bytes.Length, (Get-FileHash -LiteralPath $img.Path -Algorithm SHA256).Hash)
    foreach ($s in $img.Sections) {
        Write-Host ("    {0,-8} VA 0x{1:X8}  vsize {2,8}  raw 0x{3:X8} size {4,8}" -f $s.Name, $s.VirtualAddress, $s.VirtualSize, $s.RawAt, $s.RawSize)
    }
}

$failures = New-Object System.Collections.ArrayList
$runs = New-Object System.Collections.ArrayList   # @{ At; Length; Verdict; Name } -- At is in the full image

# --- 0. The shape -------------------------------------------------------------------------------
# Different offsets of the PE header are a difference of the headers (another DOS stub or Rich
# header -- another set of objects linked), so they fail the comparison below and do not stop it:
# a red answer that names the sections where the bodies differ is worth more than "no shape".
$shapeNotes = New-Object System.Collections.ArrayList
if ($imgFull.Pe -ne $imgBase.Pe) { [void]$shapeNotes.Add(('the PE headers start at 0x{0:X} and 0x{1:X}' -f $imgFull.Pe, $imgBase.Pe)) }
if ($imgFull.Sections.Count -ne $imgBase.Sections.Count) {
    [void]$failures.Add(('the images have {0} and {1} sections' -f $imgFull.Sections.Count, $imgBase.Sections.Count))
} else {
    for ($i = 0; $i -lt $imgFull.Sections.Count; $i++) {
        if ($imgFull.Sections[$i].Name -ne $imgBase.Sections[$i].Name) {
            [void]$failures.Add(('section {0} is {1} in one image and {2} in the other' -f $i, $imgFull.Sections[$i].Name, $imgBase.Sections[$i].Name))
        }
    }
}
$rsF = Get-Section $imgFull '.rsrc'
$rsB = Get-Section $imgBase '.rsrc'
if ($null -eq $rsF -or $null -eq $rsB) { [void]$failures.Add('there is no .rsrc section') }
if ($failures.Count -gt 0) {
    foreach ($f in $failures) { Write-Host ("  FAIL: {0}" -f $f) }
    Write-Host ' RESULT: FAIL -- the two images do not even have one shape'
    exit 1
}

# --- 1. The manifest and the resource section ------------------------------------------------------
Write-Host ''
Write-Host '--- 1. The resource section ------------------------------------------'
$resF = Get-ResourceEntries $imgFull
$resB = Get-ResourceEntries $imgBase
$manF = $resF | Where-Object { $_.Path -like '/24/*' } | Select-Object -First 1
$manB = $resB | Where-Object { $_.Path -like '/24/*' } | Select-Object -First 1
$shift = 0
if ($null -eq $manF -or $null -eq $manB) {
    [void]$failures.Add('a manifest resource (type 24) was not found in one of the images')
} else {
    $textF = [System.Text.Encoding]::UTF8.GetString($imgFull.Bytes, [int](Get-OffsetOfRva $imgFull $manF.Rva), [int]$manF.Size)
    $textB = [System.Text.Encoding]::UTF8.GetString($imgBase.Bytes, [int](Get-OffsetOfRva $imgBase $manB.Rva), [int]$manB.Size)
    $uF = Get-UiAccess $textF
    $uB = Get-UiAccess $textB
    Write-Host ("  resources: {0} and {1}; manifest {2}" -f $resF.Count, $resB.Count, $manF.Path)
    Write-Host ("  full: manifest {0} bytes at RVA 0x{1:X}, uiAccess={2}" -f $manF.Size, $manF.Rva, $uF)
    Write-Host ("  base: manifest {0} bytes at RVA 0x{1:X}, uiAccess={2}" -f $manB.Size, $manB.Rva, $uB)
    $same = (Get-ManifestSkeleton $textF) -eq (Get-ManifestSkeleton $textB)
    Write-Host ("  without XML comments and layout, the manifests differ only in uiAccess: {0}" -f $same)
    if (-not $same) { [void]$failures.Add('the manifests differ in more than their comments, their layout and uiAccess') }
    if ($RequireFullAndBase) {
        if ($uF -ne 'true') { [void]$failures.Add(('-Full is not the full image: its manifest has uiAccess={0}' -f $uF)) }
        if ($uB -ne 'false') { [void]$failures.Add(('-Base is not the base image: its manifest has uiAccess={0}' -f $uB)) }
    }

    # The same resources, in the same order of the tree.
    $pathsF = ($resF | ForEach-Object { $_.Path }) -join ' '
    $pathsB = ($resB | ForEach-Object { $_.Path }) -join ' '
    if ($pathsF -ne $pathsB) { [void]$failures.Add('the two resource trees do not hold the same resources') }
    if ($manF.Rva -ne $manB.Rva) { [void]$failures.Add('the manifest data starts at different addresses') }

    # The shift: what the data after the manifest moved by -- one value for every resource, or none.
    $after = @($resF | Where-Object { $_.Rva -gt $manF.Rva })
    $shifts = @()
    foreach ($r in $after) {
        $twin = $resB | Where-Object { $_.Path -eq $r.Path } | Select-Object -First 1
        $shifts += ($twin.Rva - $r.Rva)
    }
    $distinct = @($shifts | Sort-Object -Unique)
    if ($distinct.Count -gt 1) {
        [void]$failures.Add(('the resources after the manifest moved by different amounts: {0}' -f ($distinct -join ', ')))
    }
    if ($distinct.Count -eq 1) { $shift = [long]$distinct[0] } else { $shift = [long]($rsB.VirtualSize - $rsF.VirtualSize) }
    Write-Host ("  {0} resource(s) lie after the manifest; their data moved by {1:+#;-#;0} byte(s)" -f $after.Count, $shift)

    # The reconstruction. The full image's section, with the base manifest in place of its own
    # and everything after it moved by the shift, must be the base image's section exactly.
    $mOff = $manF.Rva - $rsF.VirtualAddress
    $allocF = $rsF.VirtualSize - $mOff
    if ($after.Count -gt 0) { $allocF = (($after | ForEach-Object { $_.Rva }) | Measure-Object -Minimum).Minimum - $manF.Rva }
    foreach ($r in $resF) {
        if (($r.EntryAt - $rsF.RawAt) + 16 -gt $mOff) { [void]$failures.Add(('the data entry of {0} lies after the manifest data: a layout this script does not know' -f $r.Path)) }
    }
    if ($allocF -lt $manF.Size -or $mOff + $allocF + $manB.Size -lt 0) { [void]$failures.Add('the manifest data does not fit the place the tree gives it') }
}

if ($failures.Count -eq 0) {
    $secF = New-Object byte[] $rsF.RawSize
    $secB = New-Object byte[] $rsB.RawSize
    [Array]::Copy($imgFull.Bytes, [int]$rsF.RawAt, $secF, 0, [int]$rsF.RawSize)
    [Array]::Copy($imgBase.Bytes, [int]$rsB.RawAt, $secB, 0, [int]$rsB.RawSize)
    $expected = New-Object byte[] $rsB.RawSize
    $headLength = [Math]::Min($mOff, $expected.Length)
    [Array]::Copy($secF, 0, $expected, 0, [int]$headLength)
    $manAtB = Get-OffsetOfRva $imgBase $manB.Rva
    [Array]::Copy($imgBase.Bytes, [int]$manAtB, $expected, [int]$mOff, [int][Math]::Min($manB.Size, $expected.Length - $mOff))
    $tailFrom = $mOff + $allocF
    $tailTo = $mOff + $allocF + $shift
    $tailLength = [Math]::Min($secF.Length - $tailFrom, $expected.Length - $tailTo)
    if ($tailLength -gt 0) { [Array]::Copy($secF, [int]$tailFrom, $expected, [int]$tailTo, [int]$tailLength) }
    foreach ($r in $after) { Set-U32 $expected ($r.EntryAt - $rsF.RawAt) ($r.Rva + $shift) }
    Set-U32 $expected ($manF.EntryAt - $rsF.RawAt + 4) $manB.Size

    $rsrcRuns = Get-DiffRuns $expected 0 $secB 0 ([Math]::Max($expected.Length, $secB.Length))
    $moved = 0
    foreach ($r in $after) { $moved += 1 }
    Write-Host ("  reconstruction: full section, base manifest in place, {0} data address(es) moved, tail moved by {1:+#;-#;0}" -f $moved, $shift)
    if ($rsrcRuns.Count -eq 0) {
        Write-Host '  the base image''s resource section is exactly that reconstruction: every other resource is the same bytes'
    } else {
        foreach ($r in $rsrcRuns) {
            [void]$runs.Add(@{ At = $rsB.RawAt + $r.Y; Length = $r.Length; Verdict = 'FORBIDDEN'; Name = ('.rsrc, not the manifest (base .rsrc+0x{0:X})' -f $r.Y) })
        }
        [void]$failures.Add(('{0} run(s) of the base resource section are not the full one with the base manifest put in' -f $rsrcRuns.Count))
    }
}

# --- 2. Everything else ------------------------------------------------------------------------------
Write-Host ''
Write-Host '--- 2. Headers and the other sections --------------------------------'
$allowF = Get-HashFields $imgFull
$allowB = Get-HashFields $imgBase

# Layout fields: each must differ by exactly its shift -- then, and only then, it is excused.
$layout = New-Object System.Collections.ArrayList
$dRaw = $rsB.RawSize - $rsF.RawSize
$align = $imgFull.SectionAlignment
$dVa = ([Math]::Ceiling($rsB.VirtualSize / $align) - [Math]::Ceiling($rsF.VirtualSize / $align)) * $align
[void]$layout.Add(@{ AtF = $rsF.At + 8; AtB = $rsB.At + 8; Delta = $shift; Name = '.rsrc VirtualSize' })
[void]$layout.Add(@{ AtF = $imgFull.Opt + 132; AtB = $imgBase.Opt + 132; Delta = $shift; Name = 'DataDirectory[RESOURCE].Size' })
$pogoF = Find-PogoSize $imgFull '.rsrc$02'
$pogoB = Find-PogoSize $imgBase '.rsrc$02'
if ($pogoF -ge 0 -and $pogoB -ge 0) {
    [void]$layout.Add(@{ AtF = $pogoF; AtB = $pogoB; Delta = $shift; Name = 'POGO record: size of .rsrc$02' })
}
[void]$layout.Add(@{ AtF = $rsF.At + 16; AtB = $rsB.At + 16; Delta = $dRaw; Name = '.rsrc SizeOfRawData' })
[void]$layout.Add(@{ AtF = $imgFull.Opt + 8; AtB = $imgBase.Opt + 8; Delta = $dRaw; Name = 'IMAGE_OPTIONAL_HEADER64.SizeOfInitializedData' })
[void]$layout.Add(@{ AtF = $imgFull.Opt + 56; AtB = $imgBase.Opt + 56; Delta = $dVa; Name = 'IMAGE_OPTIONAL_HEADER64.SizeOfImage' })
for ($i = 0; $i -lt $imgFull.Sections.Count; $i++) {
    $s = $imgFull.Sections[$i]
    $t = $imgBase.Sections[$i]
    if ($s.VirtualAddress -le $rsF.VirtualAddress) { continue }
    [void]$layout.Add(@{ AtF = $s.At + 12; AtB = $t.At + 12; Delta = $dVa; Name = ('{0} VirtualAddress' -f $s.Name) })
    [void]$layout.Add(@{ AtF = $s.At + 20; AtB = $t.At + 20; Delta = $dRaw; Name = ('{0} PointerToRawData' -f $s.Name) })
    foreach ($d in $imgFull.Dirs) {
        if ($d.Rva -ge $s.VirtualAddress -and $d.Rva -lt $s.VirtualAddress + [Math]::Max($s.VirtualSize, $s.RawSize)) {
            $twin = $imgBase.Dirs | Where-Object { $_.Index -eq $d.Index }
            [void]$layout.Add(@{ AtF = $d.At; AtB = $twin.At; Delta = $dVa; Name = ('DataDirectory[{0}].VirtualAddress' -f $d.Index) })
        }
    }
}
foreach ($f in $layout) {
    $vF = U32 $imgFull.Bytes $f.AtF
    $vB = U32 $imgBase.Bytes $f.AtB
    if (($vB - $vF) -ne $f.Delta) {
        [void]$failures.Add(('{0}: {1} -> {2}, not the shift {3:+#;-#;0}' -f $f.Name, $vF, $vB, $f.Delta))
    } elseif ($f.Delta -ne 0) {
        Write-Host ("  layout   {0,-46} {1} -> {2} ({3:+#;-#;0})" -f $f.Name, $vF, $vB, $f.Delta)
        [void]$allowF.Add(@{ Start = $f.AtF; Length = 4; Name = ('layout: ' + $f.Name) })
        [void]$allowB.Add(@{ Start = $f.AtB; Length = 4; Name = ('layout: ' + $f.Name) })
    }
}

# A differing byte at $atF of the full image and $atB of the base image is allowed when both lie
# at the same place of one named field of the list.
function Get-AllowedName([long]$atF, [long]$atB) {
    foreach ($r in $allowF) {
        if ($atF -ge $r.Start -and $atF -lt $r.Start + $r.Length) {
            foreach ($q in $allowB) {
                if ($q.Name -eq $r.Name -and ($atB - $q.Start) -eq ($atF - $r.Start)) { return $r.Name }
            }
        }
    }
    return $null
}

function Add-Classified($diffRuns, [string]$area) {
    foreach ($d in $diffRuns) {
        $k = 0
        $open = $null
        while ($k -lt $d.Length) {
            $name = Get-AllowedName ($d.X + $k) ($d.Y + $k)
            if ($null -ne $name) { $verdict = 'allowed' } else { $verdict = 'FORBIDDEN'; $name = $area }
            if ($null -ne $open -and ($open.Verdict -ne $verdict -or $open.Name -ne $name)) { [void]$runs.Add($open); $open = $null }
            if ($null -eq $open) { $open = @{ At = $d.X + $k; Length = 0; Verdict = $verdict; Name = $name } }
            $open.Length++
            $k++
        }
        if ($null -ne $open) { [void]$runs.Add($open) }
    }
}

Add-Classified (Get-DiffRuns $imgFull.Bytes 0 $imgBase.Bytes 0 ([Math]::Max($imgFull.SizeOfHeaders, $imgBase.SizeOfHeaders))) 'headers'
for ($i = 0; $i -lt $imgFull.Sections.Count; $i++) {
    $sF = $imgFull.Sections[$i]; $sB = $imgBase.Sections[$i]
    if ($sF.Name -eq '.rsrc') { continue }
    Add-Classified (Get-DiffRuns $imgFull.Bytes $sF.RawAt $imgBase.Bytes $sB.RawAt ([Math]::Max($sF.RawSize, $sB.RawSize))) $sF.Name
}
$endF = 0; $endB = 0
foreach ($s in $imgFull.Sections) { $endF = [Math]::Max($endF, $s.RawAt + $s.RawSize) }
foreach ($s in $imgBase.Sections) { $endB = [Math]::Max($endB, $s.RawAt + $s.RawSize) }
if (($imgFull.Bytes.Length -ne $endF) -or ($imgBase.Bytes.Length -ne $endB)) {
    [void]$failures.Add(('an image carries {0} / {1} byte(s) after its last section: a signed file, or not a build of this tree' -f ($imgFull.Bytes.Length - $endF), ($imgBase.Bytes.Length - $endB)))
}

# --- The runs, and the verdict ---------------------------------------------------------------------
Write-Host ''
Write-Host '--- Differing runs outside the manifest ------------------------------'
$forbidden = @($runs | Where-Object { $_.Verdict -eq 'FORBIDDEN' })
$allowed = @($runs | Where-Object { $_.Verdict -eq 'allowed' })
$shown = 0
foreach ($r in ($runs | Sort-Object { $_.At })) {
    if ($shown -ge $ShowRuns) { break }
    Write-Host ("  {0,-9} 0x{1:X8} {2,6} byte(s)  {3,-20} {4}" -f $r.Verdict, $r.At, $r.Length, (Find-Place $imgFull $r.At), $r.Name)
    $shown++
}
if ($runs.Count -gt $shown) { Write-Host ("  ... {0} more run(s) not printed" -f ($runs.Count - $shown)) }
$bytesForbidden = 0
foreach ($r in $forbidden) { $bytesForbidden += $r.Length }
Write-Host ("  runs: {0} allowed, {1} forbidden ({2} byte(s))" -f $allowed.Count, $forbidden.Count, $bytesForbidden)
foreach ($g in ($forbidden | Group-Object { ($_.Name -split ',')[0] })) {
    $sum = 0
    foreach ($r in $g.Group) { $sum += $r.Length }
    Write-Host ("    forbidden in {0,-24} {1,7} run(s) {2,9} byte(s)" -f $g.Name, $g.Count, $sum)
}
if ($forbidden.Count -gt 0) {
    $areas = @($forbidden | ForEach-Object { ($_.Name -split ',')[0] } | Sort-Object -Unique)
    [void]$failures.Add(('{0} run(s) differ outside the list, in: {1}' -f $forbidden.Count, ($areas -join ', ')))
}
foreach ($n in $shapeNotes) { [void]$failures.Add($n) }

Write-Host ''
Write-Host '======================================================================'
if ($failures.Count -gt 0) {
    foreach ($f in ($failures | Select-Object -Unique)) { Write-Host ("  FAIL: {0}" -f $f) }
    Write-Host ' RESULT: FAIL -- the two images are not one body'
    Write-Host '======================================================================'
    exit 1
}
Write-Host (' RESULT: PASS -- one body: the manifest, plus {0} run(s) of listed fields, and nothing else' -f $allowed.Count)
Write-Host '======================================================================'
exit 0
