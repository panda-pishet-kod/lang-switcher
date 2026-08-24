<#
    make-cert.ps1 -- create the personal code signing certificate of SPEC.md section 8.3,
    install its public part where Windows will trust it, and print the thumbprint the rest
    of tools\ signs with.

    THIS IS THE SCRIPT SECTION 8.3 NAMES, AND IT IS RUN ONCE.

        New-SelfSignedCertificate  (Code Signing, private key in Cert:\CurrentUser\My)
          -> export the PUBLIC part only, to a .cer
            -> install it into Cert:\LocalMachine\Root
              -> install it into Cert:\LocalMachine\TrustedPublisher
                -> print the thumbprint for tools\release.ps1

    Both installs need administrator rights. Nothing else here does, and signing later
    does not either: the private key stays in the USER store (TOOLCHAIN.md section 5).
    "Requires administrator rights once" in section 8.3 means exactly this script.

    WHY BOTH TRUST STORES. Root makes the self-issued certificate chain to a trusted root,
    so `signtool verify /pa` succeeds and the AppInfo service accepts the image. Section 8.2:
    a uiAccess="true" binary starts only when it is signed with a certificate the machine
    trusts AND lives under %ProgramFiles%. TrustedPublisher is what stops Windows asking the
    user whether to trust the publisher. One without the other is not enough.

    Cert:\CurrentUser\Root and Cert:\CurrentUser\TrustedPublisher will show the certificate
    afterwards without this script ever writing to them: those user stores are a MERGED VIEW
    of the machine stores of the same name. Measured on this machine -- the certificate sits
    under HKLM\SOFTWARE\Microsoft\SystemCertificates\{Root,TrustedPublisher}\Certificates and
    is absent from the HKCU ones, yet Get-ChildItem lists it in all four. Do not "fix" that by
    adding user copies: adding to a user Root store raises a Windows trust dialog, and the
    rollback of the setup stage knows about three stores, not five.

    WHY X509Store AND NOT Import-Certificate. Import-Certificate writes into an existing store
    and does not create one; against LocalMachine\TrustedPublisher on a machine where
    HKLM\SOFTWARE\Microsoft\SystemCertificates\TrustedPublisher does not exist yet it returns
    E_ACCESSDENIED even under an administrator. The X509Store API opened ReadWrite creates the
    store itself. TOOLCHAIN.md section 5; the same route is used by admin-setup.ps1.

    THE PRIVATE KEY NEVER TOUCHES THE DISK (SEC-01). The key is created NonExportable, only
    the public part is exported (-Type CERT, a DER .cer, no key material in it), and no .pfx
    is produced or producible. Losing it means issuing a new certificate, which is the correct
    trade for a personal-use key.

    DRY RUN. New-SelfSignedCertificate has no -WhatIf of its own, so this script provides one:
    every changing action goes through $PSCmdlet.ShouldProcess. With -WhatIf the whole plan is
    printed and nothing at all is done -- no certificate, no file, no store touched -- and the
    elevation check does not stop the run either, because a plan needs no rights.
    ConfirmImpact is High, so a real run asks before each change; add -Confirm:$false when
    that is not wanted.

    Written with a UTF-8 BYTE ORDER MARK, deliberately: Windows PowerShell 5.1 reads a .ps1
    without one in the system ANSI code page. Decision R-05. Kept free of Cyrillic anyway, so
    that it stays byte-identical in behaviour to release.ps1 and build-installer.ps1 whatever
    tool rewrites it next.
    Windows PowerShell 5.1, not 7.x: no `&&`, no `??`, no ternary operator.

    Exit codes -- non-zero on every refusal:
      0  the certificate was created and installed, or a valid one already existed
      1  administrator rights are missing and this is not a dry run
      2  the directory for the public .cer does not exist
      3  the private key store cannot be read
      4  New-SelfSignedCertificate failed, or produced a certificate that is not the one asked for
      5  the export failed, produced no file, or produced a file that is not the certificate
      6  installing into a machine store failed
      7  the final verification did not find the certificate where it was installed
     10  a confirmation prompt was answered no

    Examples:
      .\make-cert.ps1 -WhatIf                     # the plan, no rights needed, changes nothing
      .\make-cert.ps1                             # for real, from an elevated PowerShell
      .\make-cert.ps1 -Confirm:$false             # for real, without the per-step prompts
      .\make-cert.ps1 -Subject 'CN=Someone Else'
#>
[CmdletBinding(SupportsShouldProcess = $true, ConfirmImpact = 'High')]
param(
    # Subject and, since the certificate is self-issued, issuer. The default is the certificate
    # this project already signs with (TOOLCHAIN.md section 5); the script is not tied to it.
    [ValidateNotNullOrEmpty()]
    [string]$Subject = 'CN=Panda_Pishet_Kod',

    # Lifetime. Five years matches the existing certificate. NotBefore is left to the cmdlet,
    # which backdates it by ten minutes to survive clock skew.
    [ValidateRange(1, 30)]
    [int]$ValidYears = 5,

    # RSA key length. 3072 matches the existing certificate.
    [ValidateSet(2048, 3072, 4096)]
    [int]$KeyLength = 3072,

    # Signature hash. SHA-1 and MD5 are deliberately not offered.
    [ValidateSet('SHA256', 'SHA384', 'SHA512')]
    [string]$HashAlgorithm = 'SHA256',

    # Enhanced key usage: code signing, and nothing else.
    [ValidatePattern('^\d+(\.\d+)+$')]
    [string]$EnhancedKeyUsage = '1.3.6.1.5.5.7.3.3',

    # Key usage. Digital signature, and nothing else.
    [ValidateNotNullOrEmpty()]
    [string[]]$KeyUsage = @('DigitalSignature'),

    # Where the private key is created. THE USER STORE, not LocalMachine\My: that is what makes
    # `signtool sign /sha1 ...` work later without administrator rights.
    [ValidateNotNullOrEmpty()]
    [string]$PrivateKeyStore = 'Cert:\CurrentUser\My',

    # Where the public part is written. TOOLCHAIN.md section 5 keeps it here. The directory has
    # to exist already; this script creates one file in it and touches nothing else.
    [ValidateNotNullOrEmpty()]
    [string]$PublicCertPath = '<dev>\artifacts\LangSwitcher-CodeSigning.cer',

    # The LocalMachine stores the public part is installed into. Section 8.3 names these two.
    [ValidateNotNullOrEmpty()]
    [string[]]$MachineStore = @('Root', 'TrustedPublisher'),

    # Issue another certificate even though a valid one with the same subject already exists.
    # Without this the script reports the existing thumbprint and changes nothing.
    [switch]$Force
)

$ErrorActionPreference = 'Stop'

# True exactly when -WhatIf was given. Read once: every "would have" branch below keys off it,
# and the elevation check must not stop a run that is only printing a plan.
$DryRun = [bool]$WhatIfPreference

if (-not [System.IO.Path]::IsPathRooted($PublicCertPath)) {
    $PublicCertPath = Join-Path (Get-Location).ProviderPath $PublicCertPath
}
$NotAfter = (Get-Date).AddYears($ValidYears)

function Write-Section { param([string]$Text)
    Write-Host ''
    Write-Host ('--- ' + $Text + ' ' + ('-' * [Math]::Max(0, 66 - $Text.Length)))
}

function Add-CertificateToMachineStore {
    <#
        Public certificate into a LocalMachine store, through the API that creates the store
        when it is missing. See the header on Import-Certificate and E_ACCESSDENIED.
    #>
    param(
        [string]$StoreName,
        [System.Security.Cryptography.X509Certificates.X509Certificate2]$Certificate
    )
    $store = New-Object System.Security.Cryptography.X509Certificates.X509Store(
        $StoreName, [System.Security.Cryptography.X509Certificates.StoreLocation]::LocalMachine)
    try {
        $store.Open([System.Security.Cryptography.X509Certificates.OpenFlags]::ReadWrite)
        $store.Add($Certificate)
    } finally {
        $store.Close()
    }
}

Write-Host ''
Write-Host '======================================================================'
Write-Host ' Lang_Switcher -- personal code signing certificate, SPEC.md 8.3'
Write-Host '======================================================================'
Write-Host ("Subject      {0}  (self-issued)" -f $Subject)
Write-Host ("Key          RSA {0}, {1}, key usage {2}" -f $KeyLength, $HashAlgorithm, ($KeyUsage -join ', '))
Write-Host ("EKU          {0}  (code signing)" -f $EnhancedKeyUsage)
Write-Host ("Valid        {0} years, until {1:yyyy-MM-dd}" -f $ValidYears, $NotAfter)
Write-Host ("Private key  {0}, NonExportable -- no .pfx, ever" -f $PrivateKeyStore)
Write-Host ("Public file  {0}" -f $PublicCertPath)
Write-Host ("Trust stores {0}" -f ((($MachineStore | ForEach-Object { 'Cert:\LocalMachine\' + $_ })) -join ', '))
if ($DryRun) {
    Write-Host 'Mode         -WhatIf: the plan is printed below and NOTHING is done'
}
if ($Force) {
    Write-Host 'Force        -Force: an existing valid certificate will not stop the run'
}

# --- 1. Preflight -----------------------------------------------------------------------------
Write-Section 'Step 1: preflight'

$principal = New-Object Security.Principal.WindowsPrincipal(
    [Security.Principal.WindowsIdentity]::GetCurrent())
$isAdmin = $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
if ($isAdmin) {
    Write-Host '  administrator: yes'
} elseif ($DryRun) {
    Write-Host '  administrator: no -- does not matter, a plan changes nothing. Continuing.'
} else {
    Write-Host ''
    Write-Host 'FAIL: writing to Cert:\LocalMachine needs administrator rights.'
    Write-Host '      SPEC.md section 8.3: "requires administrator rights once" -- this is that once.'
    Write-Host '      Re-run from an elevated PowerShell, or add -WhatIf to see the plan without them.'
    exit 1
}

$publicDir = Split-Path -Parent $PublicCertPath
if (Test-Path -LiteralPath $publicDir) {
    Write-Host ("  public file directory: {0}" -f $publicDir)
} elseif ($DryRun) {
    Write-Host ("  public file directory {0} does NOT exist -- a real run would stop here." -f $publicDir)
} else {
    Write-Host ''
    Write-Host ("FAIL: the directory for the public certificate does not exist: {0}" -f $publicDir)
    Write-Host '      Create it, or pass -PublicCertPath pointing somewhere that exists.'
    exit 2
}

try {
    $existing = @(Get-ChildItem -Path $PrivateKeyStore |
        Where-Object { $_.Subject -eq $Subject -and $_.HasPrivateKey -and $_.NotAfter -gt (Get-Date) })
} catch {
    Write-Host ''
    Write-Host ("FAIL: cannot read {0}: {1}" -f $PrivateKeyStore, $_.Exception.Message)
    exit 3
}

if ($existing.Count -eq 0) {
    Write-Host ("  no valid certificate for {0} in {1} yet" -f $Subject, $PrivateKeyStore)
} else {
    Write-Host ("  {0} valid certificate(s) for {1} already in {2}:" -f $existing.Count, $Subject, $PrivateKeyStore)
    foreach ($c in $existing) {
        Write-Host ("    {0}  until {1:yyyy-MM-dd}" -f $c.Thumbprint, $c.NotAfter)
    }
    if ($Force) {
        Write-Host '  -Force was given: ANOTHER certificate will be issued next to it.'
    } elseif ($DryRun) {
        Write-Host '  a real run would stop right here and change nothing (-Force overrides).'
        Write-Host '  The plan below continues anyway, so that -WhatIf shows all of it.'
    } else {
        Write-Section 'Result'
        Write-Host '  Nothing was created, exported or installed: the certificate already exists.'
        Write-Host '  Section 8.3 is a once-only step, and this is what it produced:'
        Write-Host ''
        Write-Host ("    thumbprint  {0}" -f $existing[0].Thumbprint)
        Write-Host ''
        Write-Host '  Pass -Force to issue a second one anyway.'
        Write-Host '======================================================================'
        Write-Host ' RESULT: PASS, nothing to do'
        Write-Host '======================================================================'
        exit 0
    }
}

# --- 2. The certificate -------------------------------------------------------------------------
Write-Section 'Step 2: create the certificate'

$cert = $null
$createAction = ("Create a self-signed code signing certificate {0}, RSA {1}, {2}, valid until {3:yyyy-MM-dd}" -f
    $Subject, $KeyLength, $HashAlgorithm, $NotAfter)
if ($PSCmdlet.ShouldProcess($PrivateKeyStore, $createAction)) {
    try {
        $cert = New-SelfSignedCertificate `
            -Type Custom `
            -Subject $Subject `
            -CertStoreLocation $PrivateKeyStore `
            -KeyAlgorithm RSA `
            -KeyLength $KeyLength `
            -HashAlgorithm $HashAlgorithm `
            -KeyUsage $KeyUsage `
            -KeyExportPolicy NonExportable `
            -NotAfter $NotAfter `
            -TextExtension @('2.5.29.37={text}' + $EnhancedKeyUsage)
    } catch {
        Write-Host ''
        Write-Host ("FAIL: New-SelfSignedCertificate failed: {0}" -f $_.Exception.Message)
        exit 4
    }
    if (-not $cert) {
        Write-Host ''
        Write-Host 'FAIL: New-SelfSignedCertificate reported no error and returned nothing.'
        exit 4
    }
    # Asked for is not the same as got. Both properties below are the reason the certificate
    # exists at all, so neither is assumed.
    if (-not $cert.HasPrivateKey) {
        Write-Host ''
        Write-Host 'FAIL: the new certificate has no private key and cannot sign anything.'
        exit 4
    }
    $ekuExt = @($cert.Extensions | Where-Object { $_.Oid.Value -eq '2.5.29.37' })
    $ekuOk = $false
    if ($ekuExt.Count -gt 0) {
        $ekuOk = @($ekuExt[0].EnhancedKeyUsages | Where-Object { $_.Value -eq $EnhancedKeyUsage }).Count -gt 0
    }
    if (-not $ekuOk) {
        Write-Host ''
        Write-Host ("FAIL: the new certificate does not carry the enhanced key usage {0}." -f $EnhancedKeyUsage)
        Write-Host '      Without it signtool will not use it and Windows will not accept the signature.'
        exit 4
    }
    Write-Host ("  created in {0}, valid until {1:yyyy-MM-dd}, code signing, private key not exportable" -f
        $PrivateKeyStore, $cert.NotAfter)
} else {
    Write-Host '  (plan) New-SelfSignedCertificate is NOT run and no key is generated'
    if (-not $DryRun) {
        Write-Host ''
        Write-Host '  Answered no. Nothing was created; nothing else will be touched.'
        exit 10
    }
}

# --- 3. The public part -------------------------------------------------------------------------
# -Type CERT is a DER encoded certificate: the public key, the subject, the extensions and the
# signature. No private key material can be in it. SEC-01 forbids the alternative.
Write-Section 'Step 3: export the public part'

$publicOnly = $null
if ($PSCmdlet.ShouldProcess($PublicCertPath, 'Export the certificate WITHOUT its private key, DER .cer')) {
    try {
        Export-Certificate -Cert $cert -FilePath $PublicCertPath -Type CERT -Force | Out-Null
    } catch {
        Write-Host ''
        Write-Host ("FAIL: Export-Certificate failed: {0}" -f $_.Exception.Message)
        exit 5
    }
    if (-not (Test-Path -LiteralPath $PublicCertPath)) {
        Write-Host ''
        Write-Host ("FAIL: Export-Certificate reported no error but wrote no file: {0}" -f $PublicCertPath)
        exit 5
    }
    try {
        $publicOnly = New-Object System.Security.Cryptography.X509Certificates.X509Certificate2 `
            -ArgumentList $PublicCertPath
    } catch {
        Write-Host ''
        Write-Host ("FAIL: the exported file is not a readable certificate: {0}" -f $_.Exception.Message)
        exit 5
    }
    if ($publicOnly.Thumbprint -ne $cert.Thumbprint) {
        Write-Host ''
        Write-Host 'FAIL: the exported file is a different certificate than the one just created.'
        exit 5
    }
    if ($publicOnly.HasPrivateKey) {
        Write-Host ''
        Write-Host 'FAIL: the exported file carries a private key. It must not, and it is deleted now.'
        Remove-Item -LiteralPath $PublicCertPath -Force
        exit 5
    }
    Write-Host ("  written: {0}" -f $PublicCertPath)
    Write-Host '  public part only, no private key in the file'
} else {
    Write-Host '  (plan) nothing is written to disk'
    if (-not $DryRun) {
        Write-Host ''
        Write-Host '  Answered no. The certificate exists but no store will be touched.'
        exit 10
    }
}

# --- 4. Trust -----------------------------------------------------------------------------------
Write-Section 'Step 4: install the public part into the machine stores'

foreach ($storeName in $MachineStore) {
    $storePath = 'Cert:\LocalMachine\' + $storeName
    if ($PSCmdlet.ShouldProcess($storePath, 'Install the public certificate, no private key')) {
        if ($null -eq $publicOnly) {
            Write-Host ''
            Write-Host ("FAIL: there is no exported public certificate to install into {0}." -f $storePath)
            exit 6
        }
        try {
            Add-CertificateToMachineStore -StoreName $storeName -Certificate $publicOnly
        } catch {
            Write-Host ''
            Write-Host ("FAIL: cannot install into {0}: {1}" -f $storePath, $_.Exception.Message)
            Write-Host '      Administrator rights are required for this step, and only for this step.'
            exit 6
        }
        Write-Host ("  installed into {0}" -f $storePath)
    } else {
        Write-Host ("  (plan) {0} is NOT touched" -f $storePath)
    }
}

# --- 5. Verify ----------------------------------------------------------------------------------
Write-Section 'Step 5: verify'

if ($DryRun) {
    Write-Host '  (plan) nothing was changed, so there is nothing to verify'
} else {
    foreach ($storeName in $MachineStore) {
        $storePath = 'Cert:\LocalMachine\' + $storeName
        $found = @(Get-ChildItem -Path $storePath | Where-Object { $_.Thumbprint -eq $cert.Thumbprint })
        if ($found.Count -eq 0) {
            Write-Host ''
            Write-Host ("FAIL: the certificate is not in {0} after installing it there." -f $storePath)
            exit 7
        }
        if ($found[0].HasPrivateKey) {
            Write-Host ''
            Write-Host ("FAIL: {0} holds a private key for this certificate. Only the public part belongs there." -f $storePath)
            exit 7
        }
        Write-Host ("  {0}: present, public part only" -f $storePath)
    }
    Write-Host ''
    Write-Host '  Cert:\CurrentUser\Root and Cert:\CurrentUser\TrustedPublisher list it too. That is'
    Write-Host '  the merged view of the machine stores, not a second copy, and needs no second step.'
}

# --- 6. Summary ---------------------------------------------------------------------------------
Write-Section 'Result'

if ($DryRun) {
    Write-Host '  -WhatIf: no certificate was created, no file was written, no store was touched.'
    Write-Host '  Re-run from an elevated PowerShell without -WhatIf to carry the plan out.'
    Write-Host ''
    Write-Host '======================================================================'
    Write-Host ' RESULT: plan only, nothing done'
    Write-Host '======================================================================'
    exit 0
}

Write-Host ("  thumbprint (SHA-1)  {0}" -f $cert.Thumbprint)
Write-Host ("  public certificate  {0}" -f $PublicCertPath)
Write-Host ''
Write-Host '  This thumbprint is what the rest of tools\ signs with. Their -Thumbprint default is'
Write-Host '  the certificate this project has been using; a new one has to be passed explicitly'
Write-Host '  or made the new default (TOOLCHAIN.md section 5):'
Write-Host ''
Write-Host ("    .\release.ps1 -Thumbprint {0}" -f $cert.Thumbprint)
Write-Host ''
Write-Host '  Signing itself needs no administrator rights: the private key is in the user store.'
Write-Host ''
Write-Host '======================================================================'
Write-Host ' RESULT: PASS'
Write-Host '======================================================================'
exit 0
