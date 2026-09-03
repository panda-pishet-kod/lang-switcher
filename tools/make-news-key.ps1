<#
    make-news-key.ps1 -- the two signing keys of the author's feed (FR-102, task T-32-6).

    WHAT IT MAKES. Two ECDSA P-256 key pairs:

      working  -- signs news.toml every time
      reserve  -- signs nothing until the working key is lost or stolen

    Both public halves are compiled into the program (letters::feed::FEED_KEYS) and a signature
    made by EITHER is accepted. That is the whole recovery plan: losing the working key does not
    silence the feed, and a stolen working key can be answered with a letter signed by the
    reserve one telling people to update.

    WHERE THE PRIVATE HALVES GO. Encrypted PKCS#8 files in <dev>\tools\admin\news-keys\,
    with one password in a file beside them. The password file is named in capitals because it
    is an instruction: MOVE IT OFF THIS MACHINE. A password kept next to the thing it encrypts
    protects nothing.

    ⚠ THE RESERVE KEY DOES NOT LIVE ON THIS MACHINE. It is written to its encrypted file and
    nothing else keeps it: no CNG container, no export, no copy. The working key is kept in the
    Windows key store as well, because sign-news.ps1 uses it on every publication.

    ⚠ IDEMPOTENT IT IS NOT. Every run makes NEW keys, and new keys mean every installed copy of
    the program stops trusting the feed until it is updated. The script refuses to overwrite an
    existing key folder unless -Force is given, and says why.

    OUTPUT. The two public halves are printed as a ready Rust constant for src\letters.rs, in
    the BCRYPT_ECCPUBLIC_BLOB order -- X then Y, thirty-two bytes each.

    Usage:
      pwsh -File tools\make-news-key.ps1
      pwsh -File tools\make-news-key.ps1 -Force        # replace existing keys, see the warning
#>

[CmdletBinding()]
param(
    [string]$KeyDir = '<dev>\tools\admin\news-keys',
    [string]$WorkingContainer = 'LangSwitcherNewsWorking',
    [switch]$Force
)

$ErrorActionPreference = 'Stop'

function Write-Section([string]$title) {
    Write-Host ''
    Write-Host ("--- {0} {1}" -f $title, ('-' * [Math]::Max(0, 66 - $title.Length)))
}

Write-Host '======================================================================'
Write-Host ' Lang_Switcher -- the two signing keys of the author feed (FR-102)'
Write-Host '======================================================================'
Write-Host ("Key folder  {0}" -f $KeyDir)

# --- 1. Refuse to destroy keys that are already in use ----------------------------------------
$existing = Join-Path $KeyDir 'working.p8'

if ((Test-Path $existing) -and (-not $Force)) {
    Write-Host ''
    Write-Host '  FAIL: keys already exist in that folder.'
    Write-Host '  New keys make every INSTALLED copy of the program stop trusting the feed'
    Write-Host '  until it is updated with the new public halves. If that is really what you'
    Write-Host '  want, run again with -Force.'
    exit 1
}

New-Item -ItemType Directory -Force -Path $KeyDir | Out-Null

# --- 2. The password --------------------------------------------------------------------------
# Thirty-two bytes of the system CSPRNG, base64 -- long enough that the PBKDF2 iteration count
# below is not what the strength rests on.
$raw = New-Object byte[] 32
[System.Security.Cryptography.RandomNumberGenerator]::Fill($raw)
$password = [Convert]::ToBase64String($raw)

# --- 3. The two key pairs ---------------------------------------------------------------------
Write-Section 'Step 1: two ECDSA P-256 key pairs'

$curve = [System.Security.Cryptography.ECCurve]::CreateFromFriendlyName('nistP256')

$pbe = New-Object System.Security.Cryptography.PbeParameters(
    [System.Security.Cryptography.PbeEncryptionAlgorithm]::Aes256Cbc,
    [System.Security.Cryptography.HashAlgorithmName]::SHA256,
    600000)

# ⚠ **The working key is BORN in the Windows key store, and that order is the whole trick.**
# .NET can import a PKCS#8 blob only into an EPHEMERAL key -- `CngKey.Import` takes no container
# name -- so a key generated first and named afterwards cannot be had: measured, the store
# answers «the requested operation is not supported». Creating it in the container first and
# exporting the private half to the backup file afterwards gets both: `sign-news.ps1` signs
# without a password every day, and the encrypted copy exists for the day the machine does not.
$creation = New-Object System.Security.Cryptography.CngKeyCreationParameters
$creation.ExportPolicy = [System.Security.Cryptography.CngExportPolicies]::AllowPlaintextExport
$creation.KeyUsage = [System.Security.Cryptography.CngKeyUsages]::Signing
$creation.Provider = [System.Security.Cryptography.CngProvider]::MicrosoftSoftwareKeyStorageProvider
$creation.KeyCreationOptions = [System.Security.Cryptography.CngKeyCreationOptions]::None

if ([System.Security.Cryptography.CngKey]::Exists($WorkingContainer)) {
    [System.Security.Cryptography.CngKey]::Open($WorkingContainer).Delete()
    Write-Host '  the previous container was removed'
}

$container = [System.Security.Cryptography.CngKey]::Create(
    [System.Security.Cryptography.CngAlgorithm]::ECDsaP256, $WorkingContainer, $creation)

Write-Host ("  container {0}: created, the working key lives there" -f $WorkingContainer)

$made = @()

foreach ($role in @('working', 'reserve')) {
    $key = if ($role -eq 'working') {
        New-Object System.Security.Cryptography.ECDsaCng($container)
    } else {
        # The reserve key is ephemeral on purpose: it must NOT live on this machine.
        [System.Security.Cryptography.ECDsa]::Create($curve)
    }

    $parameters = $key.ExportParameters($false)

    # BCRYPT_ECCPUBLIC_BLOB is the magic, the key size and then X and Y. The program carries the
    # last two only -- sixty-four bytes -- and puts the eight-byte header on at import time,
    # because the header is a constant of the format and not of the key.
    $x = $parameters.Q.X
    $y = $parameters.Q.Y

    if ($x.Length -ne 32 -or $y.Length -ne 32) {
        throw ("P-256 must give 32 + 32 bytes, got {0} + {1}" -f $x.Length, $y.Length)
    }

    $file = Join-Path $KeyDir ("{0}.p8" -f $role)
    [System.IO.File]::WriteAllBytes($file, $key.ExportEncryptedPkcs8PrivateKey($password, $pbe))

    Write-Host ("  {0,-8} public X||Y 64 bytes, private -> {1}" -f $role, $file)

    $made += [pscustomobject]@{ Role = $role; X = $x; Y = $y; Key = $key }
}

# --- 5. The password file ----------------------------------------------------------------------
Write-Section 'Step 2: the password'

$passwordFile = Join-Path $KeyDir 'PAROL-PERENESTI-VNE-KOMPUTERA.txt'
$lines = @(
    'Lang_Switcher -- the password of the two feed signing keys (FR-102).',
    '',
    'MOVE THIS FILE OFF THIS MACHINE. Two places, not one:',
    '  1. a password manager,',
    '  2. a USB stick kept somewhere else.',
    '',
    'Then delete it from here. A password kept next to the files it encrypts protects',
    'nothing at all.',
    '',
    'It opens working.p8 and reserve.p8 (encrypted PKCS#8, AES-256-CBC, PBKDF2-SHA256).',
    '',
    $password
)
Set-Content -Path $passwordFile -Value $lines -Encoding UTF8
Write-Host ("  written to {0}" -f $passwordFile)

# --- 6. The public halves, as the Rust constant --------------------------------------------------
Write-Section 'Step 3: the public halves, ready for src\letters.rs'

function Format-Bytes($x, $y) {
    $all = @($x) + @($y)
    $rows = @()
    for ($i = 0; $i -lt $all.Length; $i += 8) {
        $slice = $all[$i..([Math]::Min($i + 7, $all.Length - 1))]
        $rows += '        ' + (($slice | ForEach-Object { '0x{0:X2},' -f $_ }) -join ' ')
    }
    $rows -join "`n"
}

Write-Host ''
Write-Host 'pub const FEED_KEYS: [[u8; 64]; 2] = ['
Write-Host '    // The working key.'
Write-Host '    ['
Write-Host (Format-Bytes $made[0].X $made[0].Y)
Write-Host '    ],'
Write-Host '    // The reserve key -- it does not live on this machine.'
Write-Host '    ['
Write-Host (Format-Bytes $made[1].X $made[1].Y)
Write-Host '    ],'
Write-Host '];'
Write-Host ''

foreach ($entry in $made) { $entry.Key.Dispose() }
$container.Dispose()

Write-Host '======================================================================'
Write-Host ' RESULT: PASS -- two keys made'
Write-Host ' NEXT: move the password file off this machine, then delete it from here.'
Write-Host '======================================================================'
