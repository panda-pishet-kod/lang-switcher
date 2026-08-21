<#
    verify-icons.ps1 -- measures what criteria 9 and 10 of task T-11-8 claim about the two
    tray icons in res\: seven PNG frames of 16/20/24/32/48/64/256 px in each .ico, and
    LoadImageW handing back every one of those sizes exactly, not a neighbour.

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
}
'@
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

$files = @('langswitcher-active.ico','langswitcher-paused.ico')
$SIZES = @(16,20,24,32,48,64,256)
$fail = 0

foreach ($f in $files) {
  $p = Join-Path $ResDir $f
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

  # Behaviour: LoadImage must hand back each size FR-90 needs, at the exact pixel size asked.
  foreach ($s in $SIZES) {
    $got = Get-LoadedIconSize $p $s
    if ($null -eq $got) { Write-Output "  ОТКАЗ: LoadImage($s) вернул NULL"; $fail++; continue }
    if ($got[0] -ne $s -or $got[1] -ne $s) {
      Write-Output ("  ОТКАЗ: LoadImage({0}) отдал {1}x{2}" -f $s,$got[0],$got[1]); $fail++
    } else {
      Write-Output ("  LoadImage({0,3}) -> {1}x{2}  ок" -f $s,$got[0],$got[1])
    }
  }
}

# Proof sheet at true pixel sizes on both taskbar colours.
$rows = @(@{f='langswitcher-active.ico';label='активна'}, @{f='langswitcher-paused.ico';label='пауза'})
$shown = @(16,20,24,32,48)
$W = 700; $H = 56 + $rows.Count*84
$bmp = New-Object System.Drawing.Bitmap($W,$H,[System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.SmoothingMode='AntiAlias'; $g.TextRenderingHint='ClearTypeGridFit'
$g.Clear([System.Drawing.Color]::FromArgb(255,250,250,250))
$fnt = New-Object System.Drawing.Font('Segoe UI',10,[System.Drawing.FontStyle]::Bold)
$brD = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::FromArgb(255,40,42,46))
$brL = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::FromArgb(255,200,204,210))
$xL=130; $xDk=420; $panW=280
$g.FillRectangle((New-Object System.Drawing.SolidBrush([System.Drawing.Color]::FromArgb(255,243,243,243))),$xL,0,$panW,$H)
$g.FillRectangle((New-Object System.Drawing.SolidBrush([System.Drawing.Color]::FromArgb(255,32,32,32))),$xDk,0,$panW,$H)
$g.DrawString('светлая панель',$fnt,$brD,$xL+12,14)
$g.DrawString('тёмная панель',$fnt,$brL,$xDk+12,14)
for ($r=0; $r -lt $rows.Count; $r++) {
  $y = 56 + $r*84
  $g.DrawString($rows[$r].label,$fnt,$brD,14,$y+28)
  $p = Join-Path $ResDir $rows[$r].f
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
