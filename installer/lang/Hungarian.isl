; Lang Switcher -- the messages of the installer that are its own, in Hungarian (stage E109, group B).
;
; Inno Setup has an official Hungarian translation (inno_official = true in res\lang\languages.toml,
; compiler:Languages\Hungarian.isl), so this file carries only the messages of the installer that are
; Lang Switcher's own. It is read after compiler:Languages\Hungarian.isl as the second MessagesFile of
; the "hungarian" entry of [Languages] in ..\LangSwitcher.iss. English.isl is the sample; the glossary of
; the language is l10n\glossary\hu.tsv of the control contour. The name of the program is not translated.
; The name of a button inside a message is in the quotation marks of the language and is the word of
; the official file (ButtonNo = Nem), without its & accelerator (RULES 16).
; AuthorSlogan (stage E99, decision 167.1): the slogan under the signature of the author, light,
; without pathos.

[CustomMessages]
LocalCertReady=A telepítő létrehoz egy tanúsítványt ezen a számítógépen, és ezzel írja alá a Lang Switcher programot, hogy az a rendszergazdaként futtatott programok ablakaiban is működjön.
LocalCertFailed=A Lang Switcher telepítve lett, de nem fog működni a rendszergazdaként futtatott programok ablakaiban: a Windows nem fogadta el az ezen a számítógépen létrehozott tanúsítványt. A részleteket a telepítési naplóban találja.
RemoveSettingsQuestion=Eltávolítja a Lang Switcher beállításfájlját is?
RemoveSettingsKeep=Válassza a „Nem” lehetőséget, ha meg szeretné őrizni a beállításokat egy későbbi újratelepítéshez.
AuthorSlogan=szívvel készült programok
