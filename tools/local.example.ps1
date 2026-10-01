<#
    local.example.ps1 -- the form of tools\local.ps1, this machine's own folders for the delivery.

    COPY THIS FILE TO tools\local.ps1, UNCOMMENT THE LINES YOU NEED AND PUT YOUR PATHS IN.
    tools\local.ps1 is not tracked (.gitignore names it): it holds the folders of one machine,
    and those are exactly what the tracked tree must not carry (stage E89, decision 152.3).
    Without the file every script below falls back to the default named for its variable.

    WHO READS IT. Every script of tools\ that reads one of these variables loads the file first:

        if (Test-Path "$PSScriptRoot\local.ps1") { . "$PSScriptRoot\local.ps1" }

    A test run that needs the variables -- the delivery's own run script -- loads it the same way
    before cargo test, so the tests see the same values the scripts do.

    Each line fills its variable only when it is empty, so a value a caller has set itself -- an
    acceptance run, a gate script -- wins over this file.

    THE VARIABLES, and what happens without each:

      LANGSW_ARTIFACTS  The artifact folder: the signed LangSwitcher.exe (release.ps1), the
                        installer LangSwitcher-setup.exe (build-installer.ps1) and the public
                        .cer of the signing certificate (make-cert.ps1).
                        Without it: <repository>\dist\, created when needed.

      LANGSW_ISCC       The full path of ISCC.exe, the Inno Setup 6 compiler.
                        Without it: ISCC.exe on PATH, then Inno Setup 6 under Program Files (x86)
                        and Program Files; found nowhere, build-installer.ps1 refuses.

      LANGSW_SIGNTOOL   The full path of signtool.exe, the Windows SDK signing tool.
                        Without it: signtool.exe on PATH, then the newest Windows Kits 10 x64 one;
                        found nowhere, release.ps1 and build-installer.ps1 refuse to sign
                        (with -SkipSign they go on without it).

      LANGSW_NEWS_KEYS  The folder of the encrypted signing keys of the author's feed:
                        make-news-key.ps1 writes them, sign-news.ps1 -Reserve reads one.
                        Without it: both refuse -- keys have no default place.

      LANGSW_NEWS_SITE  The full path of the feed file about to be published. The test
                        the_file_that_will_be_published_verifies_with_the_shipped_keys of
                        tests\feed.rs reads it.
                        Without it: the test prints SKIPPED_NO_NEWS_SITE and passes.

      LANGSW_CONTROL    The project's control folder. The bench tests\e2e writes the protocol of
                        position 2 and its raw series into its reports\ subfolder.
                        Without it: <target folder>\e2e-reports\.

    cargo is not configured here: it comes from PATH, and CARGO_HOME, RUSTUP_HOME and
    CARGO_TARGET_DIR keep cargo's own defaults unless they are set -- in the environment, or here
    in the same form as the lines below.

    ASCII only, Windows PowerShell 5.1: the scripts that load this file are 5.1 scripts, and 5.1
    reads a file without a byte order mark in the system ANSI code page (decision R-05).
#>

# if (-not $env:LANGSW_ARTIFACTS) { $env:LANGSW_ARTIFACTS = 'C:\Builds\LangSwitcher\artifacts' }
# if (-not $env:LANGSW_ISCC)      { $env:LANGSW_ISCC      = 'C:\Program Files (x86)\Inno Setup 6\ISCC.exe' }
# if (-not $env:LANGSW_SIGNTOOL)  { $env:LANGSW_SIGNTOOL  = 'C:\Program Files (x86)\Windows Kits\10\bin\10.0.26100.0\x64\signtool.exe' }
# if (-not $env:LANGSW_NEWS_KEYS) { $env:LANGSW_NEWS_KEYS = 'C:\Keys\LangSwitcher\news-keys' }
# if (-not $env:LANGSW_NEWS_SITE) { $env:LANGSW_NEWS_SITE = 'C:\Builds\LangSwitcher\news-site\news.toml' }
# if (-not $env:LANGSW_CONTROL)   { $env:LANGSW_CONTROL   = 'C:\Projects\LangSwitcher-control' }
