; Lang Switcher -- the messages of the installer that are its own, in Dutch (stage E109, group B).
;
; Inno Setup has an official Dutch translation (inno_official = true in res\lang\languages.toml,
; compiler:Languages\Dutch.isl), so this file carries only the messages of the installer that are
; Lang Switcher's own. It is read after compiler:Languages\Dutch.isl as the second MessagesFile of the
; "dutch" entry of [Languages] in ..\LangSwitcher.iss. English.isl is the sample; the glossary of the
; language is l10n\glossary\nl.tsv of the control contour. The name of the program is not translated.
; The name of a button inside a message is in the quotation marks of the language and is the word of
; the official file (ButtonNo = Nee); no & accelerators here, as in the official file (RULES 16).
; The installer calls itself "Setup", as the official file does; no pronoun of address is used, so the
; messages sit beside the "u" of the official file without a second form of address.
; AuthorSlogan (stage E99, decision 167.1): the slogan under the signature of the author, light,
; without pathos.

[CustomMessages]
LocalCertReady=Setup maakt op deze computer een certificaat aan en ondertekent Lang Switcher daarmee, zodat het programma ook werkt in vensters van programma's die als beheerder worden uitgevoerd.
LocalCertFailed=Lang Switcher is geïnstalleerd, maar werkt niet in vensters van programma's die als beheerder worden uitgevoerd: Windows heeft het certificaat dat op deze computer is aangemaakt niet geaccepteerd. Zie het installatielogboek voor details.
RemoveSettingsQuestion=Ook het instellingenbestand van Lang Switcher verwijderen?
RemoveSettingsKeep=Kies “Nee” om de instellingen te bewaren voor een latere herinstallatie.
AuthorSlogan=programma's gemaakt met hart
