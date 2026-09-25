<#
    make-icons.ps1 -- generator of the two tray icons of FR-90, variant 01 "graphite plate"
    (decision 53v of DECISIONS.md): a dark rounded plate RGB(43,47,54), a white double-headed
    arrow for "active", a muted grey pause sign for "paused".

    WHY A GENERATOR AND NOT EXPORTED ARTWORK. The icon has to be reproducible. Every frame is
    drawn from the same 100x100 vector description, so a change of proportion or tone is one
    edit here rather than seven hand-retouched bitmaps that drift apart.

    OUTPUT: the four icons of res\ (see $targets at the end), nineteen frames each -- the list and
    why each size is there is at $SIZES below. 16/20/24/32 are the sizes FR-90 names and
    SM_CXSMICON selects between; the larger ones exist because the same file is the icon of the
    executable (app.rc, IDI_APP_ACTIVE/IDI_APP_PAUSED) and of the installer (SetupIconFile), and
    Explorer and the Start menu pick those in their larger views.

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
# Task T-42-11, finding 124б.2. 28, 40 and 56 were added because the product asks for exactly
# those and for nothing near them: the tray icon is SM_CXSMICON (16 at 96 DPI) scaled to the
# monitor -- 16/20/24/28/32 at 100/125/150/175/200 % -- and the balloon icon is SM_CXICON (32)
# on the same scale -- 32/40/48/56/64. Before this task the file had 16/20/24/32/48/64/256, so
# at 175 % the shell was handed a 24 px frame stretched to 28, and at 125 % a 32 px one
# stretched to 40. Every frame is DRAWN from the same vector description, not resampled from a
# bigger one, so three more of them cost nothing but bytes.
#
# Task T-85-2, решение 144.4: nine more, for the Start menu and the other views of Windows. A
# tile of the Start menu takes 36 px at 100 % (54 at 150 % on the owner's screenshot) -- there was
# no such frame, and the shell handed out the 48 squeezed bilinearly (scratchpad-E85\
# shell-probe-base-e84.log). Added: the standard sizes of Microsoft 30, 36, 60, 72, 80, 96, and
# the tile at 125/150/175 % -- 45, 54, 63. The drawing is the same; nothing else changes.
$SIZES  = @(16, 20, 24, 28, 30, 32, 36, 40, 45, 48, 54, 56, 60, 63, 64, 72, 80, 96, 256)

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

# Task T-85-3, решение 144.3 -- the guard of the dot, in the vector: the least distance from the
# centre of the dot to the outline of the arrow as drawn in this frame, less the outer radius of the
# ring. Positive -- the dot stands clear of the arrow; zero or less -- it touches or covers it, and
# the generator refuses to write the icon rather than ship it (see Render-Frame).
function Get-DotClearance([System.Drawing.PointF[]]$outline, [double]$cx, [double]$cy, [double]$outer) {
  $least = [double]::MaxValue
  $inside = $false
  for ($i = 0; $i -lt $outline.Count; $i++) {
    $a = $outline[$i]; $b = $outline[($i + 1) % $outline.Count]
    $dx = $b.X - $a.X; $dy = $b.Y - $a.Y
    $t = [math]::Max(0.0, [math]::Min(1.0, (($cx - $a.X) * $dx + ($cy - $a.Y) * $dy) / ($dx * $dx + $dy * $dy)))
    $qx = $a.X + $t * $dx; $qy = $a.Y + $t * $dy
    $least = [math]::Min($least, [math]::Sqrt(($cx - $qx) * ($cx - $qx) + ($cy - $qy) * ($cy - $qy)))
    if ((($a.Y -gt $cy) -ne ($b.Y -gt $cy)) -and ($cx -lt ($b.X - $a.X) * ($cy - $a.Y) / ($b.Y - $a.Y) + $a.X)) {
      $inside = -not $inside
    }
  }
  if ($inside) { return -($least + $outer) }
  return $least - $outer
}

# The least clearance of the dot met while one file was drawn -- printed beside the file.
$script:LeastClearance = [double]::MaxValue

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

  # Scale the glyph about the centre of the plate: 70 % -- and, task T-85-3, решения 144.2 и 144.2а,
  # 95 % in the frames up to 30 px inclusive: the notification area at 100/125/150/175 % (16, 20,
  # 24, 28) and the standard 30. At 70 % the arrow of the 16 px frame came out about 8 x 5 px with a
  # shaft of 1.8 px -- a white speck, too small, as the owner rightly said. 32 and larger stay at
  # 70 %: the 32 px frame is also the icon of the headers of «О программе» and «От автора» and of
  # the balloon (SM_CXICON at 100 %). The pause follows the same scale.
  $small = $size -le 30
  $s = if ($small) { [single]0.95 } else { [single]0.70 }
  $d = [single](50 - 50 * $s)
  $m = New-Object System.Drawing.Drawing2D.Matrix($s, [single]0, [single]0, $s, $d, $d)
  $glyph.Transform($m); $m.Dispose()
  $br = New-Object System.Drawing.SolidBrush($ink)
  $g.FillPath($br, $glyph)
  $br.Dispose()
  # The outline as drawn, for the guard of the dot below -- the arrow is a polygon, so its path
  # points are its corners. The pause is not guarded: in the large frames the old ring has always
  # grazed the corner of its second bar (0.99 units), and those frames do not change.
  $arrow = $null
  if ($state -notlike 'paused*') { $arrow = $glyph.PathPoints }
  $glyph.Dispose()

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
    # Tuned against the proof sheet rather than guessed: at r = 17 the dot ate the right head
    # of the arrow at 16 px, which is the size the notification area actually shows.
    #
    # Task T-85-3, решение 144.3: at 95 % the tip of the right head stands at x 84 and its lower
    # corner at (63, 70), and the old place -- (78, 78), r 14, ring 5 -- covered the lower half of
    # the head (3.03 units into it). In the small frames the dot moves into the corner and shrinks
    # a little: (82, 82), r 12, ring 4 -- 5.63 units clear of the arrow, 3.84 px across at 16 px
    # (the floor is 3 px), and the second pause bar just touched. Chosen on the proof sheet of the
    # candidates, scratchpad-E85\dot-candidates-x8.png. The large frames keep the old place.
    if ($small) { $cx = 82.0; $cy = 82.0; $r = 12.0; $ring = 4.0 }
    else        { $cx = 78.0; $cy = 78.0; $r = 14.0; $ring = 5.0 }

    if ($null -ne $arrow) {
      $clearance = Get-DotClearance $arrow $cx $cy ($r + $ring)
      if ($clearance -le 0) {
        throw ("the dot of the letter touches the arrow in the {0} px frame of {1}: clearance {2:N2} units -- no file is written" -f $size, $state, $clearance)
      }
      $script:LeastClearance = [math]::Min($script:LeastClearance, $clearance)
    }

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

function Render-Set([string]$paletteName, [string]$state) {
  $frames = @()
  foreach ($s in $SIZES) { $frames += ,(Render-Frame $s $paletteName $state) }
  ,$frames
}

function Write-Ico([string]$path, $frames) {
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

# Every frame of every file is drawn before any file is written -- task T-85-3: a guard that refuses
# (the dot of the letter touching the arrow) must leave res\ as it was, not half regenerated.
$sets = @()
foreach ($t in $targets) {
  $script:LeastClearance = [double]::MaxValue
  $frames = Render-Set $t.Palette $t.State
  $sets += [pscustomobject]@{ Target = $t; Frames = $frames; Clearance = $script:LeastClearance }
}

foreach ($set in $sets) {
  $p = Join-Path $OutDir $set.Target.File
  Write-Ico $p $set.Frames
  $len = (Get-Item $p).Length
  $note = ''
  if ($set.Clearance -lt [double]::MaxValue) {
    $note = "  dot clear of the arrow by >= {0:N2} units" -f $set.Clearance
  }
  Write-Output ("{0,-34} {1,7} bytes  {2} frames{3}" -f $set.Target.File, $len, $SIZES.Count, $note)
}
