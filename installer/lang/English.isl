; Lang Switcher -- the messages of the installer that are its own, in English (stage E91).
;
; Read after the official compiler:Default.isl as the second MessagesFile of the "english"
; entry of [Languages] in ..\LangSwitcher.iss. English is the first language of that section,
; and so the one Setup uses when the language of Windows matches none of the others.
; This file is the sample the other files of this folder translate, name for name; the
; completeness sweep, tests\installer_languages.rs, holds them to it.
; The name of the program is not translated.

[CustomMessages]
LocalCertReady=Setup will create a certificate on this computer and sign Lang Switcher with it, so that it can work in windows of programs run as administrator.
LocalCertFailed=Lang Switcher was installed, but it will not work in windows of programs run as administrator: Windows did not accept the certificate created on this computer. See the setup log for details.
RemoveSettingsQuestion=Remove the Lang Switcher settings file as well?
RemoveSettingsKeep=Choose No to keep your settings for a later reinstall.
