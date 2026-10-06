<#
    accept-clean-machine.ps1 -- the acceptance of an installer on a CLEAN machine (stage E91,
    task T-91-5). The instrument of every delivery that goes out: it is kept in the tree.

    WHY A SCRIPT, AND WHY ONE REPORT. The certificate branch of the installer (the helper
    installer\local-trust.ps1 making a certificate, signing the full image, destroying the key)
    may only run on a machine that does not trust the author's certificate -- never on the
    author's own. Somebody carries this file and the installer to such a machine, runs it once
    from an elevated Windows PowerShell, and brings back ONE text file. Every row of it is PASS or
    FAIL with the value measured, and the logs of every run are appended whole, so that the cause
    of any FAIL can be read from the report without a second visit.

    WHAT IT DOES, IN ORDER (each round starts from a machine without the program):

      0  the machine: Windows, its UI language, the UAC policy (a check made with UAC below the
         recommended level is not a check -- decision 155v), Defender and Smart App Control,
         Windows PowerShell and its two cmdlets, the execution policy; the author's certificate
         must NOT be trusted here; THE POSITIVE CONTROL: the rows of round 1 measured BEFORE
         anything is installed must FAIL -- an instrument that cannot fail measures nothing;
      1  silent install, default: the helper answers 1, the program file is the FULL image signed
         by the local certificate, Valid; LocalMachine\Root holds exactly one local certificate --
         code signing only, not a certification authority, about 30 years; no certificate with a
         key in LocalMachine\My; its public part in the program folder; the program started
         through explorer.exe runs with UIAccess = 1; then the silent uninstall: no local
         certificate anywhere, no program folder;
      2  silent install with /NOLOCALCERT: the helper answers 2, the BASE image, no certificate,
         the program runs with UIAccess = 0; uninstall;
      3  two silent installs in a row: exactly ONE local certificate in Root afterwards -- the
         second one -- and the first gone; uninstall;
      4  silent install with LANGSW_TRUST_FAIL_AT=delete-key, the failure seam of the helper,
         right after the signature: the helper answers 18, names the step, undoes everything --
         no certificate in Root, none in My, no key -- and the program file is the BASE image;
         uninstall;
      5  silent install, the program started through explorer.exe, then the silent uninstall
         WITHOUT stopping it -- the way a person uninstalls (decision 155.24; rounds 1-4 stop the
         program first): the uninstaller refuses with a non-zero exit code and says so in its
         log, the program still runs, its file, the uninstaller and the local certificate are
         all still there; then the program is stopped and the uninstall removes everything.
      Then: the events of Microsoft Defender and of Code Integrity since the start, and the
      summary. Exit 0 only if every row is PASS.

    WHAT IT DOES NOT SEE -- the hand of the person, by the memo reports\ACCEPT-CLEAN-MACHINE-E91.md:
    the yellow UAC prompt of a double-click install, the language of the wizard, the line on the
    Ready page, "Launch Lang Switcher" without an error window, the word converted in Notepad and
    in Notepad run as administrator, the start after signing in again, /LANG=hebrew, and the
    window that asks to exit the running program when it is uninstalled from Apps.

    -DryRun prints the plan and the read-only measurements of step 0 and installs nothing, starts
    nothing, writes no file. It is the only way to run this script on the author's machine.

    Exit codes: 0 every row PASS; 1 a row FAIL; 2 cannot start (not elevated, no installer, the
    author's certificate is trusted here and -DryRun was not given).

    Not copied from anywhere; Windows PowerShell 5.1, ASCII (decision R-05).

    Examples (elevated Windows PowerShell, in the folder of the two files):
      powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\accept-clean-machine.ps1
      powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\accept-clean-machine.ps1 -DryRun
#>
[CmdletBinding()]
param(
    # The installer. Empty means LangSwitcher-setup.exe beside this script.
    [string]$Setup = '',
    # The report. Empty means accept-report-<computer>-<yyyyMMdd-HHmmss>.txt beside this script.
    [string]$Report = '',
    [switch]$DryRun
)

$ErrorActionPreference = 'Stop'
$Here = $PSScriptRoot
if (-not $Setup) { $Setup = Join-Path $Here 'LangSwitcher-setup.exe' }
$Stamp = Get-Date -Format 'yyyyMMdd-HHmmss'
if (-not $Report) { $Report = Join-Path $Here ('accept-report-{0}-{1}.txt' -f $env:COMPUTERNAME, $Stamp) }
$Work = Join-Path $env:TEMP ('langsw-accept-' + $Stamp)
$AppDir = Join-Path $env:ProgramFiles 'Lang_Switcher'
$AppExe = Join-Path $AppDir 'LangSwitcher.exe'
$AppCer = Join-Path $AppDir 'LangSwitcher-local.cer'
$AppTmp = Join-Path $AppDir 'LangSwitcher-full.tmp'
$Uninstaller = Join-Path $AppDir 'unins000.exe'
$LocalSubject = 'CN=Lang Switcher for Windows (local)'
$CodeSigning = '1.3.6.1.5.5.7.3.3'
$Started = Get-Date

$script:lines = New-Object System.Collections.Generic.List[string]
$script:appendix = New-Object System.Collections.Generic.List[string]
$script:pass = 0
$script:fail = 0

function Out-Line([string]$Text) {
    $script:lines.Add($Text)
    Write-Host $Text
}

function Out-Row([bool]$Ok, [string]$Name, [string]$Value) {
    if ($Ok) { $script:pass++; $v = 'PASS' } else { $script:fail++; $v = 'FAIL' }
    Out-Line ('{0}  {1} -- {2}' -f $v, $Name, $Value)
}

# The helper writes everything outside ASCII as \uXXXX (installer\local-trust.ps1); turn it back.
function ConvertFrom-Escaped([string]$Text) {
    return [regex]::Replace($Text, '\\u([0-9A-Fa-f]{4})', { param($m) [string][char][Convert]::ToInt32($m.Groups[1].Value, 16) })
}

# Compiled on first use only (Initialize-Native): Add-Type writes a temporary assembly, and -DryRun
# writes nothing.
$NativeSource = @'
using System;
using System.Runtime.InteropServices;
public static class AcceptToken {
    [DllImport("user32.dll")]
    static extern void keybd_event(byte vk, byte scan, uint flags, UIntPtr extra);

    // FR-96, the emergency exit of the program: Ctrl+Alt+Shift+F12 through the input queue. The
    // program's low-level hook sees injected keys, so this stops it where Stop-Process may not.
    public static void Emergency() {
        byte[] keys = { 0x11, 0x12, 0x10, 0x7B };
        foreach (byte k in keys) keybd_event(k, 0, 0, UIntPtr.Zero);
        for (int i = keys.Length - 1; i >= 0; i--) keybd_event(keys[i], 0, 2, UIntPtr.Zero);
    }

    [DllImport("kernel32.dll", SetLastError = true)]
    static extern IntPtr OpenProcess(uint access, bool inherit, int pid);
    [DllImport("kernel32.dll")]
    static extern bool CloseHandle(IntPtr h);
    [DllImport("advapi32.dll", SetLastError = true)]
    static extern bool OpenProcessToken(IntPtr process, uint access, out IntPtr token);
    [DllImport("advapi32.dll", SetLastError = true)]
    static extern bool GetTokenInformation(IntPtr token, int cls, IntPtr info, int length, out int returned);
    [DllImport("advapi32.dll")]
    static extern IntPtr GetSidSubAuthority(IntPtr sid, uint index);
    [DllImport("advapi32.dll")]
    static extern IntPtr GetSidSubAuthorityCount(IntPtr sid);

    static int ReadInt(IntPtr token, int cls) {
        IntPtr buffer = Marshal.AllocHGlobal(4);
        try {
            int returned;
            if (!GetTokenInformation(token, cls, buffer, 4, out returned)) return -Marshal.GetLastWin32Error();
            return Marshal.ReadInt32(buffer);
        } finally { Marshal.FreeHGlobal(buffer); }
    }

    // "UIAccess=1 Elevated=0 IntegrityLevel=0x3000", or "error N" -- the probe of T-09-2.
    public static string Describe(int pid) {
        IntPtr process = OpenProcess(0x1000, false, pid);
        if (process == IntPtr.Zero) return "error OpenProcess " + Marshal.GetLastWin32Error();
        try {
            IntPtr token;
            if (!OpenProcessToken(process, 0x0008, out token)) return "error OpenProcessToken " + Marshal.GetLastWin32Error();
            try {
                int uiAccess = ReadInt(token, 26);
                int elevated = ReadInt(token, 20);
                int returned;
                GetTokenInformation(token, 25, IntPtr.Zero, 0, out returned);
                IntPtr label = Marshal.AllocHGlobal(returned);
                string level = "?";
                try {
                    if (GetTokenInformation(token, 25, label, returned, out returned)) {
                        IntPtr sid = Marshal.ReadIntPtr(label);
                        int count = Marshal.ReadByte(GetSidSubAuthorityCount(sid));
                        level = "0x" + Marshal.ReadInt32(GetSidSubAuthority(sid, (uint)(count - 1))).ToString("X4");
                    }
                } finally { Marshal.FreeHGlobal(label); }
                return "UIAccess=" + uiAccess + " Elevated=" + elevated + " IntegrityLevel=" + level;
            } finally { CloseHandle(token); }
        } finally { CloseHandle(process); }
    }
}
'@
$script:nativeReady = $false
function Initialize-Native {
    if (-not $script:nativeReady) { Add-Type -Language CSharp -TypeDefinition $NativeSource; $script:nativeReady = $true }
}

# --- What an image is: the uiAccess value of its manifest resource -------------------------------
function Get-ImageKind([string]$Path) {
    if (-not (Test-Path -LiteralPath $Path)) { return 'absent' }
    $b = [System.IO.File]::ReadAllBytes($Path)
    $pe = [BitConverter]::ToInt32($b, 0x3C)
    $opt = $pe + 24
    $count = [BitConverter]::ToUInt16($b, $pe + 6)
    $table = $opt + [BitConverter]::ToUInt16($b, $pe + 20)
    $resRva = [BitConverter]::ToUInt32($b, $opt + 112 + 16)
    $sections = @()
    for ($i = 0; $i -lt $count; $i++) {
        $at = $table + 40 * $i
        $sections += @{ Va = [BitConverter]::ToUInt32($b, $at + 12); Size = [Math]::Max([BitConverter]::ToUInt32($b, $at + 8), [BitConverter]::ToUInt32($b, $at + 16)); Raw = [BitConverter]::ToUInt32($b, $at + 20) }
    }
    $toOffset = { param($rva) foreach ($s in $sections) { if ($rva -ge $s.Va -and $rva -lt $s.Va + $s.Size) { return $s.Raw + ($rva - $s.Va) } }; return -1 }
    $root = & $toOffset $resRva
    $node = $root
    foreach ($want in @(24, -1, -1)) {
        $n = [BitConverter]::ToUInt16($b, $node + 12) + [BitConverter]::ToUInt16($b, $node + 14)
        $next = $null
        for ($i = 0; $i -lt $n; $i++) {
            $e = $node + 16 + 8 * $i
            $id = [BitConverter]::ToUInt32($b, $e)
            if ($want -eq -1 -or $id -eq $want) { $next = [BitConverter]::ToUInt32($b, $e + 4); break }
        }
        if ($null -eq $next) { return 'no manifest' }
        if (($next -band 0x80000000) -ne 0) { $node = $root + ($next -band 0x7FFFFFFF) }
        else {
            $entry = $root + $next
            $text = [System.Text.Encoding]::UTF8.GetString($b, (& $toOffset ([BitConverter]::ToUInt32($b, $entry))), [BitConverter]::ToUInt32($b, $entry + 4))
            $m = [regex]::Match($text, 'uiAccess="(true|false)"')
            if (-not $m.Success) { return 'manifest without uiAccess' }
            if ($m.Groups[1].Value -eq 'true') { return 'FULL (uiAccess=true)' } else { return 'BASE (uiAccess=false)' }
        }
    }
    return 'no manifest'
}

function Get-LocalCertificates([string]$StoreName) {
    $store = New-Object System.Security.Cryptography.X509Certificates.X509Store($StoreName, [System.Security.Cryptography.X509Certificates.StoreLocation]::LocalMachine)
    $store.Open([System.Security.Cryptography.X509Certificates.OpenFlags]::ReadOnly)
    try { return @($store.Certificates | Where-Object { $_.Subject -eq $LocalSubject }) } finally { $store.Close() }
}

function Describe-Certificate($c) {
    $eku = @($c.Extensions | Where-Object { $_.Oid.Value -eq '2.5.29.37' } | ForEach-Object { $_.EnhancedKeyUsages } | ForEach-Object { $_.Value })
    $ku = @($c.Extensions | Where-Object { $_.Oid.Value -eq '2.5.29.15' } | ForEach-Object { [string]$_.KeyUsages })
    $bc = @($c.Extensions | Where-Object { $_.Oid.Value -eq '2.5.29.19' })
    $ca = 'no basic constraints'
    if ($bc.Count -gt 0) { $ca = 'CA=' + $bc[0].CertificateAuthority }
    return ('{0}, {1}, {2:yyyy-MM-dd}..{3:yyyy-MM-dd}, RSA {4}, EKU {5}, KU {6}, {7}, private key {8}' -f $c.Thumbprint, $c.Subject, $c.NotBefore, $c.NotAfter, $c.PublicKey.Key.KeySize, ($eku -join '+'), ($ku -join '+'), $ca, $c.HasPrivateKey)
}

function Get-StatusLines([string]$Log) {
    if (-not (Test-Path -LiteralPath $Log)) { return @() }
    return @(Get-Content -LiteralPath $Log | Where-Object { $_ -match 'local-trust:|Lang Switcher:' } | ForEach-Object { ConvertFrom-Escaped $_ })
}

function Add-Appendix([string]$Title, [string]$Path) {
    $script:appendix.Add('')
    $script:appendix.Add('=== ' + $Title + ' (' + $Path + ')')
    if (Test-Path -LiteralPath $Path) {
        foreach ($l in (Get-Content -LiteralPath $Path)) { $script:appendix.Add((ConvertFrom-Escaped $l)) }
    } else { $script:appendix.Add('(no such file)') }
}

function Invoke-Setup([string]$Name, [string[]]$Extra, [string]$FailAt) {
    $log = Join-Path $Work ($Name + '.log')
    $args2 = @('/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART', ('/LOG="' + $log + '"')) + $Extra
    [Environment]::SetEnvironmentVariable('LANGSW_TRUST_FAIL_AT', $FailAt, 'Process')
    try {
        $p = Start-Process -FilePath $Setup -ArgumentList $args2 -Wait -PassThru
        $code = $p.ExitCode
    } finally { [Environment]::SetEnvironmentVariable('LANGSW_TRUST_FAIL_AT', $null, 'Process') }
    Out-Line ('       setup {0}: exit {1}, log {2}' -f ($args2 -join ' '), $code, $log)
    Add-Appendix ('setup log, ' + $Name) $log
    $helper = -999
    foreach ($l in (Get-StatusLines $log)) {
        $m = [regex]::Match($l, 'the helper returned (-?\d+)')
        if ($m.Success) { $helper = [int]$m.Groups[1].Value }
    }
    foreach ($l in (Get-StatusLines $log | Where-Object { $_ -match 'RESULT:|FAILED|branch|BASE image|undo:|step |64-bit process' })) { Out-Line ('         | ' + ($l -replace '^.*?(local-trust:|Lang Switcher:)', '$1')) }
    return @{ Code = $code; Helper = $helper; Log = $log }
}

function Invoke-Uninstall([string]$Name) {
    $log = Join-Path $Work ($Name + '.log')
    if (-not (Test-Path -LiteralPath $Uninstaller)) { Out-Row $false ($Name + ': the uninstaller exists') $Uninstaller; return }
    Stop-Product
    $p = Start-Process -FilePath $Uninstaller -ArgumentList @('/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART', ('/LOG="' + $log + '"')) -Wait -PassThru
    # The uninstaller runs a copy of itself from a temporary folder and its first process ends at
    # once; it is finished when the program folder is gone, or after two minutes.
    $deadline = (Get-Date).AddSeconds(120)
    while ((Test-Path -LiteralPath $Uninstaller) -and (Get-Date) -lt $deadline) { Start-Sleep -Milliseconds 500 }
    Start-Sleep -Seconds 2
    Out-Line ('       uninstall: first process exit {0}, log {1}' -f $p.ExitCode, $log)
    Add-Appendix ('uninstall log, ' + $Name) $log
    foreach ($l in (Get-StatusLines $log)) { Out-Line ('         | ' + ($l -replace '^.*?(Lang Switcher:)', '$1')) }
    $root = @(Get-LocalCertificates 'Root').Count
    $my = @(Get-LocalCertificates 'My').Count
    Out-Row (($root + $my) -eq 0) ($Name + ': no local certificate is left') ('Root {0}, My {1}' -f $root, $my)
    Out-Row (-not (Test-Path -LiteralPath $AppDir)) ($Name + ': the program folder is gone') $AppDir
}

function Stop-Product {
    foreach ($p in @(Get-Process -Name 'LangSwitcher' -ErrorAction SilentlyContinue)) {
        try { Stop-Process -Id $p.Id -Force -ErrorAction Stop; Out-Line ('       stopped LangSwitcher pid {0}' -f $p.Id) }
        catch { Out-Line ('       Stop-Process could not stop pid {0}: {1}' -f $p.Id, $_.Exception.Message) }
    }
    Start-Sleep -Seconds 1
    if (@(Get-Process -Name 'LangSwitcher' -ErrorAction SilentlyContinue).Count -gt 0) {
        Initialize-Native
        [AcceptToken]::Emergency()
        Start-Sleep -Seconds 2
        Out-Line ('       FR-96 sent; LangSwitcher processes left: {0}' -f @(Get-Process -Name 'LangSwitcher' -ErrorAction SilentlyContinue).Count)
    }
}

# Started the way a user starts it: explorer.exe hands it to the shell of the session, which runs
# it with the user's own token through ShellExecute -- the road that grants uiAccess.
function Test-Launch([string]$Name, [int]$WantUiAccess) {
    Stop-Product
    $before = @(Get-Process -Name 'LangSwitcher' -ErrorAction SilentlyContinue | ForEach-Object { $_.Id })
    Start-Process -FilePath 'explorer.exe' -ArgumentList ('"' + $AppExe + '"')
    $found = $null
    $deadline = (Get-Date).AddSeconds(20)
    while (($null -eq $found) -and (Get-Date) -lt $deadline) {
        Start-Sleep -Milliseconds 500
        $found = Get-Process -Name 'LangSwitcher' -ErrorAction SilentlyContinue | Where-Object { $before -notcontains $_.Id } | Select-Object -First 1
    }
    if ($null -eq $found) {
        Out-Row $false ($Name + ': the program starts through explorer.exe') 'no LangSwitcher process within 20 s'
        return
    }
    Start-Sleep -Seconds 3
    $alive = -not $found.HasExited
    Initialize-Native
    $token = [AcceptToken]::Describe($found.Id)
    Out-Row $alive ($Name + ': the program starts through explorer.exe and keeps running') ('pid {0}, alive after 3 s: {1}' -f $found.Id, $alive)
    Out-Row ($token -match ('UIAccess=' + $WantUiAccess + ' ')) ($Name + (': the token says UIAccess = {0}' -f $WantUiAccess)) $token
    Stop-Product
}

function Test-Installed([string]$Name, [string]$WantKind, [bool]$WantLocal) {
    $kind = Get-ImageKind $AppExe
    Out-Row ($kind -eq $WantKind) ($Name + ': the program file is the ' + $WantKind.Split(' ')[0] + ' image') ($AppExe + ': ' + $kind)
    if (Test-Path -LiteralPath $AppExe) {
        $sig = Get-AuthenticodeSignature -LiteralPath $AppExe
        $signer = '(none)'
        if ($sig.SignerCertificate) { $signer = $sig.SignerCertificate.Subject + ' ' + $sig.SignerCertificate.Thumbprint }
        $msg = ConvertFrom-Escaped $sig.StatusMessage
        if ($WantLocal) {
            Out-Row (([string]$sig.Status -eq 'Valid') -and ($signer -like ($LocalSubject + '*'))) ($Name + ': its signature is Valid, by the local certificate') ('{0}, {1} -- {2}' -f $sig.Status, $signer, $msg)
        } else {
            Out-Line ('INFO  ' + $Name + (': its signature here: {0}, {1} -- {2}' -f $sig.Status, $signer, $msg))
        }
    }
    $root = @(Get-LocalCertificates 'Root')
    $my = @(Get-LocalCertificates 'My')
    $withKey = @($my | Where-Object { $_.HasPrivateKey }).Count
    if ($WantLocal) {
        Out-Row ($root.Count -eq 1) ($Name + ': exactly one local certificate in LocalMachine\Root') ('{0}: {1}' -f $root.Count, (($root | ForEach-Object { Describe-Certificate $_ }) -join ' | '))
        if ($root.Count -eq 1) {
            $c = $root[0]
            $eku = @($c.Extensions | Where-Object { $_.Oid.Value -eq '2.5.29.37' } | ForEach-Object { $_.EnhancedKeyUsages } | ForEach-Object { $_.Value })
            $bc = @($c.Extensions | Where-Object { $_.Oid.Value -eq '2.5.29.19' -and $_.CertificateAuthority })
            Out-Row (($eku.Count -eq 1) -and ($eku[0] -eq $CodeSigning)) ($Name + ': it is for code signing only') ($eku -join '+')
            Out-Row ($bc.Count -eq 0) ($Name + ': it is not a certification authority') ('CA extensions: ' + $bc.Count)
            $years = ($c.NotAfter - $c.NotBefore).TotalDays / 365.25
            Out-Row (($years -gt 29.9) -and ($years -lt 30.1)) ($Name + ': it is valid for 30 years') ('{0:N2} years, until {1:yyyy-MM-dd}' -f $years, $c.NotAfter)
            if (Test-Path -LiteralPath $AppCer) {
                $file = New-Object System.Security.Cryptography.X509Certificates.X509Certificate2 -ArgumentList $AppCer
                Out-Row (($file.Thumbprint -eq $c.Thumbprint) -and (-not $file.HasPrivateKey)) ($Name + ': its public part lies in the program folder, without a key') ($AppCer + ': ' + $file.Thumbprint)
            } else { Out-Row $false ($Name + ': its public part lies in the program folder') ($AppCer + ' is absent') }
        }
    } else {
        Out-Row ($root.Count -eq 0) ($Name + ': no local certificate in LocalMachine\Root') ('{0}' -f $root.Count)
        Out-Row (-not (Test-Path -LiteralPath $AppCer)) ($Name + ': no local certificate file in the program folder') $AppCer
    }
    Out-Row (($my.Count -eq 0) -and ($withKey -eq 0)) ($Name + ': no local certificate, and no key, in LocalMachine\My') ('{0} certificate(s), {1} with a key' -f $my.Count, $withKey)
    Out-Row (-not (Test-Path -LiteralPath $AppTmp)) ($Name + ': the temporary full image is gone') $AppTmp
    return $root
}

# ==================================================================================================
Out-Line ('Lang Switcher -- acceptance on a clean machine (tools\accept-clean-machine.ps1), {0:yyyy-MM-dd HH:mm:ss}' -f $Started)
if ($DryRun) { Out-Line 'MODE: -DryRun -- the plan and step 0 only; nothing is installed, started or written' }
Out-Line ('installer: {0}' -f $Setup)

# --- 0. The machine ------------------------------------------------------------------------------
Out-Line ''
Out-Line '--- 0. The machine'
$os = Get-CimInstance Win32_OperatingSystem
Out-Line ('INFO  Windows: {0}, version {1}, build {2}, {3}' -f $os.Caption, $os.Version, $os.BuildNumber, $os.OSArchitecture)
Out-Line ('INFO  UI language: {0}; culture {1}' -f (Get-UICulture).Name, (Get-Culture).Name)
$identity = [Security.Principal.WindowsIdentity]::GetCurrent()
$admin = (New-Object Security.Principal.WindowsPrincipal($identity)).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
Out-Line ('INFO  user {0}, elevated administrator: {1}' -f $identity.Name, $admin)
$policy = Get-ItemProperty -LiteralPath 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Policies\System'
$uacText = 'EnableLUA {0}, ConsentPromptBehaviorAdmin {1}, ConsentPromptBehaviorUser {2}, PromptOnSecureDesktop {3}' -f $policy.EnableLUA, $policy.ConsentPromptBehaviorAdmin, $policy.ConsentPromptBehaviorUser, $policy.PromptOnSecureDesktop
$uacOk = ($policy.EnableLUA -eq 1) -and ($policy.ConsentPromptBehaviorAdmin -eq 5) -and ($policy.PromptOnSecureDesktop -eq 1)
Out-Row $uacOk 'UAC is at the recommended level (1, 5, 1); below it this acceptance is not one' $uacText
try {
    $mp = Get-MpComputerStatus
    Out-Line ('INFO  Microsoft Defender: real-time protection {0}, mode {1}, engine {2}' -f $mp.RealTimeProtectionEnabled, $mp.AMRunningMode, $mp.AMEngineVersion)
} catch { Out-Line ('INFO  Microsoft Defender: Get-MpComputerStatus failed: ' + $_.Exception.Message) }
$sac = (Get-ItemProperty -LiteralPath 'HKLM:\SYSTEM\CurrentControlSet\Control\CI\Policy' -ErrorAction SilentlyContinue).VerifiedAndReputablePolicyState
Out-Line ('INFO  Smart App Control: VerifiedAndReputablePolicyState = {0} (0 off, 1 on, 2 evaluation, empty: not present)' -f $sac)
Out-Line ('INFO  Windows PowerShell {0}; New-SelfSignedCertificate {1}; Set-AuthenticodeSignature {2}' -f $PSVersionTable.PSVersion, [bool](Get-Command New-SelfSignedCertificate -ErrorAction SilentlyContinue), [bool](Get-Command Set-AuthenticodeSignature -ErrorAction SilentlyContinue))
Out-Line ('INFO  execution policy: ' + ((Get-ExecutionPolicy -List | ForEach-Object { $_.Scope.ToString() + '=' + $_.ExecutionPolicy.ToString() }) -join ', '))

if (-not (Test-Path -LiteralPath $Setup)) { Out-Line ('STOP: the installer is missing: ' + $Setup); exit 2 }
$setupSig = Get-AuthenticodeSignature -LiteralPath $Setup
$author = ''
if ($setupSig.SignerCertificate) { $author = $setupSig.SignerCertificate.Thumbprint }
Out-Line ('INFO  installer: {0} bytes, SHA-256 {1}, signature {2}, signer {3} {4}' -f (Get-Item -LiteralPath $Setup).Length, (Get-FileHash -LiteralPath $Setup -Algorithm SHA256).Hash, $setupSig.Status, $author, $setupSig.SignerCertificate.Subject)
$rootStore = New-Object System.Security.Cryptography.X509Certificates.X509Store('Root', [System.Security.Cryptography.X509Certificates.StoreLocation]::LocalMachine)
$rootStore.Open([System.Security.Cryptography.X509Certificates.OpenFlags]::ReadOnly)
$authorTrusted = @($rootStore.Certificates | Where-Object { $_.Thumbprint -eq $author }).Count -gt 0
$rootStore.Close()
Out-Row (-not $authorTrusted) 'the author''s certificate is NOT trusted here (a clean machine)' ('author {0} in LocalMachine\Root: {1}' -f $author, $authorTrusted)

# THE POSITIVE CONTROL: the rows of round 1, measured before anything is installed, must fail.
$controlKind = Get-ImageKind $AppExe
$controlRoot = @(Get-LocalCertificates 'Root').Count
$controlOk = ($controlKind -eq 'absent') -and ($controlRoot -eq 0)
Out-Row $controlOk 'POSITIVE CONTROL: before the install the rows "the program file is the FULL image" and "one local certificate in Root" are FAIL' ('program file: {0}; local certificates in Root: {1}' -f $controlKind, $controlRoot)

if ($DryRun) {
    Out-Line ''
    Out-Line 'PLAN (not carried out under -DryRun):'
    Out-Line '  1  silent install; helper 1; FULL image, Valid, local certificate (Root 1, My 0), UIAccess 1; uninstall'
    Out-Line '  2  silent install /NOLOCALCERT; helper 2; BASE image, no certificate, UIAccess 0; uninstall'
    Out-Line '  3  two silent installs; Root holds exactly the second local certificate; uninstall'
    Out-Line '  4  silent install, LANGSW_TRUST_FAIL_AT=delete-key; helper 18; BASE image; Root 0, My 0; uninstall'
    Out-Line '  5  silent install; the program runs; silent uninstall refused, nothing removed; stop; uninstall'
    Out-Line ('  report: {0}' -f $Report)
    Out-Line ('dry run: {0} PASS, {1} FAIL (step 0 only)' -f $script:pass, $script:fail)
    exit 0
}
if (-not $admin) { Out-Line 'STOP: run this from an elevated Windows PowerShell'; exit 2 }
if ($authorTrusted) { Out-Line 'STOP: this machine trusts the author''s certificate -- it is not a clean machine; nothing was done'; exit 2 }
if ((Test-Path -LiteralPath $AppDir) -or ($controlRoot -gt 0)) { Out-Line 'STOP: Lang Switcher or a local certificate is already here; uninstall it first (exit the program from its tray menu; delete the program folder if it stays); nothing was done'; exit 2 }
New-Item -ItemType Directory -Path $Work | Out-Null
Out-Line ('INFO  logs of this run: ' + $Work)

# --- 1. Default ----------------------------------------------------------------------------------
Out-Line ''
Out-Line '--- 1. Silent install, default: the full image through a local certificate'
$r = Invoke-Setup 'round1-setup' @() ''
Out-Row ($r.Code -eq 0) 'round 1: Setup succeeds' ('exit ' + $r.Code)
Out-Row ($r.Helper -eq 1) 'round 1: the helper made a local certificate and signed the full image (1)' ('helper ' + $r.Helper)
$null = Test-Installed 'round 1' 'FULL (uiAccess=true)' $true
Test-Launch 'round 1' 1
Invoke-Uninstall 'round1-uninstall'

# --- 2. /NOLOCALCERT -----------------------------------------------------------------------------
Out-Line ''
Out-Line '--- 2. Silent install /NOLOCALCERT: the base image, no certificate'
$r = Invoke-Setup 'round2-setup' @('/NOLOCALCERT') ''
Out-Row ($r.Code -eq 0) 'round 2: Setup succeeds' ('exit ' + $r.Code)
Out-Row ($r.Helper -eq 2) 'round 2: the helper refused by /NOLOCALCERT (2)' ('helper ' + $r.Helper)
$null = Test-Installed 'round 2' 'BASE (uiAccess=false)' $false
Test-Launch 'round 2' 0
Invoke-Uninstall 'round2-uninstall'

# --- 3. Twice in a row ---------------------------------------------------------------------------
Out-Line ''
Out-Line '--- 3. Two silent installs in a row: exactly one local certificate'
$r = Invoke-Setup 'round3-setup-a' @() ''
$first = @(Get-LocalCertificates 'Root' | ForEach-Object { $_.Thumbprint })
Out-Line ('       after the first install: ' + ($first -join ', '))
$r = Invoke-Setup 'round3-setup-b' @() ''
Out-Row ($r.Helper -eq 1) 'round 3: the second install made its certificate too (1)' ('helper ' + $r.Helper)
$roots = Test-Installed 'round 3' 'FULL (uiAccess=true)' $true
$second = @($roots | ForEach-Object { $_.Thumbprint })
Out-Row ((@($second | Where-Object { $first -contains $_ }).Count -eq 0) -and ($second.Count -eq 1)) 'round 3: the certificate of the first install is gone, the second one is the one left' ('first {0}; now {1}' -f ($first -join ','), ($second -join ','))
Invoke-Uninstall 'round3-uninstall'

# --- 4. The failure seam -------------------------------------------------------------------------
Out-Line ''
Out-Line '--- 4. Silent install, LANGSW_TRUST_FAIL_AT=delete-key: the undo after the signature'
$r = Invoke-Setup 'round4-setup' @() 'delete-key'
Out-Row ($r.Code -eq 0) 'round 4: Setup succeeds' ('exit ' + $r.Code)
Out-Row ($r.Helper -eq 18) 'round 4: the helper failed at delete-key (18)' ('helper ' + $r.Helper)
$named = @(Get-StatusLines $r.Log | Where-Object { $_ -match 'FAILED at step delete-key' }).Count -gt 0
Out-Row $named 'round 4: the log names the step' ('"FAILED at step delete-key" found: ' + $named)
$null = Test-Installed 'round 4' 'BASE (uiAccess=false)' $false
Invoke-Uninstall 'round4-uninstall'

# --- 5. Uninstall while the program runs (decision 155.24) -----------------------------------------
Out-Line ''
Out-Line '--- 5. Silent uninstall while the program runs: refused, nothing removed; then removed'
$r = Invoke-Setup 'round5-setup' @() ''
Out-Row ($r.Code -eq 0) 'round 5: Setup succeeds' ('exit ' + $r.Code)
Stop-Product
Start-Process -FilePath 'explorer.exe' -ArgumentList ('"' + $AppExe + '"')
$running = $null
$deadline = (Get-Date).AddSeconds(20)
while (($null -eq $running) -and (Get-Date) -lt $deadline) {
    Start-Sleep -Milliseconds 500
    $running = Get-Process -Name 'LangSwitcher' -ErrorAction SilentlyContinue | Select-Object -First 1
}
$runningText = 'no LangSwitcher process within 20 s'
if ($null -ne $running) { $runningText = 'pid ' + $running.Id }
Out-Row ($null -ne $running) 'round 5: the program runs before the uninstall' $runningText
# The program takes its mutex at start; three seconds is the wait Test-Launch gives it too.
Start-Sleep -Seconds 3
$log5 = Join-Path $Work 'round5-uninstall-refused.log'
$p5 = Start-Process -FilePath $Uninstaller -ArgumentList @('/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART', ('/LOG="' + $log5 + '"')) -Wait -PassThru
Start-Sleep -Seconds 5
Out-Line ('       uninstall while running: first process exit {0}, log {1}' -f $p5.ExitCode, $log5)
Add-Appendix 'uninstall log, round5-uninstall-refused' $log5
foreach ($l in (Get-StatusLines $log5)) { Out-Line ('         | ' + ($l -replace '^.*?(Lang Switcher:)', '$1')) }
$refused = @(Get-StatusLines $log5 | Where-Object { $_ -match 'the uninstall is cancelled while the program runs' }).Count -gt 0
Out-Row ($p5.ExitCode -ne 0) 'round 5: the uninstall of the running program is refused (exit not 0)' ('first process exit ' + $p5.ExitCode)
Out-Row $refused 'round 5: the log names the refusal' ('"the uninstall is cancelled while the program runs" found: ' + $refused)
$stillRunning = ($null -ne $running) -and (-not $running.HasExited)
Out-Row $stillRunning 'round 5: the program still runs' $runningText
$kept = (Test-Path -LiteralPath $AppExe) -and (Test-Path -LiteralPath $Uninstaller)
Out-Row $kept 'round 5: the program file and the uninstaller are still there' $AppDir
$root5 = @(Get-LocalCertificates 'Root').Count
Out-Row ($root5 -eq 1) 'round 5: the local certificate is still in Root' ('Root ' + $root5)
Invoke-Uninstall 'round5-uninstall'

# --- Defender and Code Integrity since the start ---------------------------------------------------
Out-Line ''
Out-Line '--- Events of protection since the start'
foreach ($logName in @('Microsoft-Windows-Windows Defender/Operational', 'Microsoft-Windows-CodeIntegrity/Operational')) {
    try {
        $events = @(Get-WinEvent -FilterHashtable @{ LogName = $logName; StartTime = $Started } -ErrorAction Stop)
    } catch { $events = @() }
    $alarming = @($events | Where-Object { @(1006, 1007, 1008, 1015, 1116, 1117, 1118, 1119, 3033, 3034, 3076, 3077, 3089) -contains $_.Id })
    Out-Row ($alarming.Count -eq 0) ('no blocking event in ' + $logName) ('{0} event(s), {1} of them detections or blocks' -f $events.Count, $alarming.Count)
    foreach ($e in $alarming) { Out-Line ('       {0:HH:mm:ss} id {1}: {2}' -f $e.TimeCreated, $e.Id, (($e.Message -split "`n")[0])) }
}

# --- Summary -----------------------------------------------------------------------------------------
Out-Line ''
Out-Line ('SUMMARY: {0} PASS, {1} FAIL; {2:N0} s' -f $script:pass, $script:fail, ((Get-Date) - $Started).TotalSeconds)
$all = New-Object System.Collections.Generic.List[string]
$all.AddRange($script:lines)
$all.Add('')
$all.Add('=================== APPENDIX: the logs of every run ===================')
$all.AddRange($script:appendix)
[System.IO.File]::WriteAllLines($Report, $all, (New-Object System.Text.UTF8Encoding($true)))
Write-Host ('report: ' + $Report)
if ($script:fail -gt 0) { exit 1 }
exit 0
