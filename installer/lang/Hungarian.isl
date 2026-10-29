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

; Stage E109 (the stand of 1.4.0, FAIL 7): nineteen messages of the official Hungarian translation name a
; button or a file in straight apostrophes. They are given again here word for word, with the Hungarian
; quotation marks instead of the apostrophes and nothing else changed (RULES 16).
[Messages]
SetupAppRunningError=A telepítő úgy észlelte, hogy a(z) %1 jelenleg fut.%n%nZárja be az összes példányt, majd kattintson az „OK”-ra a folytatáshoz, vagy a „Mégse”-re a kilépéshez.
UninstallAppRunningError=Az eltávolító úgy észlelte, hogy a(z) %1 jelenleg fut.%n%nZárja be az összes példányt, majd kattintson az „OK”-ra a folytatáshoz, vagy a „Mégse”-re a kilépéshez.
ClickNext=A folytatáshoz kattintson a „Tovább”-ra, a kilépéshez a „Mégse”-re.
BrowseDialogLabel=Válasszon egy könyvtárat az alábbi listából, majd kattintson az „OK”-ra.
PasswordLabel3=Kérem adja meg a jelszót, majd kattintson a „Tovább”-ra. A jelszavak kis- és nagy betű érzékenyek.
InfoBeforeClickLabel=Ha készen áll, kattintson a „Tovább”-ra.
InfoAfterClickLabel=Ha készen áll, kattintson a „Tovább”-ra.
SelectDirBrowseLabel=A folytatáshoz, kattintson a „Tovább”-ra. Ha másik könyvtárat választana, kattintson a „Tallózás”-ra.
SelectComponentsLabel2=Jelölje ki a telepítendő összetevőket; törölje a telepíteni nem kívánt összetevőket. Kattintson a „Tovább”-ra, ha készen áll a folytatásra.
SelectTasksLabel2=Jelölje ki, mely kiegészítő feladatokat hajtsa végre a telepítő a(z) [name] telepítése során, majd kattintson a „Tovább”-ra.
SelectStartMenuFolderBrowseLabel=A folytatáshoz kattintson a „Tovább”-ra. Ha másik mappát választana, kattintson a „Tallózás”-ra.
ReadyLabel2a=Kattintson a „Telepítés”-re a folytatáshoz, vagy a „Vissza”-ra a beállítások áttekintéséhez vagy megváltoztatásához.
ReadyLabel2b=Kattintson a „Telepítés”-re a folytatáshoz.
CannotContinue=A telepítés nem folytatható. A kilépéshez kattintson a „Mégse”-re.
ClickFinish=Kattintson a „Befejezés”-re a kilépéshez.
ShowReadmeCheck=Igen, szeretném elolvasni a „FONTOS” fájlt
SelectDiskLabel2=Helyezze be a(z) %1. lemezt és kattintson az „OK”-ra.%n%nHa a fájlok a lemez egy a megjelenítettől különböző mappájában találhatók, írja be a helyes útvonalat vagy kattintson a „Tallózás”-ra.
ErrorOpeningReadme=Hiba lépett fel a „FONTOS” fájl megnyitása közben.
ConfirmDeleteSharedFile2=A rendszer azt jelzi, hogy a következő megosztott fájlra már nincs szüksége egyetlen programnak sem. Eltávolítja a megosztott fájlt?%n%nHa más programok még mindig használják a megosztott fájlt, akkor az eltávolítása után lehetséges, hogy az érintett programok helytelenül fognak működni. Ha bizonytalan, válassza a „Nem”-et. A fájl megtartása nem okoz problémát a rendszerben.
