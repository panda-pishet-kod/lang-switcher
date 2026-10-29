; Lang Switcher -- the messages of the installer in Punjabi (stage E108, group A).
;
; Inno Setup has no official Punjabi translation (inno_official = false in res\lang\languages.toml),
; so this file carries the whole of it: [LangOptions], every message of the compiler:Default.isl
; (6.5.0+), and the messages of the installer that are Lang Switcher's own. It is read after
; compiler:Default.isl as the second MessagesFile of the "punjabi" entry of [Languages] in
; ..\LangSwitcher.iss, and so replaces the English text name for name.
; English.isl is the sample the own messages translate; the glossary of the language is
; l10n\glossary\pa.tsv of the control contour. The name of the program is not translated.
; As in Default.isl: a message that has no full stop there has no danda here (Setup adds its own).
; No accelerator letters (&) -- a language without an official file has none (RULES 16); the names
; of buttons inside a message are in the quotation marks of the language.

[LangOptions]
LanguageName=ਪੰਜਾਬੀ
LanguageID=$0446
LanguageCodePage=0

[Messages]

; *** Application titles
SetupAppTitle=ਸੈੱਟਅੱਪ
SetupWindowTitle=ਸੈੱਟਅੱਪ - %1
UninstallAppTitle=ਅਣਇੰਸਟਾਲ
UninstallAppFullTitle=%1 ਅਣਇੰਸਟਾਲ

; *** Misc. common
InformationTitle=ਜਾਣਕਾਰੀ
ConfirmTitle=ਪੁਸ਼ਟੀ
ErrorTitle=ਗਲਤੀ

; *** SetupLdr messages
SetupLdrStartupMessage=ਇਹ %1 ਨੂੰ ਇੰਸਟਾਲ ਕਰੇਗਾ। ਕੀ ਜਾਰੀ ਰੱਖਿਆ ਜਾਵੇ?
LdrCannotCreateTemp=ਆਰਜ਼ੀ ਫ਼ਾਈਲ ਨਹੀਂ ਬਣਾਈ ਜਾ ਸਕੀ। ਸੈੱਟਅੱਪ ਰੱਦ ਕੀਤਾ ਗਿਆ
LdrCannotExecTemp=ਆਰਜ਼ੀ ਫ਼ੋਲਡਰ ਵਿੱਚ ਫ਼ਾਈਲ ਨਹੀਂ ਚਲਾਈ ਜਾ ਸਕੀ। ਸੈੱਟਅੱਪ ਰੱਦ ਕੀਤਾ ਗਿਆ
HelpTextNote=

; *** Startup error messages
LastErrorMessage=%1.%n%nਗਲਤੀ %2: %3
SetupFileMissing=ਇੰਸਟਾਲੇਸ਼ਨ ਫ਼ੋਲਡਰ ਵਿੱਚ ਫ਼ਾਈਲ %1 ਮੌਜੂਦ ਨਹੀਂ ਹੈ। ਕਿਰਪਾ ਕਰਕੇ ਸਮੱਸਿਆ ਠੀਕ ਕਰੋ ਜਾਂ ਪ੍ਰੋਗਰਾਮ ਦੀ ਨਵੀਂ ਕਾਪੀ ਪ੍ਰਾਪਤ ਕਰੋ।
SetupFileCorrupt=ਸੈੱਟਅੱਪ ਫ਼ਾਈਲਾਂ ਖ਼ਰਾਬ ਹਨ। ਕਿਰਪਾ ਕਰਕੇ ਪ੍ਰੋਗਰਾਮ ਦੀ ਨਵੀਂ ਕਾਪੀ ਪ੍ਰਾਪਤ ਕਰੋ।
SetupFileCorruptOrWrongVer=ਸੈੱਟਅੱਪ ਫ਼ਾਈਲਾਂ ਖ਼ਰਾਬ ਹਨ, ਜਾਂ ਸੈੱਟਅੱਪ ਦੇ ਇਸ ਵਰਜਨ ਨਾਲ ਮੇਲ ਨਹੀਂ ਖਾਂਦੀਆਂ। ਕਿਰਪਾ ਕਰਕੇ ਸਮੱਸਿਆ ਠੀਕ ਕਰੋ ਜਾਂ ਪ੍ਰੋਗਰਾਮ ਦੀ ਨਵੀਂ ਕਾਪੀ ਪ੍ਰਾਪਤ ਕਰੋ।
InvalidParameter=ਕਮਾਂਡ ਲਾਈਨ ਵਿੱਚ ਗਲਤ ਪੈਰਾਮੀਟਰ ਦਿੱਤਾ ਗਿਆ:%n%n%1
SetupAlreadyRunning=ਸੈੱਟਅੱਪ ਪਹਿਲਾਂ ਹੀ ਚੱਲ ਰਿਹਾ ਹੈ।
WindowsVersionNotSupported=ਇਹ ਪ੍ਰੋਗਰਾਮ ਤੁਹਾਡੇ ਕੰਪਿਊਟਰ ਉੱਤੇ ਚੱਲ ਰਹੇ Windows ਦੇ ਵਰਜਨ ਨੂੰ ਸਪੋਰਟ ਨਹੀਂ ਕਰਦਾ।
WindowsServicePackRequired=ਇਸ ਪ੍ਰੋਗਰਾਮ ਲਈ %1 ਸਰਵਿਸ ਪੈਕ %2 ਜਾਂ ਨਵਾਂ ਚਾਹੀਦਾ ਹੈ।
NotOnThisPlatform=ਇਹ ਪ੍ਰੋਗਰਾਮ %1 ਉੱਤੇ ਨਹੀਂ ਚੱਲੇਗਾ।
OnlyOnThisPlatform=ਇਹ ਪ੍ਰੋਗਰਾਮ %1 ਉੱਤੇ ਹੀ ਚਲਾਇਆ ਜਾਣਾ ਚਾਹੀਦਾ ਹੈ।
OnlyOnTheseArchitectures=ਇਹ ਪ੍ਰੋਗਰਾਮ ਸਿਰਫ਼ ਹੇਠ ਲਿਖੇ ਪ੍ਰੋਸੈਸਰ ਆਰਕੀਟੈਕਚਰਾਂ ਲਈ ਬਣੇ Windows ਦੇ ਵਰਜਨਾਂ ਉੱਤੇ ਇੰਸਟਾਲ ਹੋ ਸਕਦਾ ਹੈ:%n%n%1
WinVersionTooLowError=ਇਸ ਪ੍ਰੋਗਰਾਮ ਲਈ %1 ਵਰਜਨ %2 ਜਾਂ ਨਵਾਂ ਚਾਹੀਦਾ ਹੈ।
WinVersionTooHighError=ਇਹ ਪ੍ਰੋਗਰਾਮ %1 ਵਰਜਨ %2 ਜਾਂ ਉਸ ਤੋਂ ਨਵੇਂ ਉੱਤੇ ਇੰਸਟਾਲ ਨਹੀਂ ਹੋ ਸਕਦਾ।
AdminPrivilegesRequired=ਇਸ ਪ੍ਰੋਗਰਾਮ ਨੂੰ ਇੰਸਟਾਲ ਕਰਨ ਵੇਲੇ ਪ੍ਰਸ਼ਾਸਕ ਵਜੋਂ ਸਾਈਨ ਇਨ ਹੋਣਾ ਲਾਜ਼ਮੀ ਹੈ।
PowerUserPrivilegesRequired=ਇਸ ਪ੍ਰੋਗਰਾਮ ਨੂੰ ਇੰਸਟਾਲ ਕਰਨ ਵੇਲੇ ਪ੍ਰਸ਼ਾਸਕ ਵਜੋਂ ਜਾਂ Power Users ਸਮੂਹ ਦੇ ਮੈਂਬਰ ਵਜੋਂ ਸਾਈਨ ਇਨ ਹੋਣਾ ਲਾਜ਼ਮੀ ਹੈ।
SetupAppRunningError=ਸੈੱਟਅੱਪ ਨੂੰ ਪਤਾ ਲੱਗਾ ਹੈ ਕਿ %1 ਇਸ ਵੇਲੇ ਚੱਲ ਰਿਹਾ ਹੈ।%n%nਕਿਰਪਾ ਕਰਕੇ ਇਸ ਦੀਆਂ ਸਾਰੀਆਂ ਚੱਲ ਰਹੀਆਂ ਵਿੰਡੋਆਂ ਹੁਣ ਬੰਦ ਕਰੋ, ਫਿਰ ਜਾਰੀ ਰੱਖਣ ਲਈ “ਠੀਕ ਹੈ” ਜਾਂ ਬਾਹਰ ਜਾਣ ਲਈ “ਰੱਦ ਕਰੋ” ਦਬਾਓ।
UninstallAppRunningError=ਅਣਇੰਸਟਾਲ ਨੂੰ ਪਤਾ ਲੱਗਾ ਹੈ ਕਿ %1 ਇਸ ਵੇਲੇ ਚੱਲ ਰਿਹਾ ਹੈ।%n%nਕਿਰਪਾ ਕਰਕੇ ਇਸ ਦੀਆਂ ਸਾਰੀਆਂ ਚੱਲ ਰਹੀਆਂ ਵਿੰਡੋਆਂ ਹੁਣ ਬੰਦ ਕਰੋ, ਫਿਰ ਜਾਰੀ ਰੱਖਣ ਲਈ “ਠੀਕ ਹੈ” ਜਾਂ ਬਾਹਰ ਜਾਣ ਲਈ “ਰੱਦ ਕਰੋ” ਦਬਾਓ।

; *** Startup questions
PrivilegesRequiredOverrideTitle=ਸੈੱਟਅੱਪ ਦਾ ਇੰਸਟਾਲ ਮੋਡ ਚੁਣੋ
PrivilegesRequiredOverrideInstruction=ਇੰਸਟਾਲ ਮੋਡ ਚੁਣੋ
PrivilegesRequiredOverrideText1=%1 ਸਾਰੇ ਵਰਤੋਂਕਾਰਾਂ ਲਈ (ਪ੍ਰਸ਼ਾਸਕ ਅਧਿਕਾਰ ਲੋੜੀਂਦੇ ਹਨ) ਜਾਂ ਸਿਰਫ਼ ਤੁਹਾਡੇ ਲਈ ਇੰਸਟਾਲ ਕੀਤਾ ਜਾ ਸਕਦਾ ਹੈ।
PrivilegesRequiredOverrideText2=%1 ਸਿਰਫ਼ ਤੁਹਾਡੇ ਲਈ ਜਾਂ ਸਾਰੇ ਵਰਤੋਂਕਾਰਾਂ ਲਈ (ਪ੍ਰਸ਼ਾਸਕ ਅਧਿਕਾਰ ਲੋੜੀਂਦੇ ਹਨ) ਇੰਸਟਾਲ ਕੀਤਾ ਜਾ ਸਕਦਾ ਹੈ।
PrivilegesRequiredOverrideAllUsers=ਸਾਰੇ ਵਰਤੋਂਕਾਰਾਂ ਲਈ ਇੰਸਟਾਲ ਕਰੋ
PrivilegesRequiredOverrideAllUsersRecommended=ਸਾਰੇ ਵਰਤੋਂਕਾਰਾਂ ਲਈ ਇੰਸਟਾਲ ਕਰੋ (ਸਿਫ਼ਾਰਸ਼ੀ)
PrivilegesRequiredOverrideCurrentUser=ਸਿਰਫ਼ ਮੇਰੇ ਲਈ ਇੰਸਟਾਲ ਕਰੋ
PrivilegesRequiredOverrideCurrentUserRecommended=ਸਿਰਫ਼ ਮੇਰੇ ਲਈ ਇੰਸਟਾਲ ਕਰੋ (ਸਿਫ਼ਾਰਸ਼ੀ)

; *** Misc. errors
ErrorCreatingDir=ਸੈੱਟਅੱਪ ਡਾਇਰੈਕਟਰੀ "%1" ਨਹੀਂ ਬਣਾ ਸਕਿਆ
ErrorTooManyFilesInDir=ਡਾਇਰੈਕਟਰੀ "%1" ਵਿੱਚ ਫ਼ਾਈਲ ਨਹੀਂ ਬਣਾਈ ਜਾ ਸਕੀ, ਕਿਉਂਕਿ ਉਸ ਵਿੱਚ ਬਹੁਤ ਜ਼ਿਆਦਾ ਫ਼ਾਈਲਾਂ ਹਨ

; *** Setup common messages
ExitSetupTitle=ਸੈੱਟਅੱਪ ਤੋਂ ਬਾਹਰ ਜਾਓ
ExitSetupMessage=ਸੈੱਟਅੱਪ ਪੂਰਾ ਨਹੀਂ ਹੋਇਆ। ਹੁਣ ਬਾਹਰ ਜਾਣ ਨਾਲ ਪ੍ਰੋਗਰਾਮ ਇੰਸਟਾਲ ਨਹੀਂ ਹੋਵੇਗਾ।%n%nਇੰਸਟਾਲੇਸ਼ਨ ਪੂਰੀ ਕਰਨ ਲਈ ਸੈੱਟਅੱਪ ਕਿਸੇ ਹੋਰ ਸਮੇਂ ਦੁਬਾਰਾ ਚਲਾਇਆ ਜਾ ਸਕਦਾ ਹੈ।%n%nਕੀ ਸੈੱਟਅੱਪ ਤੋਂ ਬਾਹਰ ਜਾਣਾ ਹੈ?
AboutSetupMenuItem=ਸੈੱਟਅੱਪ ਬਾਰੇ...
AboutSetupTitle=ਸੈੱਟਅੱਪ ਬਾਰੇ
AboutSetupMessage=%1 ਵਰਜਨ %2%n%3%n%n%1 ਦਾ ਹੋਮ ਪੇਜ:%n%4
AboutSetupNote=
TranslatorNote=

; *** Buttons
ButtonBack=< ਪਿੱਛੇ
ButtonNext=ਅੱਗੇ >
ButtonInstall=ਇੰਸਟਾਲ ਕਰੋ
ButtonOK=ਠੀਕ ਹੈ
ButtonCancel=ਰੱਦ ਕਰੋ
ButtonYes=ਹਾਂ
ButtonYesToAll=ਸਭ ਲਈ ਹਾਂ
ButtonNo=ਨਹੀਂ
ButtonNoToAll=ਸਭ ਲਈ ਨਹੀਂ
ButtonFinish=ਸਮਾਪਤ ਕਰੋ
ButtonBrowse=ਬ੍ਰਾਊਜ਼ ਕਰੋ...
ButtonWizardBrowse=ਬ੍ਰਾਊਜ਼ ਕਰੋ...
ButtonNewFolder=ਨਵਾਂ ਫ਼ੋਲਡਰ ਬਣਾਓ

; *** "Select Language" dialog messages
SelectLanguageTitle=ਸੈੱਟਅੱਪ ਦੀ ਭਾਸ਼ਾ ਚੁਣੋ
SelectLanguageLabel=ਇੰਸਟਾਲੇਸ਼ਨ ਦੌਰਾਨ ਵਰਤਣ ਲਈ ਭਾਸ਼ਾ ਚੁਣੋ।

; *** Common wizard text
ClickNext=ਜਾਰੀ ਰੱਖਣ ਲਈ “ਅੱਗੇ” ਦਬਾਓ, ਜਾਂ ਸੈੱਟਅੱਪ ਤੋਂ ਬਾਹਰ ਜਾਣ ਲਈ “ਰੱਦ ਕਰੋ” ਦਬਾਓ।
BeveledLabel=
BrowseDialogTitle=ਫ਼ੋਲਡਰ ਲਈ ਬ੍ਰਾਊਜ਼ ਕਰੋ
BrowseDialogLabel=ਹੇਠਲੀ ਸੂਚੀ ਵਿੱਚੋਂ ਇੱਕ ਫ਼ੋਲਡਰ ਚੁਣੋ, ਫਿਰ “ਠੀਕ ਹੈ” ਦਬਾਓ।
NewFolderName=ਨਵਾਂ ਫ਼ੋਲਡਰ

; *** "Welcome" wizard page
WelcomeLabel1=[name] ਸੈੱਟਅੱਪ ਵਿਜ਼ਰਡ ਵਿੱਚ ਜੀ ਆਇਆਂ ਨੂੰ
WelcomeLabel2=ਇਹ ਤੁਹਾਡੇ ਕੰਪਿਊਟਰ ਉੱਤੇ [name/ver] ਇੰਸਟਾਲ ਕਰੇਗਾ।%n%nਜਾਰੀ ਰੱਖਣ ਤੋਂ ਪਹਿਲਾਂ ਹੋਰ ਸਾਰੀਆਂ ਐਪਲੀਕੇਸ਼ਨਾਂ ਬੰਦ ਕਰਨ ਦੀ ਸਿਫ਼ਾਰਸ਼ ਕੀਤੀ ਜਾਂਦੀ ਹੈ।

; *** "Password" wizard page
WizardPassword=ਪਾਸਵਰਡ
PasswordLabel1=ਇਹ ਇੰਸਟਾਲੇਸ਼ਨ ਪਾਸਵਰਡ ਨਾਲ ਸੁਰੱਖਿਅਤ ਹੈ।
PasswordLabel3=ਕਿਰਪਾ ਕਰਕੇ ਪਾਸਵਰਡ ਦਿਓ, ਫਿਰ ਜਾਰੀ ਰੱਖਣ ਲਈ “ਅੱਗੇ” ਦਬਾਓ। ਪਾਸਵਰਡ ਵੱਡੇ-ਛੋਟੇ ਅੱਖਰਾਂ ਪ੍ਰਤੀ ਸੰਵੇਦਨਸ਼ੀਲ ਹੁੰਦੇ ਹਨ।
PasswordEditLabel=ਪਾਸਵਰਡ:
IncorrectPassword=ਦਿੱਤਾ ਗਿਆ ਪਾਸਵਰਡ ਸਹੀ ਨਹੀਂ ਹੈ। ਕਿਰਪਾ ਕਰਕੇ ਦੁਬਾਰਾ ਕੋਸ਼ਿਸ਼ ਕਰੋ।

; *** "License Agreement" wizard page
WizardLicense=ਲਾਇਸੰਸ ਇਕਰਾਰਨਾਮਾ
LicenseLabel=ਜਾਰੀ ਰੱਖਣ ਤੋਂ ਪਹਿਲਾਂ ਕਿਰਪਾ ਕਰਕੇ ਇਹ ਜ਼ਰੂਰੀ ਜਾਣਕਾਰੀ ਪੜ੍ਹੋ।
LicenseLabel3=ਕਿਰਪਾ ਕਰਕੇ ਹੇਠਾਂ ਦਿੱਤਾ ਲਾਇਸੰਸ ਇਕਰਾਰਨਾਮਾ ਪੜ੍ਹੋ। ਇੰਸਟਾਲੇਸ਼ਨ ਜਾਰੀ ਰੱਖਣ ਤੋਂ ਪਹਿਲਾਂ ਇਸ ਇਕਰਾਰਨਾਮੇ ਦੀਆਂ ਸ਼ਰਤਾਂ ਮੰਨਣੀਆਂ ਲਾਜ਼ਮੀ ਹਨ।
LicenseAccepted=ਮੈਨੂੰ ਇਕਰਾਰਨਾਮਾ ਮਨਜ਼ੂਰ ਹੈ
LicenseNotAccepted=ਮੈਨੂੰ ਇਕਰਾਰਨਾਮਾ ਮਨਜ਼ੂਰ ਨਹੀਂ ਹੈ

; *** "Information" wizard pages
WizardInfoBefore=ਜਾਣਕਾਰੀ
InfoBeforeLabel=ਜਾਰੀ ਰੱਖਣ ਤੋਂ ਪਹਿਲਾਂ ਕਿਰਪਾ ਕਰਕੇ ਇਹ ਜ਼ਰੂਰੀ ਜਾਣਕਾਰੀ ਪੜ੍ਹੋ।
InfoBeforeClickLabel=ਜਦੋਂ ਤੁਸੀਂ ਸੈੱਟਅੱਪ ਜਾਰੀ ਰੱਖਣ ਲਈ ਤਿਆਰ ਹੋਵੋ, ਤਾਂ “ਅੱਗੇ” ਦਬਾਓ।
WizardInfoAfter=ਜਾਣਕਾਰੀ
InfoAfterLabel=ਜਾਰੀ ਰੱਖਣ ਤੋਂ ਪਹਿਲਾਂ ਕਿਰਪਾ ਕਰਕੇ ਇਹ ਜ਼ਰੂਰੀ ਜਾਣਕਾਰੀ ਪੜ੍ਹੋ।
InfoAfterClickLabel=ਜਦੋਂ ਤੁਸੀਂ ਸੈੱਟਅੱਪ ਜਾਰੀ ਰੱਖਣ ਲਈ ਤਿਆਰ ਹੋਵੋ, ਤਾਂ “ਅੱਗੇ” ਦਬਾਓ।

; *** "User Information" wizard page
WizardUserInfo=ਵਰਤੋਂਕਾਰ ਦੀ ਜਾਣਕਾਰੀ
UserInfoDesc=ਕਿਰਪਾ ਕਰਕੇ ਆਪਣੀ ਜਾਣਕਾਰੀ ਦਿਓ।
UserInfoName=ਵਰਤੋਂਕਾਰ ਦਾ ਨਾਮ:
UserInfoOrg=ਸੰਗਠਨ:
UserInfoSerial=ਸੀਰੀਅਲ ਨੰਬਰ:
UserInfoNameRequired=ਨਾਮ ਦੇਣਾ ਲਾਜ਼ਮੀ ਹੈ।

; *** "Select Destination Location" wizard page
WizardSelectDir=ਮੰਜ਼ਿਲ ਦੀ ਥਾਂ ਚੁਣੋ
SelectDirDesc=[name] ਕਿੱਥੇ ਇੰਸਟਾਲ ਕੀਤਾ ਜਾਵੇ?
SelectDirLabel3=ਸੈੱਟਅੱਪ [name] ਨੂੰ ਹੇਠ ਦਿੱਤੇ ਫ਼ੋਲਡਰ ਵਿੱਚ ਇੰਸਟਾਲ ਕਰੇਗਾ।
SelectDirBrowseLabel=ਜਾਰੀ ਰੱਖਣ ਲਈ “ਅੱਗੇ” ਦਬਾਓ। ਜੇ ਕੋਈ ਹੋਰ ਫ਼ੋਲਡਰ ਚੁਣਨਾ ਹੈ, ਤਾਂ “ਬ੍ਰਾਊਜ਼ ਕਰੋ” ਦਬਾਓ।
DiskSpaceGBLabel=ਘੱਟੋ-ਘੱਟ [gb] GB ਖਾਲੀ ਡਿਸਕ ਥਾਂ ਚਾਹੀਦੀ ਹੈ।
DiskSpaceMBLabel=ਘੱਟੋ-ਘੱਟ [mb] MB ਖਾਲੀ ਡਿਸਕ ਥਾਂ ਚਾਹੀਦੀ ਹੈ।
CannotInstallToNetworkDrive=ਸੈੱਟਅੱਪ ਨੈੱਟਵਰਕ ਡਰਾਈਵ ਉੱਤੇ ਇੰਸਟਾਲ ਨਹੀਂ ਕਰ ਸਕਦਾ।
CannotInstallToUNCPath=ਸੈੱਟਅੱਪ UNC ਪਾਥ ਉੱਤੇ ਇੰਸਟਾਲ ਨਹੀਂ ਕਰ ਸਕਦਾ।
InvalidPath=ਡਰਾਈਵ ਅੱਖਰ ਸਮੇਤ ਪੂਰਾ ਪਾਥ ਦੇਣਾ ਲਾਜ਼ਮੀ ਹੈ; ਉਦਾਹਰਣ ਲਈ:%n%nC:\APP%n%nਜਾਂ ਇਸ ਰੂਪ ਵਿੱਚ UNC ਪਾਥ:%n%n\\server\share
InvalidDrive=ਚੁਣੀ ਗਈ ਡਰਾਈਵ ਜਾਂ UNC ਸ਼ੇਅਰ ਮੌਜੂਦ ਨਹੀਂ ਹੈ ਜਾਂ ਉਸ ਤੱਕ ਪਹੁੰਚ ਨਹੀਂ ਹੈ। ਕਿਰਪਾ ਕਰਕੇ ਕੋਈ ਹੋਰ ਚੁਣੋ।
DiskSpaceWarningTitle=ਡਿਸਕ ਥਾਂ ਕਾਫ਼ੀ ਨਹੀਂ
DiskSpaceWarning=ਇੰਸਟਾਲ ਕਰਨ ਲਈ ਸੈੱਟਅੱਪ ਨੂੰ ਘੱਟੋ-ਘੱਟ %1 KB ਖਾਲੀ ਥਾਂ ਚਾਹੀਦੀ ਹੈ, ਪਰ ਚੁਣੀ ਗਈ ਡਰਾਈਵ ਉੱਤੇ ਸਿਰਫ਼ %2 KB ਉਪਲਬਧ ਹੈ।%n%nਕੀ ਫਿਰ ਵੀ ਜਾਰੀ ਰੱਖਣਾ ਹੈ?
DirNameTooLong=ਫ਼ੋਲਡਰ ਦਾ ਨਾਮ ਜਾਂ ਪਾਥ ਬਹੁਤ ਲੰਮਾ ਹੈ।
InvalidDirName=ਫ਼ੋਲਡਰ ਦਾ ਨਾਮ ਸਹੀ ਨਹੀਂ ਹੈ।
BadDirName32=ਫ਼ੋਲਡਰ ਦੇ ਨਾਮ ਵਿੱਚ ਹੇਠ ਲਿਖੇ ਅੱਖਰ ਨਹੀਂ ਹੋ ਸਕਦੇ:%n%n%1
DirExistsTitle=ਫ਼ੋਲਡਰ ਪਹਿਲਾਂ ਤੋਂ ਮੌਜੂਦ ਹੈ
DirExists=ਫ਼ੋਲਡਰ:%n%n%1%n%nਪਹਿਲਾਂ ਤੋਂ ਮੌਜੂਦ ਹੈ। ਕੀ ਫਿਰ ਵੀ ਉਸੇ ਫ਼ੋਲਡਰ ਵਿੱਚ ਇੰਸਟਾਲ ਕਰਨਾ ਹੈ?
DirDoesntExistTitle=ਫ਼ੋਲਡਰ ਮੌਜੂਦ ਨਹੀਂ ਹੈ
DirDoesntExist=ਫ਼ੋਲਡਰ:%n%n%1%n%nਮੌਜੂਦ ਨਹੀਂ ਹੈ। ਕੀ ਇਹ ਫ਼ੋਲਡਰ ਬਣਾਉਣਾ ਹੈ?

; *** "Select Components" wizard page
WizardSelectComponents=ਕੰਪੋਨੈਂਟ ਚੁਣੋ
SelectComponentsDesc=ਕਿਹੜੇ ਕੰਪੋਨੈਂਟ ਇੰਸਟਾਲ ਕੀਤੇ ਜਾਣ?
SelectComponentsLabel2=ਜੋ ਕੰਪੋਨੈਂਟ ਇੰਸਟਾਲ ਕਰਨੇ ਹਨ ਉਨ੍ਹਾਂ ਨੂੰ ਚੁਣੋ; ਜੋ ਨਹੀਂ ਕਰਨੇ ਉਨ੍ਹਾਂ ਦੀ ਚੋਣ ਹਟਾਓ। ਤਿਆਰ ਹੋਣ ਤੇ “ਅੱਗੇ” ਦਬਾਓ।
FullInstallation=ਪੂਰੀ ਇੰਸਟਾਲੇਸ਼ਨ
; if possible don't translate 'Compact' as 'Minimal' (I mean 'Minimal' in your language)
CompactInstallation=ਸੰਖੇਪ ਇੰਸਟਾਲੇਸ਼ਨ
CustomInstallation=ਆਪਣੀ ਮਰਜ਼ੀ ਦੀ ਇੰਸਟਾਲੇਸ਼ਨ
NoUninstallWarningTitle=ਕੰਪੋਨੈਂਟ ਪਹਿਲਾਂ ਤੋਂ ਮੌਜੂਦ ਹਨ
NoUninstallWarning=ਸੈੱਟਅੱਪ ਨੂੰ ਪਤਾ ਲੱਗਾ ਹੈ ਕਿ ਹੇਠ ਲਿਖੇ ਕੰਪੋਨੈਂਟ ਤੁਹਾਡੇ ਕੰਪਿਊਟਰ ਉੱਤੇ ਪਹਿਲਾਂ ਹੀ ਇੰਸਟਾਲ ਹਨ:%n%n%1%n%nਇਨ੍ਹਾਂ ਦੀ ਚੋਣ ਹਟਾਉਣ ਨਾਲ ਇਹ ਅਣਇੰਸਟਾਲ ਨਹੀਂ ਹੋਣਗੇ।%n%nਕੀ ਫਿਰ ਵੀ ਜਾਰੀ ਰੱਖਣਾ ਹੈ?
ComponentSize1=%1 KB
ComponentSize2=%1 MB
ComponentsDiskSpaceGBLabel=ਮੌਜੂਦਾ ਚੋਣ ਲਈ ਘੱਟੋ-ਘੱਟ [gb] GB ਡਿਸਕ ਥਾਂ ਚਾਹੀਦੀ ਹੈ।
ComponentsDiskSpaceMBLabel=ਮੌਜੂਦਾ ਚੋਣ ਲਈ ਘੱਟੋ-ਘੱਟ [mb] MB ਡਿਸਕ ਥਾਂ ਚਾਹੀਦੀ ਹੈ।

; *** "Select Additional Tasks" wizard page
WizardSelectTasks=ਵਾਧੂ ਕੰਮ ਚੁਣੋ
SelectTasksDesc=ਕਿਹੜੇ ਵਾਧੂ ਕੰਮ ਕੀਤੇ ਜਾਣ?
SelectTasksLabel2=ਉਹ ਵਾਧੂ ਕੰਮ ਚੁਣੋ ਜੋ ਸੈੱਟਅੱਪ [name] ਇੰਸਟਾਲ ਕਰਦੇ ਸਮੇਂ ਕਰੇ, ਫਿਰ “ਅੱਗੇ” ਦਬਾਓ।

; *** "Select Start Menu Folder" wizard page
WizardSelectProgramGroup=ਸਟਾਰਟ ਮੀਨੂ ਫ਼ੋਲਡਰ ਚੁਣੋ
SelectStartMenuFolderDesc=ਸੈੱਟਅੱਪ ਪ੍ਰੋਗਰਾਮ ਦੇ ਸ਼ਾਰਟਕੱਟ ਕਿੱਥੇ ਰੱਖੇ?
SelectStartMenuFolderLabel3=ਸੈੱਟਅੱਪ ਪ੍ਰੋਗਰਾਮ ਦੇ ਸ਼ਾਰਟਕੱਟ ਹੇਠ ਦਿੱਤੇ ਸਟਾਰਟ ਮੀਨੂ ਫ਼ੋਲਡਰ ਵਿੱਚ ਬਣਾਏਗਾ।
SelectStartMenuFolderBrowseLabel=ਜਾਰੀ ਰੱਖਣ ਲਈ “ਅੱਗੇ” ਦਬਾਓ। ਜੇ ਕੋਈ ਹੋਰ ਫ਼ੋਲਡਰ ਚੁਣਨਾ ਹੈ, ਤਾਂ “ਬ੍ਰਾਊਜ਼ ਕਰੋ” ਦਬਾਓ।
MustEnterGroupName=ਫ਼ੋਲਡਰ ਦਾ ਨਾਮ ਦੇਣਾ ਲਾਜ਼ਮੀ ਹੈ।
GroupNameTooLong=ਫ਼ੋਲਡਰ ਦਾ ਨਾਮ ਜਾਂ ਪਾਥ ਬਹੁਤ ਲੰਮਾ ਹੈ।
InvalidGroupName=ਫ਼ੋਲਡਰ ਦਾ ਨਾਮ ਸਹੀ ਨਹੀਂ ਹੈ।
BadGroupName=ਫ਼ੋਲਡਰ ਦੇ ਨਾਮ ਵਿੱਚ ਹੇਠ ਲਿਖੇ ਅੱਖਰ ਨਹੀਂ ਹੋ ਸਕਦੇ:%n%n%1
NoProgramGroupCheck2=ਸਟਾਰਟ ਮੀਨੂ ਫ਼ੋਲਡਰ ਨਾ ਬਣਾਓ

; *** "Ready to Install" wizard page
WizardReady=ਇੰਸਟਾਲ ਕਰਨ ਲਈ ਤਿਆਰ
ReadyLabel1=ਸੈੱਟਅੱਪ ਹੁਣ ਤੁਹਾਡੇ ਕੰਪਿਊਟਰ ਉੱਤੇ [name] ਇੰਸਟਾਲ ਕਰਨ ਲਈ ਤਿਆਰ ਹੈ।
ReadyLabel2a=ਇੰਸਟਾਲੇਸ਼ਨ ਜਾਰੀ ਰੱਖਣ ਲਈ “ਇੰਸਟਾਲ ਕਰੋ” ਦਬਾਓ, ਜਾਂ ਕੋਈ ਸੈਟਿੰਗ ਦੇਖਣ ਜਾਂ ਬਦਲਣ ਲਈ “ਪਿੱਛੇ” ਦਬਾਓ।
ReadyLabel2b=ਇੰਸਟਾਲੇਸ਼ਨ ਜਾਰੀ ਰੱਖਣ ਲਈ “ਇੰਸਟਾਲ ਕਰੋ” ਦਬਾਓ।
ReadyMemoUserInfo=ਵਰਤੋਂਕਾਰ ਦੀ ਜਾਣਕਾਰੀ:
ReadyMemoDir=ਮੰਜ਼ਿਲ ਦੀ ਥਾਂ:
ReadyMemoType=ਸੈੱਟਅੱਪ ਦੀ ਕਿਸਮ:
ReadyMemoComponents=ਚੁਣੇ ਹੋਏ ਕੰਪੋਨੈਂਟ:
ReadyMemoGroup=ਸਟਾਰਟ ਮੀਨੂ ਫ਼ੋਲਡਰ:
ReadyMemoTasks=ਵਾਧੂ ਕੰਮ:

; *** TDownloadWizardPage wizard page and DownloadTemporaryFile
DownloadingLabel2=ਫ਼ਾਈਲਾਂ ਡਾਊਨਲੋਡ ਹੋ ਰਹੀਆਂ ਹਨ...
ButtonStopDownload=ਡਾਊਨਲੋਡ ਰੋਕੋ
StopDownload=ਕੀ ਡਾਊਨਲੋਡ ਰੋਕਣਾ ਹੈ?
ErrorDownloadAborted=ਡਾਊਨਲੋਡ ਰੱਦ ਕੀਤਾ ਗਿਆ
ErrorDownloadFailed=ਡਾਊਨਲੋਡ ਅਸਫਲ ਰਿਹਾ: %1 %2
ErrorDownloadSizeFailed=ਆਕਾਰ ਪਤਾ ਕਰਨਾ ਅਸਫਲ ਰਿਹਾ: %1 %2
ErrorProgress=ਗਲਤ ਪ੍ਰਗਤੀ: %1 / %2
ErrorFileSize=ਫ਼ਾਈਲ ਦਾ ਆਕਾਰ ਗਲਤ ਹੈ: ਉਮੀਦ %1 ਸੀ, ਮਿਲਿਆ %2

; *** TExtractionWizardPage wizard page and ExtractArchive
ExtractingLabel=ਫ਼ਾਈਲਾਂ ਕੱਢੀਆਂ ਜਾ ਰਹੀਆਂ ਹਨ...
ButtonStopExtraction=ਕੱਢਣਾ ਰੋਕੋ
StopExtraction=ਕੀ ਫ਼ਾਈਲਾਂ ਕੱਢਣਾ ਰੋਕਣਾ ਹੈ?
ErrorExtractionAborted=ਫ਼ਾਈਲਾਂ ਕੱਢਣਾ ਰੱਦ ਕੀਤਾ ਗਿਆ
ErrorExtractionFailed=ਫ਼ਾਈਲਾਂ ਕੱਢਣਾ ਅਸਫਲ ਰਿਹਾ: %1

; *** Archive extraction failure details
ArchiveIncorrectPassword=ਪਾਸਵਰਡ ਸਹੀ ਨਹੀਂ ਹੈ
ArchiveIsCorrupted=ਆਰਕਾਈਵ ਖ਼ਰਾਬ ਹੈ
ArchiveUnsupportedFormat=ਆਰਕਾਈਵ ਦਾ ਫ਼ਾਰਮੈਟ ਸਪੋਰਟ ਨਹੀਂ ਕੀਤਾ ਜਾਂਦਾ

; *** "Preparing to Install" wizard page
WizardPreparing=ਇੰਸਟਾਲੇਸ਼ਨ ਦੀ ਤਿਆਰੀ
PreparingDesc=ਸੈੱਟਅੱਪ ਤੁਹਾਡੇ ਕੰਪਿਊਟਰ ਉੱਤੇ [name] ਇੰਸਟਾਲ ਕਰਨ ਦੀ ਤਿਆਰੀ ਕਰ ਰਿਹਾ ਹੈ।
PreviousInstallNotCompleted=ਕਿਸੇ ਪਿਛਲੇ ਪ੍ਰੋਗਰਾਮ ਦੀ ਇੰਸਟਾਲੇਸ਼ਨ ਜਾਂ ਹਟਾਉਣ ਦੀ ਕਾਰਵਾਈ ਪੂਰੀ ਨਹੀਂ ਹੋਈ। ਉਸ ਨੂੰ ਪੂਰਾ ਕਰਨ ਲਈ ਕੰਪਿਊਟਰ ਰੀਸਟਾਰਟ ਕਰਨਾ ਪਵੇਗਾ।%n%nਕੰਪਿਊਟਰ ਰੀਸਟਾਰਟ ਕਰਨ ਤੋਂ ਬਾਅਦ [name] ਦੀ ਇੰਸਟਾਲੇਸ਼ਨ ਪੂਰੀ ਕਰਨ ਲਈ ਸੈੱਟਅੱਪ ਦੁਬਾਰਾ ਚਲਾਓ।
CannotContinue=ਸੈੱਟਅੱਪ ਜਾਰੀ ਨਹੀਂ ਰਹਿ ਸਕਦਾ। ਬਾਹਰ ਜਾਣ ਲਈ ਕਿਰਪਾ ਕਰਕੇ “ਰੱਦ ਕਰੋ” ਦਬਾਓ।
ApplicationsFound=ਹੇਠ ਲਿਖੀਆਂ ਐਪਲੀਕੇਸ਼ਨਾਂ ਉਨ੍ਹਾਂ ਫ਼ਾਈਲਾਂ ਦੀ ਵਰਤੋਂ ਕਰ ਰਹੀਆਂ ਹਨ ਜਿਨ੍ਹਾਂ ਨੂੰ ਸੈੱਟਅੱਪ ਨੇ ਅੱਪਡੇਟ ਕਰਨਾ ਹੈ। ਸਿਫ਼ਾਰਸ਼ ਕੀਤੀ ਜਾਂਦੀ ਹੈ ਕਿ ਸੈੱਟਅੱਪ ਨੂੰ ਇਹ ਐਪਲੀਕੇਸ਼ਨਾਂ ਆਪਣੇ-ਆਪ ਬੰਦ ਕਰਨ ਦਿੱਤਾ ਜਾਵੇ।
ApplicationsFound2=ਹੇਠ ਲਿਖੀਆਂ ਐਪਲੀਕੇਸ਼ਨਾਂ ਉਨ੍ਹਾਂ ਫ਼ਾਈਲਾਂ ਦੀ ਵਰਤੋਂ ਕਰ ਰਹੀਆਂ ਹਨ ਜਿਨ੍ਹਾਂ ਨੂੰ ਸੈੱਟਅੱਪ ਨੇ ਅੱਪਡੇਟ ਕਰਨਾ ਹੈ। ਸਿਫ਼ਾਰਸ਼ ਕੀਤੀ ਜਾਂਦੀ ਹੈ ਕਿ ਸੈੱਟਅੱਪ ਨੂੰ ਇਹ ਐਪਲੀਕੇਸ਼ਨਾਂ ਆਪਣੇ-ਆਪ ਬੰਦ ਕਰਨ ਦਿੱਤਾ ਜਾਵੇ। ਇੰਸਟਾਲੇਸ਼ਨ ਪੂਰੀ ਹੋਣ ਤੋਂ ਬਾਅਦ ਸੈੱਟਅੱਪ ਇਨ੍ਹਾਂ ਐਪਲੀਕੇਸ਼ਨਾਂ ਨੂੰ ਦੁਬਾਰਾ ਚਲਾਉਣ ਦੀ ਕੋਸ਼ਿਸ਼ ਕਰੇਗਾ।
CloseApplications=ਐਪਲੀਕੇਸ਼ਨਾਂ ਆਪਣੇ-ਆਪ ਬੰਦ ਕਰੋ
DontCloseApplications=ਐਪਲੀਕੇਸ਼ਨਾਂ ਬੰਦ ਨਾ ਕਰੋ
ErrorCloseApplications=ਸੈੱਟਅੱਪ ਸਾਰੀਆਂ ਐਪਲੀਕੇਸ਼ਨਾਂ ਆਪਣੇ-ਆਪ ਬੰਦ ਨਹੀਂ ਕਰ ਸਕਿਆ। ਜਾਰੀ ਰੱਖਣ ਤੋਂ ਪਹਿਲਾਂ ਉਹ ਸਾਰੀਆਂ ਐਪਲੀਕੇਸ਼ਨਾਂ ਬੰਦ ਕਰਨ ਦੀ ਸਿਫ਼ਾਰਸ਼ ਕੀਤੀ ਜਾਂਦੀ ਹੈ ਜੋ ਸੈੱਟਅੱਪ ਵੱਲੋਂ ਅੱਪਡੇਟ ਕੀਤੀਆਂ ਜਾਣ ਵਾਲੀਆਂ ਫ਼ਾਈਲਾਂ ਦੀ ਵਰਤੋਂ ਕਰ ਰਹੀਆਂ ਹਨ।
PrepareToInstallNeedsRestart=ਸੈੱਟਅੱਪ ਨੂੰ ਤੁਹਾਡਾ ਕੰਪਿਊਟਰ ਰੀਸਟਾਰਟ ਕਰਨਾ ਪਵੇਗਾ। ਕੰਪਿਊਟਰ ਰੀਸਟਾਰਟ ਕਰਨ ਤੋਂ ਬਾਅਦ [name] ਦੀ ਇੰਸਟਾਲੇਸ਼ਨ ਪੂਰੀ ਕਰਨ ਲਈ ਸੈੱਟਅੱਪ ਦੁਬਾਰਾ ਚਲਾਓ।%n%nਕੀ ਹੁਣ ਰੀਸਟਾਰਟ ਕਰਨਾ ਹੈ?

; *** "Installing" wizard page
WizardInstalling=ਇੰਸਟਾਲ ਹੋ ਰਿਹਾ ਹੈ
InstallingLabel=ਕਿਰਪਾ ਕਰਕੇ ਉਡੀਕ ਕਰੋ, ਸੈੱਟਅੱਪ ਤੁਹਾਡੇ ਕੰਪਿਊਟਰ ਉੱਤੇ [name] ਇੰਸਟਾਲ ਕਰ ਰਿਹਾ ਹੈ।

; *** "Setup Completed" wizard page
FinishedHeadingLabel=[name] ਸੈੱਟਅੱਪ ਵਿਜ਼ਰਡ ਪੂਰਾ ਹੋ ਰਿਹਾ ਹੈ
FinishedLabelNoIcons=ਸੈੱਟਅੱਪ ਨੇ ਤੁਹਾਡੇ ਕੰਪਿਊਟਰ ਉੱਤੇ [name] ਇੰਸਟਾਲ ਕਰਨਾ ਪੂਰਾ ਕਰ ਲਿਆ ਹੈ।
FinishedLabel=ਸੈੱਟਅੱਪ ਨੇ ਤੁਹਾਡੇ ਕੰਪਿਊਟਰ ਉੱਤੇ [name] ਇੰਸਟਾਲ ਕਰਨਾ ਪੂਰਾ ਕਰ ਲਿਆ ਹੈ। ਇੰਸਟਾਲ ਕੀਤੇ ਸ਼ਾਰਟਕੱਟ ਚੁਣ ਕੇ ਐਪਲੀਕੇਸ਼ਨ ਚਲਾਈ ਜਾ ਸਕਦੀ ਹੈ।
ClickFinish=ਸੈੱਟਅੱਪ ਤੋਂ ਬਾਹਰ ਜਾਣ ਲਈ “ਸਮਾਪਤ ਕਰੋ” ਦਬਾਓ।
FinishedRestartLabel=[name] ਦੀ ਇੰਸਟਾਲੇਸ਼ਨ ਪੂਰੀ ਕਰਨ ਲਈ ਸੈੱਟਅੱਪ ਨੂੰ ਤੁਹਾਡਾ ਕੰਪਿਊਟਰ ਰੀਸਟਾਰਟ ਕਰਨਾ ਪਵੇਗਾ। ਕੀ ਹੁਣ ਰੀਸਟਾਰਟ ਕਰਨਾ ਹੈ?
FinishedRestartMessage=[name] ਦੀ ਇੰਸਟਾਲੇਸ਼ਨ ਪੂਰੀ ਕਰਨ ਲਈ ਸੈੱਟਅੱਪ ਨੂੰ ਤੁਹਾਡਾ ਕੰਪਿਊਟਰ ਰੀਸਟਾਰਟ ਕਰਨਾ ਪਵੇਗਾ।%n%nਕੀ ਹੁਣ ਰੀਸਟਾਰਟ ਕਰਨਾ ਹੈ?
ShowReadmeCheck=ਹਾਂ, README ਫ਼ਾਈਲ ਦੇਖਣੀ ਹੈ
YesRadio=ਹਾਂ, ਕੰਪਿਊਟਰ ਹੁਣੇ ਰੀਸਟਾਰਟ ਕਰੋ
NoRadio=ਨਹੀਂ, ਰੀਸਟਾਰਟ ਬਾਅਦ ਵਿੱਚ ਕੀਤਾ ਜਾਵੇਗਾ
; used for example as 'Run MyProg.exe'
RunEntryExec=%1 ਚਲਾਓ
; used for example as 'View Readme.txt'
RunEntryShellExec=%1 ਦੇਖੋ

; *** "Setup Needs the Next Disk" stuff
ChangeDiskTitle=ਸੈੱਟਅੱਪ ਨੂੰ ਅਗਲੀ ਡਿਸਕ ਚਾਹੀਦੀ ਹੈ
SelectDiskLabel2=ਕਿਰਪਾ ਕਰਕੇ ਡਿਸਕ %1 ਪਾਓ ਅਤੇ “ਠੀਕ ਹੈ” ਦਬਾਓ।%n%nਜੇ ਇਸ ਡਿਸਕ ਦੀਆਂ ਫ਼ਾਈਲਾਂ ਹੇਠਾਂ ਦਿਖਾਏ ਫ਼ੋਲਡਰ ਤੋਂ ਇਲਾਵਾ ਕਿਸੇ ਹੋਰ ਫ਼ੋਲਡਰ ਵਿੱਚ ਮਿਲ ਸਕਦੀਆਂ ਹਨ, ਤਾਂ ਸਹੀ ਪਾਥ ਦਿਓ ਜਾਂ “ਬ੍ਰਾਊਜ਼ ਕਰੋ” ਦਬਾਓ।
PathLabel=ਪਾਥ:
FileNotInDir2=ਫ਼ਾਈਲ "%1" "%2" ਵਿੱਚ ਨਹੀਂ ਮਿਲੀ। ਕਿਰਪਾ ਕਰਕੇ ਸਹੀ ਡਿਸਕ ਪਾਓ ਜਾਂ ਕੋਈ ਹੋਰ ਫ਼ੋਲਡਰ ਚੁਣੋ।
SelectDirectoryLabel=ਕਿਰਪਾ ਕਰਕੇ ਅਗਲੀ ਡਿਸਕ ਦੀ ਥਾਂ ਦੱਸੋ।

; *** Installation phase messages
SetupAborted=ਸੈੱਟਅੱਪ ਪੂਰਾ ਨਹੀਂ ਹੋਇਆ।%n%nਕਿਰਪਾ ਕਰਕੇ ਸਮੱਸਿਆ ਠੀਕ ਕਰੋ ਅਤੇ ਸੈੱਟਅੱਪ ਦੁਬਾਰਾ ਚਲਾਓ।
AbortRetryIgnoreSelectAction=ਕਾਰਵਾਈ ਚੁਣੋ
AbortRetryIgnoreRetry=ਦੁਬਾਰਾ ਕੋਸ਼ਿਸ਼ ਕਰੋ
AbortRetryIgnoreIgnore=ਗਲਤੀ ਨੂੰ ਅਣਡਿੱਠ ਕਰੋ ਅਤੇ ਜਾਰੀ ਰੱਖੋ
AbortRetryIgnoreCancel=ਇੰਸਟਾਲੇਸ਼ਨ ਰੱਦ ਕਰੋ
RetryCancelSelectAction=ਕਾਰਵਾਈ ਚੁਣੋ
RetryCancelRetry=ਦੁਬਾਰਾ ਕੋਸ਼ਿਸ਼ ਕਰੋ
RetryCancelCancel=ਰੱਦ ਕਰੋ

; *** Installation status messages
StatusClosingApplications=ਐਪਲੀਕੇਸ਼ਨਾਂ ਬੰਦ ਕੀਤੀਆਂ ਜਾ ਰਹੀਆਂ ਹਨ...
StatusCreateDirs=ਡਾਇਰੈਕਟਰੀਆਂ ਬਣਾਈਆਂ ਜਾ ਰਹੀਆਂ ਹਨ...
StatusExtractFiles=ਫ਼ਾਈਲਾਂ ਕੱਢੀਆਂ ਜਾ ਰਹੀਆਂ ਹਨ...
StatusDownloadFiles=ਫ਼ਾਈਲਾਂ ਡਾਊਨਲੋਡ ਹੋ ਰਹੀਆਂ ਹਨ...
StatusCreateIcons=ਸ਼ਾਰਟਕੱਟ ਬਣਾਏ ਜਾ ਰਹੇ ਹਨ...
StatusCreateIniEntries=INI ਐਂਟਰੀਆਂ ਬਣਾਈਆਂ ਜਾ ਰਹੀਆਂ ਹਨ...
StatusCreateRegistryEntries=ਰਜਿਸਟਰੀ ਐਂਟਰੀਆਂ ਬਣਾਈਆਂ ਜਾ ਰਹੀਆਂ ਹਨ...
StatusRegisterFiles=ਫ਼ਾਈਲਾਂ ਰਜਿਸਟਰ ਕੀਤੀਆਂ ਜਾ ਰਹੀਆਂ ਹਨ...
StatusSavingUninstall=ਅਣਇੰਸਟਾਲ ਦੀ ਜਾਣਕਾਰੀ ਸੰਭਾਲੀ ਜਾ ਰਹੀ ਹੈ...
StatusRunProgram=ਇੰਸਟਾਲੇਸ਼ਨ ਪੂਰੀ ਕੀਤੀ ਜਾ ਰਹੀ ਹੈ...
StatusRestartingApplications=ਐਪਲੀਕੇਸ਼ਨਾਂ ਦੁਬਾਰਾ ਚਲਾਈਆਂ ਜਾ ਰਹੀਆਂ ਹਨ...
StatusRollback=ਤਬਦੀਲੀਆਂ ਵਾਪਸ ਲਈਆਂ ਜਾ ਰਹੀਆਂ ਹਨ...

; *** Misc. errors
ErrorInternal2=ਅੰਦਰੂਨੀ ਗਲਤੀ: %1
ErrorFunctionFailedNoCode=%1 ਅਸਫਲ ਰਿਹਾ
ErrorFunctionFailed=%1 ਅਸਫਲ ਰਿਹਾ; ਕੋਡ %2
ErrorFunctionFailedWithMessage=%1 ਅਸਫਲ ਰਿਹਾ; ਕੋਡ %2.%n%3
ErrorExecutingProgram=ਫ਼ਾਈਲ ਨਹੀਂ ਚਲਾਈ ਜਾ ਸਕੀ:%n%1

; *** Registry errors
ErrorRegOpenKey=ਰਜਿਸਟਰੀ ਕੁੰਜੀ ਖੋਲ੍ਹਣ ਵਿੱਚ ਗਲਤੀ:%n%1\%2
ErrorRegCreateKey=ਰਜਿਸਟਰੀ ਕੁੰਜੀ ਬਣਾਉਣ ਵਿੱਚ ਗਲਤੀ:%n%1\%2
ErrorRegWriteKey=ਰਜਿਸਟਰੀ ਕੁੰਜੀ ਵਿੱਚ ਲਿਖਣ ਵਿੱਚ ਗਲਤੀ:%n%1\%2

; *** INI errors
ErrorIniEntry=ਫ਼ਾਈਲ "%1" ਵਿੱਚ INI ਐਂਟਰੀ ਬਣਾਉਣ ਵਿੱਚ ਗਲਤੀ।

; *** File copying errors
FileAbortRetryIgnoreSkipNotRecommended=ਇਹ ਫ਼ਾਈਲ ਛੱਡੋ (ਸਿਫ਼ਾਰਸ਼ ਨਹੀਂ ਕੀਤੀ ਜਾਂਦੀ)
FileAbortRetryIgnoreIgnoreNotRecommended=ਗਲਤੀ ਨੂੰ ਅਣਡਿੱਠ ਕਰੋ ਅਤੇ ਜਾਰੀ ਰੱਖੋ (ਸਿਫ਼ਾਰਸ਼ ਨਹੀਂ ਕੀਤੀ ਜਾਂਦੀ)
SourceIsCorrupted=ਸਰੋਤ ਫ਼ਾਈਲ ਖ਼ਰਾਬ ਹੈ
SourceDoesntExist=ਸਰੋਤ ਫ਼ਾਈਲ "%1" ਮੌਜੂਦ ਨਹੀਂ ਹੈ
SourceVerificationFailed=ਸਰੋਤ ਫ਼ਾਈਲ ਦੀ ਪੜਤਾਲ ਅਸਫਲ ਰਹੀ: %1
VerificationSignatureDoesntExist=ਦਸਤਖ਼ਤ ਫ਼ਾਈਲ "%1" ਮੌਜੂਦ ਨਹੀਂ ਹੈ
VerificationSignatureInvalid=ਦਸਤਖ਼ਤ ਫ਼ਾਈਲ "%1" ਸਹੀ ਨਹੀਂ ਹੈ
VerificationKeyNotFound=ਦਸਤਖ਼ਤ ਫ਼ਾਈਲ "%1" ਅਣਜਾਣ ਕੁੰਜੀ ਵਰਤਦੀ ਹੈ
VerificationFileNameIncorrect=ਫ਼ਾਈਲ ਦਾ ਨਾਮ ਗਲਤ ਹੈ
VerificationFileTagIncorrect=ਫ਼ਾਈਲ ਦਾ ਟੈਗ ਗਲਤ ਹੈ
VerificationFileSizeIncorrect=ਫ਼ਾਈਲ ਦਾ ਆਕਾਰ ਗਲਤ ਹੈ
VerificationFileHashIncorrect=ਫ਼ਾਈਲ ਦਾ ਹੈਸ਼ ਗਲਤ ਹੈ
ExistingFileReadOnly2=ਮੌਜੂਦਾ ਫ਼ਾਈਲ ਬਦਲੀ ਨਹੀਂ ਜਾ ਸਕੀ, ਕਿਉਂਕਿ ਉਹ ਸਿਰਫ਼-ਪੜ੍ਹਨ ਲਈ ਨਿਸ਼ਾਨਬੱਧ ਹੈ।
ExistingFileReadOnlyRetry=ਸਿਰਫ਼-ਪੜ੍ਹਨ ਦਾ ਗੁਣ ਹਟਾਓ ਅਤੇ ਦੁਬਾਰਾ ਕੋਸ਼ਿਸ਼ ਕਰੋ
ExistingFileReadOnlyKeepExisting=ਮੌਜੂਦਾ ਫ਼ਾਈਲ ਰੱਖੋ
ErrorReadingExistingDest=ਮੌਜੂਦਾ ਫ਼ਾਈਲ ਪੜ੍ਹਦੇ ਸਮੇਂ ਗਲਤੀ ਹੋਈ:
FileExistsSelectAction=ਕਾਰਵਾਈ ਚੁਣੋ
FileExists2=ਫ਼ਾਈਲ ਪਹਿਲਾਂ ਤੋਂ ਮੌਜੂਦ ਹੈ।
FileExistsOverwriteExisting=ਮੌਜੂਦਾ ਫ਼ਾਈਲ ਦੀ ਥਾਂ ਨਵੀਂ ਲਿਖੋ
FileExistsKeepExisting=ਮੌਜੂਦਾ ਫ਼ਾਈਲ ਰੱਖੋ
FileExistsOverwriteOrKeepAll=ਅਗਲੇ ਟਕਰਾਵਾਂ ਲਈ ਵੀ ਇਹੀ ਕਰੋ
ExistingFileNewerSelectAction=ਕਾਰਵਾਈ ਚੁਣੋ
ExistingFileNewer2=ਮੌਜੂਦਾ ਫ਼ਾਈਲ ਉਸ ਫ਼ਾਈਲ ਤੋਂ ਨਵੀਂ ਹੈ ਜੋ ਸੈੱਟਅੱਪ ਇੰਸਟਾਲ ਕਰਨ ਦੀ ਕੋਸ਼ਿਸ਼ ਕਰ ਰਿਹਾ ਹੈ।
ExistingFileNewerOverwriteExisting=ਮੌਜੂਦਾ ਫ਼ਾਈਲ ਦੀ ਥਾਂ ਨਵੀਂ ਲਿਖੋ
ExistingFileNewerKeepExisting=ਮੌਜੂਦਾ ਫ਼ਾਈਲ ਰੱਖੋ (ਸਿਫ਼ਾਰਸ਼ੀ)
ExistingFileNewerOverwriteOrKeepAll=ਅਗਲੇ ਟਕਰਾਵਾਂ ਲਈ ਵੀ ਇਹੀ ਕਰੋ
ErrorChangingAttr=ਮੌਜੂਦਾ ਫ਼ਾਈਲ ਦੇ ਗੁਣ ਬਦਲਦੇ ਸਮੇਂ ਗਲਤੀ ਹੋਈ:
ErrorCreatingTemp=ਮੰਜ਼ਿਲ ਡਾਇਰੈਕਟਰੀ ਵਿੱਚ ਫ਼ਾਈਲ ਬਣਾਉਂਦੇ ਸਮੇਂ ਗਲਤੀ ਹੋਈ:
ErrorReadingSource=ਸਰੋਤ ਫ਼ਾਈਲ ਪੜ੍ਹਦੇ ਸਮੇਂ ਗਲਤੀ ਹੋਈ:
ErrorCopying=ਫ਼ਾਈਲ ਕਾਪੀ ਕਰਦੇ ਸਮੇਂ ਗਲਤੀ ਹੋਈ:
ErrorDownloading=ਫ਼ਾਈਲ ਡਾਊਨਲੋਡ ਕਰਦੇ ਸਮੇਂ ਗਲਤੀ ਹੋਈ:
ErrorExtracting=ਆਰਕਾਈਵ ਖੋਲ੍ਹਦੇ ਸਮੇਂ ਗਲਤੀ ਹੋਈ:
ErrorReplacingExistingFile=ਮੌਜੂਦਾ ਫ਼ਾਈਲ ਬਦਲਦੇ ਸਮੇਂ ਗਲਤੀ ਹੋਈ:
ErrorRestartReplace=RestartReplace ਅਸਫਲ ਰਿਹਾ:
ErrorRenamingTemp=ਮੰਜ਼ਿਲ ਡਾਇਰੈਕਟਰੀ ਵਿੱਚ ਫ਼ਾਈਲ ਦਾ ਨਾਮ ਬਦਲਦੇ ਸਮੇਂ ਗਲਤੀ ਹੋਈ:
ErrorRegisterServer=DLL/OCX ਰਜਿਸਟਰ ਨਹੀਂ ਹੋ ਸਕਿਆ: %1
ErrorRegSvr32Failed=RegSvr32 ਐਗਜ਼ਿਟ ਕੋਡ %1 ਨਾਲ ਅਸਫਲ ਰਿਹਾ
ErrorRegisterTypeLib=ਟਾਈਪ ਲਾਇਬ੍ਰੇਰੀ ਰਜਿਸਟਰ ਨਹੀਂ ਹੋ ਸਕੀ: %1

; *** Uninstall display name markings
; used for example as 'My Program (32-bit)'
UninstallDisplayNameMark=%1 (%2)
; used for example as 'My Program (32-bit, All users)'
UninstallDisplayNameMarks=%1 (%2, %3)
UninstallDisplayNameMark32Bit=32-ਬਿੱਟ
UninstallDisplayNameMark64Bit=64-ਬਿੱਟ
UninstallDisplayNameMarkAllUsers=ਸਾਰੇ ਵਰਤੋਂਕਾਰ
UninstallDisplayNameMarkCurrentUser=ਮੌਜੂਦਾ ਵਰਤੋਂਕਾਰ

; *** Post-installation errors
ErrorOpeningReadme=README ਫ਼ਾਈਲ ਖੋਲ੍ਹਦੇ ਸਮੇਂ ਗਲਤੀ ਹੋਈ।
ErrorRestartingComputer=ਸੈੱਟਅੱਪ ਕੰਪਿਊਟਰ ਰੀਸਟਾਰਟ ਨਹੀਂ ਕਰ ਸਕਿਆ। ਕਿਰਪਾ ਕਰਕੇ ਇਹ ਆਪ ਕਰੋ।

; *** Uninstaller messages
UninstallNotFound=ਫ਼ਾਈਲ "%1" ਮੌਜੂਦ ਨਹੀਂ ਹੈ। ਅਣਇੰਸਟਾਲ ਨਹੀਂ ਕੀਤਾ ਜਾ ਸਕਦਾ।
UninstallOpenError=ਫ਼ਾਈਲ "%1" ਖੋਲ੍ਹੀ ਨਹੀਂ ਜਾ ਸਕੀ। ਅਣਇੰਸਟਾਲ ਨਹੀਂ ਕੀਤਾ ਜਾ ਸਕਦਾ
UninstallUnsupportedVer=ਅਣਇੰਸਟਾਲ ਲੌਗ ਫ਼ਾਈਲ "%1" ਅਜਿਹੇ ਫ਼ਾਰਮੈਟ ਵਿੱਚ ਹੈ ਜਿਸ ਨੂੰ ਅਣਇੰਸਟਾਲਰ ਦਾ ਇਹ ਵਰਜਨ ਨਹੀਂ ਪਛਾਣਦਾ। ਅਣਇੰਸਟਾਲ ਨਹੀਂ ਕੀਤਾ ਜਾ ਸਕਦਾ
UninstallUnknownEntry=ਅਣਇੰਸਟਾਲ ਲੌਗ ਵਿੱਚ ਇੱਕ ਅਣਜਾਣ ਐਂਟਰੀ (%1) ਮਿਲੀ
ConfirmUninstall=ਕੀ %1 ਅਤੇ ਇਸ ਦੇ ਸਾਰੇ ਕੰਪੋਨੈਂਟ ਪੂਰੀ ਤਰ੍ਹਾਂ ਹਟਾਉਣੇ ਹਨ?
UninstallOnlyOnWin64=ਇਸ ਇੰਸਟਾਲੇਸ਼ਨ ਨੂੰ ਸਿਰਫ਼ 64-ਬਿੱਟ Windows ਉੱਤੇ ਅਣਇੰਸਟਾਲ ਕੀਤਾ ਜਾ ਸਕਦਾ ਹੈ।
OnlyAdminCanUninstall=ਇਸ ਇੰਸਟਾਲੇਸ਼ਨ ਨੂੰ ਸਿਰਫ਼ ਪ੍ਰਸ਼ਾਸਕ ਅਧਿਕਾਰਾਂ ਵਾਲਾ ਵਰਤੋਂਕਾਰ ਅਣਇੰਸਟਾਲ ਕਰ ਸਕਦਾ ਹੈ।
UninstallStatusLabel=ਕਿਰਪਾ ਕਰਕੇ ਉਡੀਕ ਕਰੋ, %1 ਤੁਹਾਡੇ ਕੰਪਿਊਟਰ ਤੋਂ ਹਟਾਇਆ ਜਾ ਰਿਹਾ ਹੈ।
UninstalledAll=%1 ਤੁਹਾਡੇ ਕੰਪਿਊਟਰ ਤੋਂ ਸਫਲਤਾਪੂਰਵਕ ਹਟਾ ਦਿੱਤਾ ਗਿਆ।
UninstalledMost=%1 ਦਾ ਅਣਇੰਸਟਾਲ ਪੂਰਾ ਹੋਇਆ।%n%nਕੁਝ ਹਿੱਸੇ ਹਟਾਏ ਨਹੀਂ ਜਾ ਸਕੇ। ਇਨ੍ਹਾਂ ਨੂੰ ਹੱਥੀਂ ਹਟਾਇਆ ਜਾ ਸਕਦਾ ਹੈ।
UninstalledAndNeedsRestart=%1 ਦਾ ਅਣਇੰਸਟਾਲ ਪੂਰਾ ਕਰਨ ਲਈ ਤੁਹਾਡਾ ਕੰਪਿਊਟਰ ਰੀਸਟਾਰਟ ਕਰਨਾ ਪਵੇਗਾ।%n%nਕੀ ਹੁਣ ਰੀਸਟਾਰਟ ਕਰਨਾ ਹੈ?
UninstallDataCorrupted="%1" ਫ਼ਾਈਲ ਖ਼ਰਾਬ ਹੈ। ਅਣਇੰਸਟਾਲ ਨਹੀਂ ਕੀਤਾ ਜਾ ਸਕਦਾ

; *** Uninstallation phase messages
ConfirmDeleteSharedFileTitle=ਸਾਂਝੀ ਫ਼ਾਈਲ ਹਟਾਉਣੀ ਹੈ?
ConfirmDeleteSharedFile2=ਸਿਸਟਮ ਦੱਸਦਾ ਹੈ ਕਿ ਹੇਠ ਲਿਖੀ ਸਾਂਝੀ ਫ਼ਾਈਲ ਹੁਣ ਕਿਸੇ ਪ੍ਰੋਗਰਾਮ ਦੁਆਰਾ ਵਰਤੀ ਨਹੀਂ ਜਾ ਰਹੀ। ਕੀ ਅਣਇੰਸਟਾਲ ਇਹ ਸਾਂਝੀ ਫ਼ਾਈਲ ਹਟਾਵੇ?%n%nਜੇ ਕੋਈ ਪ੍ਰੋਗਰਾਮ ਅਜੇ ਵੀ ਇਸ ਫ਼ਾਈਲ ਦੀ ਵਰਤੋਂ ਕਰ ਰਿਹਾ ਹੈ ਅਤੇ ਇਹ ਹਟਾ ਦਿੱਤੀ ਗਈ, ਤਾਂ ਹੋ ਸਕਦਾ ਹੈ ਕਿ ਉਹ ਪ੍ਰੋਗਰਾਮ ਠੀਕ ਤਰ੍ਹਾਂ ਕੰਮ ਨਾ ਕਰਨ। ਜੇ ਪੱਕਾ ਪਤਾ ਨਹੀਂ, ਤਾਂ “ਨਹੀਂ” ਚੁਣੋ। ਫ਼ਾਈਲ ਨੂੰ ਸਿਸਟਮ ਵਿੱਚ ਰਹਿਣ ਦੇਣ ਨਾਲ ਕੋਈ ਨੁਕਸਾਨ ਨਹੀਂ ਹੋਵੇਗਾ।
SharedFileNameLabel=ਫ਼ਾਈਲ ਦਾ ਨਾਮ:
SharedFileLocationLabel=ਥਾਂ:
WizardUninstalling=ਅਣਇੰਸਟਾਲ ਦੀ ਸਥਿਤੀ
StatusUninstalling=%1 ਅਣਇੰਸਟਾਲ ਹੋ ਰਿਹਾ ਹੈ...

; *** Shutdown block reasons
ShutdownBlockReasonInstallingApp=%1 ਇੰਸਟਾਲ ਹੋ ਰਿਹਾ ਹੈ।
ShutdownBlockReasonUninstallingApp=%1 ਅਣਇੰਸਟਾਲ ਹੋ ਰਿਹਾ ਹੈ।

; The custom messages below aren't used by Setup itself, but if you make
; use of them in your scripts, you'll want to translate them.

[CustomMessages]

NameAndVersion=%1 ਵਰਜਨ %2
AdditionalIcons=ਵਾਧੂ ਸ਼ਾਰਟਕੱਟ:
CreateDesktopIcon=ਡੈਸਕਟਾਪ ਸ਼ਾਰਟਕੱਟ ਬਣਾਓ
CreateQuickLaunchIcon=ਕਵਿੱਕ ਲਾਂਚ ਸ਼ਾਰਟਕੱਟ ਬਣਾਓ
ProgramOnTheWeb=ਵੈੱਬ ਉੱਤੇ %1
UninstallProgram=%1 ਅਣਇੰਸਟਾਲ ਕਰੋ
LaunchProgram=%1 ਚਲਾਓ
AssocFileExtension=%1 ਨੂੰ %2 ਫ਼ਾਈਲ ਐਕਸਟੈਂਸ਼ਨ ਨਾਲ ਜੋੜੋ
AssocingFileExtension=%1 ਨੂੰ %2 ਫ਼ਾਈਲ ਐਕਸਟੈਂਸ਼ਨ ਨਾਲ ਜੋੜਿਆ ਜਾ ਰਿਹਾ ਹੈ...
AutoStartProgramGroupDescription=ਸਟਾਰਟਅੱਪ:
AutoStartProgram=%1 ਆਪਣੇ-ਆਪ ਚਲਾਓ
AddonHostProgramNotFound=ਤੁਹਾਡੇ ਚੁਣੇ ਫ਼ੋਲਡਰ ਵਿੱਚ %1 ਨਹੀਂ ਮਿਲਿਆ।%n%nਕੀ ਫਿਰ ਵੀ ਜਾਰੀ ਰੱਖਣਾ ਹੈ?

; The messages that are Lang Switcher's own (the sample is English.isl).
LocalCertReady=ਸੈੱਟਅੱਪ ਇਸ ਕੰਪਿਊਟਰ ਉੱਤੇ ਇੱਕ ਸਰਟੀਫ਼ਿਕੇਟ ਬਣਾਏਗਾ ਅਤੇ ਉਸ ਨਾਲ Lang Switcher ਉੱਤੇ ਦਸਤਖ਼ਤ ਕਰੇਗਾ, ਤਾਂ ਜੋ ਇਹ ਪ੍ਰਸ਼ਾਸਕ ਵਜੋਂ ਚਲਾਏ ਗਏ ਪ੍ਰੋਗਰਾਮਾਂ ਦੀਆਂ ਵਿੰਡੋਆਂ ਵਿੱਚ ਵੀ ਕੰਮ ਕਰ ਸਕੇ।
LocalCertFailed=Lang Switcher ਇੰਸਟਾਲ ਹੋ ਗਿਆ, ਪਰ ਇਹ ਪ੍ਰਸ਼ਾਸਕ ਵਜੋਂ ਚਲਾਏ ਗਏ ਪ੍ਰੋਗਰਾਮਾਂ ਦੀਆਂ ਵਿੰਡੋਆਂ ਵਿੱਚ ਕੰਮ ਨਹੀਂ ਕਰੇਗਾ: Windows ਨੇ ਇਸ ਕੰਪਿਊਟਰ ਉੱਤੇ ਬਣਾਇਆ ਸਰਟੀਫ਼ਿਕੇਟ ਸਵੀਕਾਰ ਨਹੀਂ ਕੀਤਾ। ਵੇਰਵਿਆਂ ਲਈ ਸੈੱਟਅੱਪ ਲੌਗ ਦੇਖੋ।
RemoveSettingsQuestion=ਕੀ Lang Switcher ਦੀ ਸੈਟਿੰਗਾਂ ਫ਼ਾਈਲ ਵੀ ਹਟਾਉਣੀ ਹੈ?
RemoveSettingsKeep=ਮੁੜ ਇੰਸਟਾਲ ਕਰਨ ਲਈ ਆਪਣੀਆਂ ਸੈਟਿੰਗਾਂ ਰੱਖਣੀਆਂ ਹੋਣ, ਤਾਂ “ਨਹੀਂ” ਚੁਣੋ।
AuthorSlogan=ਦਿਲੋਂ ਬਣੇ ਪ੍ਰੋਗਰਾਮ
