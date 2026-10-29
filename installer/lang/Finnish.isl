; Lang Switcher -- the messages of the installer that are its own, in Finnish (stage E109, group B).
;
; Inno Setup has an official Finnish translation (inno_official = true in res\lang\languages.toml,
; compiler:Languages\Finnish.isl), so this file carries only the messages of the installer that are
; Lang Switcher's own. It is read after compiler:Languages\Finnish.isl as the second MessagesFile of
; the "finnish" entry of [Languages] in ..\LangSwitcher.iss. English.isl is the sample; the glossary of
; the language is l10n\glossary\fi.tsv of the control contour. The name of the program is not
; translated and is not inflected (a word with the case ending follows it: "Lang Switcher -ohjelman").
; The name of a button inside a message is in the quotation marks of the language and is the word of
; the official file (ButtonNo = Ei); the reader is addressed in the singular, as in that file.
; AuthorSlogan (stage E99, decision 167.1): the slogan under the signature of the author, light,
; without pathos.

[CustomMessages]
LocalCertReady=Asennus luo tähän tietokoneeseen varmenteen ja allekirjoittaa sillä Lang Switcher -ohjelman, jotta se toimii myös järjestelmänvalvojana käynnistettyjen ohjelmien ikkunoissa.
LocalCertFailed=Lang Switcher asennettiin, mutta se ei toimi järjestelmänvalvojana käynnistettyjen ohjelmien ikkunoissa: Windows ei hyväksynyt tähän tietokoneeseen luotua varmennetta. Lisätietoja on asennuslokissa.
RemoveSettingsQuestion=Poistetaanko myös Lang Switcher -ohjelman asetustiedosto?
RemoveSettingsKeep=Valitse ”Ei”, jos haluat säilyttää asetukset myöhempää uudelleenasennusta varten.
AuthorSlogan=sydämellä tehdyt ohjelmat
