; Lang Switcher -- the messages of the installer in Maori (te reo Māori, stage E109, group B).
;
; Inno Setup has no official Maori file, so this file is whole: [LangOptions], all the messages of
; compiler:Default.isl (version 6.5.0 sample) translated, then the stock [CustomMessages] of that
; file and the messages of the installer that are its own (the sample is English.isl). Read as the
; only MessagesFile of the "maori" entry of [Languages] in ..\LangSwitcher.iss; Default.isl may
; stand before it: what is missing here falls back to English.
; No accelerators (&): a file without an official translation has none, so that one window never
; shows two equal letters (decision of the E108 trial, RULES 16). Messages that end without a
; full stop in Default.isl end without one here: Setup adds a Latin full stop itself. The names of
; the buttons inside a message stand in the quotes of the language. The name of the program is
; not translated. Macrons (ā ē ī ō ū) are precomposed (NFC) letters.
; AuthorSlogan (stage E99, decision 167.1): the slogan of the signature of the author at the
; bottom of every page of the wizard -- after "Panda Koder" and a middle dot; light, without
; pathos. The name of the author and the address of the site stay as they are in every language.

[LangOptions]
LanguageName=Te Reo Māori
LanguageID=$0481
LanguageCodePage=0
;DialogFontName=
;DialogFontSize=9
;DialogFontBaseScaleWidth=7
;DialogFontBaseScaleHeight=15
;WelcomeFontName=Segoe UI
;WelcomeFontSize=14

[Messages]

; *** Application titles
SetupAppTitle=Whakaurunga
SetupWindowTitle=Whakaurunga - %1
UninstallAppTitle=Tango whakaurunga
UninstallAppFullTitle=Tango whakaurunga %1

; *** Misc. common
InformationTitle=Pārongo
ConfirmTitle=Whakaū
ErrorTitle=Hapa

; *** SetupLdr messages
SetupLdrStartupMessage=Ka whakauruhia e tēnei a %1. Kei te hiahia haere tonu koe?
LdrCannotCreateTemp=Kāore i taea te hanga kōnae rangitahi. Kua whakakorea te whakaurunga
LdrCannotExecTemp=Kāore i taea te whakahaere i tētahi kōnae i te kōpaki rangitahi. Kua whakakorea te whakaurunga
HelpTextNote=

; *** Startup error messages
LastErrorMessage=%1.%n%nHapa %2: %3
SetupFileMissing=Kua ngaro te kōnae %1 i te kōpaki whakaurunga. Whakatikahia te raruraru, kimihia rānei he kape hou o te hōtaka.
SetupFileCorrupt=Kua kino ngā kōnae whakaurunga. Kimihia he kape hou o te hōtaka.
SetupFileCorruptOrWrongVer=Kua kino ngā kōnae whakaurunga, kāore rānei e hāngai ana ki tēnei putanga o te kaiwhakauru. Whakatikahia te raruraru, kimihia rānei he kape hou o te hōtaka.
InvalidParameter=I tukuna he tawhā hē ki te rārangi whakahau:%n%n%1
SetupAlreadyRunning=Kei te mahi kē te kaiwhakauru.
WindowsVersionNotSupported=Kāore tēnei hōtaka e tautoko i te putanga o Windows kei tō rorohiko.
WindowsServicePackRequired=E hiahia ana tēnei hōtaka ki a %1 Service Pack %2, ki tētahi i muri mai rānei.
NotOnThisPlatform=Kāore tēnei hōtaka e mahi i runga i a %1.
OnlyOnThisPlatform=Me whakahaere tēnei hōtaka i runga i a %1.
OnlyOnTheseArchitectures=Ka taea anake te whakauru i tēnei hōtaka ki ngā putanga o Windows i hangaia mō ēnei hanganga tukatuka:%n%n%1
WinVersionTooLowError=E hiahia ana tēnei hōtaka ki a %1 putanga %2, ki tētahi i muri mai rānei.
WinVersionTooHighError=Kāore e taea te whakauru i tēnei hōtaka ki a %1 putanga %2, ki tētahi i muri mai rānei.
AdminPrivilegesRequired=Me takiuru koe hei kaiwhakahaere i te wā e whakauru ana i tēnei hōtaka.
PowerUserPrivilegesRequired=Me takiuru koe hei kaiwhakahaere, hei mema rānei o te rōpū Power Users i te wā e whakauru ana i tēnei hōtaka.
SetupAppRunningError=Kua kitea e te kaiwhakauru kei te mahi a %1.%n%nKatia ngā tauira katoa o tērā ināianei, ā, pēhia “Pai” hei haere tonu, “Whakakore” rānei hei puta.
UninstallAppRunningError=Kua kitea e te kaitango whakaurunga kei te mahi a %1.%n%nKatia ngā tauira katoa o tērā ināianei, ā, pēhia “Pai” hei haere tonu, “Whakakore” rānei hei puta.

; *** Startup questions
PrivilegesRequiredOverrideTitle=Kōwhiria te aratau whakaurunga
PrivilegesRequiredOverrideInstruction=Kōwhiria te aratau whakaurunga
PrivilegesRequiredOverrideText1=Ka taea te whakauru a %1 mō ngā kaiwhakamahi katoa (me whai mana kaiwhakahaere), mōu anake rānei.
PrivilegesRequiredOverrideText2=Ka taea te whakauru a %1 mōu anake, mō ngā kaiwhakamahi katoa rānei (me whai mana kaiwhakahaere).
PrivilegesRequiredOverrideAllUsers=Whakauru mō ngā kaiwhakamahi katoa
PrivilegesRequiredOverrideAllUsersRecommended=Whakauru mō ngā kaiwhakamahi katoa (e tūtohua ana)
PrivilegesRequiredOverrideCurrentUser=Whakauru mōku anake
PrivilegesRequiredOverrideCurrentUserRecommended=Whakauru mōku anake (e tūtohua ana)

; *** Misc. errors
ErrorCreatingDir=Kāore i taea e te kaiwhakauru te hanga i te kōpaki "%1"
ErrorTooManyFilesInDir=Kāore i taea te hanga kōnae i te kōpaki "%1" nā te maha rawa o ngā kōnae kei roto

; *** Setup common messages
ExitSetupTitle=Puta i te whakaurunga
ExitSetupMessage=Kāore anō kia oti te whakaurunga. Mēnā ka puta koe ināianei, e kore te hōtaka e whakauruhia.%n%nHei tētahi atu wā ka taea e koe te whakahaere anō i te kaiwhakauru kia oti ai te whakaurunga.%n%nKia puta i te whakaurunga?
AboutSetupMenuItem=Mō te kaiwhakauru...
AboutSetupTitle=Mō te kaiwhakauru
AboutSetupMessage=%1 putanga %2%n%3%n%nWhārangi kāinga o %1:%n%4
AboutSetupNote=
TranslatorNote=

; *** Buttons
ButtonBack=< Hoki
ButtonNext=Whai ake >
ButtonInstall=Whakauru
ButtonOK=Pai
ButtonCancel=Whakakore
ButtonYes=Āe
ButtonYesToAll=Āe ki te katoa
ButtonNo=Kāo
ButtonNoToAll=Kāo ki te katoa
ButtonFinish=Whakaoti
ButtonBrowse=Tirotiro...
ButtonWizardBrowse=Tirotiro...
ButtonNewFolder=Hanga kōpaki hou

; *** "Select Language" dialog messages
SelectLanguageTitle=Kōwhiria te reo whakaurunga
SelectLanguageLabel=Kōwhiria te reo hei whakamahi i te wā o te whakaurunga.

; *** Common wizard text
ClickNext=Pēhia “Whai ake” hei haere tonu, “Whakakore” rānei hei puta i te whakaurunga.
BeveledLabel=
BrowseDialogTitle=Tirotiro mō te kōpaki
BrowseDialogLabel=Kōwhiria tētahi kōpaki i te rārangi i raro nei, ā, pēhia “Pai”.
NewFolderName=Kōpaki hou

; *** "Welcome" wizard page
WelcomeLabel1=Nau mai ki te kaiārahi whakaurunga o [name]
WelcomeLabel2=Ka whakauruhia e tēnei a [name/ver] ki tō rorohiko.%n%nE tūtohu ana kia katia ētahi atu taupānga katoa i mua i te haere tonu.

; *** "Password" wizard page
WizardPassword=Kupuhipa
PasswordLabel1=He kupuhipa tō tēnei whakaurunga.
PasswordLabel3=Tāurutia te kupuhipa, ā, pēhia “Whai ake” hei haere tonu. He rerekē te pūmatua me te pūriki i ngā kupuhipa.
PasswordEditLabel=Kupuhipa:
IncorrectPassword=Kāore te kupuhipa i tāurutia e koe i tika. Whakamātauria anō.

; *** "License Agreement" wizard page
WizardLicense=Whakaaetanga raihana
LicenseLabel=Pānuitia ēnei kōrero hira i mua i te haere tonu.
LicenseLabel3=Pānuitia te Whakaaetanga raihana e whai ake nei. Me whakaae koe ki ngā tikanga o tēnei whakaaetanga i mua i te haere tonu o te whakaurunga.
LicenseAccepted=Ka whakaae ahau ki te whakaaetanga
LicenseNotAccepted=Kāore au e whakaae ki te whakaaetanga

; *** "Information" wizard pages
WizardInfoBefore=Pārongo
InfoBeforeLabel=Pānuitia ēnei kōrero hira i mua i te haere tonu.
InfoBeforeClickLabel=Ina rite koe ki te haere tonu, pēhia “Whai ake”.
WizardInfoAfter=Pārongo
InfoAfterLabel=Pānuitia ēnei kōrero hira i mua i te haere tonu.
InfoAfterClickLabel=Ina rite koe ki te haere tonu, pēhia “Whai ake”.

; *** "User Information" wizard page
WizardUserInfo=Pārongo kaiwhakamahi
UserInfoDesc=Tāurutia ō pārongo.
UserInfoName=Ingoa kaiwhakamahi:
UserInfoOrg=Whakahaere:
UserInfoSerial=Tau raupapa:
UserInfoNameRequired=Me tāuru koe i tētahi ingoa.

; *** "Select Destination Location" wizard page
WizardSelectDir=Kōwhiria te wāhi whakaurunga
SelectDirDesc=Hei hea te whakauru i a [name]?
SelectDirLabel3=Ka whakauruhia e te kaiwhakauru a [name] ki te kōpaki e whai ake nei.
SelectDirBrowseLabel=Hei haere tonu, pēhia “Whai ake”. Mēnā e hiahia ana koe ki te kōwhiri i tētahi atu kōpaki, pēhia “Tirotiro”.
DiskSpaceGBLabel=Kia iti rawa ko [gb] GB te wāhi kōpae wātea e hiahiatia ana.
DiskSpaceMBLabel=Kia iti rawa ko [mb] MB te wāhi kōpae wātea e hiahiatia ana.
CannotInstallToNetworkDrive=Kāore e taea e te kaiwhakauru te whakauru ki tētahi kōpae whatunga.
CannotInstallToUNCPath=Kāore e taea e te kaiwhakauru te whakauru ki tētahi ara UNC.
InvalidPath=Me tāuru koe i tētahi ara tino oti me te reta kōpae; hei tauira:%n%nC:\APP%n%nhe ara UNC rānei e penei ana:%n%n\\server\share
InvalidDrive=Kāore te kōpae, te tohatoha UNC rānei i kōwhiria e koe i te noho, kāore rānei e taea te uru atu. Kōwhiria tētahi atu.
DiskSpaceWarningTitle=Kāore e rahi te wāhi kōpae
DiskSpaceWarning=E hiahia ana te kaiwhakauru kia iti rawa ko %1 KB te wāhi wātea hei whakauru, engari e %2 KB anake kei te kōpae kua kōwhiria.%n%nKei te hiahia haere tonu koe ahakoa tērā?
DirNameTooLong=He roa rawa te ingoa o te kōpaki, te ara rānei.
InvalidDirName=Kāore te ingoa o te kōpaki i tika.
BadDirName32=Kāore e taea e ngā ingoa kōpaki te whai i ēnei tohu:%n%n%1
DirExistsTitle=Kei te noho kē te kōpaki
DirExists=Kei te noho kē te kōpaki:%n%n%1%n%nKei te hiahia koe ki te whakauru ki taua kōpaki ahakoa tērā?
DirDoesntExistTitle=Kāore te kōpaki i te noho
DirDoesntExist=Kāore te kōpaki:%n%n%1%n%ni te noho. Kei te hiahia koe kia hangaia te kōpaki?

; *** "Select Components" wizard page
WizardSelectComponents=Kōwhiria ngā wāhanga
SelectComponentsDesc=Ko ēhea wāhanga hei whakauru?
SelectComponentsLabel2=Kōwhiria ngā wāhanga e hiahia ana koe ki te whakauru; whakakorea te kōwhiringa o ngā wāhanga kāore koe e hiahia ana. Pēhia “Whai ake” ina rite koe ki te haere tonu.
FullInstallation=Whakaurunga katoa
; if possible don't translate 'Compact' as 'Minimal' (I mean 'Minimal' in your language)
CompactInstallation=Whakaurunga kuiti
CustomInstallation=Whakaurunga ritenga
NoUninstallWarningTitle=Kei te noho kē ngā wāhanga
NoUninstallWarning=Kua kitea e te kaiwhakauru kua whakauruhia kētia ēnei wāhanga ki tō rorohiko:%n%n%1%n%nKāore ēnei e tangohia mā te whakakore i te kōwhiringa.%n%nKei te hiahia haere tonu koe ahakoa tērā?
ComponentSize1=%1 KB
ComponentSize2=%1 MB
ComponentsDiskSpaceGBLabel=Kia iti rawa ko [gb] GB te wāhi kōpae e hiahiatia ana e te kōwhiringa o nāianei.
ComponentsDiskSpaceMBLabel=Kia iti rawa ko [mb] MB te wāhi kōpae e hiahiatia ana e te kōwhiringa o nāianei.

; *** "Select Additional Tasks" wizard page
WizardSelectTasks=Kōwhiria ngā mahi tāpiri
SelectTasksDesc=Ko ēhea mahi tāpiri hei mahi?
SelectTasksLabel2=Kōwhiria ngā mahi tāpiri e hiahia ana koe kia mahia e te kaiwhakauru i te wā e whakauru ana i a [name], ā, pēhia “Whai ake”.

; *** "Select Start Menu Folder" wizard page
WizardSelectProgramGroup=Kōwhiria te kōpaki Rārangi Tīmata
SelectStartMenuFolderDesc=Hei hea te waiho i ngā ara poto o te hōtaka?
SelectStartMenuFolderLabel3=Ka hangaia e te kaiwhakauru ngā ara poto o te hōtaka ki te kōpaki Rārangi Tīmata e whai ake nei.
SelectStartMenuFolderBrowseLabel=Hei haere tonu, pēhia “Whai ake”. Mēnā e hiahia ana koe ki te kōwhiri i tētahi atu kōpaki, pēhia “Tirotiro”.
MustEnterGroupName=Me tāuru koe i tētahi ingoa kōpaki.
GroupNameTooLong=He roa rawa te ingoa o te kōpaki, te ara rānei.
InvalidGroupName=Kāore te ingoa o te kōpaki i tika.
BadGroupName=Kāore e taea e te ingoa kōpaki te whai i ēnei tohu:%n%n%1
NoProgramGroupCheck2=Kaua e hanga kōpaki Rārangi Tīmata

; *** "Ready to Install" wizard page
WizardReady=Kua rite mō te whakauru
ReadyLabel1=Kua rite te kaiwhakauru ki te tīmata i te whakauru i a [name] ki tō rorohiko.
ReadyLabel2a=Pēhia “Whakauru” hei haere tonu i te whakaurunga, “Hoki” rānei mēnā e hiahia ana koe ki te arotake, ki te whakarerekē rānei i ngā tautuhinga.
ReadyLabel2b=Pēhia “Whakauru” hei haere tonu i te whakaurunga.
ReadyMemoUserInfo=Pārongo kaiwhakamahi:
ReadyMemoDir=Wāhi whakaurunga:
ReadyMemoType=Momo whakaurunga:
ReadyMemoComponents=Ngā wāhanga kua kōwhiria:
ReadyMemoGroup=Kōpaki Rārangi Tīmata:
ReadyMemoTasks=Ngā mahi tāpiri:

; *** TDownloadWizardPage wizard page and DownloadTemporaryFile
DownloadingLabel2=E tikiake ana i ngā kōnae...
ButtonStopDownload=Whakamutu i te tikiake
StopDownload=Kei te tino hiahia koe ki te whakamutu i te tikiake?
ErrorDownloadAborted=Kua whakakorea te tikiake
ErrorDownloadFailed=I hē te tikiake: %1 %2
ErrorDownloadSizeFailed=I hē te tiki i te rahi: %1 %2
ErrorProgress=He hē te kauneke: %1 o %2
ErrorFileSize=He hē te rahi o te kōnae: i tūmanakohia %1, i kitea %2

; *** TExtractionWizardPage wizard page and ExtractArchive
ExtractingLabel=E wewete ana i ngā kōnae...
ButtonStopExtraction=Whakamutu i te wewete
StopExtraction=Kei te tino hiahia koe ki te whakamutu i te wewete?
ErrorExtractionAborted=Kua whakakorea te wewete
ErrorExtractionFailed=I hē te wewete: %1

; *** Archive extraction failure details
ArchiveIncorrectPassword=Kāore te kupuhipa i tika
ArchiveIsCorrupted=Kua kino te pūranga
ArchiveUnsupportedFormat=Kāore te hōputu pūranga e tautokohia

; *** "Preparing to Install" wizard page
WizardPreparing=E whakareri ana mō te whakauru
PreparingDesc=E whakareri ana te kaiwhakauru ki te whakauru i a [name] ki tō rorohiko.
PreviousInstallNotCompleted=Kāore anō kia oti te whakauru/tango o tētahi hōtaka o mua. Me tīmata anō tō rorohiko kia oti ai tērā.%n%nI muri i te tīmata anō o tō rorohiko, whakahaerehia anō te kaiwhakauru kia oti ai te whakaurunga o [name].
CannotContinue=Kāore e taea e te kaiwhakauru te haere tonu. Pēhia “Whakakore” hei puta.
ApplicationsFound=E whakamahi ana ēnei taupānga i ētahi kōnae me whakahou e te kaiwhakauru. E tūtohu ana kia tukuna te kaiwhakauru ki te kati aunoa i ēnei taupānga.
ApplicationsFound2=E whakamahi ana ēnei taupānga i ētahi kōnae me whakahou e te kaiwhakauru. E tūtohu ana kia tukuna te kaiwhakauru ki te kati aunoa i ēnei taupānga. Ina oti te whakaurunga, ka whakamātau te kaiwhakauru ki te tīmata anō i ngā taupānga.
CloseApplications=Kati aunoa i ngā taupānga
DontCloseApplications=Kaua e kati i ngā taupānga
ErrorCloseApplications=Kāore i taea e te kaiwhakauru te kati aunoa i ngā taupānga katoa. E tūtohu ana kia katia e koe ngā taupānga katoa e whakamahi ana i ngā kōnae me whakahou e te kaiwhakauru i mua i te haere tonu.
PrepareToInstallNeedsRestart=Me tīmata anō e te kaiwhakauru tō rorohiko. I muri i te tīmata anō o tō rorohiko, whakahaerehia anō te kaiwhakauru kia oti ai te whakaurunga o [name].%n%nKei te hiahia koe ki te tīmata anō ināianei?

; *** "Installing" wizard page
WizardInstalling=E whakauru ana
InstallingLabel=Taihoa kia whakauruhia e te kaiwhakauru a [name] ki tō rorohiko.

; *** "Setup Completed" wizard page
FinishedHeadingLabel=Kua oti te kaiārahi whakaurunga o [name]
FinishedLabelNoIcons=Kua oti i te kaiwhakauru te whakauru i a [name] ki tō rorohiko.
FinishedLabel=Kua oti i te kaiwhakauru te whakauru i a [name] ki tō rorohiko. Ka taea te whakahaere i te taupānga mā te kōwhiri i ngā ara poto kua whakauruhia.
ClickFinish=Pēhia “Whakaoti” hei puta i te whakaurunga.
FinishedRestartLabel=Kia oti ai te whakaurunga o [name], me tīmata anō e te kaiwhakauru tō rorohiko. Kei te hiahia koe ki te tīmata anō ināianei?
FinishedRestartMessage=Kia oti ai te whakaurunga o [name], me tīmata anō e te kaiwhakauru tō rorohiko.%n%nKei te hiahia koe ki te tīmata anō ināianei?
ShowReadmeCheck=Āe, e hiahia ana ahau ki te tiro i te kōnae README
YesRadio=Āe, tīmata anō i te rorohiko ināianei
NoRadio=Kāo, ka tīmata anō ahau i te rorohiko ā muri ake nei
; used for example as 'Run MyProg.exe'
RunEntryExec=Whakahaere %1
; used for example as 'View Readme.txt'
RunEntryShellExec=Tiro %1

; *** "Setup Needs the Next Disk" stuff
ChangeDiskTitle=E hiahia ana te kaiwhakauru ki te kōpae e whai ake nei
SelectDiskLabel2=Whakauruhia te Kōpae %1, ā, pēhia “Pai”.%n%nMēnā kei tētahi atu kōpaki ngā kōnae o tēnei kōpae i tērā e whakaatuhia ana i raro nei, tāurutia te ara tika, pēhia rānei “Tirotiro”.
PathLabel=Ara:
FileNotInDir2=Kāore i kitea te kōnae "%1" i "%2". Whakauruhia te kōpae tika, kōwhiria rānei tētahi atu kōpaki.
SelectDirectoryLabel=Tohua te wāhi o te kōpae e whai ake nei.

; *** Installation phase messages
SetupAborted=Kāore anō kia oti te whakaurunga.%n%nWhakatikahia te raruraru, ā, whakahaerehia anō te kaiwhakauru.
AbortRetryIgnoreSelectAction=Kōwhiria he mahi
AbortRetryIgnoreRetry=Whakamātau anō
AbortRetryIgnoreIgnore=Warewaretia te hapa, ā, haere tonu
AbortRetryIgnoreCancel=Whakakore i te whakaurunga
RetryCancelSelectAction=Kōwhiria he mahi
RetryCancelRetry=Whakamātau anō
RetryCancelCancel=Whakakore

; *** Installation status messages
StatusClosingApplications=E kati ana i ngā taupānga...
StatusCreateDirs=E hanga ana i ngā kōpaki...
StatusExtractFiles=E wewete ana i ngā kōnae...
StatusDownloadFiles=E tikiake ana i ngā kōnae...
StatusCreateIcons=E hanga ana i ngā ara poto...
StatusCreateIniEntries=E hanga ana i ngā urunga INI...
StatusCreateRegistryEntries=E hanga ana i ngā urunga rēhita...
StatusRegisterFiles=E rēhita ana i ngā kōnae...
StatusSavingUninstall=E tiaki ana i ngā kōrero tango whakaurunga...
StatusRunProgram=E whakaoti ana i te whakaurunga...
StatusRestartingApplications=E tīmata anō ana i ngā taupānga...
StatusRollback=E whakahoki ana i ngā panoni...

; *** Misc. errors
ErrorInternal2=Hapa o roto: %1
ErrorFunctionFailedNoCode=I hē a %1
ErrorFunctionFailed=I hē a %1; waehere %2
ErrorFunctionFailedWithMessage=I hē a %1; waehere %2.%n%3
ErrorExecutingProgram=Kāore i taea te whakahaere i te kōnae:%n%1

; *** Registry errors
ErrorRegOpenKey=Hapa i te whakatuwhera i te kī rēhita:%n%1\%2
ErrorRegCreateKey=Hapa i te hanga i te kī rēhita:%n%1\%2
ErrorRegWriteKey=Hapa i te tuhi ki te kī rēhita:%n%1\%2

; *** INI errors
ErrorIniEntry=Hapa i te hanga i tētahi urunga INI i te kōnae "%1".

; *** File copying errors
FileAbortRetryIgnoreSkipNotRecommended=Peka i tēnei kōnae (kāore e tūtohua)
FileAbortRetryIgnoreIgnoreNotRecommended=Warewaretia te hapa, ā, haere tonu (kāore e tūtohua)
SourceIsCorrupted=Kua kino te kōnae pūtake
SourceDoesntExist=Kāore te kōnae pūtake "%1" i te noho
SourceVerificationFailed=I hē te manatoko i te kōnae pūtake: %1
VerificationSignatureDoesntExist=Kāore te kōnae waitohu "%1" i te noho
VerificationSignatureInvalid=Kāore te kōnae waitohu "%1" i tika
VerificationKeyNotFound=E whakamahi ana te kōnae waitohu "%1" i tētahi kī kāore e mōhiotia
VerificationFileNameIncorrect=He hē te ingoa o te kōnae
VerificationFileTagIncorrect=He hē te tohu o te kōnae
VerificationFileSizeIncorrect=He hē te rahi o te kōnae
VerificationFileHashIncorrect=He hē te hash o te kōnae
ExistingFileReadOnly2=Kāore i taea te whakakapi i te kōnae onāianei nā te mea kua tohua hei pānui anake.
ExistingFileReadOnlyRetry=Tangohia te āhuatanga pānui anake, ā, whakamātau anō
ExistingFileReadOnlyKeepExisting=Pupuri i te kōnae onāianei
ErrorReadingExistingDest=I puta he hapa i te wā e pānui ana i te kōnae onāianei:
FileExistsSelectAction=Kōwhiria he mahi
FileExists2=Kei te noho kē te kōnae.
FileExistsOverwriteExisting=Tuhia ki runga i te kōnae onāianei
FileExistsKeepExisting=Pupuri i te kōnae onāianei
FileExistsOverwriteOrKeepAll=Pēnei hoki mō ngā tauwhatinga e whai ake nei
ExistingFileNewerSelectAction=Kōwhiria he mahi
ExistingFileNewer2=He hou ake te kōnae onāianei i tērā e whakamātau ana te kaiwhakauru ki te whakauru.
ExistingFileNewerOverwriteExisting=Tuhia ki runga i te kōnae onāianei
ExistingFileNewerKeepExisting=Pupuri i te kōnae onāianei (e tūtohua ana)
ExistingFileNewerOverwriteOrKeepAll=Pēnei hoki mō ngā tauwhatinga e whai ake nei
ErrorChangingAttr=I puta he hapa i te wā e whakarerekē ana i ngā āhuatanga o te kōnae onāianei:
ErrorCreatingTemp=I puta he hapa i te wā e hanga ana i tētahi kōnae i te kōpaki whāinga:
ErrorReadingSource=I puta he hapa i te wā e pānui ana i te kōnae pūtake:
ErrorCopying=I puta he hapa i te wā e tāruatia ana tētahi kōnae:
ErrorDownloading=I puta he hapa i te wā e tikiake ana i tētahi kōnae:
ErrorExtracting=I puta he hapa i te wā e wewete ana i tētahi pūranga:
ErrorReplacingExistingFile=I puta he hapa i te wā e whakakapi ana i te kōnae onāianei:
ErrorRestartReplace=I hē a RestartReplace:
ErrorRenamingTemp=I puta he hapa i te wā e whakaingoa anō ana i tētahi kōnae i te kōpaki whāinga:
ErrorRegisterServer=Kāore i taea te rēhita i te DLL/OCX: %1
ErrorRegSvr32Failed=I hē a RegSvr32 me te waehere puta %1
ErrorRegisterTypeLib=Kāore i taea te rēhita i te whare pukapuka momo: %1

; *** Uninstall display name markings
; used for example as 'My Program (32-bit)'
UninstallDisplayNameMark=%1 (%2)
; used for example as 'My Program (32-bit, All users)'
UninstallDisplayNameMarks=%1 (%2, %3)
UninstallDisplayNameMark32Bit=32-bit
UninstallDisplayNameMark64Bit=64-bit
UninstallDisplayNameMarkAllUsers=Ngā kaiwhakamahi katoa
UninstallDisplayNameMarkCurrentUser=Te kaiwhakamahi o nāianei

; *** Post-installation errors
ErrorOpeningReadme=I puta he hapa i te wā e whakatuwhera ana i te kōnae README.
ErrorRestartingComputer=Kāore i taea e te kaiwhakauru te tīmata anō i te rorohiko. Mahia ā-ringa.

; *** Uninstaller messages
UninstallNotFound=Kāore te kōnae "%1" i te noho. Kāore e taea te tango whakaurunga.
UninstallOpenError=Kāore i taea te whakatuwhera i te kōnae "%1". Kāore e taea te tango whakaurunga
UninstallUnsupportedVer=Kāore tēnei putanga o te kaitango whakaurunga e mōhio ki te hōputu o te kōnae pūkete tango "%1". Kāore e taea te tango whakaurunga
UninstallUnknownEntry=I kitea tētahi urunga kāore e mōhiotia (%1) i te pūkete tango
ConfirmUninstall=Kei te tino hiahia koe ki te tango katoa i a %1 me ōna wāhanga katoa?
UninstallOnlyOnWin64=Ka taea anake te tango i tēnei whakaurunga i runga i a Windows 64-bit.
OnlyAdminCanUninstall=Ka taea anake te tango i tēnei whakaurunga e tētahi kaiwhakamahi whai mana kaiwhakahaere.
UninstallStatusLabel=Taihoa kia tangohia a %1 i tō rorohiko.
UninstalledAll=Kua tangohia a %1 i tō rorohiko.
UninstalledMost=Kua oti te tango i a %1.%n%nKāore i taea te tango i ētahi wāhanga. Ka taea te tango ā-ringa.
UninstalledAndNeedsRestart=Kia oti ai te tango i a %1, me tīmata anō tō rorohiko.%n%nKei te hiahia koe ki te tīmata anō ināianei?
UninstallDataCorrupted=Kua kino te kōnae "%1". Kāore e taea te tango whakaurunga

; *** Uninstallation phase messages
ConfirmDeleteSharedFileTitle=Tangohia te kōnae tuari?
ConfirmDeleteSharedFile2=E kī ana te pūnaha kāore he hōtaka e whakamahi ana i te kōnae tuari e whai ake nei. Kei te hiahia koe kia tangohia e te kaitango whakaurunga tēnei kōnae tuari?%n%nMēnā e whakamahi tonu ana tētahi hōtaka i tēnei kōnae, ā, ka tangohia, tērā pea ka kore e tika te mahi a aua hōtaka. Mēnā kāore koe i te tino mōhio, kōwhiria “Kāo”. Kāore he kino ki te waiho i te kōnae ki tō pūnaha.
SharedFileNameLabel=Ingoa kōnae:
SharedFileLocationLabel=Wāhi:
WizardUninstalling=Tūnga tango whakaurunga
StatusUninstalling=E tango ana i a %1...

; *** Shutdown block reasons
ShutdownBlockReasonInstallingApp=E whakauru ana i a %1.
ShutdownBlockReasonUninstallingApp=E tango ana i a %1.

; The custom messages below aren't used by Setup itself, but if you make
; use of them in your scripts, you'll want to translate them.

[CustomMessages]

NameAndVersion=%1 putanga %2
AdditionalIcons=Ngā ara poto tāpiri:
CreateDesktopIcon=Hanga ara poto mō te papamahi
CreateQuickLaunchIcon=Hanga ara poto Quick Launch
ProgramOnTheWeb=%1 i te Tukutuku
UninstallProgram=Tango whakaurunga %1
LaunchProgram=Whakahaere %1
AssocFileExtension=Hono i a %1 ki te toronga kōnae %2
AssocingFileExtension=E hono ana i a %1 ki te toronga kōnae %2...
AutoStartProgramGroupDescription=Tīmatanga:
AutoStartProgram=Tīmata aunoa i a %1
AddonHostProgramNotFound=Kāore i kitea a %1 i te kōpaki i kōwhiria e koe.%n%nKei te hiahia koe ki te haere tonu ahakoa tērā?

; The messages that are Lang Switcher's own (the sample is English.isl).
LocalCertReady=Ka hangaia e te kaiwhakauru tētahi tiwhikete ki tēnei rorohiko, ā, ka waitohua a Lang Switcher mā taua tiwhikete, kia mahi ai i ngā matapihi o ngā hōtaka e whakahaerehia ana hei kaiwhakahaere.
LocalCertFailed=Kua whakauruhia a Lang Switcher, engari e kore e mahi i ngā matapihi o ngā hōtaka e whakahaerehia ana hei kaiwhakahaere: kāore a Windows i whakaae ki te tiwhikete i hangaia ki tēnei rorohiko. Tirohia te pūkete whakaurunga mō ngā taipitopito.
RemoveSettingsQuestion=Tangohia hoki te kōnae tautuhinga o Lang Switcher?
RemoveSettingsKeep=Kōwhiria “Kāo” hei pupuri i ō tautuhinga mō te whakauru anō ā muri ake nei.
AuthorSlogan=he hōtaka i hangaia mā te ngākau
