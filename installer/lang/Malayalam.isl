; Lang Switcher -- the messages of the installer in Malayalam (stage E108, group A).
;
; Inno Setup has no official Malayalam translation (inno_official = false in res\lang\languages.toml),
; so this file carries the whole of it: [LangOptions], every message of the compiler:Default.isl
; (6.5.0+), and the messages of the installer that are Lang Switcher's own. It is read after
; compiler:Default.isl as the second MessagesFile of the "malayalam" entry of [Languages] in
; ..\LangSwitcher.iss, and so replaces the English text name for name.
; English.isl is the sample the own messages translate; the glossary of the language is
; l10n\glossary\ml.tsv of the control contour. The name of the program is not translated.
; As in Default.isl: a message that has no full stop there has none here (Setup adds its own).
; No & accelerators: a file without an official translation puts none (RULES 16), so that one
; window never shows two equal letters. The name of a button inside a message is in the quotation
; marks of the language and is the word of the button as this file writes it.

[LangOptions]
LanguageName=മലയാളം
LanguageID=$044C
LanguageCodePage=0

[Messages]

; *** Application titles
SetupAppTitle=സെറ്റപ്പ്
SetupWindowTitle=സെറ്റപ്പ് - %1
UninstallAppTitle=അൺഇൻസ്റ്റാൾ
UninstallAppFullTitle=%1 അൺഇൻസ്റ്റാൾ

; *** Misc. common
InformationTitle=വിവരം
ConfirmTitle=സ്ഥിരീകരിക്കുക
ErrorTitle=പിശക്

; *** SetupLdr messages
SetupLdrStartupMessage=ഇത് %1 ഇൻസ്റ്റാൾ ചെയ്യും. തുടരാൻ നിങ്ങൾ ആഗ്രഹിക്കുന്നുണ്ടോ?
LdrCannotCreateTemp=താൽക്കാലിക ഫയൽ സൃഷ്ടിക്കാനായില്ല. സെറ്റപ്പ് റദ്ദാക്കി
LdrCannotExecTemp=താൽക്കാലിക ഡയറക്ടറിയിലെ ഫയൽ പ്രവർത്തിപ്പിക്കാനായില്ല. സെറ്റപ്പ് റദ്ദാക്കി
HelpTextNote=

; *** Startup error messages
LastErrorMessage=%1.%n%nപിശക് %2: %3
SetupFileMissing=ഇൻസ്റ്റലേഷൻ ഡയറക്ടറിയിൽ %1 ഫയൽ ഇല്ല. പ്രശ്നം പരിഹരിക്കുക, അല്ലെങ്കിൽ പ്രോഗ്രാമിന്റെ പുതിയ പകർപ്പ് നേടുക.
SetupFileCorrupt=സെറ്റപ്പ് ഫയലുകൾ കേടായി. പ്രോഗ്രാമിന്റെ പുതിയ പകർപ്പ് നേടുക.
SetupFileCorruptOrWrongVer=സെറ്റപ്പ് ഫയലുകൾ കേടായി, അല്ലെങ്കിൽ സെറ്റപ്പിന്റെ ഈ പതിപ്പുമായി പൊരുത്തപ്പെടുന്നില്ല. പ്രശ്നം പരിഹരിക്കുക, അല്ലെങ്കിൽ പ്രോഗ്രാമിന്റെ പുതിയ പകർപ്പ് നേടുക.
InvalidParameter=കമാൻഡ് ലൈനിൽ അസാധുവായ പരാമീറ്റർ നൽകി:%n%n%1
SetupAlreadyRunning=സെറ്റപ്പ് ഇതിനകം പ്രവർത്തിക്കുന്നു.
WindowsVersionNotSupported=നിങ്ങളുടെ കമ്പ്യൂട്ടറിൽ പ്രവർത്തിക്കുന്ന Windows പതിപ്പിനെ ഈ പ്രോഗ്രാം പിന്തുണയ്ക്കുന്നില്ല.
WindowsServicePackRequired=ഈ പ്രോഗ്രാമിന് %1 സർവീസ് പാക്ക് %2 അല്ലെങ്കിൽ അതിനു ശേഷമുള്ളത് ആവശ്യമാണ്.
NotOnThisPlatform=ഈ പ്രോഗ്രാം %1-ൽ പ്രവർത്തിക്കില്ല.
OnlyOnThisPlatform=ഈ പ്രോഗ്രാം %1-ൽ പ്രവർത്തിപ്പിക്കണം.
OnlyOnTheseArchitectures=ഇനിപ്പറയുന്ന പ്രോസസർ ആർക്കിടെക്ചറുകൾക്കായി രൂപകൽപ്പന ചെയ്ത Windows പതിപ്പുകളിൽ മാത്രമേ ഈ പ്രോഗ്രാം ഇൻസ്റ്റാൾ ചെയ്യാനാകൂ:%n%n%1
WinVersionTooLowError=ഈ പ്രോഗ്രാമിന് %1 പതിപ്പ് %2 അല്ലെങ്കിൽ അതിനു ശേഷമുള്ളത് ആവശ്യമാണ്.
WinVersionTooHighError=ഈ പ്രോഗ്രാം %1 പതിപ്പ് %2 അല്ലെങ്കിൽ അതിനു ശേഷമുള്ളതിൽ ഇൻസ്റ്റാൾ ചെയ്യാനാകില്ല.
AdminPrivilegesRequired=ഈ പ്രോഗ്രാം ഇൻസ്റ്റാൾ ചെയ്യുമ്പോൾ നിങ്ങൾ അഡ്മിനിസ്ട്രേറ്ററായി സൈൻ ഇൻ ചെയ്തിരിക്കണം.
PowerUserPrivilegesRequired=ഈ പ്രോഗ്രാം ഇൻസ്റ്റാൾ ചെയ്യുമ്പോൾ നിങ്ങൾ അഡ്മിനിസ്ട്രേറ്ററായോ പവർ യൂസേഴ്സ് ഗ്രൂപ്പിലെ അംഗമായോ സൈൻ ഇൻ ചെയ്തിരിക്കണം.
SetupAppRunningError=%1 ഇപ്പോൾ പ്രവർത്തിക്കുന്നതായി സെറ്റപ്പ് കണ്ടെത്തി.%n%nദയവായി അതിന്റെ എല്ലാ ഇൻസ്റ്റൻസുകളും ഇപ്പോൾ അടയ്ക്കുക, തുടർന്ന് തുടരാൻ “ശരി” അല്ലെങ്കിൽ പുറത്തുകടക്കാൻ “റദ്ദാക്കുക” ക്ലിക്ക് ചെയ്യുക.
UninstallAppRunningError=%1 ഇപ്പോൾ പ്രവർത്തിക്കുന്നതായി അൺഇൻസ്റ്റാൾ കണ്ടെത്തി.%n%nദയവായി അതിന്റെ എല്ലാ ഇൻസ്റ്റൻസുകളും ഇപ്പോൾ അടയ്ക്കുക, തുടർന്ന് തുടരാൻ “ശരി” അല്ലെങ്കിൽ പുറത്തുകടക്കാൻ “റദ്ദാക്കുക” ക്ലിക്ക് ചെയ്യുക.

; *** Startup questions
PrivilegesRequiredOverrideTitle=സെറ്റപ്പ് ഇൻസ്റ്റാൾ മോഡ് തിരഞ്ഞെടുക്കുക
PrivilegesRequiredOverrideInstruction=ഇൻസ്റ്റാൾ മോഡ് തിരഞ്ഞെടുക്കുക
PrivilegesRequiredOverrideText1=%1 എല്ലാ ഉപയോക്താക്കൾക്കുമായി (അഡ്മിനിസ്ട്രേറ്റീവ് അവകാശങ്ങൾ ആവശ്യമാണ്) അല്ലെങ്കിൽ നിങ്ങൾക്ക് മാത്രമായി ഇൻസ്റ്റാൾ ചെയ്യാം.
PrivilegesRequiredOverrideText2=%1 നിങ്ങൾക്ക് മാത്രമായി അല്ലെങ്കിൽ എല്ലാ ഉപയോക്താക്കൾക്കുമായി (അഡ്മിനിസ്ട്രേറ്റീവ് അവകാശങ്ങൾ ആവശ്യമാണ്) ഇൻസ്റ്റാൾ ചെയ്യാം.
PrivilegesRequiredOverrideAllUsers=എല്ലാ ഉപയോക്താക്കൾക്കുമായി ഇൻസ്റ്റാൾ ചെയ്യുക
PrivilegesRequiredOverrideAllUsersRecommended=എല്ലാ ഉപയോക്താക്കൾക്കുമായി ഇൻസ്റ്റാൾ ചെയ്യുക (ശുപാർശ ചെയ്യുന്നത്)
PrivilegesRequiredOverrideCurrentUser=എനിക്ക് മാത്രമായി ഇൻസ്റ്റാൾ ചെയ്യുക
PrivilegesRequiredOverrideCurrentUserRecommended=എനിക്ക് മാത്രമായി ഇൻസ്റ്റാൾ ചെയ്യുക (ശുപാർശ ചെയ്യുന്നത്)

; *** Misc. errors
ErrorCreatingDir="%1" ഡയറക്ടറി സൃഷ്ടിക്കാൻ സെറ്റപ്പിന് കഴിഞ്ഞില്ല
ErrorTooManyFilesInDir="%1" ഡയറക്ടറിയിൽ ഫയൽ സൃഷ്ടിക്കാനായില്ല, കാരണം അതിൽ വളരെയധികം ഫയലുകളുണ്ട്

; *** Setup common messages
ExitSetupTitle=സെറ്റപ്പിൽ നിന്ന് പുറത്തുകടക്കുക
ExitSetupMessage=സെറ്റപ്പ് പൂർത്തിയായിട്ടില്ല. ഇപ്പോൾ പുറത്തുകടന്നാൽ പ്രോഗ്രാം ഇൻസ്റ്റാൾ ചെയ്യപ്പെടില്ല.%n%nഇൻസ്റ്റലേഷൻ പൂർത്തിയാക്കാൻ മറ്റൊരു സമയത്ത് സെറ്റപ്പ് വീണ്ടും പ്രവർത്തിപ്പിക്കാം.%n%nസെറ്റപ്പിൽ നിന്ന് പുറത്തുകടക്കണോ?
AboutSetupMenuItem=സെറ്റപ്പിനെക്കുറിച്ച്...
AboutSetupTitle=സെറ്റപ്പിനെക്കുറിച്ച്
AboutSetupMessage=%1 പതിപ്പ് %2%n%3%n%n%1 ഹോം പേജ്:%n%4
AboutSetupNote=
TranslatorNote=

; *** Buttons
ButtonBack=< പിന്നോട്ട്
ButtonNext=അടുത്തത് >
ButtonInstall=ഇൻസ്റ്റാൾ
ButtonOK=ശരി
ButtonCancel=റദ്ദാക്കുക
ButtonYes=അതെ
ButtonYesToAll=എല്ലാത്തിനും അതെ
ButtonNo=ഇല്ല
ButtonNoToAll=എല്ലാത്തിനും ഇല്ല
ButtonFinish=പൂർത്തിയാക്കുക
ButtonBrowse=ബ്രൗസ്...
ButtonWizardBrowse=ബ്രൗസ്...
ButtonNewFolder=പുതിയ ഫോൾഡർ ഉണ്ടാക്കുക

; *** "Select Language" dialog messages
SelectLanguageTitle=സെറ്റപ്പ് ഭാഷ തിരഞ്ഞെടുക്കുക
SelectLanguageLabel=ഇൻസ്റ്റലേഷൻ സമയത്ത് ഉപയോഗിക്കേണ്ട ഭാഷ തിരഞ്ഞെടുക്കുക.

; *** Common wizard text
ClickNext=തുടരാൻ “അടുത്തത്” അല്ലെങ്കിൽ സെറ്റപ്പിൽ നിന്ന് പുറത്തുകടക്കാൻ “റദ്ദാക്കുക” ക്ലിക്ക് ചെയ്യുക.
BeveledLabel=
BrowseDialogTitle=ഫോൾഡർ തിരയുക
BrowseDialogLabel=താഴെയുള്ള പട്ടികയിൽ നിന്ന് ഒരു ഫോൾഡർ തിരഞ്ഞെടുത്ത ശേഷം “ശരി” ക്ലിക്ക് ചെയ്യുക.
NewFolderName=പുതിയ ഫോൾഡർ

; *** "Welcome" wizard page
WelcomeLabel1=[name] സെറ്റപ്പ് വിസാർഡിലേക്ക് സ്വാഗതം
WelcomeLabel2=ഇത് നിങ്ങളുടെ കമ്പ്യൂട്ടറിൽ [name/ver] ഇൻസ്റ്റാൾ ചെയ്യും.%n%nതുടരുന്നതിന് മുമ്പ് മറ്റ് എല്ലാ ആപ്ലിക്കേഷനുകളും അടയ്ക്കാൻ ശുപാർശ ചെയ്യുന്നു.

; *** "Password" wizard page
WizardPassword=പാസ്വേഡ്
PasswordLabel1=ഈ ഇൻസ്റ്റലേഷൻ പാസ്വേഡ് ഉപയോഗിച്ച് സംരക്ഷിച്ചിരിക്കുന്നു.
PasswordLabel3=പാസ്വേഡ് നൽകിയ ശേഷം തുടരാൻ “അടുത്തത്” ക്ലിക്ക് ചെയ്യുക. പാസ്വേഡുകളിൽ വലിയക്ഷരവും ചെറിയക്ഷരവും വ്യത്യാസപ്പെടും.
PasswordEditLabel=പാസ്വേഡ്:
IncorrectPassword=നിങ്ങൾ നൽകിയ പാസ്വേഡ് ശരിയല്ല. വീണ്ടും ശ്രമിക്കുക.

; *** "License Agreement" wizard page
WizardLicense=ലൈസൻസ് കരാർ
LicenseLabel=തുടരുന്നതിന് മുമ്പ് ഇനിപ്പറയുന്ന പ്രധാന വിവരങ്ങൾ വായിക്കുക.
LicenseLabel3=ഇനിപ്പറയുന്ന ലൈസൻസ് കരാർ വായിക്കുക. ഇൻസ്റ്റലേഷൻ തുടരുന്നതിന് മുമ്പ് ഈ കരാറിന്റെ വ്യവസ്ഥകൾ നിങ്ങൾ അംഗീകരിക്കണം.
LicenseAccepted=ഞാൻ കരാർ അംഗീകരിക്കുന്നു
LicenseNotAccepted=ഞാൻ കരാർ അംഗീകരിക്കുന്നില്ല

; *** "Information" wizard pages
WizardInfoBefore=വിവരം
InfoBeforeLabel=തുടരുന്നതിന് മുമ്പ് ഇനിപ്പറയുന്ന പ്രധാന വിവരങ്ങൾ വായിക്കുക.
InfoBeforeClickLabel=സെറ്റപ്പ് തുടരാൻ തയ്യാറാകുമ്പോൾ “അടുത്തത്” ക്ലിക്ക് ചെയ്യുക.
WizardInfoAfter=വിവരം
InfoAfterLabel=തുടരുന്നതിന് മുമ്പ് ഇനിപ്പറയുന്ന പ്രധാന വിവരങ്ങൾ വായിക്കുക.
InfoAfterClickLabel=സെറ്റപ്പ് തുടരാൻ തയ്യാറാകുമ്പോൾ “അടുത്തത്” ക്ലിക്ക് ചെയ്യുക.

; *** "User Information" wizard page
WizardUserInfo=ഉപയോക്തൃ വിവരം
UserInfoDesc=നിങ്ങളുടെ വിവരങ്ങൾ നൽകുക.
UserInfoName=ഉപയോക്തൃനാമം:
UserInfoOrg=സ്ഥാപനം:
UserInfoSerial=സീരിയൽ നമ്പർ:
UserInfoNameRequired=നിങ്ങൾ ഒരു പേര് നൽകണം.

; *** "Select Destination Location" wizard page
WizardSelectDir=ലക്ഷ്യസ്ഥാനം തിരഞ്ഞെടുക്കുക
SelectDirDesc=[name] എവിടെ ഇൻസ്റ്റാൾ ചെയ്യണം?
SelectDirLabel3=സെറ്റപ്പ് [name] ഇനിപ്പറയുന്ന ഫോൾഡറിൽ ഇൻസ്റ്റാൾ ചെയ്യും.
SelectDirBrowseLabel=തുടരാൻ “അടുത്തത്” ക്ലിക്ക് ചെയ്യുക. മറ്റൊരു ഫോൾഡർ തിരഞ്ഞെടുക്കണമെങ്കിൽ “ബ്രൗസ്” ക്ലിക്ക് ചെയ്യുക.
DiskSpaceGBLabel=കുറഞ്ഞത് [gb] GB ഒഴിഞ്ഞ ഡിസ്ക് സ്ഥലം ആവശ്യമാണ്.
DiskSpaceMBLabel=കുറഞ്ഞത് [mb] MB ഒഴിഞ്ഞ ഡിസ്ക് സ്ഥലം ആവശ്യമാണ്.
CannotInstallToNetworkDrive=നെറ്റ്വർക്ക് ഡ്രൈവിൽ സെറ്റപ്പിന് ഇൻസ്റ്റാൾ ചെയ്യാനാകില്ല.
CannotInstallToUNCPath=UNC പാത്തിൽ സെറ്റപ്പിന് ഇൻസ്റ്റാൾ ചെയ്യാനാകില്ല.
InvalidPath=ഡ്രൈവ് അക്ഷരത്തോടുകൂടിയ പൂർണ്ണ പാത്ത് നൽകണം; ഉദാഹരണത്തിന്:%n%nC:\APP%n%nഅല്ലെങ്കിൽ ഈ രൂപത്തിലുള്ള UNC പാത്ത്:%n%n\\server\share
InvalidDrive=നിങ്ങൾ തിരഞ്ഞെടുത്ത ഡ്രൈവോ UNC ഷെയറോ നിലവിലില്ല അല്ലെങ്കിൽ ലഭ്യമല്ല. മറ്റൊന്ന് തിരഞ്ഞെടുക്കുക.
DiskSpaceWarningTitle=ആവശ്യത്തിന് ഡിസ്ക് സ്ഥലമില്ല
DiskSpaceWarning=ഇൻസ്റ്റാൾ ചെയ്യാൻ സെറ്റപ്പിന് കുറഞ്ഞത് %1 KB ഒഴിഞ്ഞ സ്ഥലം ആവശ്യമാണ്, എന്നാൽ തിരഞ്ഞെടുത്ത ഡ്രൈവിൽ %2 KB മാത്രമേ ലഭ്യമുള്ളൂ.%n%nഎന്നിട്ടും തുടരണോ?
DirNameTooLong=ഫോൾഡറിന്റെ പേരോ പാത്തോ വളരെ നീളമുള്ളതാണ്.
InvalidDirName=ഫോൾഡറിന്റെ പേര് അസാധുവാണ്.
BadDirName32=ഫോൾഡർ പേരുകളിൽ ഇനിപ്പറയുന്ന അക്ഷരങ്ങൾ ഉൾപ്പെടുത്താനാകില്ല:%n%n%1
DirExistsTitle=ഫോൾഡർ നിലവിലുണ്ട്
DirExists=ഈ ഫോൾഡർ:%n%n%1%n%nഇതിനകം നിലവിലുണ്ട്. എന്നിട്ടും ആ ഫോൾഡറിൽ ഇൻസ്റ്റാൾ ചെയ്യണോ?
DirDoesntExistTitle=ഫോൾഡർ നിലവിലില്ല
DirDoesntExist=ഈ ഫോൾഡർ:%n%n%1%n%nനിലവിലില്ല. ഫോൾഡർ സൃഷ്ടിക്കണോ?

; *** "Select Components" wizard page
WizardSelectComponents=ഘടകങ്ങൾ തിരഞ്ഞെടുക്കുക
SelectComponentsDesc=ഏതൊക്കെ ഘടകങ്ങൾ ഇൻസ്റ്റാൾ ചെയ്യണം?
SelectComponentsLabel2=ഇൻസ്റ്റാൾ ചെയ്യേണ്ട ഘടകങ്ങൾ തിരഞ്ഞെടുക്കുക; ഇൻസ്റ്റാൾ ചെയ്യേണ്ടാത്തവയുടെ ടിക്ക് നീക്കുക. തുടരാൻ തയ്യാറാകുമ്പോൾ “അടുത്തത്” ക്ലിക്ക് ചെയ്യുക.
FullInstallation=പൂർണ്ണ ഇൻസ്റ്റലേഷൻ
; if possible don't translate 'Compact' as 'Minimal' (I mean 'Minimal' in your language)
CompactInstallation=കോംപാക്റ്റ് ഇൻസ്റ്റലേഷൻ
CustomInstallation=ഇഷ്ടാനുസൃത ഇൻസ്റ്റലേഷൻ
NoUninstallWarningTitle=ഘടകങ്ങൾ നിലവിലുണ്ട്
NoUninstallWarning=ഇനിപ്പറയുന്ന ഘടകങ്ങൾ നിങ്ങളുടെ കമ്പ്യൂട്ടറിൽ ഇതിനകം ഇൻസ്റ്റാൾ ചെയ്തിട്ടുണ്ടെന്ന് സെറ്റപ്പ് കണ്ടെത്തി:%n%n%1%n%nഇവ തിരഞ്ഞെടുക്കാതിരുന്നാൽ ഇവ അൺഇൻസ്റ്റാൾ ആകില്ല.%n%nഎന്നിട്ടും തുടരണോ?
ComponentSize1=%1 KB
ComponentSize2=%1 MB
ComponentsDiskSpaceGBLabel=നിലവിലെ തിരഞ്ഞെടുപ്പിന് കുറഞ്ഞത് [gb] GB ഡിസ്ക് സ്ഥലം ആവശ്യമാണ്.
ComponentsDiskSpaceMBLabel=നിലവിലെ തിരഞ്ഞെടുപ്പിന് കുറഞ്ഞത് [mb] MB ഡിസ്ക് സ്ഥലം ആവശ്യമാണ്.

; *** "Select Additional Tasks" wizard page
WizardSelectTasks=അധിക ടാസ്കുകൾ തിരഞ്ഞെടുക്കുക
SelectTasksDesc=ഏതൊക്കെ അധിക ടാസ്കുകൾ ചെയ്യണം?
SelectTasksLabel2=[name] ഇൻസ്റ്റാൾ ചെയ്യുമ്പോൾ സെറ്റപ്പ് ചെയ്യേണ്ട അധിക ടാസ്കുകൾ തിരഞ്ഞെടുത്ത ശേഷം “അടുത്തത്” ക്ലിക്ക് ചെയ്യുക.

; *** "Select Start Menu Folder" wizard page
WizardSelectProgramGroup=സ്റ്റാർട്ട് മെനു ഫോൾഡർ തിരഞ്ഞെടുക്കുക
SelectStartMenuFolderDesc=പ്രോഗ്രാമിന്റെ കുറുക്കുവഴികൾ സെറ്റപ്പ് എവിടെ സ്ഥാപിക്കണം?
SelectStartMenuFolderLabel3=സെറ്റപ്പ് പ്രോഗ്രാമിന്റെ കുറുക്കുവഴികൾ ഇനിപ്പറയുന്ന സ്റ്റാർട്ട് മെനു ഫോൾഡറിൽ സൃഷ്ടിക്കും.
SelectStartMenuFolderBrowseLabel=തുടരാൻ “അടുത്തത്” ക്ലിക്ക് ചെയ്യുക. മറ്റൊരു ഫോൾഡർ തിരഞ്ഞെടുക്കണമെങ്കിൽ “ബ്രൗസ്” ക്ലിക്ക് ചെയ്യുക.
MustEnterGroupName=നിങ്ങൾ ഒരു ഫോൾഡർ പേര് നൽകണം.
GroupNameTooLong=ഫോൾഡറിന്റെ പേരോ പാത്തോ വളരെ നീളമുള്ളതാണ്.
InvalidGroupName=ഫോൾഡറിന്റെ പേര് അസാധുവാണ്.
BadGroupName=ഫോൾഡറിന്റെ പേരിൽ ഇനിപ്പറയുന്ന അക്ഷരങ്ങൾ ഉൾപ്പെടുത്താനാകില്ല:%n%n%1
NoProgramGroupCheck2=സ്റ്റാർട്ട് മെനു ഫോൾഡർ സൃഷ്ടിക്കരുത്

; *** "Ready to Install" wizard page
WizardReady=ഇൻസ്റ്റാൾ ചെയ്യാൻ തയ്യാർ
ReadyLabel1=നിങ്ങളുടെ കമ്പ്യൂട്ടറിൽ [name] ഇൻസ്റ്റാൾ ചെയ്യാൻ സെറ്റപ്പ് ഇപ്പോൾ തയ്യാറാണ്.
ReadyLabel2a=ഇൻസ്റ്റലേഷൻ തുടരാൻ “ഇൻസ്റ്റാൾ” ക്ലിക്ക് ചെയ്യുക, ക്രമീകരണങ്ങൾ പരിശോധിക്കാനോ മാറ്റാനോ “പിന്നോട്ട്” ക്ലിക്ക് ചെയ്യുക.
ReadyLabel2b=ഇൻസ്റ്റലേഷൻ തുടരാൻ “ഇൻസ്റ്റാൾ” ക്ലിക്ക് ചെയ്യുക.
ReadyMemoUserInfo=ഉപയോക്തൃ വിവരം:
ReadyMemoDir=ലക്ഷ്യസ്ഥാനം:
ReadyMemoType=സെറ്റപ്പ് തരം:
ReadyMemoComponents=തിരഞ്ഞെടുത്ത ഘടകങ്ങൾ:
ReadyMemoGroup=സ്റ്റാർട്ട് മെനു ഫോൾഡർ:
ReadyMemoTasks=അധിക ടാസ്കുകൾ:

; *** TDownloadWizardPage wizard page and DownloadTemporaryFile
DownloadingLabel2=ഫയലുകൾ ഡൗൺലോഡ് ചെയ്യുന്നു...
ButtonStopDownload=ഡൗൺലോഡ് നിർത്തുക
StopDownload=ഡൗൺലോഡ് നിർത്തണമെന്ന് ഉറപ്പാണോ?
ErrorDownloadAborted=ഡൗൺലോഡ് റദ്ദാക്കി
ErrorDownloadFailed=ഡൗൺലോഡ് പരാജയപ്പെട്ടു: %1 %2
ErrorDownloadSizeFailed=വലുപ്പം കണ്ടെത്താനായില്ല: %1 %2
ErrorProgress=അസാധുവായ പുരോഗതി: %2-ൽ %1
ErrorFileSize=അസാധുവായ ഫയൽ വലുപ്പം: പ്രതീക്ഷിച്ചത് %1, കണ്ടത് %2

; *** TExtractionWizardPage wizard page and ExtractArchive
ExtractingLabel=ഫയലുകൾ എക്സ്ട്രാക്റ്റ് ചെയ്യുന്നു...
ButtonStopExtraction=എക്സ്ട്രാക്ഷൻ നിർത്തുക
StopExtraction=എക്സ്ട്രാക്ഷൻ നിർത്തണമെന്ന് ഉറപ്പാണോ?
ErrorExtractionAborted=എക്സ്ട്രാക്ഷൻ റദ്ദാക്കി
ErrorExtractionFailed=എക്സ്ട്രാക്ഷൻ പരാജയപ്പെട്ടു: %1

; *** Archive extraction failure details
ArchiveIncorrectPassword=പാസ്വേഡ് തെറ്റാണ്
ArchiveIsCorrupted=ആർക്കൈവ് കേടായി
ArchiveUnsupportedFormat=ആർക്കൈവ് ഫോർമാറ്റ് പിന്തുണയ്ക്കുന്നില്ല

; *** "Preparing to Install" wizard page
WizardPreparing=ഇൻസ്റ്റാൾ ചെയ്യാൻ ഒരുങ്ങുന്നു
PreparingDesc=നിങ്ങളുടെ കമ്പ്യൂട്ടറിൽ [name] ഇൻസ്റ്റാൾ ചെയ്യാൻ സെറ്റപ്പ് ഒരുങ്ങുന്നു.
PreviousInstallNotCompleted=മുമ്പത്തെ പ്രോഗ്രാമിന്റെ ഇൻസ്റ്റലേഷൻ/നീക്കംചെയ്യൽ പൂർത്തിയായിട്ടില്ല. ആ ഇൻസ്റ്റലേഷൻ പൂർത്തിയാക്കാൻ നിങ്ങളുടെ കമ്പ്യൂട്ടർ റീസ്റ്റാർട്ട് ചെയ്യണം.%n%nകമ്പ്യൂട്ടർ റീസ്റ്റാർട്ട് ചെയ്ത ശേഷം [name] ഇൻസ്റ്റലേഷൻ പൂർത്തിയാക്കാൻ സെറ്റപ്പ് വീണ്ടും പ്രവർത്തിപ്പിക്കുക.
CannotContinue=സെറ്റപ്പിന് തുടരാനാകില്ല. പുറത്തുകടക്കാൻ “റദ്ദാക്കുക” ക്ലിക്ക് ചെയ്യുക.
ApplicationsFound=സെറ്റപ്പ് അപ്ഡേറ്റ് ചെയ്യേണ്ട ഫയലുകൾ ഇനിപ്പറയുന്ന ആപ്ലിക്കേഷനുകൾ ഉപയോഗിക്കുന്നു. ഈ ആപ്ലിക്കേഷനുകൾ സ്വയമേവ അടയ്ക്കാൻ സെറ്റപ്പിനെ അനുവദിക്കാൻ ശുപാർശ ചെയ്യുന്നു.
ApplicationsFound2=സെറ്റപ്പ് അപ്ഡേറ്റ് ചെയ്യേണ്ട ഫയലുകൾ ഇനിപ്പറയുന്ന ആപ്ലിക്കേഷനുകൾ ഉപയോഗിക്കുന്നു. ഈ ആപ്ലിക്കേഷനുകൾ സ്വയമേവ അടയ്ക്കാൻ സെറ്റപ്പിനെ അനുവദിക്കാൻ ശുപാർശ ചെയ്യുന്നു. ഇൻസ്റ്റലേഷൻ പൂർത്തിയായ ശേഷം ആപ്ലിക്കേഷനുകൾ വീണ്ടും ആരംഭിക്കാൻ സെറ്റപ്പ് ശ്രമിക്കും.
CloseApplications=ആപ്ലിക്കേഷനുകൾ സ്വയമേവ അടയ്ക്കുക
DontCloseApplications=ആപ്ലിക്കേഷനുകൾ അടയ്ക്കരുത്
ErrorCloseApplications=എല്ലാ ആപ്ലിക്കേഷനുകളും സ്വയമേവ അടയ്ക്കാൻ സെറ്റപ്പിന് കഴിഞ്ഞില്ല. തുടരുന്നതിന് മുമ്പ്, സെറ്റപ്പ് അപ്ഡേറ്റ് ചെയ്യേണ്ട ഫയലുകൾ ഉപയോഗിക്കുന്ന എല്ലാ ആപ്ലിക്കേഷനുകളും അടയ്ക്കാൻ ശുപാർശ ചെയ്യുന്നു.
PrepareToInstallNeedsRestart=സെറ്റപ്പിന് നിങ്ങളുടെ കമ്പ്യൂട്ടർ റീസ്റ്റാർട്ട് ചെയ്യേണ്ടതുണ്ട്. കമ്പ്യൂട്ടർ റീസ്റ്റാർട്ട് ചെയ്ത ശേഷം [name] ഇൻസ്റ്റലേഷൻ പൂർത്തിയാക്കാൻ സെറ്റപ്പ് വീണ്ടും പ്രവർത്തിപ്പിക്കുക.%n%nഇപ്പോൾ റീസ്റ്റാർട്ട് ചെയ്യണോ?

; *** "Installing" wizard page
WizardInstalling=ഇൻസ്റ്റാൾ ചെയ്യുന്നു
InstallingLabel=സെറ്റപ്പ് നിങ്ങളുടെ കമ്പ്യൂട്ടറിൽ [name] ഇൻസ്റ്റാൾ ചെയ്യുന്നതുവരെ കാത്തിരിക്കുക.

; *** "Setup Completed" wizard page
FinishedHeadingLabel=[name] സെറ്റപ്പ് വിസാർഡ് പൂർത്തിയാക്കുന്നു
FinishedLabelNoIcons=നിങ്ങളുടെ കമ്പ്യൂട്ടറിൽ [name] ഇൻസ്റ്റാൾ ചെയ്യുന്നത് സെറ്റപ്പ് പൂർത്തിയാക്കി.
FinishedLabel=നിങ്ങളുടെ കമ്പ്യൂട്ടറിൽ [name] ഇൻസ്റ്റാൾ ചെയ്യുന്നത് സെറ്റപ്പ് പൂർത്തിയാക്കി. ഇൻസ്റ്റാൾ ചെയ്ത കുറുക്കുവഴികൾ തിരഞ്ഞെടുത്ത് ആപ്ലിക്കേഷൻ ആരംഭിക്കാം.
ClickFinish=സെറ്റപ്പിൽ നിന്ന് പുറത്തുകടക്കാൻ “പൂർത്തിയാക്കുക” ക്ലിക്ക് ചെയ്യുക.
FinishedRestartLabel=[name] ഇൻസ്റ്റലേഷൻ പൂർത്തിയാക്കാൻ സെറ്റപ്പിന് നിങ്ങളുടെ കമ്പ്യൂട്ടർ റീസ്റ്റാർട്ട് ചെയ്യേണ്ടതുണ്ട്. ഇപ്പോൾ റീസ്റ്റാർട്ട് ചെയ്യണോ?
FinishedRestartMessage=[name] ഇൻസ്റ്റലേഷൻ പൂർത്തിയാക്കാൻ സെറ്റപ്പിന് നിങ്ങളുടെ കമ്പ്യൂട്ടർ റീസ്റ്റാർട്ട് ചെയ്യേണ്ടതുണ്ട്.%n%nഇപ്പോൾ റീസ്റ്റാർട്ട് ചെയ്യണോ?
ShowReadmeCheck=അതെ, README ഫയൽ കാണണം
YesRadio=അതെ, കമ്പ്യൂട്ടർ ഇപ്പോൾ റീസ്റ്റാർട്ട് ചെയ്യുക
NoRadio=ഇല്ല, കമ്പ്യൂട്ടർ ഞാൻ പിന്നീട് റീസ്റ്റാർട്ട് ചെയ്യാം
; used for example as 'Run MyProg.exe'
RunEntryExec=%1 പ്രവർത്തിപ്പിക്കുക
; used for example as 'View Readme.txt'
RunEntryShellExec=%1 കാണുക

; *** "Setup Needs the Next Disk" stuff
ChangeDiskTitle=സെറ്റപ്പിന് അടുത്ത ഡിസ്ക് ആവശ്യമാണ്
SelectDiskLabel2=ഡിസ്ക് %1 ഇടുക, തുടർന്ന് “ശരി” ക്ലിക്ക് ചെയ്യുക.%n%nഈ ഡിസ്കിലെ ഫയലുകൾ താഴെ കാണിച്ചിരിക്കുന്ന ഫോൾഡറിൽ നിന്ന് വ്യത്യസ്തമായ ഫോൾഡറിലാണെങ്കിൽ, ശരിയായ പാത്ത് നൽകുക അല്ലെങ്കിൽ “ബ്രൗസ്” ക്ലിക്ക് ചെയ്യുക.
PathLabel=പാത്ത്:
FileNotInDir2="%2" എന്നതിൽ "%1" ഫയൽ കണ്ടെത്താനായില്ല. ശരിയായ ഡിസ്ക് ഇടുക അല്ലെങ്കിൽ മറ്റൊരു ഫോൾഡർ തിരഞ്ഞെടുക്കുക.
SelectDirectoryLabel=അടുത്ത ഡിസ്കിന്റെ സ്ഥാനം വ്യക്തമാക്കുക.

; *** Installation phase messages
SetupAborted=സെറ്റപ്പ് പൂർത്തിയായില്ല.%n%nപ്രശ്നം പരിഹരിച്ച് സെറ്റപ്പ് വീണ്ടും പ്രവർത്തിപ്പിക്കുക.
AbortRetryIgnoreSelectAction=പ്രവർത്തനം തിരഞ്ഞെടുക്കുക
AbortRetryIgnoreRetry=വീണ്ടും ശ്രമിക്കുക
AbortRetryIgnoreIgnore=പിശക് അവഗണിച്ച് തുടരുക
AbortRetryIgnoreCancel=ഇൻസ്റ്റലേഷൻ റദ്ദാക്കുക
RetryCancelSelectAction=പ്രവർത്തനം തിരഞ്ഞെടുക്കുക
RetryCancelRetry=വീണ്ടും ശ്രമിക്കുക
RetryCancelCancel=റദ്ദാക്കുക

; *** Installation status messages
StatusClosingApplications=ആപ്ലിക്കേഷനുകൾ അടയ്ക്കുന്നു...
StatusCreateDirs=ഡയറക്ടറികൾ സൃഷ്ടിക്കുന്നു...
StatusExtractFiles=ഫയലുകൾ എക്സ്ട്രാക്റ്റ് ചെയ്യുന്നു...
StatusDownloadFiles=ഫയലുകൾ ഡൗൺലോഡ് ചെയ്യുന്നു...
StatusCreateIcons=കുറുക്കുവഴികൾ സൃഷ്ടിക്കുന്നു...
StatusCreateIniEntries=INI എൻട്രികൾ സൃഷ്ടിക്കുന്നു...
StatusCreateRegistryEntries=റജിസ്ട്രി എൻട്രികൾ സൃഷ്ടിക്കുന്നു...
StatusRegisterFiles=ഫയലുകൾ രജിസ്റ്റർ ചെയ്യുന്നു...
StatusSavingUninstall=അൺഇൻസ്റ്റാൾ വിവരങ്ങൾ സംരക്ഷിക്കുന്നു...
StatusRunProgram=ഇൻസ്റ്റലേഷൻ പൂർത്തിയാക്കുന്നു...
StatusRestartingApplications=ആപ്ലിക്കേഷനുകൾ റീസ്റ്റാർട്ട് ചെയ്യുന്നു...
StatusRollback=മാറ്റങ്ങൾ പഴയപടിയാക്കുന്നു...

; *** Misc. errors
ErrorInternal2=ആന്തരിക പിശക്: %1
ErrorFunctionFailedNoCode=%1 പരാജയപ്പെട്ടു
ErrorFunctionFailed=%1 പരാജയപ്പെട്ടു; കോഡ് %2
ErrorFunctionFailedWithMessage=%1 പരാജയപ്പെട്ടു; കോഡ് %2.%n%3
ErrorExecutingProgram=ഫയൽ പ്രവർത്തിപ്പിക്കാനായില്ല:%n%1

; *** Registry errors
ErrorRegOpenKey=റജിസ്ട്രി കീ തുറക്കുന്നതിൽ പിശക്:%n%1\%2
ErrorRegCreateKey=റജിസ്ട്രി കീ സൃഷ്ടിക്കുന്നതിൽ പിശക്:%n%1\%2
ErrorRegWriteKey=റജിസ്ട്രി കീയിൽ എഴുതുന്നതിൽ പിശക്:%n%1\%2

; *** INI errors
ErrorIniEntry="%1" ഫയലിൽ INI എൻട്രി സൃഷ്ടിക്കുന്നതിൽ പിശക്.

; *** File copying errors
FileAbortRetryIgnoreSkipNotRecommended=ഈ ഫയൽ ഒഴിവാക്കുക (ശുപാർശ ചെയ്യുന്നില്ല)
FileAbortRetryIgnoreIgnoreNotRecommended=പിശക് അവഗണിച്ച് തുടരുക (ശുപാർശ ചെയ്യുന്നില്ല)
SourceIsCorrupted=സോഴ്സ് ഫയൽ കേടായി
SourceDoesntExist="%1" എന്ന സോഴ്സ് ഫയൽ നിലവിലില്ല
SourceVerificationFailed=സോഴ്സ് ഫയലിന്റെ പരിശോധന പരാജയപ്പെട്ടു: %1
VerificationSignatureDoesntExist="%1" എന്ന ഒപ്പ് ഫയൽ നിലവിലില്ല
VerificationSignatureInvalid="%1" എന്ന ഒപ്പ് ഫയൽ അസാധുവാണ്
VerificationKeyNotFound="%1" എന്ന ഒപ്പ് ഫയൽ അജ്ഞാതമായ ഒരു കീ ഉപയോഗിക്കുന്നു
VerificationFileNameIncorrect=ഫയലിന്റെ പേര് തെറ്റാണ്
VerificationFileTagIncorrect=ഫയലിന്റെ ടാഗ് തെറ്റാണ്
VerificationFileSizeIncorrect=ഫയലിന്റെ വലുപ്പം തെറ്റാണ്
VerificationFileHashIncorrect=ഫയലിന്റെ ഹാഷ് തെറ്റാണ്
ExistingFileReadOnly2=നിലവിലുള്ള ഫയൽ റീഡ്-ഒൺലി ആയി അടയാളപ്പെടുത്തിയിരിക്കുന്നതിനാൽ മാറ്റിസ്ഥാപിക്കാനായില്ല.
ExistingFileReadOnlyRetry=റീഡ്-ഒൺലി ആട്രിബ്യൂട്ട് നീക്കി വീണ്ടും ശ്രമിക്കുക
ExistingFileReadOnlyKeepExisting=നിലവിലുള്ള ഫയൽ നിലനിർത്തുക
ErrorReadingExistingDest=നിലവിലുള്ള ഫയൽ വായിക്കാൻ ശ്രമിക്കുമ്പോൾ ഒരു പിശക് സംഭവിച്ചു:
FileExistsSelectAction=പ്രവർത്തനം തിരഞ്ഞെടുക്കുക
FileExists2=ഫയൽ ഇതിനകം നിലവിലുണ്ട്.
FileExistsOverwriteExisting=നിലവിലുള്ള ഫയലിനു മുകളിൽ എഴുതുക
FileExistsKeepExisting=നിലവിലുള്ള ഫയൽ നിലനിർത്തുക
FileExistsOverwriteOrKeepAll=അടുത്ത വൈരുദ്ധ്യങ്ങൾക്കും ഇത് ചെയ്യുക
ExistingFileNewerSelectAction=പ്രവർത്തനം തിരഞ്ഞെടുക്കുക
ExistingFileNewer2=നിലവിലുള്ള ഫയൽ സെറ്റപ്പ് ഇൻസ്റ്റാൾ ചെയ്യാൻ ശ്രമിക്കുന്നതിനേക്കാൾ പുതിയതാണ്.
ExistingFileNewerOverwriteExisting=നിലവിലുള്ള ഫയലിനു മുകളിൽ എഴുതുക
ExistingFileNewerKeepExisting=നിലവിലുള്ള ഫയൽ നിലനിർത്തുക (ശുപാർശ ചെയ്യുന്നത്)
ExistingFileNewerOverwriteOrKeepAll=അടുത്ത വൈരുദ്ധ്യങ്ങൾക്കും ഇത് ചെയ്യുക
ErrorChangingAttr=നിലവിലുള്ള ഫയലിന്റെ ആട്രിബ്യൂട്ടുകൾ മാറ്റാൻ ശ്രമിക്കുമ്പോൾ ഒരു പിശക് സംഭവിച്ചു:
ErrorCreatingTemp=ലക്ഷ്യ ഡയറക്ടറിയിൽ ഒരു ഫയൽ സൃഷ്ടിക്കാൻ ശ്രമിക്കുമ്പോൾ ഒരു പിശക് സംഭവിച്ചു:
ErrorReadingSource=സോഴ്സ് ഫയൽ വായിക്കാൻ ശ്രമിക്കുമ്പോൾ ഒരു പിശക് സംഭവിച്ചു:
ErrorCopying=ഒരു ഫയൽ പകർത്താൻ ശ്രമിക്കുമ്പോൾ ഒരു പിശക് സംഭവിച്ചു:
ErrorDownloading=ഒരു ഫയൽ ഡൗൺലോഡ് ചെയ്യാൻ ശ്രമിക്കുമ്പോൾ ഒരു പിശക് സംഭവിച്ചു:
ErrorExtracting=ഒരു ആർക്കൈവ് എക്സ്ട്രാക്റ്റ് ചെയ്യാൻ ശ്രമിക്കുമ്പോൾ ഒരു പിശക് സംഭവിച്ചു:
ErrorReplacingExistingFile=നിലവിലുള്ള ഫയൽ മാറ്റിസ്ഥാപിക്കാൻ ശ്രമിക്കുമ്പോൾ ഒരു പിശക് സംഭവിച്ചു:
ErrorRestartReplace=RestartReplace പരാജയപ്പെട്ടു:
ErrorRenamingTemp=ലക്ഷ്യ ഡയറക്ടറിയിലെ ഒരു ഫയലിന്റെ പേര് മാറ്റാൻ ശ്രമിക്കുമ്പോൾ ഒരു പിശക് സംഭവിച്ചു:
ErrorRegisterServer=DLL/OCX രജിസ്റ്റർ ചെയ്യാനായില്ല: %1
ErrorRegSvr32Failed=RegSvr32 എക്സിറ്റ് കോഡ് %1 ഉപയോഗിച്ച് പരാജയപ്പെട്ടു
ErrorRegisterTypeLib=ടൈപ്പ് ലൈബ്രറി രജിസ്റ്റർ ചെയ്യാനായില്ല: %1

; *** Uninstall display name markings
; used for example as 'My Program (32-bit)'
UninstallDisplayNameMark=%1 (%2)
; used for example as 'My Program (32-bit, All users)'
UninstallDisplayNameMarks=%1 (%2, %3)
UninstallDisplayNameMark32Bit=32-ബിറ്റ്
UninstallDisplayNameMark64Bit=64-ബിറ്റ്
UninstallDisplayNameMarkAllUsers=എല്ലാ ഉപയോക്താക്കളും
UninstallDisplayNameMarkCurrentUser=നിലവിലെ ഉപയോക്താവ്

; *** Post-installation errors
ErrorOpeningReadme=README ഫയൽ തുറക്കാൻ ശ്രമിക്കുമ്പോൾ ഒരു പിശക് സംഭവിച്ചു.
ErrorRestartingComputer=കമ്പ്യൂട്ടർ റീസ്റ്റാർട്ട് ചെയ്യാൻ സെറ്റപ്പിന് കഴിഞ്ഞില്ല. ദയവായി ഇത് സ്വമേധയാ ചെയ്യുക.

; *** Uninstaller messages
UninstallNotFound="%1" ഫയൽ നിലവിലില്ല. അൺഇൻസ്റ്റാൾ ചെയ്യാനാകില്ല.
UninstallOpenError="%1" ഫയൽ തുറക്കാനായില്ല. അൺഇൻസ്റ്റാൾ ചെയ്യാനാകില്ല
UninstallUnsupportedVer=അൺഇൻസ്റ്റാൾ ലോഗ് ഫയൽ "%1" ഈ അൺഇൻസ്റ്റാളറിന്റെ പതിപ്പിന് തിരിച്ചറിയാനാകാത്ത ഫോർമാറ്റിലാണ്. അൺഇൻസ്റ്റാൾ ചെയ്യാനാകില്ല
UninstallUnknownEntry=അൺഇൻസ്റ്റാൾ ലോഗിൽ ഒരു അജ്ഞാത എൻട്രി (%1) കണ്ടെത്തി
ConfirmUninstall=%1 ഉം അതിന്റെ എല്ലാ ഘടകങ്ങളും പൂർണ്ണമായും നീക്കം ചെയ്യണമെന്ന് ഉറപ്പാണോ?
UninstallOnlyOnWin64=ഈ ഇൻസ്റ്റലേഷൻ 64-ബിറ്റ് Windows-ൽ മാത്രമേ അൺഇൻസ്റ്റാൾ ചെയ്യാനാകൂ.
OnlyAdminCanUninstall=അഡ്മിനിസ്ട്രേറ്റീവ് അവകാശങ്ങളുള്ള ഉപയോക്താവിന് മാത്രമേ ഈ ഇൻസ്റ്റലേഷൻ അൺഇൻസ്റ്റാൾ ചെയ്യാനാകൂ.
UninstallStatusLabel=%1 നിങ്ങളുടെ കമ്പ്യൂട്ടറിൽ നിന്ന് നീക്കം ചെയ്യുന്നതുവരെ കാത്തിരിക്കുക.
UninstalledAll=%1 നിങ്ങളുടെ കമ്പ്യൂട്ടറിൽ നിന്ന് വിജയകരമായി നീക്കം ചെയ്തു.
UninstalledMost=%1 അൺഇൻസ്റ്റാൾ പൂർത്തിയായി.%n%nചില ഘടകങ്ങൾ നീക്കം ചെയ്യാനായില്ല. ഇവ സ്വമേധയാ നീക്കം ചെയ്യാം.
UninstalledAndNeedsRestart=%1 അൺഇൻസ്റ്റാൾ പൂർത്തിയാക്കാൻ നിങ്ങളുടെ കമ്പ്യൂട്ടർ റീസ്റ്റാർട്ട് ചെയ്യണം.%n%nഇപ്പോൾ റീസ്റ്റാർട്ട് ചെയ്യണോ?
UninstallDataCorrupted="%1" ഫയൽ കേടായി. അൺഇൻസ്റ്റാൾ ചെയ്യാനാകില്ല

; *** Uninstallation phase messages
ConfirmDeleteSharedFileTitle=പങ്കിട്ട ഫയൽ നീക്കം ചെയ്യണോ?
ConfirmDeleteSharedFile2=ഇനിപ്പറയുന്ന പങ്കിട്ട ഫയൽ ഇപ്പോൾ ഒരു പ്രോഗ്രാമും ഉപയോഗിക്കുന്നില്ലെന്ന് സിസ്റ്റം സൂചിപ്പിക്കുന്നു. ഈ പങ്കിട്ട ഫയൽ അൺഇൻസ്റ്റാളർ നീക്കം ചെയ്യണോ?%n%nഏതെങ്കിലും പ്രോഗ്രാം ഇപ്പോഴും ഈ ഫയൽ ഉപയോഗിക്കുന്നുണ്ടെങ്കിൽ, ഇത് നീക്കം ചെയ്താൽ ആ പ്രോഗ്രാമുകൾ ശരിയായി പ്രവർത്തിച്ചേക്കില്ല. ഉറപ്പില്ലെങ്കിൽ “ഇല്ല” തിരഞ്ഞെടുക്കുക. ഫയൽ സിസ്റ്റത്തിൽ നിലനിർത്തുന്നത് ഒരു ദോഷവും ചെയ്യില്ല.
SharedFileNameLabel=ഫയലിന്റെ പേര്:
SharedFileLocationLabel=സ്ഥാനം:
WizardUninstalling=അൺഇൻസ്റ്റാൾ നില
StatusUninstalling=%1 അൺഇൻസ്റ്റാൾ ചെയ്യുന്നു...

; *** Shutdown block reasons
ShutdownBlockReasonInstallingApp=%1 ഇൻസ്റ്റാൾ ചെയ്യുന്നു.
ShutdownBlockReasonUninstallingApp=%1 അൺഇൻസ്റ്റാൾ ചെയ്യുന്നു.

; The custom messages below aren't used by Setup itself, but if you make
; use of them in your scripts, you'll want to translate them.

[CustomMessages]

NameAndVersion=%1 പതിപ്പ് %2
AdditionalIcons=അധിക കുറുക്കുവഴികൾ:
CreateDesktopIcon=ഒരു ഡെസ്ക്ടോപ്പ് കുറുക്കുവഴി സൃഷ്ടിക്കുക
CreateQuickLaunchIcon=ഒരു ക്വിക്ക് ലോഞ്ച് കുറുക്കുവഴി സൃഷ്ടിക്കുക
ProgramOnTheWeb=വെബിൽ %1
UninstallProgram=%1 അൺഇൻസ്റ്റാൾ ചെയ്യുക
LaunchProgram=%1 ആരംഭിക്കുക
AssocFileExtension=%1 നെ %2 ഫയൽ എക്സ്റ്റൻഷനുമായി ബന്ധിപ്പിക്കുക
AssocingFileExtension=%1 നെ %2 ഫയൽ എക്സ്റ്റൻഷനുമായി ബന്ധിപ്പിക്കുന്നു...
AutoStartProgramGroupDescription=സ്റ്റാർട്ടപ്പ്:
AutoStartProgram=%1 സ്വയമേവ ആരംഭിക്കുക
AddonHostProgramNotFound=നിങ്ങൾ തിരഞ്ഞെടുത്ത ഫോൾഡറിൽ %1 കണ്ടെത്താനായില്ല.%n%nഎന്നിട്ടും തുടരണോ?

; The messages that are Lang Switcher's own (the sample is English.isl).
LocalCertReady=സെറ്റപ്പ് ഈ കമ്പ്യൂട്ടറിൽ ഒരു സർട്ടിഫിക്കറ്റ് സൃഷ്ടിച്ച് അതുപയോഗിച്ച് Lang Switcher-ൽ ഒപ്പിടും; അങ്ങനെ അഡ്മിനിസ്ട്രേറ്ററായി പ്രവർത്തിപ്പിച്ച പ്രോഗ്രാമുകളുടെ വിൻഡോകളിലും ഇത് പ്രവർത്തിക്കും.
LocalCertFailed=Lang Switcher ഇൻസ്റ്റാൾ ചെയ്തു, എന്നാൽ അഡ്മിനിസ്ട്രേറ്ററായി പ്രവർത്തിപ്പിച്ച പ്രോഗ്രാമുകളുടെ വിൻഡോകളിൽ ഇത് പ്രവർത്തിക്കില്ല: ഈ കമ്പ്യൂട്ടറിൽ സൃഷ്ടിച്ച സർട്ടിഫിക്കറ്റ് Windows സ്വീകരിച്ചില്ല. വിശദാംശങ്ങൾക്ക് സെറ്റപ്പ് ലോഗ് കാണുക.
RemoveSettingsQuestion=Lang Switcher ക്രമീകരണ ഫയലും നീക്കം ചെയ്യണോ?
RemoveSettingsKeep=വീണ്ടും ഇൻസ്റ്റാൾ ചെയ്യുമ്പോൾ ഉപയോഗിക്കാൻ ക്രമീകരണങ്ങൾ നിലനിർത്തണമെങ്കിൽ “ഇല്ല” തിരഞ്ഞെടുക്കുക.
AuthorSlogan=ഹൃദയത്തോടെ നിർമ്മിച്ച പ്രോഗ്രാമുകൾ
