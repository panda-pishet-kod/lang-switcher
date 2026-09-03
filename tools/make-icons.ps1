<#
    make-icons.ps1 -- generator of the two tray icons of FR-90, variant 01 "graphite plate"
    (decision 53v of DECISIONS.md): a dark rounded plate RGB(43,47,54), a white double-headed
    arrow for "active", a muted grey pause sign for "paused".

    WHY A GENERATOR AND NOT EXPORTED ARTWORK. The icon has to be reproducible. Every frame is
    drawn from the same 100x100 vector description, so a change of proportion or tone is one
    edit here rather than seven hand-retouched bitmaps that drift apart.

    OUTPUT: res\langswitcher-active.ico and res\langswitcher-paused.ico, seven frames each.
    16/20/24/32 are the sizes FR-90 names and SM_CXSMICON selects between; 48/64/256 exist
    because the same file is the icon of the executable (app.rc, IDI_APP_ACTIVE/IDI_APP_PAUSED)
    and of the installer (SetupIconFile), and Explorer picks those in its larger views.

    Frames are stored as PNG, which is what the icons this replaces already did: Windows Vista
    and later read PNG frames from an .ico, and at 256 px a BMP frame would cost 256 KB.

    ONE SET FOR BOTH TASKBAR THEMES, deliberately: the dark plate is what reads on a light
    taskbar, the light glyph is what reads on a dark one, so FR-90 is satisfied by the drawing
    itself and no theme-switching of icon sets exists to go wrong. The mirror-image light
    palette (variant 02) is kept in the table below so trying it is one argument rather than
    a redraw -- but no file in res\ is made from it.
#>

Add-Type -AssemblyName System.Drawing
$ErrorActionPreference = 'Stop'

# The script lives in tools\, the icons live in res\ of the same working copy.
$OutDir = Join-Path (Split-Path -Parent $PSScriptRoot) 'res'
$SIZES  = @(16, 20, 24, 32, 48, 64, 256)

# Supersampling factor. The glyph is drawn at SIZE*SS and boxed down, which is what keeps the
# 2 px shaft of the arrow from breaking up at 16 px.
$SS = 8

function Col([int]$r,[int]$g,[int]$b) { [System.Drawing.Color]::FromArgb(255,$r,$g,$b) }

# The two palettes of the design. 'graphite' is variant 01 -- a dark plate carrying a white
# glyph, which reads on a light taskbar by its plate and on a dark one by its glyph. 'light'
# is variant 02, the mirror image, kept here so switching is one argument rather than a redraw.
$PALETTE = @{
  graphite = @{ Plate = (Col 43 47 54);    Active = (Col 245 246 247); Paused = (Col 138 144 152) }
  light    = @{ Plate = (Col 228 231 234); Active = (Col 35 38 43);    Paused = (Col 154 160 168) }
}

function New-RoundedPath([single]$x,[single]$y,[single]$w,[single]$h,[single]$r) {
  $p = New-Object System.Drawing.Drawing2D.GraphicsPath
  $d = $r * 2
  $p.AddArc($x, $y, $d, $d, 180, 90)
  $p.AddArc($x+$w-$d, $y, $d, $d, 270, 90)
  $p.AddArc($x+$w-$d, $y+$h-$d, $d, $d, 0, 90)
  $p.AddArc($x, $y+$h-$d, $d, $d, 90, 90)
  $p.CloseFigure()
  $p
}

function PtF([single]$x,[single]$y) { New-Object System.Drawing.PointF($x,$y) }

# The double-headed arrow of FR-90's "active" state, in a 100x100 space. The shaft is 16 units
# tall, which is 2.56 px at 16 px -- above the one-pixel threshold where anti-aliasing turns a
# line into a grey smear.
function New-ArrowPath {
  $p = New-Object System.Drawing.Drawing2D.GraphicsPath
  $p.AddPolygon([System.Drawing.PointF[]]@(
    (PtF 14 50),(PtF 36 29),(PtF 36 42),(PtF 64 42),(PtF 64 29),
    (PtF 86 50),(PtF 64 71),(PtF 64 58),(PtF 36 58),(PtF 36 71)))
  $p
}

# The "paused" state. It differs from "active" by shape AND by tone, deliberately: at 16 px the
# glyph is four pixels of detail, and one signal is not enough to tell the states apart.
function New-PausePath {
  $p = New-Object System.Drawing.Drawing2D.GraphicsPath
  $p.AddPath((New-RoundedPath 28 26 17 48 4), $false)
  $p.AddPath((New-RoundedPath 55 26 17 48 4), $false)
  $p
}

function Render-Frame([int]$size, [string]$paletteName, [string]$state) {
  $pal = $PALETTE[$paletteName]
  $n = $size * $SS

  $big = New-Object System.Drawing.Bitmap($n, $n, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
  $g = [System.Drawing.Graphics]::FromImage($big)
  $g.SmoothingMode = 'AntiAlias'
  $g.PixelOffsetMode = 'HighQuality'
  $g.Clear([System.Drawing.Color]::Transparent)
  $g.ScaleTransform([single]($n/100.0), [single]($n/100.0))

  $plate = New-RoundedPath 2 2 96 96 24
  $br = New-Object System.Drawing.SolidBrush($pal.Plate)
  $g.FillPath($br, $plate)
  $br.Dispose(); $plate.Dispose()

  if ($state -eq 'paused' -or $state -eq 'paused-unread') { $glyph = New-PausePath; $ink = $pal.Paused }
  else { $glyph = New-ArrowPath; $ink = $pal.Active }

  # Scale the glyph to 70% about the centre of the plate.
  $s = [single]0.70
  $d = [single](50 - 50 * $s)
  $m = New-Object System.Drawing.Drawing2D.Matrix($s, [single]0, [single]0, $s, $d, $d)
  $glyph.Transform($m); $m.Dispose()
  $br = New-Object System.Drawing.SolidBrush($ink)
  $g.FillPath($br, $glyph)
  $br.Dispose(); $glyph.Dispose()

  # FR-90's third state, task T-32-4: "there is an unread letter". The mark is a dot in the
  # icon's OWN colours -- filled with the colour of the glyph, ringed with the colour of the
  # plate -- so it reads on a light taskbar and on a dark one by exactly the contrast the two
  # states above already rely on, and no second icon set exists to go wrong.
  #
  # WHY A RING AND NOT A BARE DOT. The dot sits on the corner of the plate and hangs over its
  # edge; without the ring its lower-right arc would be the glyph colour against whatever the
  # taskbar happens to be, which is the one place in this drawing where the taskbar's own
  # colour can touch the ink.
  if ($state -like '*-unread') {
    $cx = 74.0; $cy = 74.0; $r = 17.0; $ring = 6.0
    $ringBrush = New-Object System.Drawing.SolidBrush($pal.Plate)
    $g.FillEllipse($ringBrush, [single]($cx-$r-$ring), [single]($cy-$r-$ring),
                   [single](2*($r+$ring)), [single](2*($r+$ring)))
    $ringBrush.Dispose()
    $dotBrush = New-Object System.Drawing.SolidBrush($ink)
    $g.FillEllipse($dotBrush, [single]($cx-$r), [single]($cy-$r), [single](2*$r), [single](2*$r))
    $dotBrush.Dispose()
  }

  $g.Dispose()

  $dst = New-Object System.Drawing.Bitmap($size, $size, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
  $g2 = [System.Drawing.Graphics]::FromImage($dst)
  $g2.InterpolationMode = 'HighQualityBicubic'
  $g2.PixelOffsetMode = 'HighQuality'
  $g2.CompositingQuality = 'HighQuality'
  # TileFlipXY, not the plain rectangle overload: without it the downscale samples past the
  # edge of the source and leaves a transparent halo on all four sides.
  $ia = New-Object System.Drawing.Imaging.ImageAttributes
  $ia.SetWrapMode([System.Drawing.Drawing2D.WrapMode]::TileFlipXY)
  $g2.DrawImage($big, (New-Object System.Drawing.Rectangle(0,0,$size,$size)),
                0, 0, $n, $n, [System.Drawing.GraphicsUnit]::Pixel, $ia)
  $g2.Dispose(); $ia.Dispose(); $big.Dispose()

  $ms = New-Object System.IO.MemoryStream
  $dst.Save($ms, [System.Drawing.Imaging.ImageFormat]::Png)
  $dst.Dispose()
  $bytes = $ms.ToArray()
  $ms.Dispose()
  ,$bytes
}

function Write-Ico([string]$path, [string]$paletteName, [string]$state) {
  $frames = @()
  foreach ($s in $SIZES) { $frames += ,(Render-Frame $s $paletteName $state) }

  $fs = New-Object System.IO.FileStream($path, [System.IO.FileMode]::Create)
  $bw = New-Object System.IO.BinaryWriter($fs)

  # ICONDIR
  $bw.Write([uint16]0)                  # reserved
  $bw.Write([uint16]1)                  # type: 1 = icon
  $bw.Write([uint16]$SIZES.Count)

  # ICONDIRENTRY x N. Image data starts after the directory.
  $offset = 6 + 16 * $SIZES.Count
  for ($i = 0; $i -lt $SIZES.Count; $i++) {
    $size = $SIZES[$i]
    # 256 is written as 0: the field is one byte and 256 does not fit in it.
    $dim = $size
    if ($size -ge 256) { $dim = 0 }
    $bw.Write([byte]$dim)               # width
    $bw.Write([byte]$dim)               # height
    $bw.Write([byte]0)                  # colours in palette: 0 = truecolour
    $bw.Write([byte]0)                  # reserved
    $bw.Write([uint16]1)                # colour planes
    $bw.Write([uint16]32)               # bits per pixel
    $bw.Write([uint32]$frames[$i].Length)
    $bw.Write([uint32]$offset)
    $offset += $frames[$i].Length
  }

  foreach ($f in $frames) { $bw.Write($f) }

  $bw.Flush(); $bw.Close(); $fs.Close()
}

# Exactly the two files app.rc and the installer reference, both from the graphite palette.
# The light palette stays available above but produces no file in res\.
$targets = @(
  @{ File = 'langswitcher-active.ico'; Palette = 'graphite'; State = 'active' },
  @{ File = 'langswitcher-paused.ico'; Palette = 'graphite'; State = 'paused' },
  # FR-90's third state, task T-32-4: the same two icons carrying the dot of FR-101. Two files
  # and not one, because the state of the program and the state of the letters are independent:
  # a person can pause the program with a letter unread.
  @{ File = 'langswitcher-active-unread.ico'; Palette = 'graphite'; State = 'active-unread' },
  @{ File = 'langswitcher-paused-unread.ico'; Palette = 'graphite'; State = 'paused-unread' }
)

foreach ($t in $targets) {
  $p = Join-Path $OutDir $t.File
  Write-Ico $p $t.Palette $t.State
  $len = (Get-Item $p).Length
  Write-Output ("{0,-34} {1,7} bytes  {2} frames" -f $t.File, $len, $SIZES.Count)
}
