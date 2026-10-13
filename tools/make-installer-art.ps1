<#
    make-installer-art.ps1 -- generator of the pictures of the installer wizard, stage E99,
    decision 167.1 of DECISIONS.md: variant B "colour and bamboo" with the signature B1, chosen by
    the owner on the mock-up (question 167).

    WHY A GENERATOR AND NOT EXPORTED ARTWORK. The same reason as tools\make-icons.ps1: the
    pictures are drawn from the vector description of the site, so a change there is a change of
    numbers here, and two runs give the same bytes.

    OUTPUT: installer\art\, twenty PNG files --
      back-<theme>-<scale>.png  the background of the wizard (WizardBackImageFile and
                                WizardBackImageFileDynamicDark): the bamboo of the margins of the
                                site, a column at the left edge and one at the right, as tall as
                                the picture, on transparency -- WizardBackColor shows through;
      logo-<theme>-<scale>.png  the logo of the signature of the author (B1), extracted by [Code];
    for <theme> light and dark and <scale> 100, 125, 150, 175, 200.

    WHERE THE NUMBERS COME FROM -- the sources of the site panda-pishet-kod.dev (its repository
    PandaKod) at 91f4e99, palette, bamboo and logo unchanged since 55e309b; one for one:
      gen\bamboo.mjs  stalk (line 18), leaf (31), sprig (38), PLAN_TALL -- the margins of a page,
                      a column of 300 x 1000 (49), column (65); the rounding n() to two decimals;
      gen\site.css    light: --bg #FAF9F6, --ink #1C1B18, --accent #1F7A4A, --bam-far/mid/near
                      0.07/0.11/0.17 (lines 8, 9, 13, 16); dark: #23272F, #D6DAE0, #6CC894,
                      0.10/0.16/0.24 (91, 92, 96, 99);
      gen\build.mjs   logo_svg, 28 x 28 (230): ears, patches, nose -- currentColor = --ink; the
                      face -- --bg; the stalk and the leaf -- --accent.
    The column keeps its proportion and is scaled to the height of the picture: 0.3 H wide, H/1000
    a unit, and it clips what grows past its edges, as the nested svg of the mock-up does
    (preserveAspectRatio xMidYMax slice of the site, which at this proportion is no offset). An
    element with an opacity is drawn as one fill, the way SVG composites an element: the
    segments of a stalk are one path (their round caps overlap and must not darken twice), and
    each leaf is its own element.

    THE SIZES -- the table of the Inno Setup 6.7 help, WizardBackImageFile, for the default font:
    596x432, 796x576, 994x720, 1193x864, 1272x922 at 100, 125, 150, 175, 200 %; Inno takes the
    nearest. The logo is 28 logical pixels, ScaleX(28) of [Code]; Inno scales by the base units of
    its font, so its factor is the ratio of those areas, not the DPI: 28, 37, 47, 56, 60.

    Every picture is drawn before any file is written (the lesson of make-icons.ps1, T-85-3).
    The guard of the result is tests\installer_art.rs.
#>

Add-Type -AssemblyName System.Drawing
$ErrorActionPreference = 'Stop'

$OutDir = Join-Path (Split-Path -Parent $PSScriptRoot) 'installer\art'

$SCALES = @(
  @{ Name = '100'; W = 596;  H = 432; Logo = 28 },
  @{ Name = '125'; W = 796;  H = 576; Logo = 37 },
  @{ Name = '150'; W = 994;  H = 720; Logo = 47 },
  @{ Name = '175'; W = 1193; H = 864; Logo = 56 },
  @{ Name = '200'; W = 1272; H = 922; Logo = 60 }
)

function Hex([string]$rgb) {
  [System.Drawing.Color]::FromArgb(255, [Convert]::ToInt32($rgb.Substring(1, 2), 16),
    [Convert]::ToInt32($rgb.Substring(3, 2), 16), [Convert]::ToInt32($rgb.Substring(5, 2), 16))
}

$THEMES = @(
  @{ Name = 'light'; Bg = (Hex '#FAF9F6'); Ink = (Hex '#1C1B18'); Accent = (Hex '#1F7A4A'); Far = 0.07; Mid = 0.11; Near = 0.17 },
  @{ Name = 'dark';  Bg = (Hex '#23272F'); Ink = (Hex '#D6DAE0'); Accent = (Hex '#6CC894'); Far = 0.10; Mid = 0.16; Near = 0.24 }
)

# gen\bamboo.mjs, PLAN_TALL: the margins of a page, a column of 300 x 1000.
$PLAN_TALL = @(
  @{ Xr = 0.62; Top = 0.06; W = 9;  Leaf = 58;  Layer = 'Far' },
  @{ Xr = 0.36; Top = 0.26; W = 12; Leaf = 74;  Layer = 'Mid' },
  @{ Xr = 0.80; Top = 0.34; W = 10; Leaf = 62;  Layer = 'Far' },
  @{ Xr = 0.14; Top = 0.46; W = 17; Leaf = 104; Layer = 'Near' },
  @{ Xr = 0.50; Top = 0.58; W = 13; Leaf = 80;  Layer = 'Mid' }
)
$COLUMN_W = 300
$COLUMN_H = 1000

# n() of gen\bamboo.mjs: Number(v.toFixed(2)).
function N([double]$v) { [math]::Round($v, 2, [MidpointRounding]::AwayFromZero) }

# A colour of the accent with the opacity of a layer, the way the browser rounds it to a byte.
function Layer-Brush($theme, [string]$layer) {
  $alpha = [int][math]::Round($theme[$layer] * 255, [MidpointRounding]::AwayFromZero)
  New-Object System.Drawing.SolidBrush([System.Drawing.Color]::FromArgb($alpha, $theme.Accent))
}

# stalk(): the segments of a stem from y1 up to y2, w thick, round caps -- one path, one fill.
function Add-Stalk($path, [double]$x, [double]$y1, [double]$y2, [double]$w) {
  $seg = $w * 3.4
  $gap = $w * 0.62
  $y = $y1
  $r = (N $w) / 2
  $cx = N $x
  while ($y -gt $y2) {
    $to = [math]::Max($y - $seg, $y2)
    $bottom = N $y
    $top = N $to
    $path.StartFigure()
    $path.AddArc([single]($cx - $r), [single]($top - $r), [single](2 * $r), [single](2 * $r), 180, 180)
    $path.AddLine([single]($cx + $r), [single]$top, [single]($cx + $r), [single]$bottom)
    $path.AddArc([single]($cx - $r), [single]($bottom - $r), [single](2 * $r), [single](2 * $r), 0, 180)
    $path.AddLine([single]($cx - $r), [single]$bottom, [single]($cx - $r), [single]$top)
    $path.CloseFigure()
    $y = $to - $gap
  }
}

# leaf(): an almond of length len from (x, y) at angle degrees (0 -- to the right).
function New-Leaf([double]$x, [double]$y, [double]$len, [double]$angle) {
  $w = $len * 0.22
  $p = New-Object System.Drawing.Drawing2D.GraphicsPath
  $p.AddBezier([single]0, [single]0, [single](N ($len * 0.3)), [single](N (-$w)),
    [single](N ($len * 0.72)), [single](N (-$w * 0.82)), [single](N $len), [single]0)
  $p.AddBezier([single](N $len), [single]0, [single](N ($len * 0.72)), [single](N ($w * 0.82)),
    [single](N ($len * 0.3)), [single](N $w), [single]0, [single]0)
  $p.CloseFigure()
  $m = New-Object System.Drawing.Drawing2D.Matrix
  $m.Translate([single](N $x), [single](N $y))
  $m.Rotate([single](N $angle))
  $p.Transform($m)
  $m.Dispose()
  $p
}

# column() and sprig(): five sprigs of PLAN_TALL, leaves turned towards the content.
function Draw-Column($g, $theme, [string]$side) {
  $flip = if ($side -eq 'left') { 1 } else { -1 }
  foreach ($s in $PLAN_TALL) {
    $x = if ($side -eq 'left') { $COLUMN_W * $s.Xr } else { $COLUMN_W * (1 - $s.Xr) }
    $y1 = $COLUMN_H * 1.04
    $top = $COLUMN_H * $s.Top
    $brush = Layer-Brush $theme $s.Layer

    $stalk = New-Object System.Drawing.Drawing2D.GraphicsPath([System.Drawing.Drawing2D.FillMode]::Winding)
    Add-Stalk $stalk $x $y1 $top $s.W
    $g.FillPath($brush, $stalk)
    $stalk.Dispose()

    $L = $s.Leaf
    $leaves = if ($flip -gt 0) {
      @(@{ Y = $top + $s.W * 1.2; Len = $L;        A = -34 },
        @{ Y = $top + $s.W * 4.4; Len = $L * 0.82; A = -8 },
        @{ Y = $top + $s.W * 6.2; Len = $L * 0.7;  A = -150 })
    } else {
      @(@{ Y = $top + $s.W * 1.2; Len = $L;        A = -146 },
        @{ Y = $top + $s.W * 4.4; Len = $L * 0.82; A = -172 },
        @{ Y = $top + $s.W * 6.2; Len = $L * 0.7;  A = -30 })
    }
    foreach ($l in $leaves) {
      $leaf = New-Leaf $x $l.Y $l.Len $l.A
      $g.FillPath($brush, $leaf)
      $leaf.Dispose()
    }
    $brush.Dispose()
  }
}

function Get-PngBytes($bmp) {
  $ms = New-Object System.IO.MemoryStream
  $bmp.Save($ms, [System.Drawing.Imaging.ImageFormat]::Png)
  $bytes = $ms.ToArray()
  $ms.Dispose()
  , $bytes
}

function Render-Back($theme, $scale) {
  $W = $scale.W; $H = $scale.H
  $bmp = New-Object System.Drawing.Bitmap($W, $H, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
  $g = [System.Drawing.Graphics]::FromImage($bmp)
  $g.SmoothingMode = 'AntiAlias'
  $g.PixelOffsetMode = 'HighQuality'
  $g.Clear([System.Drawing.Color]::Transparent)
  $unit = $H / $COLUMN_H
  $colW = $COLUMN_W * $unit
  foreach ($side in 'left', 'right') {
    $left = if ($side -eq 'left') { 0.0 } else { $W - $colW }
    $g.ResetTransform()
    $g.SetClip((New-Object System.Drawing.RectangleF([single]$left, [single]0, [single]$colW, [single]$H)))
    $g.TranslateTransform([single]$left, [single]0)
    $g.ScaleTransform([single]$unit, [single]$unit)
    Draw-Column $g $theme $side
    $g.ResetClip()
  }
  $g.Dispose()
  $bytes = Get-PngBytes $bmp
  $bmp.Dispose()
  , $bytes
}

# logo_svg of the site, in its 28 x 28 box, drawn 8 times larger and boxed down -- the way the
# small frames of make-icons.ps1 are drawn.
$SS = 8
function Render-Logo($theme, [int]$size) {
  $n = $size * $SS
  $big = New-Object System.Drawing.Bitmap($n, $n, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
  $g = [System.Drawing.Graphics]::FromImage($big)
  $g.SmoothingMode = 'AntiAlias'
  $g.PixelOffsetMode = 'HighQuality'
  $g.Clear([System.Drawing.Color]::Transparent)
  $g.ScaleTransform([single]($n / 28.0), [single]($n / 28.0))

  $ink = New-Object System.Drawing.SolidBrush($theme.Ink)
  $bg = New-Object System.Drawing.SolidBrush($theme.Bg)
  $accent = New-Object System.Drawing.SolidBrush($theme.Accent)

  # <circle cx="5.5" cy="7.5" r="3.8"/> and <circle cx="19.5" cy="7.5" r="3.8"/> -- the ears
  $g.FillEllipse($ink, [single](5.5 - 3.8), [single](7.5 - 3.8), [single]7.6, [single]7.6)
  $g.FillEllipse($ink, [single](19.5 - 3.8), [single](7.5 - 3.8), [single]7.6, [single]7.6)
  # <circle cx="12.5" cy="15.5" r="9.5" fill="var(--bg)" stroke="currentColor" stroke-width="2"/>
  $g.FillEllipse($bg, [single](12.5 - 9.5), [single](15.5 - 9.5), [single]19, [single]19)
  $pen = New-Object System.Drawing.Pen($theme.Ink, [single]2)
  $g.DrawEllipse($pen, [single](12.5 - 9.5), [single](15.5 - 9.5), [single]19, [single]19)
  $pen.Dispose()
  # <ellipse cx="8.8" cy="14.4" rx="2.3" ry="3" transform="rotate(-20 8.8 14.4)"/> and the mirror at +20
  foreach ($e in @(@{ X = 8.8; A = -20 }, @{ X = 16.2; A = 20 })) {
    $p = New-Object System.Drawing.Drawing2D.GraphicsPath
    $p.AddEllipse([single]($e.X - 2.3), [single](14.4 - 3), [single]4.6, [single]6)
    $m = New-Object System.Drawing.Drawing2D.Matrix
    $m.RotateAt([single]$e.A, (New-Object System.Drawing.PointF([single]$e.X, [single]14.4)))
    $p.Transform($m)
    $g.FillPath($ink, $p)
    $m.Dispose(); $p.Dispose()
  }
  # <circle cx="12.5" cy="19.2" r="1.5"/> -- the nose
  $g.FillEllipse($ink, [single](12.5 - 1.5), [single](19.2 - 1.5), [single]3, [single]3)
  # <path d="M25 27V21.2M25 18.6V12.4M25 9.8V5" stroke="var(--accent)" stroke-width="2.4" stroke-linecap="round"/>
  $stem = New-Object System.Drawing.Pen($theme.Accent, [single]2.4)
  $stem.StartCap = [System.Drawing.Drawing2D.LineCap]::Round
  $stem.EndCap = [System.Drawing.Drawing2D.LineCap]::Round
  foreach ($s in @(@(27, 21.2), @(18.6, 12.4), @(9.8, 5))) {
    $g.DrawLine($stem, [single]25, [single]$s[0], [single]25, [single]$s[1])
  }
  $stem.Dispose()
  # <path d="M25 6.8c1-2.8 2.6-3.4 2.9-3c-.2 1.7-1.6 3.1-2.9 3z"/> -- the leaf, relative curves made absolute
  $leaf = New-Object System.Drawing.Drawing2D.GraphicsPath
  $leaf.AddBezier([single]25, [single]6.8, [single]26, [single]4, [single]27.6, [single]3.4, [single]27.9, [single]3.8)
  $leaf.AddBezier([single]27.9, [single]3.8, [single]27.7, [single]5.5, [single]26.3, [single]6.9, [single]25, [single]6.8)
  $leaf.CloseFigure()
  $g.FillPath($accent, $leaf)
  $leaf.Dispose()

  $ink.Dispose(); $bg.Dispose(); $accent.Dispose()
  $g.Dispose()

  $dst = New-Object System.Drawing.Bitmap($size, $size, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
  $g2 = [System.Drawing.Graphics]::FromImage($dst)
  $g2.InterpolationMode = 'HighQualityBicubic'
  $g2.PixelOffsetMode = 'HighQuality'
  $g2.CompositingQuality = 'HighQuality'
  # TileFlipXY: without it the downscale samples past the edge and leaves a transparent halo.
  $ia = New-Object System.Drawing.Imaging.ImageAttributes
  $ia.SetWrapMode([System.Drawing.Drawing2D.WrapMode]::TileFlipXY)
  $g2.DrawImage($big, (New-Object System.Drawing.Rectangle(0, 0, $size, $size)),
    0, 0, $n, $n, [System.Drawing.GraphicsUnit]::Pixel, $ia)
  $g2.Dispose(); $ia.Dispose(); $big.Dispose()
  $bytes = Get-PngBytes $dst
  $dst.Dispose()
  , $bytes
}

$files = New-Object 'System.Collections.Generic.List[object]'
foreach ($theme in $THEMES) {
  foreach ($scale in $SCALES) {
    $files.Add([pscustomobject]@{ Name = "back-$($theme.Name)-$($scale.Name).png"; Bytes = (Render-Back $theme $scale) })
    $files.Add([pscustomobject]@{ Name = "logo-$($theme.Name)-$($scale.Name).png"; Bytes = (Render-Logo $theme $scale.Logo) })
  }
}

New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
$sha = [System.Security.Cryptography.SHA256]::Create()
foreach ($f in $files) {
  $path = Join-Path $OutDir $f.Name
  [System.IO.File]::WriteAllBytes($path, $f.Bytes)
  $hash = ([System.BitConverter]::ToString($sha.ComputeHash($f.Bytes))).Replace('-', '')
  Write-Output ("{0,-22} {1,7} bytes  {2}" -f $f.Name, $f.Bytes.Length, $hash)
}
$sha.Dispose()
