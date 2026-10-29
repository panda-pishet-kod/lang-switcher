; Lang Switcher -- the messages of the installer that are its own, in Tamil (stage E108, group A).
;
; Inno Setup has an official Tamil translation (inno_official = true in res\lang\languages.toml,
; compiler:Languages\Tamil.isl), so this file carries only the messages of the installer that are
; Lang Switcher's own. It is read after compiler:Languages\Tamil.isl as the second MessagesFile of the
; "tamil" entry of [Languages] in ..\LangSwitcher.iss. English.isl is the sample; the glossary of the
; language is l10n\glossary\ta.tsv of the control contour. The name of the program is not translated.
; The name of a button inside a message is in the quotation marks of the language and is the word of
; the official file (ButtonNo = இல்); no & accelerators here, as in the official file (RULES 16).
; AuthorSlogan (stage E99, decision 167.1): the slogan under the signature of the author, light,
; without pathos.

[CustomMessages]
LocalCertReady=அமைவு இந்தக் கணினியில் ஒரு சான்றிதழை உருவாக்கி, Lang Switcher நிரலை அதைக் கொண்டு கையொப்பமிடும்; இதனால் நிர்வாகியாக இயக்கப்பட்ட நிரல்களின் சாளரங்களிலும் அது செயல்படும்.
LocalCertFailed=Lang Switcher நிறுவப்பட்டது, ஆனால் நிர்வாகியாக இயக்கப்பட்ட நிரல்களின் சாளரங்களில் அது செயல்படாது: இந்தக் கணினியில் உருவாக்கிய சான்றிதழை Windows ஏற்கவில்லை. விவரங்களுக்கு அமைவுப் பதிவைப் பார்க்கவும்.
RemoveSettingsQuestion=Lang Switcher அமைப்புகள் கோப்பையும் நீக்கலாமா?
RemoveSettingsKeep=அமைப்புகளை மீண்டும் நிறுவுவதற்காக வைத்திருக்க “இல்” என்பதைத் தேர்ந்தெடுக்கவும்.
AuthorSlogan=மனதார உருவாக்கிய நிரல்கள்
