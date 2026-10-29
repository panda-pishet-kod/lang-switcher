; Lang Switcher -- the messages of the installer in Uyghur (stage E108, group A).
;
; Inno Setup has no official Uyghur file, so this file is whole: [LangOptions], all the messages of
; compiler:Default.isl (version 6.5.0 sample) translated, then the stock [CustomMessages] of that
; file and the messages of the installer that are its own (the sample is English.isl). Read as the
; only MessagesFile of the "uyghur" entry of [Languages] in ..\LangSwitcher.iss; Default.isl may
; stand before it: what is missing here falls back to English.
; Right to left: RightToLeft=yes, no direction marks in the strings. No accelerators (&) in a file
; that has no official translation (the rule of the stage, like Tamil.isl): two equal letters in
; one window are avoided. The names of the buttons inside a message stand in the guillemets of the
; language. Messages that end without a full stop in Default.isl end without one here: Setup adds a
; full stop itself. The Uyghur alphabet of the Arabic script (its own vowels and the letters
; ئ ە ې ى ۆ ۇ ۈ ۋ ڭ ھ); the name of the program is not translated. Suffixes after the Latin names
; of the program are avoided where the sentence can be built without them.
; AuthorSlogan (stage E99, decision 167.1): the slogan of the signature of the author at the
; bottom of every page of the wizard -- after "Panda Koder" and a middle dot; light, without
; pathos. The name of the author and the address of the site stay as they are in every language.

[LangOptions]
LanguageName=ئۇيغۇرچە
LanguageID=$0480
LanguageCodePage=0
RightToLeft=yes
;DialogFontName=
;DialogFontSize=9
;DialogFontBaseScaleWidth=7
;DialogFontBaseScaleHeight=15
;WelcomeFontName=Segoe UI
;WelcomeFontSize=14

[Messages]

; *** Application titles
SetupAppTitle=ئورنىتىش
SetupWindowTitle=ئورنىتىش - %1
UninstallAppTitle=چىقىرىۋېتىش
UninstallAppFullTitle=چىقىرىۋېتىش: %1

; *** Misc. common
InformationTitle=ئۇچۇر
ConfirmTitle=جەزملەش
ErrorTitle=خاتالىق

; *** SetupLdr messages
SetupLdrStartupMessage=%1 ئورنىتىلىدۇ. داۋاملاشتۇرامسىز؟
LdrCannotCreateTemp=ۋاقىتلىق ھۆججەت قۇرغىلى بولمىدى. ئورنىتىش توختىتىلدى
LdrCannotExecTemp=ۋاقىتلىق قىسقۇچتىكى ھۆججەتنى ئىجرا قىلغىلى بولمىدى. ئورنىتىش توختىتىلدى
HelpTextNote=

; *** Startup error messages
LastErrorMessage=%1.%n%nخاتالىق %2: %3
SetupFileMissing=%1 ھۆججىتى ئورنىتىش قىسقۇچىدا يوق. مەسىلىنى ھەل قىلىڭ ياكى پروگراممىنىڭ يېڭى نۇسخىسىغا ئېرىشىڭ.
SetupFileCorrupt=ئورنىتىش ھۆججەتلىرى بۇزۇلغان. پروگراممىنىڭ يېڭى نۇسخىسىغا ئېرىشىڭ.
SetupFileCorruptOrWrongVer=ئورنىتىش ھۆججەتلىرى بۇزۇلغان ياكى ئورنىتىشنىڭ بۇ نەشرىگە ماس كەلمەيدۇ. مەسىلىنى ھەل قىلىڭ ياكى پروگراممىنىڭ يېڭى نۇسخىسىغا ئېرىشىڭ.
InvalidParameter=بۇيرۇق قۇرىدا ئىناۋەتسىز پارامېتىر بېرىلدى:%n%n%1
SetupAlreadyRunning=ئورنىتىش ئاللىقاچان ئىجرا بولۇۋاتىدۇ.
WindowsVersionNotSupported=بۇ پروگرامما كومپيۇتېرىڭىزدا ئىجرا بولۇۋاتقان Windows نەشرىنى قوللىمايدۇ.
WindowsServicePackRequired=بۇ پروگراممىغا كېرەك: %1 Service Pack %2 ياكى ئۇنىڭدىن يېڭىسى.
NotOnThisPlatform=بۇ پروگرامما تۆۋەندىكى سىستېمىدا ئىجرا بولمايدۇ: %1.
OnlyOnThisPlatform=بۇ پروگراممىنى مۇنۇ سىستېمىدا ئىجرا قىلىش كېرەك: %1.
OnlyOnTheseArchitectures=بۇ پروگراممىنى پەقەت تۆۋەندىكى پروسېسسور قۇرۇلمىلىرى ئۈچۈن لايىھەلەنگەن Windows نەشرىلىرىگە ئورنىتىشقا بولىدۇ:%n%n%1
WinVersionTooLowError=بۇ پروگراممىغا كېرەك: %1، نەشرى %2 ياكى ئۇنىڭدىن يېڭىسى.
WinVersionTooHighError=بۇ پروگراممىنى ئورنىتىشقا بولمايدۇ — سىستېما: %1، نەشرى %2 ياكى ئۇنىڭدىن يېڭىسى.
AdminPrivilegesRequired=بۇ پروگراممىنى ئورنىتىشتا باشقۇرغۇچى سۈپىتىدە كىرگەن بولۇشىڭىز كېرەك.
PowerUserPrivilegesRequired=بۇ پروگراممىنى ئورنىتىشتا باشقۇرغۇچى سۈپىتىدە ياكى Power Users گۇرۇپپىسىنىڭ ئەزاسى سۈپىتىدە كىرگەن بولۇشىڭىز كېرەك.
SetupAppRunningError=ئورنىتىش بايقىدى: %1 ھازىر ئىجرا بولۇۋاتىدۇ.%n%nئۇنىڭ بارلىق ئۈلگىلىرىنى ھازىرلا تاقاڭ، ئاندىن داۋاملاشتۇرۇش ئۈچۈن «ماقۇل»، چىقىش ئۈچۈن «بىكار قىلىش» نى چېكىڭ.
UninstallAppRunningError=چىقىرىۋېتىش بايقىدى: %1 ھازىر ئىجرا بولۇۋاتىدۇ.%n%nئۇنىڭ بارلىق ئۈلگىلىرىنى ھازىرلا تاقاڭ، ئاندىن داۋاملاشتۇرۇش ئۈچۈن «ماقۇل»، چىقىش ئۈچۈن «بىكار قىلىش» نى چېكىڭ.

; *** Startup questions
PrivilegesRequiredOverrideTitle=ئورنىتىش ئۇسۇلىنى تاللاڭ
PrivilegesRequiredOverrideInstruction=ئورنىتىش ئۇسۇلىنى تاللاڭ
PrivilegesRequiredOverrideText1=%1 بارلىق ئىشلەتكۈچىلەر ئۈچۈن ئورنىتىلىشى مۇمكىن (باشقۇرغۇچى ھوقۇقى كېرەك)، ياكى پەقەت سىز ئۈچۈن.
PrivilegesRequiredOverrideText2=%1 پەقەت سىز ئۈچۈن ئورنىتىلىشى مۇمكىن، ياكى بارلىق ئىشلەتكۈچىلەر ئۈچۈن (باشقۇرغۇچى ھوقۇقى كېرەك).
PrivilegesRequiredOverrideAllUsers=بارلىق ئىشلەتكۈچىلەر ئۈچۈن ئورنىتىش
PrivilegesRequiredOverrideAllUsersRecommended=بارلىق ئىشلەتكۈچىلەر ئۈچۈن ئورنىتىش (تەۋسىيە قىلىنىدۇ)
PrivilegesRequiredOverrideCurrentUser=پەقەت ماڭا ئورنىتىش
PrivilegesRequiredOverrideCurrentUserRecommended=پەقەت ماڭا ئورنىتىش (تەۋسىيە قىلىنىدۇ)

; *** Misc. errors
ErrorCreatingDir=ئورنىتىش "%1" قىسقۇچىنى قۇرالمىدى
ErrorTooManyFilesInDir="%1" قىسقۇچىدا ھۆججەت بەك كۆپ بولغاچقا، ئۇنىڭدا ھۆججەت قۇرغىلى بولمىدى

; *** Setup common messages
ExitSetupTitle=ئورنىتىشتىن چىقىش
ExitSetupMessage=ئورنىتىش تاماملانمىدى. ھازىر چىقسىڭىز، پروگرامما ئورنىتىلمايدۇ.%n%nئورنىتىشنى تاماملاش ئۈچۈن ئورنىتىشنى باشقا ۋاقىتتا قايتا ئىجرا قىلالايسىز.%n%nئورنىتىشتىن چىقامسىز؟
AboutSetupMenuItem=ئورنىتىش ھەققىدە...
AboutSetupTitle=ئورنىتىش ھەققىدە
AboutSetupMessage=%1 نەشرى %2%n%3%n%n%1 باش بېتى:%n%4
AboutSetupNote=
TranslatorNote=

; *** Buttons
ButtonBack=< ئارقىغا
ButtonNext=كېيىنكى >
ButtonInstall=ئورنىتىش
ButtonOK=ماقۇل
ButtonCancel=بىكار قىلىش
ButtonYes=ھەئە
ButtonYesToAll=ھەممىسىگە ھەئە
ButtonNo=ياق
ButtonNoToAll=ھەممىسىگە ياق
ButtonFinish=ئاخىرلاشتۇرۇش
ButtonBrowse=كۆرۈش...
ButtonWizardBrowse=كۆرۈش...
ButtonNewFolder=يېڭى قىسقۇچ قۇرۇش

; *** "Select Language" dialog messages
SelectLanguageTitle=ئورنىتىش تىلىنى تاللاڭ
SelectLanguageLabel=ئورنىتىش جەريانىدا ئىشلىتىدىغان تىلنى تاللاڭ.

; *** Common wizard text
ClickNext=داۋاملاشتۇرۇش ئۈچۈن «كېيىنكى»، ئورنىتىشتىن چىقىش ئۈچۈن «بىكار قىلىش» نى چېكىڭ.
BeveledLabel=
BrowseDialogTitle=قىسقۇچ ئىزدەش
BrowseDialogLabel=تۆۋەندىكى تىزىملىكتىن بىر قىسقۇچنى تاللاڭ، ئاندىن «ماقۇل» نى چېكىڭ.
NewFolderName=يېڭى قىسقۇچ

; *** "Welcome" wizard page
WelcomeLabel1=[name] ئورنىتىش يېتەكچىسىگە خۇش كېلىپسىز
WelcomeLabel2=بۇ كومپيۇتېرىڭىزغا [name/ver] نى ئورنىتىدۇ.%n%nداۋاملاشتۇرۇشتىن بۇرۇن باشقا بارلىق پروگراممىلارنى تاقاش تەۋسىيە قىلىنىدۇ.

; *** "Password" wizard page
WizardPassword=پارول
PasswordLabel1=بۇ ئورنىتىش پارول بىلەن قوغدالغان.
PasswordLabel3=پارولنى كىرگۈزۈڭ، ئاندىن داۋاملاشتۇرۇش ئۈچۈن «كېيىنكى» نى چېكىڭ. پارولدا چوڭ-كىچىك ھەرپ پەرقلىنىدۇ.
PasswordEditLabel=پارول:
IncorrectPassword=كىرگۈزگەن پارولىڭىز توغرا ئەمەس. قايتا سىناپ بېقىڭ.

; *** "License Agreement" wizard page
WizardLicense=ئىجازەت كېلىشىمى
LicenseLabel=داۋاملاشتۇرۇشتىن بۇرۇن تۆۋەندىكى مۇھىم ئۇچۇرنى ئوقۇڭ.
LicenseLabel3=تۆۋەندىكى ئىجازەت كېلىشىمىنى ئوقۇڭ. ئورنىتىشنى داۋاملاشتۇرۇشتىن بۇرۇن بۇ كېلىشىمنىڭ شەرتلىرىنى قوبۇل قىلىشىڭىز كېرەك.
LicenseAccepted=كېلىشىمنى قوبۇل قىلىمەن
LicenseNotAccepted=كېلىشىمنى قوبۇل قىلمايمەن

; *** "Information" wizard pages
WizardInfoBefore=ئۇچۇر
InfoBeforeLabel=داۋاملاشتۇرۇشتىن بۇرۇن تۆۋەندىكى مۇھىم ئۇچۇرنى ئوقۇڭ.
InfoBeforeClickLabel=ئورنىتىشنى داۋاملاشتۇرۇشقا تەييار بولغاندا «كېيىنكى» نى چېكىڭ.
WizardInfoAfter=ئۇچۇر
InfoAfterLabel=داۋاملاشتۇرۇشتىن بۇرۇن تۆۋەندىكى مۇھىم ئۇچۇرنى ئوقۇڭ.
InfoAfterClickLabel=ئورنىتىشنى داۋاملاشتۇرۇشقا تەييار بولغاندا «كېيىنكى» نى چېكىڭ.

; *** "User Information" wizard page
WizardUserInfo=ئىشلەتكۈچى ئۇچۇرى
UserInfoDesc=ئۇچۇرىڭىزنى كىرگۈزۈڭ.
UserInfoName=ئىشلەتكۈچى ئىسمى:
UserInfoOrg=تەشكىلات:
UserInfoSerial=تەرتىپ نومۇرى:
UserInfoNameRequired=ئىسىم كىرگۈزۈشىڭىز كېرەك.

; *** "Select Destination Location" wizard page
WizardSelectDir=نىشان ئورنىنى تاللاڭ
SelectDirDesc=[name] قەيەرگە ئورنىتىلسۇن؟
SelectDirLabel3=ئورنىتىش [name] نى تۆۋەندىكى قىسقۇچقا ئورنىتىدۇ.
SelectDirBrowseLabel=داۋاملاشتۇرۇش ئۈچۈن «كېيىنكى» نى چېكىڭ. باشقا قىسقۇچ تاللىماقچى بولسىڭىز «كۆرۈش» نى چېكىڭ.
DiskSpaceGBLabel=كەم دېگەندە [gb] GB بوش دىسكا بوشلۇقى كېرەك.
DiskSpaceMBLabel=كەم دېگەندە [mb] MB بوش دىسكا بوشلۇقى كېرەك.
CannotInstallToNetworkDrive=ئورنىتىش تور دىسكىسىغا ئورنىتالمايدۇ.
CannotInstallToUNCPath=ئورنىتىش UNC يولىغا ئورنىتالمايدۇ.
InvalidPath=دىسكا ھەرپى بار تولۇق يولنى كىرگۈزۈشىڭىز كېرەك؛ مەسىلەن:%n%nC:\APP%n%nياكى UNC يولى مۇنداق شەكىلدە:%n%n\\server\share
InvalidDrive=تاللىغان دىسكىڭىز ياكى UNC ئورتاقلىقى يوق ياكى زىيارەت قىلغىلى بولمايدۇ. باشقىسىنى تاللاڭ.
DiskSpaceWarningTitle=دىسكا بوشلۇقى يېتىشمەيدۇ
DiskSpaceWarning=ئورنىتىشقا كەم دېگەندە %1 KB بوش بوشلۇق كېرەك، لېكىن تاللانغان دىسكىدا پەقەت %2 KB بار.%n%nشۇنداقتىمۇ داۋاملاشتۇرامسىز؟
DirNameTooLong=قىسقۇچ ئىسمى ياكى يولى بەك ئۇزۇن.
InvalidDirName=قىسقۇچ ئىسمى ئىناۋەتسىز.
BadDirName32=قىسقۇچ ئىسمىدا تۆۋەندىكى بەلگىلەر بولسا بولمايدۇ:%n%n%1
DirExistsTitle=قىسقۇچ بار
DirExists=مۇنۇ قىسقۇچ:%n%n%1%n%nئاللىقاچان بار. شۇنداقتىمۇ شۇ قىسقۇچقا ئورنىتامسىز؟
DirDoesntExistTitle=قىسقۇچ يوق
DirDoesntExist=مۇنۇ قىسقۇچ:%n%n%1%n%nيوق. بۇ قىسقۇچنى قۇرامسىز؟

; *** "Select Components" wizard page
WizardSelectComponents=بۆلەكلەرنى تاللاڭ
SelectComponentsDesc=قايسى بۆلەكلەر ئورنىتىلسۇن؟
SelectComponentsLabel2=ئورنىتىدىغان بۆلەكلەرنى تاللاڭ؛ ئورنىتىشنى خالىمايدىغانلىرىنىڭ بەلگىسىنى ئېلىۋېتىڭ. تەييار بولغاندا «كېيىنكى» نى چېكىڭ.
FullInstallation=تولۇق ئورنىتىش
; if possible don't translate 'Compact' as 'Minimal' (I mean 'Minimal' in your language)
CompactInstallation=ئىخچام ئورنىتىش
CustomInstallation=ئىختىيارىي ئورنىتىش
NoUninstallWarningTitle=بۆلەكلەر بار
NoUninstallWarning=ئورنىتىش بايقىدى: تۆۋەندىكى بۆلەكلەر كومپيۇتېرىڭىزغا ئاللىقاچان ئورنىتىلغان:%n%n%1%n%nبۇ بۆلەكلەرنىڭ بەلگىسىنى ئېلىۋەتسىڭىز ئۇلار چىقىرىۋېتىلمەيدۇ.%n%nشۇنداقتىمۇ داۋاملاشتۇرامسىز؟
ComponentSize1=%1 KB
ComponentSize2=%1 MB
ComponentsDiskSpaceGBLabel=ھازىرقى تاللاشقا كەم دېگەندە [gb] GB دىسكا بوشلۇقى كېرەك.
ComponentsDiskSpaceMBLabel=ھازىرقى تاللاشقا كەم دېگەندە [mb] MB دىسكا بوشلۇقى كېرەك.

; *** "Select Additional Tasks" wizard page
WizardSelectTasks=قوشۇمچە ۋەزىپىلەرنى تاللاڭ
SelectTasksDesc=قايسى قوشۇمچە ۋەزىپىلەر ئورۇنلانسۇن؟
SelectTasksLabel2=[name] ئورنىتىلىۋاتقاندا ئورنىتىش ئورۇنلىسۇن دەپ خالىغان قوشۇمچە ۋەزىپىلەرنى تاللاڭ، ئاندىن «كېيىنكى» نى چېكىڭ.

; *** "Select Start Menu Folder" wizard page
WizardSelectProgramGroup=باشلاش تىزىملىكى قىسقۇچىنى تاللاڭ
SelectStartMenuFolderDesc=ئورنىتىش پروگراممىنىڭ قىسقا يوللىرىنى قەيەرگە قويسۇن؟
SelectStartMenuFolderLabel3=ئورنىتىش پروگراممىنىڭ قىسقا يوللىرىنى تۆۋەندىكى باشلاش تىزىملىكى قىسقۇچىدا قۇرىدۇ.
SelectStartMenuFolderBrowseLabel=داۋاملاشتۇرۇش ئۈچۈن «كېيىنكى» نى چېكىڭ. باشقا قىسقۇچ تاللىماقچى بولسىڭىز «كۆرۈش» نى چېكىڭ.
MustEnterGroupName=قىسقۇچ ئىسمىنى كىرگۈزۈشىڭىز كېرەك.
GroupNameTooLong=قىسقۇچ ئىسمى ياكى يولى بەك ئۇزۇن.
InvalidGroupName=قىسقۇچ ئىسمى ئىناۋەتسىز.
BadGroupName=قىسقۇچ ئىسمىدا تۆۋەندىكى بەلگىلەر بولسا بولمايدۇ:%n%n%1
NoProgramGroupCheck2=باشلاش تىزىملىكى قىسقۇچى قۇرۇلمىسۇن

; *** "Ready to Install" wizard page
WizardReady=ئورنىتىشقا تەييار
ReadyLabel1=ئورنىتىش ھازىر كومپيۇتېرىڭىزغا [name] نى ئورنىتىشقا تەييار.
ReadyLabel2a=ئورنىتىشنى داۋاملاشتۇرۇش ئۈچۈن «ئورنىتىش» نى چېكىڭ، ياكى تەڭشەكلەرنى كۆرمەكچى ياكى ئۆزگەرتمەكچى بولسىڭىز «ئارقىغا» نى چېكىڭ.
ReadyLabel2b=ئورنىتىشنى داۋاملاشتۇرۇش ئۈچۈن «ئورنىتىش» نى چېكىڭ.
ReadyMemoUserInfo=ئىشلەتكۈچى ئۇچۇرى:
ReadyMemoDir=نىشان ئورنى:
ReadyMemoType=ئورنىتىش تۈرى:
ReadyMemoComponents=تاللانغان بۆلەكلەر:
ReadyMemoGroup=باشلاش تىزىملىكى قىسقۇچى:
ReadyMemoTasks=قوشۇمچە ۋەزىپىلەر:

; *** TDownloadWizardPage wizard page and DownloadTemporaryFile
DownloadingLabel2=ھۆججەتلەر چۈشۈرۈلىۋاتىدۇ...
ButtonStopDownload=چۈشۈرۈشنى توختىتىش
StopDownload=چۈشۈرۈشنى راستتىنلا توختاتماقچىمۇسىز؟
ErrorDownloadAborted=چۈشۈرۈش توختىتىلدى
ErrorDownloadFailed=چۈشۈرۈش مەغلۇب بولدى: %1 %2
ErrorDownloadSizeFailed=چوڭلۇقىنى ئېلىش مەغلۇب بولدى: %1 %2
ErrorProgress=ئىناۋەتسىز ئىلگىرىلەش: %1 / %2
ErrorFileSize=ھۆججەت چوڭلۇقى ئىناۋەتسىز: كۈتۈلگىنى %1، تېپىلغىنى %2

; *** TExtractionWizardPage wizard page and ExtractArchive
ExtractingLabel=ھۆججەتلەر ئېچىلىۋاتىدۇ...
ButtonStopExtraction=ئېچىشنى توختىتىش
StopExtraction=ئېچىشنى راستتىنلا توختاتماقچىمۇسىز؟
ErrorExtractionAborted=ئېچىش توختىتىلدى
ErrorExtractionFailed=ئېچىش مەغلۇب بولدى: %1

; *** Archive extraction failure details
ArchiveIncorrectPassword=پارول خاتا
ArchiveIsCorrupted=ئارخىپ بۇزۇلغان
ArchiveUnsupportedFormat=ئارخىپ فورماتى قوللىنىلمايدۇ

; *** "Preparing to Install" wizard page
WizardPreparing=ئورنىتىشقا تەييارلىق
PreparingDesc=ئورنىتىش كومپيۇتېرىڭىزغا [name] نى ئورنىتىشقا تەييارلىنىۋاتىدۇ.
PreviousInstallNotCompleted=ئالدىنقى پروگراممىنىڭ ئورنىتىلىشى ياكى چىقىرىلىشى تاماملانمىغان. ئۇ ئورنىتىشنى تاماملاش ئۈچۈن كومپيۇتېرىڭىزنى قايتا قوزغىتىشىڭىز كېرەك.%n%nكومپيۇتېرنى قايتا قوزغاتقاندىن كېيىن [name] نى ئورنىتىشنى تاماملاش ئۈچۈن ئورنىتىشنى قايتا ئىجرا قىلىڭ.
CannotContinue=ئورنىتىشنى داۋاملاشتۇرغىلى بولمايدۇ. چىقىش ئۈچۈن «بىكار قىلىش» نى چېكىڭ.
ApplicationsFound=تۆۋەندىكى پروگراممىلار ئورنىتىش يېڭىلايدىغان ھۆججەتلەرنى ئىشلىتىۋاتىدۇ. ئورنىتىشنىڭ بۇ پروگراممىلارنى ئاپتوماتىك تاقىشىغا يول قويۇش تەۋسىيە قىلىنىدۇ.
ApplicationsFound2=تۆۋەندىكى پروگراممىلار ئورنىتىش يېڭىلايدىغان ھۆججەتلەرنى ئىشلىتىۋاتىدۇ. ئورنىتىشنىڭ بۇ پروگراممىلارنى ئاپتوماتىك تاقىشىغا يول قويۇش تەۋسىيە قىلىنىدۇ. ئورنىتىش تاماملانغاندىن كېيىن، ئورنىتىش بۇ پروگراممىلارنى قايتا قوزغىتىشقا ئۇرۇنىدۇ.
CloseApplications=پروگراممىلارنى ئاپتوماتىك تاقاش
DontCloseApplications=پروگراممىلارنى تاقىماسلىق
ErrorCloseApplications=ئورنىتىش بارلىق پروگراممىلارنى ئاپتوماتىك تاقىيالمىدى. داۋاملاشتۇرۇشتىن بۇرۇن ئورنىتىش يېڭىلايدىغان ھۆججەتلەرنى ئىشلىتىۋاتقان بارلىق پروگراممىلارنى تاقاش تەۋسىيە قىلىنىدۇ.
PrepareToInstallNeedsRestart=ئورنىتىش كومپيۇتېرىڭىزنى قايتا قوزغىتىشى كېرەك. قايتا قوزغاتقاندىن كېيىن [name] نى ئورنىتىشنى تاماملاش ئۈچۈن ئورنىتىشنى قايتا ئىجرا قىلىڭ.%n%nھازىر قايتا قوزغىتامسىز؟

; *** "Installing" wizard page
WizardInstalling=ئورنىتىلىۋاتىدۇ
InstallingLabel=ئورنىتىش كومپيۇتېرىڭىزغا [name] نى ئورنىتىپ بولغۇچە سەل ساقلاڭ.

; *** "Setup Completed" wizard page
FinishedHeadingLabel=[name] ئورنىتىش يېتەكچىسى تاماملىنىۋاتىدۇ
FinishedLabelNoIcons=ئورنىتىش كومپيۇتېرىڭىزغا [name] نى ئورنىتىپ بولدى.
FinishedLabel=ئورنىتىش كومپيۇتېرىڭىزغا [name] نى ئورنىتىپ بولدى. ئورنىتىلغان قىسقا يوللارنى تاللاپ پروگراممىنى ئىجرا قىلىشقا بولىدۇ.
ClickFinish=ئورنىتىشتىن چىقىش ئۈچۈن «ئاخىرلاشتۇرۇش» نى چېكىڭ.
FinishedRestartLabel=[name] نى ئورنىتىشنى تاماملاش ئۈچۈن ئورنىتىش كومپيۇتېرىڭىزنى قايتا قوزغىتىشى كېرەك. ھازىر قايتا قوزغىتامسىز؟
FinishedRestartMessage=[name] نى ئورنىتىشنى تاماملاش ئۈچۈن ئورنىتىش كومپيۇتېرىڭىزنى قايتا قوزغىتىشى كېرەك.%n%nھازىر قايتا قوزغىتامسىز؟
ShowReadmeCheck=ھەئە، README ھۆججىتىنى كۆرمەكچىمەن
YesRadio=ھەئە، كومپيۇتېرنى ھازىر قايتا قوزغىتىش
NoRadio=ياق، كومپيۇتېرنى كېيىن ئۆزۈم قايتا قوزغىتىمەن
; used for example as 'Run MyProg.exe'
RunEntryExec=ئىجرا قىلىش: %1
; used for example as 'View Readme.txt'
RunEntryShellExec=كۆرۈش: %1

; *** "Setup Needs the Next Disk" stuff
ChangeDiskTitle=ئورنىتىشقا كېيىنكى دىسكا كېرەك
SelectDiskLabel2=دىسكا %1 نى سېلىڭ ۋە «ماقۇل» نى چېكىڭ.%n%nئەگەر بۇ دىسكىدىكى ھۆججەتلەر تۆۋەندە كۆرسىتىلگەندىن باشقا قىسقۇچتا بولسا، توغرا يولنى كىرگۈزۈڭ ياكى «كۆرۈش» نى چېكىڭ.
PathLabel=يول:
FileNotInDir2="%1" ھۆججىتى "%2" دىن تېپىلمىدى. توغرا دىسكىنى سېلىڭ ياكى باشقا قىسقۇچنى تاللاڭ.
SelectDirectoryLabel=كېيىنكى دىسكىنىڭ ئورنىنى كۆرسىتىڭ.

; *** Installation phase messages
SetupAborted=ئورنىتىش تاماملانمىدى.%n%nمەسىلىنى ھەل قىلىپ ئورنىتىشنى قايتا ئىجرا قىلىڭ.
AbortRetryIgnoreSelectAction=ھەرىكەتنى تاللاڭ
AbortRetryIgnoreRetry=قايتا سىناش
AbortRetryIgnoreIgnore=خاتالىققا پەرۋا قىلماي داۋاملاشتۇرۇش
AbortRetryIgnoreCancel=ئورنىتىشنى بىكار قىلىش
RetryCancelSelectAction=ھەرىكەتنى تاللاڭ
RetryCancelRetry=قايتا سىناش
RetryCancelCancel=بىكار قىلىش

; *** Installation status messages
StatusClosingApplications=پروگراممىلار تاقىلىۋاتىدۇ...
StatusCreateDirs=قىسقۇچلار قۇرۇلۇۋاتىدۇ...
StatusExtractFiles=ھۆججەتلەر ئېچىلىۋاتىدۇ...
StatusDownloadFiles=ھۆججەتلەر چۈشۈرۈلىۋاتىدۇ...
StatusCreateIcons=قىسقا يوللار قۇرۇلۇۋاتىدۇ...
StatusCreateIniEntries=INI تۈرلىرى قۇرۇلۇۋاتىدۇ...
StatusCreateRegistryEntries=رېيېستىر تۈرلىرى قۇرۇلۇۋاتىدۇ...
StatusRegisterFiles=ھۆججەتلەر تىزىملىتىلىۋاتىدۇ...
StatusSavingUninstall=چىقىرىۋېتىش ئۇچۇرى ساقلىنىۋاتىدۇ...
StatusRunProgram=ئورنىتىش ئاخىرلاشتۇرۇلۇۋاتىدۇ...
StatusRestartingApplications=پروگراممىلار قايتا قوزغىتىلىۋاتىدۇ...
StatusRollback=ئۆزگەرتىشلەر قايتۇرۇلۇۋاتىدۇ...

; *** Misc. errors
ErrorInternal2=ئىچكى خاتالىق: %1
ErrorFunctionFailedNoCode=%1 مەغلۇب بولدى
ErrorFunctionFailed=%1 مەغلۇب بولدى؛ كود %2
ErrorFunctionFailedWithMessage=%1 مەغلۇب بولدى؛ كود %2.%n%3
ErrorExecutingProgram=ھۆججەتنى ئىجرا قىلغىلى بولمىدى:%n%1

; *** Registry errors
ErrorRegOpenKey=رېيېستىر ئاچقۇچىنى ئېچىشتا خاتالىق:%n%1\%2
ErrorRegCreateKey=رېيېستىر ئاچقۇچىنى قۇرۇشتا خاتالىق:%n%1\%2
ErrorRegWriteKey=رېيېستىر ئاچقۇچىغا يېزىشتا خاتالىق:%n%1\%2

; *** INI errors
ErrorIniEntry="%1" ھۆججىتىدە INI تۈرىنى قۇرۇشتا خاتالىق.

; *** File copying errors
FileAbortRetryIgnoreSkipNotRecommended=بۇ ھۆججەتنى ئاتلاپ ئۆتۈش (تەۋسىيە قىلىنمايدۇ)
FileAbortRetryIgnoreIgnoreNotRecommended=خاتالىققا پەرۋا قىلماي داۋاملاشتۇرۇش (تەۋسىيە قىلىنمايدۇ)
SourceIsCorrupted=مەنبە ھۆججەت بۇزۇلغان
SourceDoesntExist="%1" مەنبە ھۆججىتى يوق
SourceVerificationFailed=مەنبە ھۆججەتنى دەلىللەش مەغلۇب بولدى: %1
VerificationSignatureDoesntExist="%1" ئىمزا ھۆججىتى يوق
VerificationSignatureInvalid="%1" ئىمزا ھۆججىتى ئىناۋەتسىز
VerificationKeyNotFound="%1" ئىمزا ھۆججىتى نامەلۇم ئاچقۇچنى ئىشلىتىدۇ
VerificationFileNameIncorrect=ھۆججەت ئىسمى خاتا
VerificationFileTagIncorrect=ھۆججەت بەلگىسى خاتا
VerificationFileSizeIncorrect=ھۆججەت چوڭلۇقى خاتا
VerificationFileHashIncorrect=ھۆججەتنىڭ ھاش قىممىتى خاتا
ExistingFileReadOnly2=بار ھۆججەتنى ئالماشتۇرغىلى بولمىدى، چۈنكى ئۇ «پەقەت ئوقۇشقا بولىدىغان» دەپ بەلگىلەنگەن.
ExistingFileReadOnlyRetry=پەقەت ئوقۇشقا بولىدىغان خاسلىقنى چىقىرىۋېتىپ قايتا سىناش
ExistingFileReadOnlyKeepExisting=بار ھۆججەتنى ساقلاپ قېلىش
ErrorReadingExistingDest=بار ھۆججەتنى ئوقۇشقا ئۇرۇنغاندا خاتالىق كۆرۈلدى:
FileExistsSelectAction=ھەرىكەتنى تاللاڭ
FileExists2=ھۆججەت ئاللىقاچان بار.
FileExistsOverwriteExisting=بار ھۆججەتنىڭ ئۈستىگە يېزىش
FileExistsKeepExisting=بار ھۆججەتنى ساقلاپ قېلىش
FileExistsOverwriteOrKeepAll=كېيىنكى توقۇنۇشلاردىمۇ شۇنداق قىلىش
ExistingFileNewerSelectAction=ھەرىكەتنى تاللاڭ
ExistingFileNewer2=بار ھۆججەت ئورنىتىش ئورنىتىماقچى بولغان ھۆججەتتىن يېڭىراق.
ExistingFileNewerOverwriteExisting=بار ھۆججەتنىڭ ئۈستىگە يېزىش
ExistingFileNewerKeepExisting=بار ھۆججەتنى ساقلاپ قېلىش (تەۋسىيە قىلىنىدۇ)
ExistingFileNewerOverwriteOrKeepAll=كېيىنكى توقۇنۇشلاردىمۇ شۇنداق قىلىش
ErrorChangingAttr=بار ھۆججەتنىڭ خاسلىقىنى ئۆزگەرتىشكە ئۇرۇنغاندا خاتالىق كۆرۈلدى:
ErrorCreatingTemp=نىشان قىسقۇچتا ھۆججەت قۇرۇشقا ئۇرۇنغاندا خاتالىق كۆرۈلدى:
ErrorReadingSource=مەنبە ھۆججەتنى ئوقۇشقا ئۇرۇنغاندا خاتالىق كۆرۈلدى:
ErrorCopying=ھۆججەتنى كۆچۈرۈشكە ئۇرۇنغاندا خاتالىق كۆرۈلدى:
ErrorDownloading=ھۆججەت چۈشۈرۈشكە ئۇرۇنغاندا خاتالىق كۆرۈلدى:
ErrorExtracting=ئارخىپنى ئېچىشقا ئۇرۇنغاندا خاتالىق كۆرۈلدى:
ErrorReplacingExistingFile=بار ھۆججەتنى ئالماشتۇرۇشقا ئۇرۇنغاندا خاتالىق كۆرۈلدى:
ErrorRestartReplace=RestartReplace مەغلۇب بولدى:
ErrorRenamingTemp=نىشان قىسقۇچتىكى ھۆججەتنىڭ ئىسمىنى ئۆزگەرتىشكە ئۇرۇنغاندا خاتالىق كۆرۈلدى:
ErrorRegisterServer=DLL/OCX نى تىزىملاتقىلى بولمىدى: %1
ErrorRegSvr32Failed=RegSvr32 چىقىش كودى %1 بىلەن مەغلۇب بولدى
ErrorRegisterTypeLib=تىپ ئامبىرىنى تىزىملاتقىلى بولمىدى: %1

; *** Uninstall display name markings
; used for example as 'My Program (32-bit)'
UninstallDisplayNameMark=%1 (%2)
; used for example as 'My Program (32-bit, All users)'
UninstallDisplayNameMarks=%1 (%2، %3)
UninstallDisplayNameMark32Bit=32 بىتلىق
UninstallDisplayNameMark64Bit=64 بىتلىق
UninstallDisplayNameMarkAllUsers=بارلىق ئىشلەتكۈچىلەر
UninstallDisplayNameMarkCurrentUser=ھازىرقى ئىشلەتكۈچى

; *** Post-installation errors
ErrorOpeningReadme=README ھۆججىتىنى ئېچىشقا ئۇرۇنغاندا خاتالىق كۆرۈلدى.
ErrorRestartingComputer=ئورنىتىش كومپيۇتېرنى قايتا قوزغىتالمىدى. بۇنى قولدا قىلىڭ.

; *** Uninstaller messages
UninstallNotFound="%1" ھۆججىتى يوق. چىقىرىۋەتكىلى بولمايدۇ.
UninstallOpenError="%1" ھۆججىتىنى ئاچقىلى بولمىدى. چىقىرىۋەتكىلى بولمايدۇ
UninstallUnsupportedVer=چىقىرىۋېتىش خاتىرە ھۆججىتى "%1" نىڭ فورماتىنى چىقىرىۋېتىشنىڭ بۇ نەشرى تونۇمايدۇ. چىقىرىۋەتكىلى بولمايدۇ
UninstallUnknownEntry=چىقىرىۋېتىش خاتىرىسىدە نامەلۇم تۈر (%1) ئۇچرىدى
ConfirmUninstall=%1 ۋە ئۇنىڭ بارلىق بۆلەكلىرىنى پۈتۈنلەي چىقىرىۋېتىشنى راستتىنلا خالامسىز؟
UninstallOnlyOnWin64=بۇ ئورنىتىشنى پەقەت 64 بىتلىق Windows سىستېمىسىدىلا چىقىرىۋەتكىلى بولىدۇ.
OnlyAdminCanUninstall=بۇ ئورنىتىشنى پەقەت باشقۇرغۇچى ھوقۇقى بار ئىشلەتكۈچى چىقىرىۋېتەلەيدۇ.
UninstallStatusLabel=%1 كومپيۇتېرىڭىزدىن چىقىرىلىپ بولغۇچە سەل ساقلاڭ.
UninstalledAll=%1 كومپيۇتېرىڭىزدىن مۇۋەپپەقىيەتلىك چىقىرىۋېتىلدى.
UninstalledMost=%1 چىقىرىۋېتىش تامام.%n%nبەزى ئېلېمېنتلار چىقىرىلمىدى. ئۇلارنى قولدا چىقىرىۋەتكىلى بولىدۇ.
UninstalledAndNeedsRestart=%1 نى چىقىرىۋېتىشنى تاماملاش ئۈچۈن كومپيۇتېرىڭىزنى قايتا قوزغىتىش كېرەك.%n%nھازىر قايتا قوزغىتامسىز؟
UninstallDataCorrupted="%1" ھۆججىتى بۇزۇلغان. چىقىرىۋەتكىلى بولمايدۇ

; *** Uninstallation phase messages
ConfirmDeleteSharedFileTitle=ئورتاق ھۆججەت چىقىرىۋېتىلسۇنمۇ؟
ConfirmDeleteSharedFile2=سىستېما كۆرسىتىشىچە، تۆۋەندىكى ئورتاق ھۆججەتنى ھېچقانداق پروگرامما ئىشلەتمەيدۇ. چىقىرىۋېتىش بۇ ئورتاق ھۆججەتنى چىقىرىۋەتسۇنمۇ؟%n%nئەگەر بەزى پروگراممىلار بۇ ھۆججەتنى يەنىلا ئىشلىتىۋاتقان بولسا، ئۇ چىقىرىۋېتىلسە، شۇ پروگراممىلار نورمال ئىشلىمەسلىكى مۇمكىن. ئىشەنچىڭىز بولمىسا «ياق» نى تاللاڭ. ھۆججەتنى سىستېمىڭىزدا قالدۇرۇش ھېچقانداق زىيان ئېلىپ كەلمەيدۇ.
SharedFileNameLabel=ھۆججەت ئىسمى:
SharedFileLocationLabel=ئورنى:
WizardUninstalling=چىقىرىۋېتىش ھالىتى
StatusUninstalling=%1 چىقىرىۋېتىلىۋاتىدۇ...

; *** Shutdown block reasons
ShutdownBlockReasonInstallingApp=%1 ئورنىتىلىۋاتىدۇ.
ShutdownBlockReasonUninstallingApp=%1 چىقىرىۋېتىلىۋاتىدۇ.

; The custom messages below aren't used by Setup itself, but if you make
; use of them in your scripts, you'll want to translate them.

[CustomMessages]

NameAndVersion=%1 نەشرى %2
AdditionalIcons=قوشۇمچە قىسقا يوللار:
CreateDesktopIcon=ئۈستەل يۈزىدە قىسقا يول قۇرۇش
CreateQuickLaunchIcon=Quick Launch قىسقا يولى قۇرۇش
ProgramOnTheWeb=توردىكى %1
UninstallProgram=چىقىرىۋېتىش: %1
LaunchProgram=ئىجرا قىلىش: %1
AssocFileExtension=%1 نى %2 ھۆججەت كېڭەيتمىسى بىلەن باغلاش
AssocingFileExtension=%1 نى %2 ھۆججەت كېڭەيتمىسى بىلەن باغلاۋاتىدۇ...
AutoStartProgramGroupDescription=قوزغىتىش:
AutoStartProgram=%1 نى ئاپتوماتىك قوزغىتىش
AddonHostProgramNotFound=%1 تاللىغان قىسقۇچىڭىزدا تېپىلمىدى.%n%nشۇنداقتىمۇ داۋاملاشتۇرامسىز؟

; The messages of the installer of Lang Switcher that are its own (the sample is English.isl).
LocalCertReady=ئورنىتىش بۇ كومپيۇتېردا گۇۋاھنامە ياساپ، Lang Switcher نى ئۇنىڭ بىلەن ئىمزالايدۇ، شۇنىڭ بىلەن ئۇ باشقۇرغۇچى سۈپىتىدە ئىجرا بولغان پروگراممىلارنىڭ كۆزنەكلىرىدىمۇ ئىشلىيەلەيدۇ.
LocalCertFailed=Lang Switcher ئورنىتىلدى، لېكىن ئۇ باشقۇرغۇچى سۈپىتىدە ئىجرا بولغان پروگراممىلارنىڭ كۆزنەكلىرىدە ئىشلىمەيدۇ: Windows بۇ كومپيۇتېردا ياسالغان گۇۋاھنامىنى قوبۇل قىلمىدى. تەپسىلاتى ئۈچۈن ئورنىتىش خاتىرىسىگە قاراڭ.
RemoveSettingsQuestion=Lang Switcher نىڭ تەڭشەك ھۆججىتىمۇ چىقىرىۋېتىلسۇنمۇ؟
RemoveSettingsKeep=كېيىن قايتا ئورنىتىش ئۈچۈن تەڭشەكلەرنى ساقلاپ قالماقچى بولسىڭىز، «ياق» نى تاللاڭ.
AuthorSlogan=كۆڭۈل بىلەن ياسالغان پروگراممىلار
