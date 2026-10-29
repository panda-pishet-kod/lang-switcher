; Lang Switcher -- the messages of the installer in Kannada (stage E108, group A).
;
; Inno Setup has no official Kannada translation (inno_official = false in res\lang\languages.toml),
; so this file carries the whole of it: [LangOptions], every message of the compiler:Default.isl
; (6.5.0+), and the messages of the installer that are Lang Switcher's own. It is read after
; compiler:Default.isl as the second MessagesFile of the "kannada" entry of [Languages] in
; ..\LangSwitcher.iss, and so replaces the English text name for name.
; English.isl is the sample the own messages translate; the glossary of the language is
; l10n\glossary\kn.tsv of the control contour. The name of the program is not translated.
; No accelerators (&): a file without an official translation puts none, so that one window never
; shows two equal letters (RULES 16). The names of buttons inside a message stand in the quotation
; marks of the language. As in Default.isl: a message that has no full stop there has none here
; (Setup adds its own).

[LangOptions]
LanguageName=ಕನ್ನಡ
LanguageID=$044B
LanguageCodePage=0

[Messages]

; *** Application titles
SetupAppTitle=ಸೆಟಪ್
SetupWindowTitle=ಸೆಟಪ್ - %1
UninstallAppTitle=ಅಸ್ಥಾಪನೆ
UninstallAppFullTitle=%1 ಅಸ್ಥಾಪನೆ

; *** Misc. common
InformationTitle=ಮಾಹಿತಿ
ConfirmTitle=ದೃಢೀಕರಿಸಿ
ErrorTitle=ದೋಷ

; *** SetupLdr messages
SetupLdrStartupMessage=ಇದು %1 ಅನ್ನು ಸ್ಥಾಪಿಸುತ್ತದೆ. ಮುಂದುವರಿಸಲು ಬಯಸುತ್ತೀರಾ?
LdrCannotCreateTemp=ತಾತ್ಕಾಲಿಕ ಫೈಲ್ ರಚಿಸಲು ಸಾಧ್ಯವಾಗಲಿಲ್ಲ. ಸೆಟಪ್ ಸ್ಥಗಿತಗೊಂಡಿದೆ
LdrCannotExecTemp=ತಾತ್ಕಾಲಿಕ ಡೈರೆಕ್ಟರಿಯಲ್ಲಿನ ಫೈಲ್ ಅನ್ನು ಚಲಾಯಿಸಲು ಸಾಧ್ಯವಾಗಲಿಲ್ಲ. ಸೆಟಪ್ ಸ್ಥಗಿತಗೊಂಡಿದೆ
HelpTextNote=

; *** Startup error messages
LastErrorMessage=%1.%n%nದೋಷ %2: %3
SetupFileMissing=ಸ್ಥಾಪನೆ ಡೈರೆಕ್ಟರಿಯಲ್ಲಿ %1 ಫೈಲ್ ಇಲ್ಲ. ದಯವಿಟ್ಟು ಸಮಸ್ಯೆಯನ್ನು ಸರಿಪಡಿಸಿ ಅಥವಾ ಪ್ರೋಗ್ರಾಂನ ಹೊಸ ಪ್ರತಿಯನ್ನು ಪಡೆಯಿರಿ.
SetupFileCorrupt=ಸೆಟಪ್ ಫೈಲ್‌ಗಳು ಹಾಳಾಗಿವೆ. ದಯವಿಟ್ಟು ಪ್ರೋಗ್ರಾಂನ ಹೊಸ ಪ್ರತಿಯನ್ನು ಪಡೆಯಿರಿ.
SetupFileCorruptOrWrongVer=ಸೆಟಪ್ ಫೈಲ್‌ಗಳು ಹಾಳಾಗಿವೆ ಅಥವಾ ಸೆಟಪ್‌ನ ಈ ಆವೃತ್ತಿಗೆ ಹೊಂದಿಕೆಯಾಗುವುದಿಲ್ಲ. ದಯವಿಟ್ಟು ಸಮಸ್ಯೆಯನ್ನು ಸರಿಪಡಿಸಿ ಅಥವಾ ಪ್ರೋಗ್ರಾಂನ ಹೊಸ ಪ್ರತಿಯನ್ನು ಪಡೆಯಿರಿ.
InvalidParameter=ಕಮಾಂಡ್ ಲೈನ್‌ನಲ್ಲಿ ಅಮಾನ್ಯ ಪ್ಯಾರಾಮೀಟರ್ ನೀಡಲಾಗಿದೆ:%n%n%1
SetupAlreadyRunning=ಸೆಟಪ್ ಈಗಾಗಲೇ ಚಾಲನೆಯಲ್ಲಿದೆ.
WindowsVersionNotSupported=ಈ ಪ್ರೋಗ್ರಾಂ ನಿಮ್ಮ ಕಂಪ್ಯೂಟರ್‌ನಲ್ಲಿನ Windows ಆವೃತ್ತಿಯನ್ನು ಬೆಂಬಲಿಸುವುದಿಲ್ಲ.
WindowsServicePackRequired=ಈ ಪ್ರೋಗ್ರಾಂಗೆ %1 ಸರ್ವಿಸ್ ಪ್ಯಾಕ್ %2 ಅಥವಾ ನಂತರದ್ದು ಬೇಕು.
NotOnThisPlatform=ಈ ಪ್ರೋಗ್ರಾಂ %1 ನಲ್ಲಿ ಚಾಲನೆಯಾಗುವುದಿಲ್ಲ.
OnlyOnThisPlatform=ಈ ಪ್ರೋಗ್ರಾಂ ಅನ್ನು %1 ನಲ್ಲಿ ಮಾತ್ರ ಚಲಾಯಿಸಬೇಕು.
OnlyOnTheseArchitectures=ಈ ಪ್ರೋಗ್ರಾಂ ಅನ್ನು ಕೆಳಗಿನ ಪ್ರೊಸೆಸರ್ ಆರ್ಕಿಟೆಕ್ಚರ್‌ಗಳಿಗಾಗಿ ರೂಪಿಸಿದ Windows ಆವೃತ್ತಿಗಳಲ್ಲಿ ಮಾತ್ರ ಸ್ಥಾಪಿಸಬಹುದು:%n%n%1
WinVersionTooLowError=ಈ ಪ್ರೋಗ್ರಾಂಗೆ %1 ಆವೃತ್ತಿ %2 ಅಥವಾ ನಂತರದ್ದು ಬೇಕು.
WinVersionTooHighError=ಈ ಪ್ರೋಗ್ರಾಂ ಅನ್ನು %1 ಆವೃತ್ತಿ %2 ಅಥವಾ ನಂತರದ್ದರಲ್ಲಿ ಸ್ಥಾಪಿಸಲು ಸಾಧ್ಯವಿಲ್ಲ.
AdminPrivilegesRequired=ಈ ಪ್ರೋಗ್ರಾಂ ಅನ್ನು ಸ್ಥಾಪಿಸುವಾಗ ನೀವು ನಿರ್ವಾಹಕರಾಗಿ ಸೈನ್ ಇನ್ ಆಗಿರಬೇಕು.
PowerUserPrivilegesRequired=ಈ ಪ್ರೋಗ್ರಾಂ ಅನ್ನು ಸ್ಥಾಪಿಸುವಾಗ ನೀವು ನಿರ್ವಾಹಕರಾಗಿ ಅಥವಾ Power Users ಗುಂಪಿನ ಸದಸ್ಯರಾಗಿ ಸೈನ್ ಇನ್ ಆಗಿರಬೇಕು.
SetupAppRunningError=%1 ಪ್ರಸ್ತುತ ಚಾಲನೆಯಲ್ಲಿದೆ ಎಂದು ಸೆಟಪ್ ಪತ್ತೆಹಚ್ಚಿದೆ.%n%nದಯವಿಟ್ಟು ಅದರ ಎಲ್ಲ ನಿದರ್ಶನಗಳನ್ನು ಈಗ ಮುಚ್ಚಿ, ಮುಂದುವರಿಸಲು “ಸರಿ” ಅಥವಾ ನಿರ್ಗಮಿಸಲು “ರದ್ದು” ಒತ್ತಿ.
UninstallAppRunningError=%1 ಪ್ರಸ್ತುತ ಚಾಲನೆಯಲ್ಲಿದೆ ಎಂದು ಅಸ್ಥಾಪಕ ಪತ್ತೆಹಚ್ಚಿದೆ.%n%nದಯವಿಟ್ಟು ಅದರ ಎಲ್ಲ ನಿದರ್ಶನಗಳನ್ನು ಈಗ ಮುಚ್ಚಿ, ಮುಂದುವರಿಸಲು “ಸರಿ” ಅಥವಾ ನಿರ್ಗಮಿಸಲು “ರದ್ದು” ಒತ್ತಿ.

; *** Startup questions
PrivilegesRequiredOverrideTitle=ಸೆಟಪ್ ಸ್ಥಾಪನೆ ಮೋಡ್ ಆಯ್ಕೆಮಾಡಿ
PrivilegesRequiredOverrideInstruction=ಸ್ಥಾಪನೆ ಮೋಡ್ ಆಯ್ಕೆಮಾಡಿ
PrivilegesRequiredOverrideText1=%1 ಅನ್ನು ಎಲ್ಲ ಬಳಕೆದಾರರಿಗಾಗಿ (ನಿರ್ವಾಹಕ ಹಕ್ಕುಗಳು ಬೇಕು) ಅಥವಾ ನಿಮಗಾಗಿ ಮಾತ್ರ ಸ್ಥಾಪಿಸಬಹುದು.
PrivilegesRequiredOverrideText2=%1 ಅನ್ನು ನಿಮಗಾಗಿ ಮಾತ್ರ ಅಥವಾ ಎಲ್ಲ ಬಳಕೆದಾರರಿಗಾಗಿ (ನಿರ್ವಾಹಕ ಹಕ್ಕುಗಳು ಬೇಕು) ಸ್ಥಾಪಿಸಬಹುದು.
PrivilegesRequiredOverrideAllUsers=ಎಲ್ಲ ಬಳಕೆದಾರರಿಗಾಗಿ ಸ್ಥಾಪಿಸಿ
PrivilegesRequiredOverrideAllUsersRecommended=ಎಲ್ಲ ಬಳಕೆದಾರರಿಗಾಗಿ ಸ್ಥಾಪಿಸಿ (ಶಿಫಾರಸು ಮಾಡಲಾಗಿದೆ)
PrivilegesRequiredOverrideCurrentUser=ನನಗಾಗಿ ಮಾತ್ರ ಸ್ಥಾಪಿಸಿ
PrivilegesRequiredOverrideCurrentUserRecommended=ನನಗಾಗಿ ಮಾತ್ರ ಸ್ಥಾಪಿಸಿ (ಶಿಫಾರಸು ಮಾಡಲಾಗಿದೆ)

; *** Misc. errors
ErrorCreatingDir=ಸೆಟಪ್ "%1" ಡೈರೆಕ್ಟರಿಯನ್ನು ರಚಿಸಲು ಸಾಧ್ಯವಾಗಲಿಲ್ಲ
ErrorTooManyFilesInDir="%1" ಡೈರೆಕ್ಟರಿಯಲ್ಲಿ ಹಲವಾರು ಫೈಲ್‌ಗಳಿರುವುದರಿಂದ ಅದರಲ್ಲಿ ಫೈಲ್ ರಚಿಸಲು ಸಾಧ್ಯವಾಗಲಿಲ್ಲ

; *** Setup common messages
ExitSetupTitle=ಸೆಟಪ್‌ನಿಂದ ನಿರ್ಗಮಿಸಿ
ExitSetupMessage=ಸೆಟಪ್ ಪೂರ್ಣಗೊಂಡಿಲ್ಲ. ಈಗ ನಿರ್ಗಮಿಸಿದರೆ, ಪ್ರೋಗ್ರಾಂ ಸ್ಥಾಪನೆಯಾಗುವುದಿಲ್ಲ.%n%nಸ್ಥಾಪನೆಯನ್ನು ಪೂರ್ಣಗೊಳಿಸಲು ನೀವು ಸೆಟಪ್ ಅನ್ನು ಬೇರೆ ಸಮಯದಲ್ಲಿ ಮತ್ತೆ ಚಲಾಯಿಸಬಹುದು.%n%nಸೆಟಪ್‌ನಿಂದ ನಿರ್ಗಮಿಸಬೇಕೇ?
AboutSetupMenuItem=ಸೆಟಪ್ ಕುರಿತು...
AboutSetupTitle=ಸೆಟಪ್ ಕುರಿತು
AboutSetupMessage=%1 ಆವೃತ್ತಿ %2%n%3%n%n%1 ಮುಖಪುಟ:%n%4
AboutSetupNote=
TranslatorNote=

; *** Buttons
ButtonBack=< ಹಿಂದೆ
ButtonNext=ಮುಂದೆ >
ButtonInstall=ಸ್ಥಾಪಿಸಿ
ButtonOK=ಸರಿ
ButtonCancel=ರದ್ದು
ButtonYes=ಹೌದು
ButtonYesToAll=ಎಲ್ಲಕ್ಕೂ ಹೌದು
ButtonNo=ಇಲ್ಲ
ButtonNoToAll=ಎಲ್ಲಕ್ಕೂ ಇಲ್ಲ
ButtonFinish=ಮುಕ್ತಾಯ
ButtonBrowse=ಬ್ರೌಸ್ ಮಾಡಿ...
ButtonWizardBrowse=ಬ್ರೌಸ್ ಮಾಡಿ...
ButtonNewFolder=ಹೊಸ ಫೋಲ್ಡರ್ ರಚಿಸಿ

; *** "Select Language" dialog messages
SelectLanguageTitle=ಸೆಟಪ್ ಭಾಷೆಯನ್ನು ಆಯ್ಕೆಮಾಡಿ
SelectLanguageLabel=ಸ್ಥಾಪನೆಯ ಸಮಯದಲ್ಲಿ ಬಳಸಬೇಕಾದ ಭಾಷೆಯನ್ನು ಆಯ್ಕೆಮಾಡಿ.

; *** Common wizard text
ClickNext=ಮುಂದುವರಿಸಲು “ಮುಂದೆ” ಒತ್ತಿ, ಅಥವಾ ಸೆಟಪ್‌ನಿಂದ ನಿರ್ಗಮಿಸಲು “ರದ್ದು” ಒತ್ತಿ.
BeveledLabel=
BrowseDialogTitle=ಫೋಲ್ಡರ್‌ಗಾಗಿ ಬ್ರೌಸ್ ಮಾಡಿ
BrowseDialogLabel=ಕೆಳಗಿನ ಪಟ್ಟಿಯಲ್ಲಿ ಒಂದು ಫೋಲ್ಡರ್ ಆಯ್ಕೆಮಾಡಿ, ನಂತರ “ಸರಿ” ಒತ್ತಿ.
NewFolderName=ಹೊಸ ಫೋಲ್ಡರ್

; *** "Welcome" wizard page
WelcomeLabel1=[name] ಸೆಟಪ್ ವಿಝಾರ್ಡ್‌ಗೆ ಸ್ವಾಗತ
WelcomeLabel2=ಇದು ನಿಮ್ಮ ಕಂಪ್ಯೂಟರ್‌ನಲ್ಲಿ [name/ver] ಅನ್ನು ಸ್ಥಾಪಿಸುತ್ತದೆ.%n%nಮುಂದುವರಿಯುವ ಮೊದಲು ಉಳಿದ ಎಲ್ಲ ಅಪ್ಲಿಕೇಶನ್‌ಗಳನ್ನು ಮುಚ್ಚುವುದು ಒಳ್ಳೆಯದು.

; *** "Password" wizard page
WizardPassword=ಪಾಸ್‌ವರ್ಡ್
PasswordLabel1=ಈ ಸ್ಥಾಪನೆ ಪಾಸ್‌ವರ್ಡ್‌ನಿಂದ ರಕ್ಷಿತವಾಗಿದೆ.
PasswordLabel3=ದಯವಿಟ್ಟು ಪಾಸ್‌ವರ್ಡ್ ನೀಡಿ, ನಂತರ ಮುಂದುವರಿಸಲು “ಮುಂದೆ” ಒತ್ತಿ. ಪಾಸ್‌ವರ್ಡ್‌ಗಳಲ್ಲಿ ದೊಡ್ಡ ಮತ್ತು ಸಣ್ಣ ಅಕ್ಷರಗಳು ಬೇರೆ ಬೇರೆ ಎಂದು ಪರಿಗಣಿಸಲಾಗುತ್ತದೆ.
PasswordEditLabel=ಪಾಸ್‌ವರ್ಡ್:
IncorrectPassword=ನೀವು ನೀಡಿದ ಪಾಸ್‌ವರ್ಡ್ ಸರಿಯಾಗಿಲ್ಲ. ದಯವಿಟ್ಟು ಮತ್ತೆ ಪ್ರಯತ್ನಿಸಿ.

; *** "License Agreement" wizard page
WizardLicense=ಪರವಾನಗಿ ಒಪ್ಪಂದ
LicenseLabel=ಮುಂದುವರಿಯುವ ಮೊದಲು ದಯವಿಟ್ಟು ಈ ಪ್ರಮುಖ ಮಾಹಿತಿಯನ್ನು ಓದಿ.
LicenseLabel3=ದಯವಿಟ್ಟು ಕೆಳಗಿನ ಪರವಾನಗಿ ಒಪ್ಪಂದವನ್ನು ಓದಿ. ಸ್ಥಾಪನೆಯನ್ನು ಮುಂದುವರಿಸುವ ಮೊದಲು ನೀವು ಈ ಒಪ್ಪಂದದ ನಿಯಮಗಳನ್ನು ಒಪ್ಪಿಕೊಳ್ಳಬೇಕು.
LicenseAccepted=ನಾನು ಒಪ್ಪಂದವನ್ನು ಒಪ್ಪಿಕೊಳ್ಳುತ್ತೇನೆ
LicenseNotAccepted=ನಾನು ಒಪ್ಪಂದವನ್ನು ಒಪ್ಪಿಕೊಳ್ಳುವುದಿಲ್ಲ

; *** "Information" wizard pages
WizardInfoBefore=ಮಾಹಿತಿ
InfoBeforeLabel=ಮುಂದುವರಿಯುವ ಮೊದಲು ದಯವಿಟ್ಟು ಈ ಪ್ರಮುಖ ಮಾಹಿತಿಯನ್ನು ಓದಿ.
InfoBeforeClickLabel=ಸೆಟಪ್ ಮುಂದುವರಿಸಲು ಸಿದ್ಧವಾದಾಗ “ಮುಂದೆ” ಒತ್ತಿ.
WizardInfoAfter=ಮಾಹಿತಿ
InfoAfterLabel=ಮುಂದುವರಿಯುವ ಮೊದಲು ದಯವಿಟ್ಟು ಈ ಪ್ರಮುಖ ಮಾಹಿತಿಯನ್ನು ಓದಿ.
InfoAfterClickLabel=ಸೆಟಪ್ ಮುಂದುವರಿಸಲು ಸಿದ್ಧವಾದಾಗ “ಮುಂದೆ” ಒತ್ತಿ.

; *** "User Information" wizard page
WizardUserInfo=ಬಳಕೆದಾರರ ಮಾಹಿತಿ
UserInfoDesc=ದಯವಿಟ್ಟು ನಿಮ್ಮ ವಿವರಗಳನ್ನು ನಮೂದಿಸಿ.
UserInfoName=ಬಳಕೆದಾರರ ಹೆಸರು:
UserInfoOrg=ಸಂಸ್ಥೆ:
UserInfoSerial=ಕ್ರಮಸಂಖ್ಯೆ:
UserInfoNameRequired=ನೀವು ಹೆಸರನ್ನು ನಮೂದಿಸಬೇಕು.

; *** "Select Destination Location" wizard page
WizardSelectDir=ಗಮ್ಯಸ್ಥಾನವನ್ನು ಆಯ್ಕೆಮಾಡಿ
SelectDirDesc=[name] ಅನ್ನು ಎಲ್ಲಿ ಸ್ಥಾಪಿಸಬೇಕು?
SelectDirLabel3=ಸೆಟಪ್ [name] ಅನ್ನು ಕೆಳಗಿನ ಫೋಲ್ಡರ್‌ನಲ್ಲಿ ಸ್ಥಾಪಿಸುತ್ತದೆ.
SelectDirBrowseLabel=ಮುಂದುವರಿಸಲು “ಮುಂದೆ” ಒತ್ತಿ. ಬೇರೆ ಫೋಲ್ಡರ್ ಆಯ್ಕೆಮಾಡಲು ಬಯಸಿದರೆ “ಬ್ರೌಸ್ ಮಾಡಿ” ಒತ್ತಿ.
DiskSpaceGBLabel=ಕನಿಷ್ಠ [gb] GB ಖಾಲಿ ಡಿಸ್ಕ್ ಸ್ಥಳ ಬೇಕು.
DiskSpaceMBLabel=ಕನಿಷ್ಠ [mb] MB ಖಾಲಿ ಡಿಸ್ಕ್ ಸ್ಥಳ ಬೇಕು.
CannotInstallToNetworkDrive=ಸೆಟಪ್ ನೆಟ್‌ವರ್ಕ್ ಡ್ರೈವ್‌ನಲ್ಲಿ ಸ್ಥಾಪಿಸಲು ಸಾಧ್ಯವಿಲ್ಲ.
CannotInstallToUNCPath=ಸೆಟಪ್ UNC ಪಾತ್‌ನಲ್ಲಿ ಸ್ಥಾಪಿಸಲು ಸಾಧ್ಯವಿಲ್ಲ.
InvalidPath=ನೀವು ಡ್ರೈವ್ ಅಕ್ಷರದೊಂದಿಗೆ ಪೂರ್ಣ ಪಾತ್ ನಮೂದಿಸಬೇಕು; ಉದಾಹರಣೆಗೆ:%n%nC:\APP%n%nಅಥವಾ ಈ ರೂಪದಲ್ಲಿ UNC ಪಾತ್:%n%n\\server\share
InvalidDrive=ನೀವು ಆಯ್ಕೆಮಾಡಿದ ಡ್ರೈವ್ ಅಥವಾ UNC ಶೇರ್ ಅಸ್ತಿತ್ವದಲ್ಲಿಲ್ಲ ಅಥವಾ ಲಭ್ಯವಿಲ್ಲ. ದಯವಿಟ್ಟು ಬೇರೆಯದನ್ನು ಆಯ್ಕೆಮಾಡಿ.
DiskSpaceWarningTitle=ಡಿಸ್ಕ್ ಸ್ಥಳ ಸಾಲದು
DiskSpaceWarning=ಸ್ಥಾಪಿಸಲು ಸೆಟಪ್‌ಗೆ ಕನಿಷ್ಠ %1 KB ಖಾಲಿ ಸ್ಥಳ ಬೇಕು, ಆದರೆ ಆಯ್ಕೆಮಾಡಿದ ಡ್ರೈವ್‌ನಲ್ಲಿ %2 KB ಮಾತ್ರ ಲಭ್ಯವಿದೆ.%n%nಹಾಗಿದ್ದರೂ ಮುಂದುವರಿಸಲು ಬಯಸುತ್ತೀರಾ?
DirNameTooLong=ಫೋಲ್ಡರ್ ಹೆಸರು ಅಥವಾ ಪಾತ್ ತುಂಬಾ ಉದ್ದವಾಗಿದೆ.
InvalidDirName=ಫೋಲ್ಡರ್ ಹೆಸರು ಅಮಾನ್ಯವಾಗಿದೆ.
BadDirName32=ಫೋಲ್ಡರ್ ಹೆಸರುಗಳಲ್ಲಿ ಕೆಳಗಿನ ಅಕ್ಷರಗಳು ಇರಬಾರದು:%n%n%1
DirExistsTitle=ಫೋಲ್ಡರ್ ಅಸ್ತಿತ್ವದಲ್ಲಿದೆ
DirExists=ಈ ಫೋಲ್ಡರ್:%n%n%1%n%nಈಗಾಗಲೇ ಅಸ್ತಿತ್ವದಲ್ಲಿದೆ. ಹಾಗಿದ್ದರೂ ಆ ಫೋಲ್ಡರ್‌ನಲ್ಲೇ ಸ್ಥಾಪಿಸಲು ಬಯಸುತ್ತೀರಾ?
DirDoesntExistTitle=ಫೋಲ್ಡರ್ ಅಸ್ತಿತ್ವದಲ್ಲಿಲ್ಲ
DirDoesntExist=ಈ ಫೋಲ್ಡರ್:%n%n%1%n%nಅಸ್ತಿತ್ವದಲ್ಲಿಲ್ಲ. ಫೋಲ್ಡರ್ ಅನ್ನು ರಚಿಸಲು ಬಯಸುತ್ತೀರಾ?

; *** "Select Components" wizard page
WizardSelectComponents=ಘಟಕಗಳನ್ನು ಆಯ್ಕೆಮಾಡಿ
SelectComponentsDesc=ಯಾವ ಘಟಕಗಳನ್ನು ಸ್ಥಾಪಿಸಬೇಕು?
SelectComponentsLabel2=ನೀವು ಸ್ಥಾಪಿಸಲು ಬಯಸುವ ಘಟಕಗಳನ್ನು ಆಯ್ಕೆಮಾಡಿ; ಬೇಡದ ಘಟಕಗಳ ಆಯ್ಕೆಯನ್ನು ತೆಗೆಯಿರಿ. ಸಿದ್ಧವಾದಾಗ “ಮುಂದೆ” ಒತ್ತಿ.
FullInstallation=ಪೂರ್ಣ ಸ್ಥಾಪನೆ
; if possible don't translate 'Compact' as 'Minimal' (I mean 'Minimal' in your language)
CompactInstallation=ಸಂಕ್ಷಿಪ್ತ ಸ್ಥಾಪನೆ
CustomInstallation=ಕಸ್ಟಮ್ ಸ್ಥಾಪನೆ
NoUninstallWarningTitle=ಘಟಕಗಳು ಅಸ್ತಿತ್ವದಲ್ಲಿವೆ
NoUninstallWarning=ಕೆಳಗಿನ ಘಟಕಗಳು ನಿಮ್ಮ ಕಂಪ್ಯೂಟರ್‌ನಲ್ಲಿ ಈಗಾಗಲೇ ಸ್ಥಾಪಿತವಾಗಿವೆ ಎಂದು ಸೆಟಪ್ ಪತ್ತೆಹಚ್ಚಿದೆ:%n%n%1%n%nಈ ಘಟಕಗಳ ಆಯ್ಕೆ ತೆಗೆದರೆ ಅವು ಅಸ್ಥಾಪನೆಯಾಗುವುದಿಲ್ಲ.%n%nಹಾಗಿದ್ದರೂ ಮುಂದುವರಿಸಲು ಬಯಸುತ್ತೀರಾ?
ComponentSize1=%1 KB
ComponentSize2=%1 MB
ComponentsDiskSpaceGBLabel=ಪ್ರಸ್ತುತ ಆಯ್ಕೆಗೆ ಕನಿಷ್ಠ [gb] GB ಡಿಸ್ಕ್ ಸ್ಥಳ ಬೇಕು.
ComponentsDiskSpaceMBLabel=ಪ್ರಸ್ತುತ ಆಯ್ಕೆಗೆ ಕನಿಷ್ಠ [mb] MB ಡಿಸ್ಕ್ ಸ್ಥಳ ಬೇಕು.

; *** "Select Additional Tasks" wizard page
WizardSelectTasks=ಹೆಚ್ಚುವರಿ ಕಾರ್ಯಗಳನ್ನು ಆಯ್ಕೆಮಾಡಿ
SelectTasksDesc=ಯಾವ ಹೆಚ್ಚುವರಿ ಕಾರ್ಯಗಳನ್ನು ನಿರ್ವಹಿಸಬೇಕು?
SelectTasksLabel2=[name] ಅನ್ನು ಸ್ಥಾಪಿಸುವಾಗ ಸೆಟಪ್ ನಿರ್ವಹಿಸಬೇಕೆಂದು ನೀವು ಬಯಸುವ ಹೆಚ್ಚುವರಿ ಕಾರ್ಯಗಳನ್ನು ಆಯ್ಕೆಮಾಡಿ, ನಂತರ “ಮುಂದೆ” ಒತ್ತಿ.

; *** "Select Start Menu Folder" wizard page
WizardSelectProgramGroup=ಪ್ರಾರಂಭ ಮೆನು ಫೋಲ್ಡರ್ ಆಯ್ಕೆಮಾಡಿ
SelectStartMenuFolderDesc=ಪ್ರೋಗ್ರಾಂನ ಶಾರ್ಟ್‌ಕಟ್‌ಗಳನ್ನು ಸೆಟಪ್ ಎಲ್ಲಿ ಇಡಬೇಕು?
SelectStartMenuFolderLabel3=ಸೆಟಪ್ ಪ್ರೋಗ್ರಾಂನ ಶಾರ್ಟ್‌ಕಟ್‌ಗಳನ್ನು ಕೆಳಗಿನ ಪ್ರಾರಂಭ ಮೆನು ಫೋಲ್ಡರ್‌ನಲ್ಲಿ ರಚಿಸುತ್ತದೆ.
SelectStartMenuFolderBrowseLabel=ಮುಂದುವರಿಸಲು “ಮುಂದೆ” ಒತ್ತಿ. ಬೇರೆ ಫೋಲ್ಡರ್ ಆಯ್ಕೆಮಾಡಲು ಬಯಸಿದರೆ “ಬ್ರೌಸ್ ಮಾಡಿ” ಒತ್ತಿ.
MustEnterGroupName=ನೀವು ಫೋಲ್ಡರ್ ಹೆಸರನ್ನು ನಮೂದಿಸಬೇಕು.
GroupNameTooLong=ಫೋಲ್ಡರ್ ಹೆಸರು ಅಥವಾ ಪಾತ್ ತುಂಬಾ ಉದ್ದವಾಗಿದೆ.
InvalidGroupName=ಫೋಲ್ಡರ್ ಹೆಸರು ಅಮಾನ್ಯವಾಗಿದೆ.
BadGroupName=ಫೋಲ್ಡರ್ ಹೆಸರಿನಲ್ಲಿ ಕೆಳಗಿನ ಅಕ್ಷರಗಳು ಇರಬಾರದು:%n%n%1
NoProgramGroupCheck2=ಪ್ರಾರಂಭ ಮೆನು ಫೋಲ್ಡರ್ ರಚಿಸಬೇಡಿ

; *** "Ready to Install" wizard page
WizardReady=ಸ್ಥಾಪಿಸಲು ಸಿದ್ಧ
ReadyLabel1=ಸೆಟಪ್ ಈಗ ನಿಮ್ಮ ಕಂಪ್ಯೂಟರ್‌ನಲ್ಲಿ [name] ಅನ್ನು ಸ್ಥಾಪಿಸಲು ಪ್ರಾರಂಭಿಸಲು ಸಿದ್ಧವಾಗಿದೆ.
ReadyLabel2a=ಸ್ಥಾಪನೆಯನ್ನು ಮುಂದುವರಿಸಲು “ಸ್ಥಾಪಿಸಿ” ಒತ್ತಿ, ಅಥವಾ ಯಾವುದೇ ಸೆಟ್ಟಿಂಗ್‌ಗಳನ್ನು ಪರಿಶೀಲಿಸಲು ಅಥವಾ ಬದಲಾಯಿಸಲು ಬಯಸಿದರೆ “ಹಿಂದೆ” ಒತ್ತಿ.
ReadyLabel2b=ಸ್ಥಾಪನೆಯನ್ನು ಮುಂದುವರಿಸಲು “ಸ್ಥಾಪಿಸಿ” ಒತ್ತಿ.
ReadyMemoUserInfo=ಬಳಕೆದಾರರ ಮಾಹಿತಿ:
ReadyMemoDir=ಗಮ್ಯಸ್ಥಾನ:
ReadyMemoType=ಸೆಟಪ್ ಪ್ರಕಾರ:
ReadyMemoComponents=ಆಯ್ಕೆಮಾಡಿದ ಘಟಕಗಳು:
ReadyMemoGroup=ಪ್ರಾರಂಭ ಮೆನು ಫೋಲ್ಡರ್:
ReadyMemoTasks=ಹೆಚ್ಚುವರಿ ಕಾರ್ಯಗಳು:

; *** TDownloadWizardPage wizard page and DownloadTemporaryFile
DownloadingLabel2=ಫೈಲ್‌ಗಳನ್ನು ಡೌನ್‌ಲೋಡ್ ಮಾಡಲಾಗುತ್ತಿದೆ...
ButtonStopDownload=ಡೌನ್‌ಲೋಡ್ ನಿಲ್ಲಿಸಿ
StopDownload=ಡೌನ್‌ಲೋಡ್ ನಿಲ್ಲಿಸಬೇಕೆಂದು ನಿಮಗೆ ಖಚಿತವಾಗಿದೆಯೇ?
ErrorDownloadAborted=ಡೌನ್‌ಲೋಡ್ ಸ್ಥಗಿತಗೊಂಡಿದೆ
ErrorDownloadFailed=ಡೌನ್‌ಲೋಡ್ ವಿಫಲವಾಯಿತು: %1 %2
ErrorDownloadSizeFailed=ಗಾತ್ರ ಪಡೆಯುವುದು ವಿಫಲವಾಯಿತು: %1 %2
ErrorProgress=ಅಮಾನ್ಯ ಪ್ರಗತಿ: %2 ರಲ್ಲಿ %1
ErrorFileSize=ಅಮಾನ್ಯ ಫೈಲ್ ಗಾತ್ರ: %1 ನಿರೀಕ್ಷಿಸಲಾಗಿತ್ತು, %2 ಕಂಡುಬಂದಿದೆ

; *** TExtractionWizardPage wizard page and ExtractArchive
ExtractingLabel=ಫೈಲ್‌ಗಳನ್ನು ಹೊರತೆಗೆಯಲಾಗುತ್ತಿದೆ...
ButtonStopExtraction=ಹೊರತೆಗೆಯುವುದನ್ನು ನಿಲ್ಲಿಸಿ
StopExtraction=ಹೊರತೆಗೆಯುವುದನ್ನು ನಿಲ್ಲಿಸಬೇಕೆಂದು ನಿಮಗೆ ಖಚಿತವಾಗಿದೆಯೇ?
ErrorExtractionAborted=ಹೊರತೆಗೆಯುವಿಕೆ ಸ್ಥಗಿತಗೊಂಡಿದೆ
ErrorExtractionFailed=ಹೊರತೆಗೆಯುವಿಕೆ ವಿಫಲವಾಯಿತು: %1

; *** Archive extraction failure details
ArchiveIncorrectPassword=ಪಾಸ್‌ವರ್ಡ್ ತಪ್ಪಾಗಿದೆ
ArchiveIsCorrupted=ಆರ್ಕೈವ್ ಹಾಳಾಗಿದೆ
ArchiveUnsupportedFormat=ಆರ್ಕೈವ್ ಸ್ವರೂಪಕ್ಕೆ ಬೆಂಬಲವಿಲ್ಲ

; *** "Preparing to Install" wizard page
WizardPreparing=ಸ್ಥಾಪನೆಗೆ ಸಿದ್ಧಪಡಿಸಲಾಗುತ್ತಿದೆ
PreparingDesc=ಸೆಟಪ್ ನಿಮ್ಮ ಕಂಪ್ಯೂಟರ್‌ನಲ್ಲಿ [name] ಅನ್ನು ಸ್ಥಾಪಿಸಲು ಸಿದ್ಧವಾಗುತ್ತಿದೆ.
PreviousInstallNotCompleted=ಹಿಂದಿನ ಪ್ರೋಗ್ರಾಂನ ಸ್ಥಾಪನೆ ಅಥವಾ ತೆಗೆದುಹಾಕುವಿಕೆ ಪೂರ್ಣಗೊಂಡಿಲ್ಲ. ಆ ಸ್ಥಾಪನೆಯನ್ನು ಪೂರ್ಣಗೊಳಿಸಲು ನೀವು ಕಂಪ್ಯೂಟರ್ ಅನ್ನು ಮರುಪ್ರಾರಂಭಿಸಬೇಕು.%n%nಕಂಪ್ಯೂಟರ್ ಮರುಪ್ರಾರಂಭಿಸಿದ ನಂತರ, [name] ಸ್ಥಾಪನೆಯನ್ನು ಪೂರ್ಣಗೊಳಿಸಲು ಸೆಟಪ್ ಅನ್ನು ಮತ್ತೆ ಚಲಾಯಿಸಿ.
CannotContinue=ಸೆಟಪ್ ಮುಂದುವರಿಸಲು ಸಾಧ್ಯವಿಲ್ಲ. ನಿರ್ಗಮಿಸಲು ದಯವಿಟ್ಟು “ರದ್ದು” ಒತ್ತಿ.
ApplicationsFound=ಸೆಟಪ್ ನವೀಕರಿಸಬೇಕಾದ ಫೈಲ್‌ಗಳನ್ನು ಕೆಳಗಿನ ಅಪ್ಲಿಕೇಶನ್‌ಗಳು ಬಳಸುತ್ತಿವೆ. ಈ ಅಪ್ಲಿಕೇಶನ್‌ಗಳನ್ನು ಸೆಟಪ್ ತಾನಾಗಿ ಮುಚ್ಚಲು ಅನುಮತಿಸುವುದು ಒಳ್ಳೆಯದು.
ApplicationsFound2=ಸೆಟಪ್ ನವೀಕರಿಸಬೇಕಾದ ಫೈಲ್‌ಗಳನ್ನು ಕೆಳಗಿನ ಅಪ್ಲಿಕೇಶನ್‌ಗಳು ಬಳಸುತ್ತಿವೆ. ಈ ಅಪ್ಲಿಕೇಶನ್‌ಗಳನ್ನು ಸೆಟಪ್ ತಾನಾಗಿ ಮುಚ್ಚಲು ಅನುಮತಿಸುವುದು ಒಳ್ಳೆಯದು. ಸ್ಥಾಪನೆ ಪೂರ್ಣಗೊಂಡ ನಂತರ, ಸೆಟಪ್ ಆ ಅಪ್ಲಿಕೇಶನ್‌ಗಳನ್ನು ಮತ್ತೆ ಪ್ರಾರಂಭಿಸಲು ಪ್ರಯತ್ನಿಸುತ್ತದೆ.
CloseApplications=ಅಪ್ಲಿಕೇಶನ್‌ಗಳನ್ನು ಸ್ವಯಂಚಾಲಿತವಾಗಿ ಮುಚ್ಚಿ
DontCloseApplications=ಅಪ್ಲಿಕೇಶನ್‌ಗಳನ್ನು ಮುಚ್ಚಬೇಡಿ
ErrorCloseApplications=ಎಲ್ಲ ಅಪ್ಲಿಕೇಶನ್‌ಗಳನ್ನು ತಾನಾಗಿ ಮುಚ್ಚಲು ಸೆಟಪ್‌ಗೆ ಸಾಧ್ಯವಾಗಲಿಲ್ಲ. ಮುಂದುವರಿಯುವ ಮೊದಲು ಸೆಟಪ್ ನವೀಕರಿಸಬೇಕಾದ ಫೈಲ್‌ಗಳನ್ನು ಬಳಸುತ್ತಿರುವ ಎಲ್ಲ ಅಪ್ಲಿಕೇಶನ್‌ಗಳನ್ನು ಮುಚ್ಚುವುದು ಒಳ್ಳೆಯದು.
PrepareToInstallNeedsRestart=ಸೆಟಪ್ ನಿಮ್ಮ ಕಂಪ್ಯೂಟರ್ ಅನ್ನು ಮರುಪ್ರಾರಂಭಿಸಬೇಕು. ಕಂಪ್ಯೂಟರ್ ಮರುಪ್ರಾರಂಭಿಸಿದ ನಂತರ, [name] ಸ್ಥಾಪನೆಯನ್ನು ಪೂರ್ಣಗೊಳಿಸಲು ಸೆಟಪ್ ಅನ್ನು ಮತ್ತೆ ಚಲಾಯಿಸಿ.%n%nಈಗ ಮರುಪ್ರಾರಂಭಿಸಬೇಕೇ?

; *** "Installing" wizard page
WizardInstalling=ಸ್ಥಾಪಿಸಲಾಗುತ್ತಿದೆ
InstallingLabel=ಸೆಟಪ್ ನಿಮ್ಮ ಕಂಪ್ಯೂಟರ್‌ನಲ್ಲಿ [name] ಅನ್ನು ಸ್ಥಾಪಿಸುತ್ತಿದೆ, ದಯವಿಟ್ಟು ಕಾಯಿರಿ.

; *** "Setup Completed" wizard page
FinishedHeadingLabel=[name] ಸೆಟಪ್ ವಿಝಾರ್ಡ್ ಪೂರ್ಣಗೊಳ್ಳುತ್ತಿದೆ
FinishedLabelNoIcons=ಸೆಟಪ್ ನಿಮ್ಮ ಕಂಪ್ಯೂಟರ್‌ನಲ್ಲಿ [name] ಸ್ಥಾಪನೆಯನ್ನು ಮುಗಿಸಿದೆ.
FinishedLabel=ಸೆಟಪ್ ನಿಮ್ಮ ಕಂಪ್ಯೂಟರ್‌ನಲ್ಲಿ [name] ಸ್ಥಾಪನೆಯನ್ನು ಮುಗಿಸಿದೆ. ಸ್ಥಾಪಿತ ಶಾರ್ಟ್‌ಕಟ್‌ಗಳನ್ನು ಆಯ್ಕೆಮಾಡುವ ಮೂಲಕ ಅಪ್ಲಿಕೇಶನ್ ಅನ್ನು ಪ್ರಾರಂಭಿಸಬಹುದು.
ClickFinish=ಸೆಟಪ್‌ನಿಂದ ನಿರ್ಗಮಿಸಲು “ಮುಕ್ತಾಯ” ಒತ್ತಿ.
FinishedRestartLabel=[name] ಸ್ಥಾಪನೆಯನ್ನು ಪೂರ್ಣಗೊಳಿಸಲು ಸೆಟಪ್ ನಿಮ್ಮ ಕಂಪ್ಯೂಟರ್ ಅನ್ನು ಮರುಪ್ರಾರಂಭಿಸಬೇಕು. ಈಗ ಮರುಪ್ರಾರಂಭಿಸಬೇಕೇ?
FinishedRestartMessage=[name] ಸ್ಥಾಪನೆಯನ್ನು ಪೂರ್ಣಗೊಳಿಸಲು ಸೆಟಪ್ ನಿಮ್ಮ ಕಂಪ್ಯೂಟರ್ ಅನ್ನು ಮರುಪ್ರಾರಂಭಿಸಬೇಕು.%n%nಈಗ ಮರುಪ್ರಾರಂಭಿಸಬೇಕೇ?
ShowReadmeCheck=ಹೌದು, README ಫೈಲ್ ನೋಡಲು ಬಯಸುತ್ತೇನೆ
YesRadio=ಹೌದು, ಕಂಪ್ಯೂಟರ್ ಅನ್ನು ಈಗಲೇ ಮರುಪ್ರಾರಂಭಿಸಿ
NoRadio=ಇಲ್ಲ, ಕಂಪ್ಯೂಟರ್ ಅನ್ನು ನಾನು ನಂತರ ಮರುಪ್ರಾರಂಭಿಸುತ್ತೇನೆ
; used for example as 'Run MyProg.exe'
RunEntryExec=%1 ಅನ್ನು ಚಲಾಯಿಸಿ
; used for example as 'View Readme.txt'
RunEntryShellExec=%1 ಅನ್ನು ನೋಡಿ

; *** "Setup Needs the Next Disk" stuff
ChangeDiskTitle=ಸೆಟಪ್‌ಗೆ ಮುಂದಿನ ಡಿಸ್ಕ್ ಬೇಕು
SelectDiskLabel2=ದಯವಿಟ್ಟು ಡಿಸ್ಕ್ %1 ಅನ್ನು ಸೇರಿಸಿ, “ಸರಿ” ಒತ್ತಿ.%n%nಈ ಡಿಸ್ಕ್‌ನ ಫೈಲ್‌ಗಳು ಕೆಳಗೆ ತೋರಿಸಿದ ಫೋಲ್ಡರ್‌ಗಿಂತ ಬೇರೆ ಫೋಲ್ಡರ್‌ನಲ್ಲಿದ್ದರೆ, ಸರಿಯಾದ ಪಾತ್ ನಮೂದಿಸಿ ಅಥವಾ “ಬ್ರೌಸ್ ಮಾಡಿ” ಒತ್ತಿ.
PathLabel=ಪಾತ್:
FileNotInDir2="%2" ನಲ್ಲಿ "%1" ಫೈಲ್ ಕಂಡುಬಂದಿಲ್ಲ. ದಯವಿಟ್ಟು ಸರಿಯಾದ ಡಿಸ್ಕ್ ಸೇರಿಸಿ ಅಥವಾ ಬೇರೆ ಫೋಲ್ಡರ್ ಆಯ್ಕೆಮಾಡಿ.
SelectDirectoryLabel=ದಯವಿಟ್ಟು ಮುಂದಿನ ಡಿಸ್ಕ್‌ನ ಸ್ಥಳವನ್ನು ಸೂಚಿಸಿ.

; *** Installation phase messages
SetupAborted=ಸೆಟಪ್ ಪೂರ್ಣಗೊಂಡಿಲ್ಲ.%n%nದಯವಿಟ್ಟು ಸಮಸ್ಯೆಯನ್ನು ಸರಿಪಡಿಸಿ ಸೆಟಪ್ ಅನ್ನು ಮತ್ತೆ ಚಲಾಯಿಸಿ.
AbortRetryIgnoreSelectAction=ಕ್ರಿಯೆಯನ್ನು ಆಯ್ಕೆಮಾಡಿ
AbortRetryIgnoreRetry=ಮತ್ತೆ ಪ್ರಯತ್ನಿಸಿ
AbortRetryIgnoreIgnore=ದೋಷವನ್ನು ನಿರ್ಲಕ್ಷಿಸಿ ಮುಂದುವರಿಸಿ
AbortRetryIgnoreCancel=ಸ್ಥಾಪನೆಯನ್ನು ರದ್ದುಮಾಡಿ
RetryCancelSelectAction=ಕ್ರಿಯೆಯನ್ನು ಆಯ್ಕೆಮಾಡಿ
RetryCancelRetry=ಮತ್ತೆ ಪ್ರಯತ್ನಿಸಿ
RetryCancelCancel=ರದ್ದು

; *** Installation status messages
StatusClosingApplications=ಅಪ್ಲಿಕೇಶನ್‌ಗಳನ್ನು ಮುಚ್ಚಲಾಗುತ್ತಿದೆ...
StatusCreateDirs=ಡೈರೆಕ್ಟರಿಗಳನ್ನು ರಚಿಸಲಾಗುತ್ತಿದೆ...
StatusExtractFiles=ಫೈಲ್‌ಗಳನ್ನು ಹೊರತೆಗೆಯಲಾಗುತ್ತಿದೆ...
StatusDownloadFiles=ಫೈಲ್‌ಗಳನ್ನು ಡೌನ್‌ಲೋಡ್ ಮಾಡಲಾಗುತ್ತಿದೆ...
StatusCreateIcons=ಶಾರ್ಟ್‌ಕಟ್‌ಗಳನ್ನು ರಚಿಸಲಾಗುತ್ತಿದೆ...
StatusCreateIniEntries=INI ನಮೂದುಗಳನ್ನು ರಚಿಸಲಾಗುತ್ತಿದೆ...
StatusCreateRegistryEntries=ರಿಜಿಸ್ಟ್ರಿ ನಮೂದುಗಳನ್ನು ರಚಿಸಲಾಗುತ್ತಿದೆ...
StatusRegisterFiles=ಫೈಲ್‌ಗಳನ್ನು ನೋಂದಾಯಿಸಲಾಗುತ್ತಿದೆ...
StatusSavingUninstall=ಅಸ್ಥಾಪನೆ ಮಾಹಿತಿಯನ್ನು ಉಳಿಸಲಾಗುತ್ತಿದೆ...
StatusRunProgram=ಸ್ಥಾಪನೆಯನ್ನು ಮುಗಿಸಲಾಗುತ್ತಿದೆ...
StatusRestartingApplications=ಅಪ್ಲಿಕೇಶನ್‌ಗಳನ್ನು ಮರುಪ್ರಾರಂಭಿಸಲಾಗುತ್ತಿದೆ...
StatusRollback=ಬದಲಾವಣೆಗಳನ್ನು ಹಿಂತೆಗೆದುಕೊಳ್ಳಲಾಗುತ್ತಿದೆ...

; *** Misc. errors
ErrorInternal2=ಆಂತರಿಕ ದೋಷ: %1
ErrorFunctionFailedNoCode=%1 ವಿಫಲವಾಯಿತು
ErrorFunctionFailed=%1 ವಿಫಲವಾಯಿತು; ಕೋಡ್ %2
ErrorFunctionFailedWithMessage=%1 ವಿಫಲವಾಯಿತು; ಕೋಡ್ %2.%n%3
ErrorExecutingProgram=ಫೈಲ್ ಅನ್ನು ಚಲಾಯಿಸಲು ಸಾಧ್ಯವಾಗಲಿಲ್ಲ:%n%1

; *** Registry errors
ErrorRegOpenKey=ರಿಜಿಸ್ಟ್ರಿ ಕೀ ತೆರೆಯುವಲ್ಲಿ ದೋಷ:%n%1\%2
ErrorRegCreateKey=ರಿಜಿಸ್ಟ್ರಿ ಕೀ ರಚಿಸುವಲ್ಲಿ ದೋಷ:%n%1\%2
ErrorRegWriteKey=ರಿಜಿಸ್ಟ್ರಿ ಕೀಗೆ ಬರೆಯುವಲ್ಲಿ ದೋಷ:%n%1\%2

; *** INI errors
ErrorIniEntry="%1" ಫೈಲ್‌ನಲ್ಲಿ INI ನಮೂದು ರಚಿಸುವಲ್ಲಿ ದೋಷ.

; *** File copying errors
FileAbortRetryIgnoreSkipNotRecommended=ಈ ಫೈಲ್ ಅನ್ನು ಬಿಟ್ಟುಬಿಡಿ (ಶಿಫಾರಸು ಮಾಡಲಾಗುವುದಿಲ್ಲ)
FileAbortRetryIgnoreIgnoreNotRecommended=ದೋಷವನ್ನು ನಿರ್ಲಕ್ಷಿಸಿ ಮುಂದುವರಿಸಿ (ಶಿಫಾರಸು ಮಾಡಲಾಗುವುದಿಲ್ಲ)
SourceIsCorrupted=ಮೂಲ ಫೈಲ್ ಹಾಳಾಗಿದೆ
SourceDoesntExist="%1" ಮೂಲ ಫೈಲ್ ಅಸ್ತಿತ್ವದಲ್ಲಿಲ್ಲ
SourceVerificationFailed=ಮೂಲ ಫೈಲ್‌ನ ಪರಿಶೀಲನೆ ವಿಫಲವಾಯಿತು: %1
VerificationSignatureDoesntExist="%1" ಸಹಿ ಫೈಲ್ ಅಸ್ತಿತ್ವದಲ್ಲಿಲ್ಲ
VerificationSignatureInvalid="%1" ಸಹಿ ಫೈಲ್ ಅಮಾನ್ಯವಾಗಿದೆ
VerificationKeyNotFound="%1" ಸಹಿ ಫೈಲ್ ಅಪರಿಚಿತ ಕೀ ಬಳಸುತ್ತದೆ
VerificationFileNameIncorrect=ಫೈಲ್‌ನ ಹೆಸರು ತಪ್ಪಾಗಿದೆ
VerificationFileTagIncorrect=ಫೈಲ್‌ನ ಟ್ಯಾಗ್ ತಪ್ಪಾಗಿದೆ
VerificationFileSizeIncorrect=ಫೈಲ್‌ನ ಗಾತ್ರ ತಪ್ಪಾಗಿದೆ
VerificationFileHashIncorrect=ಫೈಲ್‌ನ ಹ್ಯಾಶ್ ತಪ್ಪಾಗಿದೆ
ExistingFileReadOnly2=ಅಸ್ತಿತ್ವದಲ್ಲಿರುವ ಫೈಲ್ ಅನ್ನು ಓದಲು-ಮಾತ್ರ ಎಂದು ಗುರುತಿಸಿರುವುದರಿಂದ ಅದನ್ನು ಬದಲಿಸಲು ಸಾಧ್ಯವಾಗಲಿಲ್ಲ.
ExistingFileReadOnlyRetry=ಓದಲು-ಮಾತ್ರ ಗುಣಲಕ್ಷಣ ತೆಗೆದು ಮತ್ತೆ ಪ್ರಯತ್ನಿಸಿ
ExistingFileReadOnlyKeepExisting=ಅಸ್ತಿತ್ವದಲ್ಲಿರುವ ಫೈಲ್ ಅನ್ನು ಉಳಿಸಿಕೊಳ್ಳಿ
ErrorReadingExistingDest=ಅಸ್ತಿತ್ವದಲ್ಲಿರುವ ಫೈಲ್ ಅನ್ನು ಓದಲು ಪ್ರಯತ್ನಿಸುವಾಗ ದೋಷ ಉಂಟಾಯಿತು:
FileExistsSelectAction=ಕ್ರಿಯೆಯನ್ನು ಆಯ್ಕೆಮಾಡಿ
FileExists2=ಫೈಲ್ ಈಗಾಗಲೇ ಅಸ್ತಿತ್ವದಲ್ಲಿದೆ.
FileExistsOverwriteExisting=ಅಸ್ತಿತ್ವದಲ್ಲಿರುವ ಫೈಲ್ ಅನ್ನು ಬದಲಿಸಿ
FileExistsKeepExisting=ಅಸ್ತಿತ್ವದಲ್ಲಿರುವ ಫೈಲ್ ಅನ್ನು ಉಳಿಸಿಕೊಳ್ಳಿ
FileExistsOverwriteOrKeepAll=ಮುಂದಿನ ಸಂಘರ್ಷಗಳಿಗೂ ಇದೇ ಮಾಡಿ
ExistingFileNewerSelectAction=ಕ್ರಿಯೆಯನ್ನು ಆಯ್ಕೆಮಾಡಿ
ExistingFileNewer2=ಅಸ್ತಿತ್ವದಲ್ಲಿರುವ ಫೈಲ್ ಸೆಟಪ್ ಸ್ಥಾಪಿಸಲು ಪ್ರಯತ್ನಿಸುತ್ತಿರುವುದಕ್ಕಿಂತ ಹೊಸದಾಗಿದೆ.
ExistingFileNewerOverwriteExisting=ಅಸ್ತಿತ್ವದಲ್ಲಿರುವ ಫೈಲ್ ಅನ್ನು ಬದಲಿಸಿ
ExistingFileNewerKeepExisting=ಅಸ್ತಿತ್ವದಲ್ಲಿರುವ ಫೈಲ್ ಅನ್ನು ಉಳಿಸಿಕೊಳ್ಳಿ (ಶಿಫಾರಸು ಮಾಡಲಾಗಿದೆ)
ExistingFileNewerOverwriteOrKeepAll=ಮುಂದಿನ ಸಂಘರ್ಷಗಳಿಗೂ ಇದೇ ಮಾಡಿ
ErrorChangingAttr=ಅಸ್ತಿತ್ವದಲ್ಲಿರುವ ಫೈಲ್‌ನ ಗುಣಲಕ್ಷಣಗಳನ್ನು ಬದಲಾಯಿಸಲು ಪ್ರಯತ್ನಿಸುವಾಗ ದೋಷ ಉಂಟಾಯಿತು:
ErrorCreatingTemp=ಗಮ್ಯಸ್ಥಾನ ಡೈರೆಕ್ಟರಿಯಲ್ಲಿ ಫೈಲ್ ರಚಿಸಲು ಪ್ರಯತ್ನಿಸುವಾಗ ದೋಷ ಉಂಟಾಯಿತು:
ErrorReadingSource=ಮೂಲ ಫೈಲ್ ಅನ್ನು ಓದಲು ಪ್ರಯತ್ನಿಸುವಾಗ ದೋಷ ಉಂಟಾಯಿತು:
ErrorCopying=ಫೈಲ್ ಅನ್ನು ನಕಲಿಸಲು ಪ್ರಯತ್ನಿಸುವಾಗ ದೋಷ ಉಂಟಾಯಿತು:
ErrorDownloading=ಫೈಲ್ ಅನ್ನು ಡೌನ್‌ಲೋಡ್ ಮಾಡಲು ಪ್ರಯತ್ನಿಸುವಾಗ ದೋಷ ಉಂಟಾಯಿತು:
ErrorExtracting=ಆರ್ಕೈವ್ ಅನ್ನು ಹೊರತೆಗೆಯಲು ಪ್ರಯತ್ನಿಸುವಾಗ ದೋಷ ಉಂಟಾಯಿತು:
ErrorReplacingExistingFile=ಅಸ್ತಿತ್ವದಲ್ಲಿರುವ ಫೈಲ್ ಅನ್ನು ಬದಲಿಸಲು ಪ್ರಯತ್ನಿಸುವಾಗ ದೋಷ ಉಂಟಾಯಿತು:
ErrorRestartReplace=RestartReplace ವಿಫಲವಾಯಿತು:
ErrorRenamingTemp=ಗಮ್ಯಸ್ಥಾನ ಡೈರೆಕ್ಟರಿಯಲ್ಲಿ ಫೈಲ್‌ನ ಹೆಸರು ಬದಲಾಯಿಸಲು ಪ್ರಯತ್ನಿಸುವಾಗ ದೋಷ ಉಂಟಾಯಿತು:
ErrorRegisterServer=DLL/OCX ಅನ್ನು ನೋಂದಾಯಿಸಲು ಸಾಧ್ಯವಾಗಲಿಲ್ಲ: %1
ErrorRegSvr32Failed=RegSvr32 ನಿರ್ಗಮನ ಕೋಡ್ %1 ನೊಂದಿಗೆ ವಿಫಲವಾಯಿತು
ErrorRegisterTypeLib=ಟೈಪ್ ಲೈಬ್ರರಿಯನ್ನು ನೋಂದಾಯಿಸಲು ಸಾಧ್ಯವಾಗಲಿಲ್ಲ: %1

; *** Uninstall display name markings
; used for example as 'My Program (32-bit)'
UninstallDisplayNameMark=%1 (%2)
; used for example as 'My Program (32-bit, All users)'
UninstallDisplayNameMarks=%1 (%2, %3)
UninstallDisplayNameMark32Bit=32-ಬಿಟ್
UninstallDisplayNameMark64Bit=64-ಬಿಟ್
UninstallDisplayNameMarkAllUsers=ಎಲ್ಲ ಬಳಕೆದಾರರು
UninstallDisplayNameMarkCurrentUser=ಪ್ರಸ್ತುತ ಬಳಕೆದಾರ

; *** Post-installation errors
ErrorOpeningReadme=README ಫೈಲ್ ತೆರೆಯಲು ಪ್ರಯತ್ನಿಸುವಾಗ ದೋಷ ಉಂಟಾಯಿತು.
ErrorRestartingComputer=ಸೆಟಪ್‌ಗೆ ಕಂಪ್ಯೂಟರ್ ಅನ್ನು ಮರುಪ್ರಾರಂಭಿಸಲು ಸಾಧ್ಯವಾಗಲಿಲ್ಲ. ದಯವಿಟ್ಟು ಇದನ್ನು ಕೈಯಾರೆ ಮಾಡಿ.

; *** Uninstaller messages
UninstallNotFound="%1" ಫೈಲ್ ಅಸ್ತಿತ್ವದಲ್ಲಿಲ್ಲ. ಅಸ್ಥಾಪಿಸಲು ಸಾಧ್ಯವಿಲ್ಲ.
UninstallOpenError="%1" ಫೈಲ್ ತೆರೆಯಲು ಸಾಧ್ಯವಾಗಲಿಲ್ಲ. ಅಸ್ಥಾಪಿಸಲು ಸಾಧ್ಯವಿಲ್ಲ
UninstallUnsupportedVer="%1" ಅಸ್ಥಾಪನೆ ಲಾಗ್ ಫೈಲ್ ಈ ಅಸ್ಥಾಪಕದ ಆವೃತ್ತಿ ಗುರುತಿಸದ ಸ್ವರೂಪದಲ್ಲಿದೆ. ಅಸ್ಥಾಪಿಸಲು ಸಾಧ್ಯವಿಲ್ಲ
UninstallUnknownEntry=ಅಸ್ಥಾಪನೆ ಲಾಗ್‌ನಲ್ಲಿ ಅಪರಿಚಿತ ನಮೂದು (%1) ಎದುರಾಯಿತು
ConfirmUninstall=%1 ಮತ್ತು ಅದರ ಎಲ್ಲ ಘಟಕಗಳನ್ನು ಸಂಪೂರ್ಣವಾಗಿ ತೆಗೆದುಹಾಕಬೇಕೆಂದು ನಿಮಗೆ ಖಚಿತವಾಗಿದೆಯೇ?
UninstallOnlyOnWin64=ಈ ಸ್ಥಾಪನೆಯನ್ನು 64-ಬಿಟ್ Windows ನಲ್ಲಿ ಮಾತ್ರ ಅಸ್ಥಾಪಿಸಬಹುದು.
OnlyAdminCanUninstall=ಈ ಸ್ಥಾಪನೆಯನ್ನು ನಿರ್ವಾಹಕ ಹಕ್ಕುಗಳಿರುವ ಬಳಕೆದಾರರು ಮಾತ್ರ ಅಸ್ಥಾಪಿಸಬಹುದು.
UninstallStatusLabel=ನಿಮ್ಮ ಕಂಪ್ಯೂಟರ್‌ನಿಂದ %1 ಅನ್ನು ತೆಗೆದುಹಾಕಲಾಗುತ್ತಿದೆ, ದಯವಿಟ್ಟು ಕಾಯಿರಿ.
UninstalledAll=%1 ಅನ್ನು ನಿಮ್ಮ ಕಂಪ್ಯೂಟರ್‌ನಿಂದ ಯಶಸ್ವಿಯಾಗಿ ತೆಗೆದುಹಾಕಲಾಗಿದೆ.
UninstalledMost=%1 ಅಸ್ಥಾಪನೆ ಪೂರ್ಣಗೊಂಡಿದೆ.%n%nಕೆಲವು ಅಂಶಗಳನ್ನು ತೆಗೆದುಹಾಕಲು ಸಾಧ್ಯವಾಗಲಿಲ್ಲ. ಅವುಗಳನ್ನು ಕೈಯಾರೆ ತೆಗೆದುಹಾಕಬಹುದು.
UninstalledAndNeedsRestart=%1 ಅಸ್ಥಾಪನೆಯನ್ನು ಪೂರ್ಣಗೊಳಿಸಲು ನಿಮ್ಮ ಕಂಪ್ಯೂಟರ್ ಅನ್ನು ಮರುಪ್ರಾರಂಭಿಸಬೇಕು.%n%nಈಗ ಮರುಪ್ರಾರಂಭಿಸಬೇಕೇ?
UninstallDataCorrupted="%1" ಫೈಲ್ ಹಾಳಾಗಿದೆ. ಅಸ್ಥಾಪಿಸಲು ಸಾಧ್ಯವಿಲ್ಲ

; *** Uninstallation phase messages
ConfirmDeleteSharedFileTitle=ಹಂಚಿಕೆಯ ಫೈಲ್ ತೆಗೆದುಹಾಕಬೇಕೇ?
ConfirmDeleteSharedFile2=ಕೆಳಗಿನ ಹಂಚಿಕೆಯ ಫೈಲ್ ಅನ್ನು ಇನ್ನು ಯಾವ ಪ್ರೋಗ್ರಾಂ ಕೂಡ ಬಳಸುತ್ತಿಲ್ಲ ಎಂದು ಸಿಸ್ಟಮ್ ಸೂಚಿಸುತ್ತಿದೆ. ಈ ಹಂಚಿಕೆಯ ಫೈಲ್ ಅನ್ನು ಅಸ್ಥಾಪಕ ತೆಗೆದುಹಾಕಬೇಕೆಂದು ನೀವು ಬಯಸುತ್ತೀರಾ?%n%nಯಾವುದಾದರೂ ಪ್ರೋಗ್ರಾಂ ಇನ್ನೂ ಈ ಫೈಲ್ ಅನ್ನು ಬಳಸುತ್ತಿದ್ದು ಇದನ್ನು ತೆಗೆದುಹಾಕಿದರೆ, ಆ ಪ್ರೋಗ್ರಾಂಗಳು ಸರಿಯಾಗಿ ಕೆಲಸ ಮಾಡದಿರಬಹುದು. ಖಚಿತವಿಲ್ಲದಿದ್ದರೆ “ಇಲ್ಲ” ಆಯ್ಕೆಮಾಡಿ. ಫೈಲ್ ಅನ್ನು ಸಿಸ್ಟಮ್‌ನಲ್ಲಿ ಉಳಿಸಿದರೆ ಯಾವುದೇ ಹಾನಿಯಾಗುವುದಿಲ್ಲ.
SharedFileNameLabel=ಫೈಲ್ ಹೆಸರು:
SharedFileLocationLabel=ಸ್ಥಳ:
WizardUninstalling=ಅಸ್ಥಾಪನೆ ಸ್ಥಿತಿ
StatusUninstalling=%1 ಅನ್ನು ಅಸ್ಥಾಪಿಸಲಾಗುತ್ತಿದೆ...

; *** Shutdown block reasons
ShutdownBlockReasonInstallingApp=%1 ಅನ್ನು ಸ್ಥಾಪಿಸಲಾಗುತ್ತಿದೆ.
ShutdownBlockReasonUninstallingApp=%1 ಅನ್ನು ಅಸ್ಥಾಪಿಸಲಾಗುತ್ತಿದೆ.

; The custom messages below aren't used by Setup itself, but if you make
; use of them in your scripts, you'll want to translate them.

[CustomMessages]

NameAndVersion=%1 ಆವೃತ್ತಿ %2
AdditionalIcons=ಹೆಚ್ಚುವರಿ ಶಾರ್ಟ್‌ಕಟ್‌ಗಳು:
CreateDesktopIcon=ಡೆಸ್ಕ್‌ಟಾಪ್ ಶಾರ್ಟ್‌ಕಟ್ ರಚಿಸಿ
CreateQuickLaunchIcon=ಕ್ವಿಕ್ ಲಾಂಚ್ ಶಾರ್ಟ್‌ಕಟ್ ರಚಿಸಿ
ProgramOnTheWeb=ವೆಬ್‌ನಲ್ಲಿ %1
UninstallProgram=%1 ಅನ್ನು ಅಸ್ಥಾಪಿಸಿ
LaunchProgram=%1 ಅನ್ನು ಪ್ರಾರಂಭಿಸಿ
AssocFileExtension=%1 ಅನ್ನು %2 ಫೈಲ್ ವಿಸ್ತರಣೆಯೊಂದಿಗೆ ಸಂಯೋಜಿಸಿ
AssocingFileExtension=%1 ಅನ್ನು %2 ಫೈಲ್ ವಿಸ್ತರಣೆಯೊಂದಿಗೆ ಸಂಯೋಜಿಸಲಾಗುತ್ತಿದೆ...
AutoStartProgramGroupDescription=ಆರಂಭ:
AutoStartProgram=%1 ಅನ್ನು ಸ್ವಯಂಚಾಲಿತವಾಗಿ ಪ್ರಾರಂಭಿಸಿ
AddonHostProgramNotFound=ನೀವು ಆಯ್ಕೆಮಾಡಿದ ಫೋಲ್ಡರ್‌ನಲ್ಲಿ %1 ಕಂಡುಬಂದಿಲ್ಲ.%n%nಹಾಗಿದ್ದರೂ ಮುಂದುವರಿಸಲು ಬಯಸುತ್ತೀರಾ?

; The messages that are Lang Switcher's own (the sample is English.isl).
LocalCertReady=ಸೆಟಪ್ ಈ ಕಂಪ್ಯೂಟರ್‌ನಲ್ಲಿ ಒಂದು ಪ್ರಮಾಣಪತ್ರವನ್ನು ರಚಿಸಿ ಅದರಿಂದ Lang Switcher ಗೆ ಸಹಿ ಹಾಕುತ್ತದೆ; ಇದರಿಂದ ಪ್ರೋಗ್ರಾಂ ನಿರ್ವಾಹಕರಾಗಿ ಚಲಾಯಿಸಿದ ಪ್ರೋಗ್ರಾಂಗಳ ವಿಂಡೋಗಳಲ್ಲಿಯೂ ಕೆಲಸ ಮಾಡುತ್ತದೆ.
LocalCertFailed=Lang Switcher ಸ್ಥಾಪನೆಯಾಯಿತು, ಆದರೆ ಅದು ನಿರ್ವಾಹಕರಾಗಿ ಚಲಾಯಿಸಿದ ಪ್ರೋಗ್ರಾಂಗಳ ವಿಂಡೋಗಳಲ್ಲಿ ಕೆಲಸ ಮಾಡುವುದಿಲ್ಲ: ಈ ಕಂಪ್ಯೂಟರ್‌ನಲ್ಲಿ ರಚಿಸಿದ ಪ್ರಮಾಣಪತ್ರವನ್ನು Windows ಒಪ್ಪಿಕೊಳ್ಳಲಿಲ್ಲ. ವಿವರಗಳಿಗಾಗಿ ಸೆಟಪ್ ಲಾಗ್ ನೋಡಿ.
RemoveSettingsQuestion=Lang Switcher ಸೆಟ್ಟಿಂಗ್‌ಗಳ ಫೈಲ್ ಅನ್ನೂ ತೆಗೆದುಹಾಕಬೇಕೇ?
RemoveSettingsKeep=ನಂತರ ಮತ್ತೆ ಸ್ಥಾಪಿಸುವಾಗ ಬಳಸಲು ನಿಮ್ಮ ಸೆಟ್ಟಿಂಗ್‌ಗಳನ್ನು ಉಳಿಸಿಕೊಳ್ಳಬೇಕಾದರೆ “ಇಲ್ಲ” ಆಯ್ಕೆಮಾಡಿ.
AuthorSlogan=ಮನಸ್ಸಿನಿಂದ ಮಾಡಿದ ಪ್ರೋಗ್ರಾಂಗಳು
