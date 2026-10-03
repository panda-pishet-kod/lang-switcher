<#
    local-trust.ps1 -- the helper of the installer: makes the full image of Lang Switcher one that
    this machine trusts (stage E91, task T-91-2, question 155).

    WHY IT EXISTS. Windows starts an image whose manifest asks for uiAccess="true" only when the
    machine trusts its signature AND it lies in %ProgramFiles%; with a signature the machine does
    not trust it does not start at all (error 8235, "A referral was returned from the server",
    measured by the owner on a clean machine, decision 155v). The author's certificate is
    self-issued, so no machine but the author's trusts it. The owner chose (155.1, 155.5, 155.6)
    the way AutoHotkey works: the installer creates a certificate ON THIS MACHINE, signs the full
    image with it and destroys the private key -- without a page and without a question.

    WHAT IT DOES. The installer has already put the BASE image down as LangSwitcher.exe (it
    starts anywhere) and the FULL image beside it under a temporary name. This script decides
    whether the full one may replace the base one, and makes it so where it can:

      check      Get-AuthenticodeSignature of the full image. It must be signed by the author
                 (-AuthorThumbprint) and intact. Valid on this machine -> exit 0: the author's
                 signature is trusted here (the author's own machine, or a real certificate one
                 day), nothing is created.
      cleanup    a local certificate of an earlier installation is removed: the one named by
                 LangSwitcher-local.cer in the program folder and any certificate with the
                 local subject, from LocalMachine\Root and LocalMachine\My (with its key).
                 Done in every branch: the base image or the author's signature needs none.
      create     New-SelfSignedCertificate in LocalMachine\My: CN=Lang Switcher for Windows
                 (local), code signing only, digital signature only, no basic constraints (not
                 a certification authority), RSA 3072, SHA-256, 30 years, key not exportable.
      export     its public part, DER, to <program folder>\LangSwitcher-local.cer -- for the
                 uninstaller and for the curious; checked to carry no key.
      trust      the public part into LocalMachine\Root, and nowhere else.
      sign       Set-AuthenticodeSignature of the full image with it, SHA-256, no timestamp
                 (Setup does not use the network). The author's signature is replaced.
      delete-key the certificate and its private key leave LocalMachine\My (-DeleteKey); the
                 key container is checked to be gone.
      verify     Get-AuthenticodeSignature: Valid, signed by the new certificate -> exit 1.

    ANY FAILURE undoes everything this run created -- the certificate in My with its key, the
    certificate in Root, the .cer file -- and exits with the code of the step; the installer then
    keeps the base image. The exit codes are the contract with installer\LangSwitcher.iss:

       0  the author's signature is trusted here; the full image stays as it is
       1  a local certificate was created, the full image signed with it and verified
       2  -NoLocalCert (Setup /NOLOCALCERT): nothing created, the base image stays
      10  an unexpected error outside the steps
      11  check: the full image is not signed, or not intact (HashMismatch and the like)
      12  check: the full image is signed, but not by the author's certificate
      13  cleanup: an earlier local certificate could not be removed
      14  create: New-SelfSignedCertificate is missing or failed, or made another certificate
      15  export: the public part could not be written or read back
      16  trust: LocalMachine\Root refused it
      17  sign: Set-AuthenticodeSignature did not give a Valid signature
      18  delete-key: the private key or the certificate in My could not be removed
      19  verify: the signature of the full image is not Valid with the new certificate
      20  the undo after a failure left something behind -- named in the output

    THE FAILURE SEAM (task T-91-2): the environment variable LANGSW_TRUST_FAIL_AT naming a step
    (check, cleanup, create, export, trust, sign, delete-key, verify) makes that step fail as if
    Windows had refused it, so the undo can be measured on a clean machine
    (tools\accept-clean-machine.ps1). In anyone's hands it can do no more than leave the base image.

    THE OUTPUT goes to the setup log: Setup runs this script through ExecAndLogOutput, and every
    line printed here becomes a line of the log. English, ASCII, no secrets: a certificate has
    none, and the key never leaves the key store.

    Not copied from anywhere: written for this project. Windows PowerShell 5.1, ASCII only
    (decision R-05), run by Setup as:
      powershell.exe -NoProfile -NonInteractive -ExecutionPolicy Bypass -File local-trust.ps1
        -FullImage <program folder>\LangSwitcher-full.tmp -AppDir <program folder>
        -AuthorThumbprint <SHA-1> [-NoLocalCert]
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$FullImage,
    [Parameter(Mandatory = $true)][string]$AppDir,
    [Parameter(Mandatory = $true)][string]$AuthorThumbprint,
    [switch]$NoLocalCert
)

$ErrorActionPreference = 'Stop'
$LocalSubject = 'CN=Lang Switcher for Windows (local)'
$CodeSigning = '1.3.6.1.5.5.7.3.3'
$CerName = 'LangSwitcher-local.cer'
$CerPath = Join-Path $AppDir $CerName
$FailAt = [Environment]::GetEnvironmentVariable('LANGSW_TRUST_FAIL_AT')

# What this run created, so that a failure can undo exactly that and nothing else.
$script:created = @{ Thumbprint = ''; KeyName = ''; KeyProvider = ''; MachineKey = $true; InMy = $false; InRoot = $false; Cer = $false }

# Every line leaves as ASCII. Windows answers in the language of the system (the status message
# of a signature, the text of an exception), and how Setup decodes the output of a console
# program is not documented; so a character outside ASCII is written as \uXXXX, which no code
# page can garble and tools\accept-clean-machine.ps1 turns back into text in its report.
function Say([string]$Text) {
    $sb = New-Object System.Text.StringBuilder
    foreach ($ch in ('local-trust: ' + $Text).ToCharArray()) {
        $n = [int]$ch
        if (($n -ge 32 -and $n -lt 127) -or $n -eq 9) { [void]$sb.Append($ch) } else { [void]$sb.AppendFormat('\u{0:X4}', $n) }
    }
    Write-Output $sb.ToString()
}

function Enter-Step([string]$Name) {
    Say ('step ' + $Name)
    if ($FailAt -and ($FailAt -eq $Name)) {
        throw ('forced failure at step ' + $Name + ' (LANGSW_TRUST_FAIL_AT)')
    }
}

function Get-MachineStore([string]$Name, [string]$Mode) {
    $store = New-Object System.Security.Cryptography.X509Certificates.X509Store($Name, [System.Security.Cryptography.X509Certificates.StoreLocation]::LocalMachine)
    if ($Mode -eq 'ReadWrite') { $store.Open([System.Security.Cryptography.X509Certificates.OpenFlags]::ReadWrite) }
    else { $store.Open([System.Security.Cryptography.X509Certificates.OpenFlags]::ReadOnly) }
    return $store
}

# Thumbprints of one machine store, by thumbprint or by the local subject. Returned unrolled, and
# every caller wraps the call in @(): measured on 5.1, @() around a function that returns an
# array through the comma operator counts ONE element even when the array is empty.
function Find-InStore([string]$StoreName, [string]$Thumbprint) {
    $found = @()
    $store = Get-MachineStore $StoreName 'ReadOnly'
    try {
        foreach ($c in $store.Certificates) {
            if (($Thumbprint -and $c.Thumbprint -eq $Thumbprint) -or ((-not $Thumbprint) -and $c.Subject -eq $LocalSubject)) {
                $found += $c.Thumbprint
            }
        }
    } finally { $store.Close() }
    return $found
}

function Remove-FromRoot([string]$Thumbprint) {
    $store = Get-MachineStore 'Root' 'ReadWrite'
    try {
        foreach ($c in @($store.Certificates)) {
            if ($c.Thumbprint -eq $Thumbprint) { $store.Remove($c) }
        }
    } finally { $store.Close() }
}

# A certificate of LocalMachine\My together with its private key.
function Remove-FromMy([string]$Thumbprint) {
    $path = 'Cert:\LocalMachine\My\' + $Thumbprint
    if (Test-Path -LiteralPath $path) { Remove-Item -LiteralPath $path -DeleteKey }
}

function Test-KeyGone([string]$KeyName, [string]$Provider, [bool]$MachineKey) {
    if (-not $KeyName) { return $true }
    $options = [System.Security.Cryptography.CngKeyOpenOptions]::None
    if ($MachineKey) { $options = [System.Security.Cryptography.CngKeyOpenOptions]::MachineKey }
    $p = New-Object System.Security.Cryptography.CngProvider($Provider)
    return (-not [System.Security.Cryptography.CngKey]::Exists($KeyName, $p, $options))
}

# Undo what this run created. Returns $true when nothing of it is left.
function Undo-Created {
    $left = @()
    $t = $script:created.Thumbprint
    if ($t) {
        try { Remove-FromMy $t } catch { Say ('undo: removing from LocalMachine\My failed: ' + $_.Exception.Message) }
        try { Remove-FromRoot $t } catch { Say ('undo: removing from LocalMachine\Root failed: ' + $_.Exception.Message) }
        if (@(Find-InStore 'My' $t).Count -gt 0) { $left += 'LocalMachine\My' }
        if (@(Find-InStore 'Root' $t).Count -gt 0) { $left += 'LocalMachine\Root' }
        if (-not (Test-KeyGone $script:created.KeyName $script:created.KeyProvider $script:created.MachineKey)) { $left += 'the private key' }
    }
    if ($script:created.Cer -and (Test-Path -LiteralPath $CerPath)) {
        try { Remove-Item -LiteralPath $CerPath -Force } catch { Say ('undo: deleting ' + $CerPath + ' failed: ' + $_.Exception.Message) }
        if (Test-Path -LiteralPath $CerPath) { $left += $CerPath }
    }
    if ($left.Count -gt 0) {
        Say ('undo: LEFT BEHIND: ' + ($left -join ', '))
        return $false
    }
    if ($t) { Say ('undo: certificate ' + $t + ' removed from LocalMachine\My and LocalMachine\Root, key gone, file gone') }
    else { Say 'undo: nothing had been created' }
    return $true
}

function Stop-WithFailure([int]$Code, [string]$Step, [string]$Message) {
    Say ('FAILED at step ' + $Step + ': ' + $Message)
    $clean = Undo-Created
    if (-not $clean) {
        Say ('RESULT: exit 20 -- step ' + $Step + ' failed (code ' + $Code + ') and the undo left something behind')
        exit 20
    }
    Say ('RESULT: exit ' + $Code + ' -- step ' + $Step + ' failed; nothing of this run is left; the base image stays')
    exit $Code
}

Say ('Windows PowerShell {0}, 64-bit process {1}, {2}' -f $PSVersionTable.PSVersion, [Environment]::Is64BitProcess, $PSHOME)
Say ('Windows {0}, user {1}\{2}' -f [Environment]::OSVersion.Version, $env:USERDOMAIN, $env:USERNAME)
Say ('full image {0}; program folder {1}; author certificate {2}' -f $FullImage, $AppDir, $AuthorThumbprint)
if ($FailAt) { Say ('LANGSW_TRUST_FAIL_AT = ' + $FailAt + ' -- a failure will be forced at that step') }

try {
    # --- /NOLOCALCERT ---------------------------------------------------------------------------
    if ($NoLocalCert) {
        Say 'Setup was run with /NOLOCALCERT: no certificate will be created; the base image stays'
        $step = 'cleanup'
        try {
            Enter-Step $step
            $old = @(Find-InStore 'Root' '') + @(Find-InStore 'My' '')
            foreach ($t in ($old | Sort-Object -Unique)) { Remove-FromMy $t; Remove-FromRoot $t; Say ('removed the earlier local certificate ' + $t) }
            if (Test-Path -LiteralPath $CerPath) { Remove-Item -LiteralPath $CerPath -Force; Say ('deleted ' + $CerPath) }
        } catch { Stop-WithFailure 13 $step $_.Exception.Message }
        Say 'RESULT: exit 2 -- refused by /NOLOCALCERT'
        exit 2
    }

    # --- check ----------------------------------------------------------------------------------
    $step = 'check'
    $trustedHere = $false
    try {
        Enter-Step $step
        if (-not (Test-Path -LiteralPath $FullImage)) { throw ('the full image is missing: ' + $FullImage) }
        $sig = Get-AuthenticodeSignature -LiteralPath $FullImage
        $signer = ''
        if ($sig.SignerCertificate) { $signer = $sig.SignerCertificate.Thumbprint + ' ' + $sig.SignerCertificate.Subject }
        Say ('signature of the full image: Status {0}; signer {1}; timestamp {2}' -f $sig.Status, $signer, [bool]$sig.TimeStamperCertificate)
        Say ('  status message: ' + $sig.StatusMessage)
        $bad = @('NotSigned', 'HashMismatch', 'NotSupportedFileFormat', 'Incompatible')
        if (($bad -contains [string]$sig.Status) -or (-not $sig.SignerCertificate)) {
            Stop-WithFailure 11 $step ('the full image is not signed or not intact: ' + $sig.Status)
        }
        if ($sig.SignerCertificate.Thumbprint -ne $AuthorThumbprint) {
            Stop-WithFailure 12 $step ('the full image is signed by ' + $sig.SignerCertificate.Thumbprint + ', not by the author')
        }
        $trustedHere = ([string]$sig.Status -eq 'Valid')
    } catch { Stop-WithFailure 11 $step $_.Exception.Message }

    # --- cleanup ----------------------------------------------------------------------------------
    $step = 'cleanup'
    try {
        Enter-Step $step
        $old = @()
        if (Test-Path -LiteralPath $CerPath) {
            $prev = New-Object System.Security.Cryptography.X509Certificates.X509Certificate2 -ArgumentList $CerPath
            $old += $prev.Thumbprint
            Say ('earlier local certificate named by ' + $CerName + ': ' + $prev.Thumbprint)
        }
        $old += @(Find-InStore 'Root' '') + @(Find-InStore 'My' '')
        $old = @($old | Where-Object { $_ -and $_ -ne $AuthorThumbprint } | Sort-Object -Unique)
        foreach ($t in $old) {
            Remove-FromMy $t
            Remove-FromRoot $t
            if ((@(Find-InStore 'Root' $t).Count + @(Find-InStore 'My' $t).Count) -gt 0) { throw ('the earlier local certificate ' + $t + ' is still there') }
            Say ('removed the earlier local certificate ' + $t + ' from LocalMachine\Root and LocalMachine\My')
        }
        if (Test-Path -LiteralPath $CerPath) { Remove-Item -LiteralPath $CerPath -Force; Say ('deleted the earlier ' + $CerPath) }
        if ($old.Count -eq 0) { Say 'no earlier local certificate' }
    } catch { Stop-WithFailure 13 $step $_.Exception.Message }

    if ($trustedHere) {
        Say 'RESULT: exit 0 -- the author''s signature is trusted on this machine; the full image stays as it is; no certificate created'
        exit 0
    }
    Say 'the author''s signature is not trusted on this machine: a local certificate will be made'

    # --- create -----------------------------------------------------------------------------------
    $step = 'create'
    $cert = $null
    try {
        Enter-Step $step
        if (-not (Get-Command New-SelfSignedCertificate -ErrorAction SilentlyContinue)) { throw 'the cmdlet New-SelfSignedCertificate is not available' }
        $cert = New-SelfSignedCertificate -Type Custom -Subject $LocalSubject -FriendlyName 'Lang Switcher for Windows (local)' `
            -CertStoreLocation 'Cert:\LocalMachine\My' -KeyAlgorithm RSA -KeyLength 3072 -HashAlgorithm SHA256 `
            -KeyUsage DigitalSignature -KeyExportPolicy NonExportable -NotAfter (Get-Date).AddYears(30) `
            -TextExtension @('2.5.29.37={text}' + $CodeSigning)
        if (-not $cert) { throw 'New-SelfSignedCertificate returned nothing' }
        $script:created.Thumbprint = $cert.Thumbprint
        $script:created.InMy = $true
        $rsa = [System.Security.Cryptography.X509Certificates.RSACertificateExtensions]::GetRSAPrivateKey($cert)
        if ($rsa -is [System.Security.Cryptography.RSACng]) {
            $script:created.KeyName = $rsa.Key.UniqueName
            $script:created.KeyProvider = $rsa.Key.Provider.Provider
            $script:created.MachineKey = $rsa.Key.IsMachineKey
        }
        Say ('created {0} in LocalMachine\My, {1} .. {2}, key {3} ({4}, machine key {5})' -f $cert.Thumbprint, $cert.NotBefore.ToString('yyyy-MM-dd'), $cert.NotAfter.ToString('yyyy-MM-dd'), $script:created.KeyName, $script:created.KeyProvider, $script:created.MachineKey)
        if (-not $cert.HasPrivateKey) { throw 'the new certificate has no private key' }
        $eku = @($cert.Extensions | Where-Object { $_.Oid.Value -eq '2.5.29.37' })
        $ekuList = @()
        if ($eku.Count -gt 0) { $ekuList = @($eku[0].EnhancedKeyUsages | ForEach-Object { $_.Value }) }
        if (($ekuList.Count -ne 1) -or ($ekuList[0] -ne $CodeSigning)) { throw ('enhanced key usage is not code signing alone: ' + ($ekuList -join ', ')) }
        $ku = @($cert.Extensions | Where-Object { $_.Oid.Value -eq '2.5.29.15' })
        if (($ku.Count -ne 1) -or ([string]$ku[0].KeyUsages -ne 'DigitalSignature')) { throw 'key usage is not digital signature alone' }
        $bc = @($cert.Extensions | Where-Object { $_.Oid.Value -eq '2.5.29.19' })
        if (($bc.Count -gt 0) -and $bc[0].CertificateAuthority) { throw 'the certificate is a certification authority' }
        Say 'checked: code signing only, digital signature only, not a certification authority'
    } catch { Stop-WithFailure 14 $step $_.Exception.Message }

    # --- export ---------------------------------------------------------------------------------
    $step = 'export'
    try {
        Enter-Step $step
        $der = $cert.Export([System.Security.Cryptography.X509Certificates.X509ContentType]::Cert)
        [System.IO.File]::WriteAllBytes($CerPath, $der)
        $script:created.Cer = $true
        $back = New-Object System.Security.Cryptography.X509Certificates.X509Certificate2 -ArgumentList $CerPath
        if ($back.Thumbprint -ne $cert.Thumbprint) { throw 'the written file is another certificate' }
        if ($back.HasPrivateKey) { throw 'the written file carries a private key' }
        Say ('public part written: ' + $CerPath + ', ' + $der.Length + ' bytes, no key')
    } catch { Stop-WithFailure 15 $step $_.Exception.Message }

    # --- trust ----------------------------------------------------------------------------------
    $step = 'trust'
    try {
        Enter-Step $step
        $store = Get-MachineStore 'Root' 'ReadWrite'
        try { $store.Add($back) } finally { $store.Close() }
        $script:created.InRoot = $true
        if (@(Find-InStore 'Root' $cert.Thumbprint).Count -ne 1) { throw 'the certificate is not in LocalMachine\Root after adding it' }
        Say 'public part added to LocalMachine\Root'
    } catch { Stop-WithFailure 16 $step $_.Exception.Message }

    # --- sign -----------------------------------------------------------------------------------
    $step = 'sign'
    try {
        Enter-Step $step
        $signed = Set-AuthenticodeSignature -LiteralPath $FullImage -Certificate $cert -HashAlgorithm SHA256
        Say ('Set-AuthenticodeSignature: Status {0} -- {1}' -f $signed.Status, $signed.StatusMessage)
        if ([string]$signed.Status -ne 'Valid') { throw ('the signature is ' + $signed.Status) }
    } catch { Stop-WithFailure 17 $step $_.Exception.Message }

    # --- delete-key -----------------------------------------------------------------------------
    $step = 'delete-key'
    try {
        Enter-Step $step
        Remove-FromMy $cert.Thumbprint
        $script:created.InMy = $false
        if (@(Find-InStore 'My' $cert.Thumbprint).Count -gt 0) { throw 'the certificate is still in LocalMachine\My' }
        if (-not (Test-KeyGone $script:created.KeyName $script:created.KeyProvider $script:created.MachineKey)) { throw 'the private key still exists' }
        if ($script:created.KeyName) { Say ('private key destroyed: the key container ' + $script:created.KeyName + ' does not exist any more') }
        else { Say 'private key destroyed with the certificate (the key is not a CNG key: its container could not be checked)' }
    } catch { Stop-WithFailure 18 $step $_.Exception.Message }

    # --- verify ---------------------------------------------------------------------------------
    $step = 'verify'
    try {
        Enter-Step $step
        $final = Get-AuthenticodeSignature -LiteralPath $FullImage
        $finalSigner = ''
        if ($final.SignerCertificate) { $finalSigner = $final.SignerCertificate.Thumbprint }
        Say ('signature of the full image now: Status {0}; signer {1}' -f $final.Status, $finalSigner)
        if (([string]$final.Status -ne 'Valid') -or ($finalSigner -ne $cert.Thumbprint)) { throw ('the full image is not Valid with the new certificate: ' + $final.Status) }
    } catch { Stop-WithFailure 19 $step $_.Exception.Message }

    Say ('RESULT: exit 1 -- local certificate {0} created, the full image signed with it and Valid, the key destroyed' -f $cert.Thumbprint)
    exit 1
} catch {
    Say ('unexpected error: ' + $_.Exception.Message)
    $null = Undo-Created
    Say 'RESULT: exit 10 -- unexpected error; the base image stays'
    exit 10
}
