<#
    verify-icons.ps1 -- measures what criteria 9 and 10 of task T-11-8 claim about the eight
    icons in res\ -- the four of the main set and, since task T-86-1 (decision 145.1), the four
    of the darkbar set langswitcher-darkbar-*.ico the tray shows on a dark taskbar: one PNG
    frame of every size of $SIZES below in each .ico -- nineteen since task T-85-2, the list
    kept in step with tools\make-icons.ps1 -- and LoadImageW handing back every one of those
    sizes exactly, not a neighbour.

    TWO DELIBERATE DETOURS AROUND System.Drawing.Icon -- do not "improve" them back.

    1. FRAME SELECTION IS CHECKED WITH LoadImageW, NOT WITH new Icon(path, w, h). The .ico
       directory stores a 256 px frame with a width byte of ZERO (the field is one byte and
       256 does not fit), System.Drawing.Icon reads that zero literally and silently falls
       back to the next size down. LoadImageW is also what the product itself calls -- FR-90
       selects by SM_CXSMICON -- and what the shell uses, so it is the reader the icon must
       satisfy, not merely a convenient one.

    2. THE PROOF SHEET IS DRAWN BY DECODING EACH PNG FRAME DIRECTLY, NOT VIA Icon.ToBitmap().
       ToBitmap() cannot decode PNG-compressed frames and throws; Bitmap.FromStream on the
       frame's own bytes can.

    3. THE SIZE LoadImageW ANSWERS PROVES NOTHING ABOUT THE FRAME -- task T-85-2, measured. Asked
       for a size the file lacks, LoadImageW stretches the nearest frame and answers exactly the
       size asked: the old ten-frame files passed "LoadImage(36) -> 36x36" for all nine sizes they
       did not carry. So each size is checked twice: the file must DECLARE a frame of it, and the
       bitmap LoadImageW builds must BE that frame, pixel by pixel -- alpha everywhere, colour
       where alpha is 255 (the loaded bitmap is premultiplied at the anti-aliased edge). A control
       at the end compares a frame with a frame of the other file and must see them differ.

    The proof sheet (-ProofPath, default %TEMP%\icons-proof.png) shows both states at the true
    pixel sizes 16/20/24/32/48 on a light and a dark panel, for the human judgement of
    criterion 13. It is evidence for the report, not a repository artifact.

    Exit: prints 'ИТОГ: все проверки пройдены' and exits 0 when every check passes;
    otherwise counts every failure, prints 'ИТОГ: ОТКАЗОВ N' and exits 1.
#>

param(
  [string]$ProofPath = (Join-Path $env:TEMP 'icons-proof.png')
)

Add-Type -AssemblyName System.Drawing
$ErrorActionPreference = 'Stop'

# The script lives in tools\, the icons live in res\ of the same working copy.
$ResDir = Join-Path (Split-Path -Parent $PSScriptRoot) 'res'

if (-not ('Ico' -as [type])) {
Add-Type @'
using System;
using System.Runtime.InteropServices;
public class Ico {
  [DllImport("user32.dll", CharSet=CharSet.Unicode, SetLastError=true)]
  public static extern IntPtr LoadImageW(IntPtr h, string name, uint type, int cx, int cy, uint flags);
  [DllImport("user32.dll", SetLastError=true)] public static extern bool DestroyIcon(IntPtr h);
  [DllImport("user32.dll", SetLastError=true)] public static extern bool GetIconInfo(IntPtr h, out ICONINFO i);
  [DllImport("gdi32.dll", SetLastError=true)] public static extern int GetObject(IntPtr h, int c, ref BITMAP b);
  [DllImport("gdi32.dll")] public static extern bool DeleteObject(IntPtr h);
  [StructLayout(LayoutKind.Sequential)] public struct ICONINFO {
    public bool fIcon; public int xHotspot; public int yHotspot; public IntPtr hbmMask; public IntPtr hbmColor;
  }
  [StructLayout(LayoutKind.Sequential)] public struct BITMAP {
    public int bmType; public int bmWidth; public int bmHeight; public int bmWidthBytes;
    public ushort bmPlanes; public ushort bmBitsPixel; public IntPtr bmBits;
  }
  [StructLayout(LayoutKind.Sequential)] public struct BITMAPINFOHEADER {
    public uint biSize; public int biWidth; public int biHeight; public ushort biPlanes; public ushort biBitCount;
    public uint biCompression; public uint biSizeImage; public int biXPelsPerMeter; public int biYPelsPerMeter;
    public uint biClrUsed; public uint biClrImportant;
  }
  [DllImport("gdi32.dll")] public static extern int GetDIBits(IntPtr hdc, IntPtr hbm, uint start, uint lines, [Out] int[] bits, ref BITMAPINFOHEADER bmi, uint usage);
  [DllImport("user32.dll")] public static extern IntPtr GetDC(IntPtr hwnd);
  [DllImport("user32.dll")] public static extern int ReleaseDC(IntPtr hwnd, IntPtr hdc);
  // The colour bitmap LoadImageW builds at a size, as 32 bpp top-down ARGB -- task T-85-2.
  public static int[] Pixels(string path, int size) {
    IntPtr h = LoadImageW(IntPtr.Zero, path, 1, size, size, 0x10);
    if (h == IntPtr.Zero) return null;
    ICONINFO info; GetIconInfo(h, out info);
    BITMAPINFOHEADER bmi = new BITMAPINFOHEADER();
    bmi.biSize = 40; bmi.biWidth = size; bmi.biHeight = -size; bmi.biPlanes = 1; bmi.biBitCount = 32;
    int[] bits = new int[size * size];
    IntPtr dc = GetDC(IntPtr.Zero);
    int lines = GetDIBits(dc, info.hbmColor, 0, (uint)size, bits, ref bmi, 0);
    ReleaseDC(IntPtr.Zero, dc);
    if (info.hbmColor != IntPtr.Zero) DeleteObject(info.hbmColor);
    if (info.hbmMask != IntPtr.Zero) DeleteObject(info.hbmMask);
    DestroyIcon(h);
    return lines == size ? bits : null;
  }
}
'@
}

# The PNG frame of one size decoded from the file, as 32 bpp ARGB -- or $null when the file has none.
function Get-FramePixels([byte[]]$bytes, [int]$px) {
  $n = [BitConverter]::ToUInt16($bytes,4)
  for ($i = 0; $i -lt $n; $i++) {
    $o = 6 + 16*$i
    $decl = [int]$bytes[$o]; if ($decl -eq 0) { $decl = 256 }
    if ($decl -ne $px) { continue }
    $len = [int][BitConverter]::ToUInt32($bytes,$o+8); $off = [int][BitConverter]::ToUInt32($bytes,$o+12)
    $ms = New-Object System.IO.MemoryStream($bytes, $off, $len)
    $img = [System.Drawing.Image]::FromStream($ms); $bm = New-Object System.Drawing.Bitmap($img)
    $data = $bm.LockBits((New-Object System.Drawing.Rectangle(0,0,$px,$px)), 'ReadOnly', 'Format32bppArgb')
    $pixels = New-Object int[] ($px*$px)
    [System.Runtime.InteropServices.Marshal]::Copy($data.Scan0, $pixels, 0, $px*$px)
    $bm.UnlockBits($data); $bm.Dispose(); $img.Dispose(); $ms.Dispose()
    return ,$pixels
  }
  return $null
}

# How many pixels of two pictures of one size differ: alpha anywhere, colour where alpha is 255.
function Get-PixelDifference([int[]]$a, [int[]]$b) {
  $d = 0
  for ($i = 0; $i -lt $a.Length; $i++) {
    $aa = ($a[$i] -shr 24) -band 0xFF; $ab = ($b[$i] -shr 24) -band 0xFF
    if ($aa -ne $ab -or ($aa -eq 255 -and ($a[$i] -band 0xFFFFFF) -ne ($b[$i] -band 0xFFFFFF))) { $d++ }
  }
  return $d
}

function Get-LoadedIconSize([string]$path, [int]$px) {
  $h = [Ico]::LoadImageW([IntPtr]::Zero, $path, 1, $px, $px, 0x10)   # IMAGE_ICON, LR_LOADFROMFILE
  if ($h -eq [IntPtr]::Zero) { return $null }
  $info = New-Object Ico+ICONINFO
  $null = [Ico]::GetIconInfo($h, [ref]$info)
  $bm = New-Object Ico+BITMAP
  $null = [Ico]::GetObject($info.hbmColor, [System.Runtime.InteropServices.Marshal]::SizeOf($bm), [ref]$bm)
  $w = $bm.bmWidth; $ht = $bm.bmHeight
  if ($info.hbmColor -ne [IntPtr]::Zero) { $null = [Ico]::DeleteObject($info.hbmColor) }
  if ($info.hbmMask  -ne [IntPtr]::Zero) { $null = [Ico]::DeleteObject($info.hbmMask) }
  $null = [Ico]::DestroyIcon($h)
  return @($w, $ht)
}

# FR-90's third state, task T-32-4: the same two icons carrying the dot of FR-101. Four files
# and not two -- the state of the program and the state of the letters are independent.
# Task T-86-1, decision 145.1: four more -- the darkbar set the tray shows on a dark taskbar,
# the same four states without the plate (app.rc 105..108). Eight files, all held to the same
# nineteen frames and to the same "own frame" rule.
$files = @('langswitcher-active.ico','langswitcher-paused.ico',
           'langswitcher-active-unread.ico','langswitcher-paused-unread.ico',
           'langswitcher-darkbar-active.ico','langswitcher-darkbar-paused.ico',
           'langswitcher-darkbar-active-unread.ico','langswitcher-darkbar-paused-unread.ico')
# Task T-42-11, finding 124б.2: 28, 40 and 56 joined the list. The product asks for the small
# metric (16 at 96 DPI) and the large one (32) scaled to the monitor -- 16/20/24/28/32 and
# 32/40/48/56/64 at 100/125/150/175/200 % -- and before this task the shell was handed a 24 px
# frame stretched to 28 at 175 %, and a 32 px one stretched to 40 at 125 %.
# Task T-85-2, решение 144.4: nine more -- 30, 36, 45, 54, 60, 63, 72, 80, 96 -- for the Start menu
# (a tile is 36 px at 100 %) and the other views of Windows. Kept in step with $SIZES of the
# generator by hand: the two scripts share no file.
$SIZES = @(16,20,24,28,30,32,36,40,45,48,54,56,60,63,64,72,80,96,256)
$fail = 0

foreach ($f in $files) {
  $p = Join-Path $ResDir $f
  # A missing file is a refusal of its own and not a crash of the instrument -- task T-86-1,
  # whose red «до» this is: on the tree before it the four darkbar files did not exist.
  if (-not (Test-Path -LiteralPath $p)) { Write-Output "$f : ОТКАЗ: файла нет в res\"; $fail++; continue }
  $b = [System.IO.File]::ReadAllBytes($p)
  $count = [BitConverter]::ToUInt16($b,4)
  Write-Output "$f : type=$([BitConverter]::ToUInt16($b,2)) frames=$count размер=$($b.Length)"
  if ([BitConverter]::ToUInt16($b,0) -ne 0) { Write-Output '  ОТКАЗ: поле reserved не ноль'; $fail++ }
  if ($count -ne $SIZES.Count) { Write-Output "  ОТКАЗ: кадров $count, требуется $($SIZES.Count)"; $fail++ }

  # Structure: every directory entry must point inside the file and at a real PNG of the
  # dimensions the entry declares.
  for ($i = 0; $i -lt $count; $i++) {
    $o = 6 + 16*$i
    $w = $b[$o]; $len = [BitConverter]::ToUInt32($b,$o+8); $off = [BitConverter]::ToUInt32($b,$o+12)
    $declared = $w; if ($w -eq 0) { $declared = 256 }
    if ($off + $len -gt $b.Length) { Write-Output "  ОТКАЗ: кадр $i выходит за конец файла"; $fail++; continue }
    $isPng = ($b[$off] -eq 0x89 -and $b[$off+1] -eq 0x50 -and $b[$off+2] -eq 0x4E -and $b[$off+3] -eq 0x47)
    $pw = [BitConverter]::ToUInt32(($b[($off+19)..($off+16)]),0)
    $ph = [BitConverter]::ToUInt32(($b[($off+23)..($off+20)]),0)
    $bpp = [BitConverter]::ToUInt16($b,$o+6)
    $ok = ($isPng -and $pw -eq $declared -and $ph -eq $declared -and $bpp -eq 32)
    if (-not $ok) { $fail++ }
    Write-Output ("  кадр {0}: заявлен {1,3}  PNG {2}x{3}  {4} bpp  {5,6} байт  {6}" -f $i,$declared,$pw,$ph,$bpp,$len,$(if($ok){'ок'}else{'ОТКАЗ'}))
  }

  # Behaviour: LoadImage must hand back each size FR-90 needs, at the exact pixel size asked --
  # and, task T-85-2, it must be the file's OWN frame of that size, not a neighbour stretched to
  # it (detour 3 in the header: the size alone cannot fail).
  foreach ($s in $SIZES) {
    $got = Get-LoadedIconSize $p $s
    if ($null -eq $got) { Write-Output "  ОТКАЗ: LoadImage($s) вернул NULL"; $fail++; continue }
    if ($got[0] -ne $s -or $got[1] -ne $s) {
      Write-Output ("  ОТКАЗ: LoadImage({0}) отдал {1}x{2}" -f $s,$got[0],$got[1]); $fail++; continue
    }
    $own = Get-FramePixels $b $s
    if ($null -eq $own) {
      Write-Output ("  ОТКАЗ: кадра {0} в файле нет -- LoadImage({0}) растянул соседа" -f $s); $fail++; continue
    }
    $loaded = [Ico]::Pixels($p, $s)
    if ($null -eq $loaded) { Write-Output "  ОТКАЗ: растр LoadImage($s) не прочитан"; $fail++; continue }
    $diff = Get-PixelDifference $own $loaded
    if ($diff -ne 0) {
      Write-Output ("  ОТКАЗ: LoadImage({0}) отдал не свой кадр -- {1} пикселей расходятся" -f $s,$diff); $fail++
    } else {
      Write-Output ("  LoadImage({0,3}) -> {1}x{2}, свой кадр  ок" -f $s,$got[0],$got[1])
    }
  }
}

# The control of detour 3: the comparison must be able to fail. The 16 px bitmap LoadImageW builds
# from the "active" file against the 16 px frame of the "paused" file -- two different drawings.
$ctlLoaded = [Ico]::Pixels((Join-Path $ResDir 'langswitcher-active.ico'), 16)
$ctlOther = Get-FramePixels ([System.IO.File]::ReadAllBytes((Join-Path $ResDir 'langswitcher-paused.ico'))) 16
if ($null -eq $ctlLoaded -or $null -eq $ctlOther) {
  Write-Output 'ОТКАЗ: контроль сличения не поднялся'; $fail++
} else {
  $ctl = Get-PixelDifference $ctlOther $ctlLoaded
  if ($ctl -eq 0) { Write-Output 'ОТКАЗ: контроль -- сличение не видит разницы двух рисунков'; $fail++ }
  else { Write-Output "контроль сличения: активный 16 против паузы 16 -- $ctl пикселей расходятся  ок" }
}

# Proof sheet at true pixel sizes on both taskbar colours.
$rows = @(@{f='langswitcher-active.ico';label='активна'},
          @{f='langswitcher-paused.ico';label='пауза'},
          @{f='langswitcher-active-unread.ico';label='активна + письмо'},
          @{f='langswitcher-paused-unread.ico';label='пауза + письмо'},
          # Task T-86-1: the darkbar set -- what the tray shows on a dark taskbar. Drawn on both
          # panels like the rest: the dark panel is its place, the light one is for comparison.
          @{f='langswitcher-darkbar-active.ico';label='без плиты: активна'},
          @{f='langswitcher-darkbar-paused.ico';label='без плиты: пауза'},
          @{f='langswitcher-darkbar-active-unread.ico';label='без плиты: активна + письмо'},
          @{f='langswitcher-darkbar-paused-unread.ico';label='без плиты: пауза + письмо'})
# Task T-85-3: 28 and 30 joined the sheet -- the frames whose glyph grew to 95 % (решения 144.2 и
# 144.2а) are 16 to 30, and the owner judges them here before the delivery (144.3); 32 and 48 stay
# beside them as the frames that did not change.
$shown = @(16,20,24,28,30,32,48)
$W = 920; $H = 56 + $rows.Count*84
$bmp = New-Object System.Drawing.Bitmap($W,$H,[System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.SmoothingMode='AntiAlias'; $g.TextRenderingHint='ClearTypeGridFit'
$g.Clear([System.Drawing.Color]::FromArgb(255,250,250,250))
$fnt = New-Object System.Drawing.Font('Segoe UI',10,[System.Drawing.FontStyle]::Bold)
$brD = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::FromArgb(255,40,42,46))
$brL = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::FromArgb(255,200,204,210))
$xL=230; $xDk=575; $panW=335
$g.FillRectangle((New-Object System.Drawing.SolidBrush([System.Drawing.Color]::FromArgb(255,243,243,243))),$xL,0,$panW,$H)
$g.FillRectangle((New-Object System.Drawing.SolidBrush([System.Drawing.Color]::FromArgb(255,32,32,32))),$xDk,0,$panW,$H)
$g.DrawString('светлая панель',$fnt,$brD,$xL+12,14)
$g.DrawString('тёмная панель',$fnt,$brL,$xDk+12,14)
for ($r=0; $r -lt $rows.Count; $r++) {
  $y = 56 + $r*84
  $g.DrawString($rows[$r].label,$fnt,$brD,14,$y+28)
  $p = Join-Path $ResDir $rows[$r].f
  # Counted above already; the sheet only leaves the row empty.
  if (-not (Test-Path -LiteralPath $p)) { continue }
  foreach ($panel in @($xL,$xDk)) {
    $x = $panel + 16
    foreach ($s in $shown) {
      # Straight from the PNG frame: Icon.ToBitmap() cannot decode PNG-compressed frames.
      $bytes = [System.IO.File]::ReadAllBytes($p)
      $n = [BitConverter]::ToUInt16($bytes,4)
      for ($i = 0; $i -lt $n; $i++) {
        $o = 6 + 16*$i
        $decl = $bytes[$o]; if ($decl -eq 0) { $decl = 256 }
        if ($decl -ne $s) { continue }
        $len = [BitConverter]::ToUInt32($bytes,$o+8); $off = [BitConverter]::ToUInt32($bytes,$o+12)
        $ms = New-Object System.IO.MemoryStream($bytes, $off, $len)
        $bm = [System.Drawing.Bitmap]::FromStream($ms)
        $g.DrawImage($bm, $x, ($y + 36 - [int]($s/2)), $s, $s)
        $bm.Dispose(); $ms.Dispose()
      }
      $x += $s + 14
    }
  }
}
$g.Dispose()
$bmp.Save($ProofPath,[System.Drawing.Imaging.ImageFormat]::Png)
$bmp.Dispose()
Write-Output "лист доказательств: $ProofPath"

if ($fail -eq 0) { Write-Output 'ИТОГ: все проверки пройдены'; exit 0 }
else { Write-Output "ИТОГ: ОТКАЗОВ $fail"; exit 1 }
