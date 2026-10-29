; Lang Switcher -- the messages of the installer in Albanian (stage E109, group B).
;
; Inno Setup has no official Albanian translation (inno_official = false in res\lang\languages.toml),
; so this file carries the whole of it: [LangOptions], every message of the compiler:Default.isl
; (6.5.0+), and the messages of the installer that are Lang Switcher's own. It is read after
; compiler:Default.isl as the second MessagesFile of the "albanian" entry of [Languages] in
; ..\LangSwitcher.iss, and so replaces the English text name for name.
; English.isl is the sample the own messages translate; the glossary of the language is
; l10n\glossary\sq.tsv of the control contour. The name of the program is not translated.
; No accelerator "&" is set (as in Tamil.isl): the window must not show two equal letters.
; The names of the buttons inside a message stand in the guillemets of the language.
; As in Default.isl: a message that has no full stop there has none here (Setup adds its own).

[LangOptions]
LanguageName=Shqip
LanguageID=$041C
LanguageCodePage=0

[Messages]

; *** Application titles
SetupAppTitle=Instalimi
SetupWindowTitle=Instalimi - %1
UninstallAppTitle=Çinstalimi
UninstallAppFullTitle=Çinstalimi i %1

; *** Misc. common
InformationTitle=Informacion
ConfirmTitle=Konfirmim
ErrorTitle=Gabim

; *** SetupLdr messages
SetupLdrStartupMessage=Do të instalohet %1. Dëshironi të vazhdoni?
LdrCannotCreateTemp=Nuk u krijua dot një skedar i përkohshëm. Instalimi u ndërpre
LdrCannotExecTemp=Nuk u ekzekutua dot skedari në dosjen e përkohshme. Instalimi u ndërpre
HelpTextNote=

; *** Startup error messages
LastErrorMessage=%1.%n%nGabimi %2: %3
SetupFileMissing=Skedari %1 mungon në dosjen e instalimit. Ju lutemi korrigjoni problemin ose merrni një kopje të re të programit.
SetupFileCorrupt=Skedarët e instalimit janë dëmtuar. Ju lutemi merrni një kopje të re të programit.
SetupFileCorruptOrWrongVer=Skedarët e instalimit janë dëmtuar ose nuk përputhen me këtë version të instalimit. Ju lutemi korrigjoni problemin ose merrni një kopje të re të programit.
InvalidParameter=Në rreshtin e komandës u dha një parametër i pavlefshëm:%n%n%1
SetupAlreadyRunning=Instalimi po ekzekutohet tashmë.
WindowsVersionNotSupported=Ky program nuk e mbështet versionin e Windows që përdor kompjuteri juaj.
WindowsServicePackRequired=Ky program kërkon %1 Service Pack %2 ose më të ri.
NotOnThisPlatform=Ky program nuk do të funksionojë në %1.
OnlyOnThisPlatform=Ky program duhet ekzekutuar në %1.
OnlyOnTheseArchitectures=Ky program mund të instalohet vetëm në versione të Windows të ndërtuara për këto arkitektura procesori:%n%n%1
WinVersionTooLowError=Ky program kërkon %1 versionin %2 ose më të ri.
WinVersionTooHighError=Ky program nuk mund të instalohet në %1 versionin %2 ose më të ri.
AdminPrivilegesRequired=Gjatë instalimit të këtij programi duhet të jeni identifikuar si administrator.
PowerUserPrivilegesRequired=Gjatë instalimit të këtij programi duhet të jeni identifikuar si administrator ose si anëtar i grupit Power Users.
SetupAppRunningError=Instalimi zbuloi se %1 po ekzekutohet tani.%n%nMbyllini tani të gjitha kopjet e tij, pastaj shtypni «OK» për të vazhduar ose «Anulo» për të dalë.
UninstallAppRunningError=Çinstalimi zbuloi se %1 po ekzekutohet tani.%n%nMbyllini tani të gjitha kopjet e tij, pastaj shtypni «OK» për të vazhduar ose «Anulo» për të dalë.

; *** Startup questions
PrivilegesRequiredOverrideTitle=Zgjidhni mënyrën e instalimit
PrivilegesRequiredOverrideInstruction=Zgjidhni mënyrën e instalimit
PrivilegesRequiredOverrideText1=%1 mund të instalohet për të gjithë përdoruesit (kërkon të drejta administratori) ose vetëm për ju.
PrivilegesRequiredOverrideText2=%1 mund të instalohet vetëm për ju ose për të gjithë përdoruesit (kërkon të drejta administratori).
PrivilegesRequiredOverrideAllUsers=Instalo për të gjithë përdoruesit
PrivilegesRequiredOverrideAllUsersRecommended=Instalo për të gjithë përdoruesit (e rekomanduar)
PrivilegesRequiredOverrideCurrentUser=Instalo vetëm për mua
PrivilegesRequiredOverrideCurrentUserRecommended=Instalo vetëm për mua (e rekomanduar)

; *** Misc. errors
ErrorCreatingDir=Instalimi nuk e krijoi dot dosjen "%1"
ErrorTooManyFilesInDir=Nuk u krijua dot një skedar në dosjen "%1", sepse ajo përmban shumë skedarë

; *** Setup common messages
ExitSetupTitle=Dil nga instalimi
ExitSetupMessage=Instalimi nuk ka përfunduar. Nëse dilni tani, programi nuk do të instalohet.%n%nMund ta ekzekutoni instalimin sërish në një kohë tjetër për ta përfunduar.%n%nTë dilet nga instalimi?
AboutSetupMenuItem=Rreth instalimit...
AboutSetupTitle=Rreth instalimit
AboutSetupMessage=%1 versioni %2%n%3%n%nFaqja kryesore e %1:%n%4
AboutSetupNote=
TranslatorNote=

; *** Buttons
ButtonBack=< Prapa
ButtonNext=Tjetra >
ButtonInstall=Instalo
ButtonOK=OK
ButtonCancel=Anulo
ButtonYes=Po
ButtonYesToAll=Po për të gjitha
ButtonNo=Jo
ButtonNoToAll=Jo për të gjitha
ButtonFinish=Përfundo
ButtonBrowse=Shfleto...
ButtonWizardBrowse=Shfleto...
ButtonNewFolder=Krijo dosje të re

; *** "Select Language" dialog messages
SelectLanguageTitle=Zgjidhni gjuhën e instalimit
SelectLanguageLabel=Zgjidhni gjuhën që do të përdoret gjatë instalimit.

; *** Common wizard text
ClickNext=Shtypni «Tjetra» për të vazhduar ose «Anulo» për të dalë nga instalimi.
BeveledLabel=
BrowseDialogTitle=Shfletoni për dosje
BrowseDialogLabel=Zgjidhni një dosje në listën më poshtë, pastaj shtypni «OK».
NewFolderName=Dosje e re

; *** "Welcome" wizard page
WelcomeLabel1=Mirë se vini te magjistari i instalimit të [name]
WelcomeLabel2=Do të instalohet [name/ver] në kompjuterin tuaj.%n%nRekomandohet të mbyllni të gjitha aplikacionet e tjera para se të vazhdoni.

; *** "Password" wizard page
WizardPassword=Fjalëkalimi
PasswordLabel1=Ky instalim është i mbrojtur me fjalëkalim.
PasswordLabel3=Shkruani fjalëkalimin, pastaj shtypni «Tjetra» për të vazhduar. Fjalëkalimet dallojnë shkronjat e mëdha nga të vogla.
PasswordEditLabel=Fjalëkalimi:
IncorrectPassword=Fjalëkalimi që shkruat nuk është i saktë. Provoni sërish.

; *** "License Agreement" wizard page
WizardLicense=Marrëveshja e licencës
LicenseLabel=Para se të vazhdoni, lexoni informacionin e rëndësishëm më poshtë.
LicenseLabel3=Lexoni marrëveshjen e licencës më poshtë. Para se të vazhdoni me instalimin, duhet të pranoni kushtet e kësaj marrëveshjeje.
LicenseAccepted=E pranoj marrëveshjen
LicenseNotAccepted=Nuk e pranoj marrëveshjen

; *** "Information" wizard pages
WizardInfoBefore=Informacion
InfoBeforeLabel=Para se të vazhdoni, lexoni informacionin e rëndësishëm më poshtë.
InfoBeforeClickLabel=Kur të jeni gati të vazhdoni me instalimin, shtypni «Tjetra».
WizardInfoAfter=Informacion
InfoAfterLabel=Para se të vazhdoni, lexoni informacionin e rëndësishëm më poshtë.
InfoAfterClickLabel=Kur të jeni gati të vazhdoni me instalimin, shtypni «Tjetra».

; *** "User Information" wizard page
WizardUserInfo=Të dhënat e përdoruesit
UserInfoDesc=Shkruani të dhënat tuaja.
UserInfoName=Emri i përdoruesit:
UserInfoOrg=Organizata:
UserInfoSerial=Numri serial:
UserInfoNameRequired=Duhet të shkruani një emër.

; *** "Select Destination Location" wizard page
WizardSelectDir=Zgjidhni vendndodhjen e instalimit
SelectDirDesc=Ku të instalohet [name]?
SelectDirLabel3=Instalimi do ta instalojë [name] në dosjen e mëposhtme.
SelectDirBrowseLabel=Për të vazhduar, shtypni «Tjetra». Nëse dëshironi të zgjidhni një dosje tjetër, shtypni «Shfleto».
DiskSpaceGBLabel=Nevojiten të paktën [gb] GB hapësirë e lirë në disk.
DiskSpaceMBLabel=Nevojiten të paktën [mb] MB hapësirë e lirë në disk.
CannotInstallToNetworkDrive=Instalimi nuk mund të bëhet në një disk rrjeti.
CannotInstallToUNCPath=Instalimi nuk mund të bëhet në një shteg UNC.
InvalidPath=Duhet të shkruani një shteg të plotë me shkronjën e diskut; për shembull:%n%nC:\APP%n%nose një shteg UNC me formën:%n%n\\server\share
InvalidDrive=Disku ose pjesa e përbashkët UNC që zgjodhët nuk ekziston ose nuk është e arritshme. Zgjidhni një tjetër.
DiskSpaceWarningTitle=Hapësirë e pamjaftueshme në disk
DiskSpaceWarning=Instalimi kërkon të paktën %1 KB hapësirë të lirë, por disku i zgjedhur ka vetëm %2 KB.%n%nDëshironi të vazhdoni gjithsesi?
DirNameTooLong=Emri ose shtegu i dosjes është shumë i gjatë.
InvalidDirName=Emri i dosjes nuk është i vlefshëm.
BadDirName32=Emrat e dosjeve nuk mund të përmbajnë asnjë nga shenjat e mëposhtme:%n%n%1
DirExistsTitle=Dosja ekziston
DirExists=Dosja:%n%n%1%n%nekziston tashmë. Dëshironi të instaloni në atë dosje gjithsesi?
DirDoesntExistTitle=Dosja nuk ekziston
DirDoesntExist=Dosja:%n%n%1%n%nnuk ekziston. Dëshironi që dosja të krijohet?

; *** "Select Components" wizard page
WizardSelectComponents=Zgjidhni përbërësit
SelectComponentsDesc=Cilët përbërës duhet të instalohen?
SelectComponentsLabel2=Zgjidhni përbërësit që dëshironi të instaloni; hiqni shenjën nga përbërësit që nuk dëshironi. Kur të jeni gati, shtypni «Tjetra».
FullInstallation=Instalim i plotë
; if possible don't translate 'Compact' as 'Minimal' (I mean 'Minimal' in your language)
CompactInstallation=Instalim kompakt
CustomInstallation=Instalim i personalizuar
NoUninstallWarningTitle=Përbërësit ekzistojnë
NoUninstallWarning=Instalimi zbuloi se përbërësit e mëposhtëm janë instaluar tashmë në kompjuterin tuaj:%n%n%1%n%nHeqja e shenjës nga këta përbërës nuk do t'i çinstalojë.%n%nDëshironi të vazhdoni gjithsesi?
ComponentSize1=%1 KB
ComponentSize2=%1 MB
ComponentsDiskSpaceGBLabel=Zgjedhja aktuale kërkon të paktën [gb] GB hapësirë në disk.
ComponentsDiskSpaceMBLabel=Zgjedhja aktuale kërkon të paktën [mb] MB hapësirë në disk.

; *** "Select Additional Tasks" wizard page
WizardSelectTasks=Zgjidhni detyrat shtesë
SelectTasksDesc=Cilat detyra shtesë duhet të kryhen?
SelectTasksLabel2=Zgjidhni detyrat shtesë që dëshironi të kryejë instalimi gjatë instalimit të [name], pastaj shtypni «Tjetra».

; *** "Select Start Menu Folder" wizard page
WizardSelectProgramGroup=Zgjidhni dosjen e menusë «Start»
SelectStartMenuFolderDesc=Ku t'i vendosë instalimi shkurtoret e programit?
SelectStartMenuFolderLabel3=Instalimi do t'i krijojë shkurtoret e programit në dosjen e mëposhtme të menusë «Start».
SelectStartMenuFolderBrowseLabel=Për të vazhduar, shtypni «Tjetra». Nëse dëshironi të zgjidhni një dosje tjetër, shtypni «Shfleto».
MustEnterGroupName=Duhet të shkruani një emër dosjeje.
GroupNameTooLong=Emri ose shtegu i dosjes është shumë i gjatë.
InvalidGroupName=Emri i dosjes nuk është i vlefshëm.
BadGroupName=Emri i dosjes nuk mund të përmbajë asnjë nga shenjat e mëposhtme:%n%n%1
NoProgramGroupCheck2=Mos krijo dosje në menunë «Start»

; *** "Ready to Install" wizard page
WizardReady=Gati për instalim
ReadyLabel1=Instalimi është gati të fillojë instalimin e [name] në kompjuterin tuaj.
ReadyLabel2a=Shtypni «Instalo» për të vazhduar me instalimin ose «Prapa» nëse dëshironi të shqyrtoni ose ndryshoni cilësimet.
ReadyLabel2b=Shtypni «Instalo» për të vazhduar me instalimin.
ReadyMemoUserInfo=Të dhënat e përdoruesit:
ReadyMemoDir=Vendndodhja e instalimit:
ReadyMemoType=Lloji i instalimit:
ReadyMemoComponents=Përbërësit e zgjedhur:
ReadyMemoGroup=Dosja në menunë «Start»:
ReadyMemoTasks=Detyrat shtesë:

; *** TDownloadWizardPage wizard page and DownloadTemporaryFile
DownloadingLabel2=Po shkarkohen skedarët...
ButtonStopDownload=Ndalo shkarkimin
StopDownload=Dëshironi vërtet ta ndaloni shkarkimin?
ErrorDownloadAborted=Shkarkimi u ndërpre
ErrorDownloadFailed=Shkarkimi dështoi: %1 %2
ErrorDownloadSizeFailed=Marrja e madhësisë dështoi: %1 %2
ErrorProgress=Progres i pavlefshëm: %1 nga %2
ErrorFileSize=Madhësi e pavlefshme e skedarit: pritej %1, u gjet %2

; *** TExtractionWizardPage wizard page and ExtractArchive
ExtractingLabel=Po nxirren skedarët...
ButtonStopExtraction=Ndalo nxjerrjen
StopExtraction=Dëshironi vërtet ta ndaloni nxjerrjen?
ErrorExtractionAborted=Nxjerrja u ndërpre
ErrorExtractionFailed=Nxjerrja dështoi: %1

; *** Archive extraction failure details
ArchiveIncorrectPassword=Fjalëkalimi është i pasaktë
ArchiveIsCorrupted=Arkivi është i dëmtuar
ArchiveUnsupportedFormat=Formati i arkivit nuk mbështetet

; *** "Preparing to Install" wizard page
WizardPreparing=Përgatitja për instalim
PreparingDesc=Instalimi po përgatitet të instalojë [name] në kompjuterin tuaj.
PreviousInstallNotCompleted=Instalimi ose heqja e një programi të mëparshëm nuk përfundoi. Duhet të rinisni kompjuterin për ta përfunduar.%n%nPas rinisjes së kompjuterit, ekzekutoni instalimin sërish për të përfunduar instalimin e [name].
CannotContinue=Instalimi nuk mund të vazhdojë. Shtypni «Anulo» për të dalë.
ApplicationsFound=Aplikacionet e mëposhtme po përdorin skedarë që duhen përditësuar nga instalimi. Rekomandohet t'i lejoni instalimit t'i mbyllë automatikisht këto aplikacione.
ApplicationsFound2=Aplikacionet e mëposhtme po përdorin skedarë që duhen përditësuar nga instalimi. Rekomandohet t'i lejoni instalimit t'i mbyllë automatikisht këto aplikacione. Pasi të përfundojë instalimi, ai do të provojë t'i nisë sërish aplikacionet.
CloseApplications=Mbyll aplikacionet automatikisht
DontCloseApplications=Mos i mbyll aplikacionet
ErrorCloseApplications=Instalimi nuk i mbylli dot automatikisht të gjitha aplikacionet. Rekomandohet t'i mbyllni vetë të gjitha aplikacionet që përdorin skedarë që duhen përditësuar nga instalimi, para se të vazhdoni.
PrepareToInstallNeedsRestart=Instalimi duhet ta rinisë kompjuterin. Pas rinisjes, ekzekutoni instalimin sërish për të përfunduar instalimin e [name].%n%nDëshironi ta rinisni tani?

; *** "Installing" wizard page
WizardInstalling=Po instalohet
InstallingLabel=Prisni ndërsa instalimi po instalon [name] në kompjuterin tuaj.

; *** "Setup Completed" wizard page
FinishedHeadingLabel=Po përfundon magjistari i instalimit të [name]
FinishedLabelNoIcons=Instalimi përfundoi instalimin e [name] në kompjuterin tuaj.
FinishedLabel=Instalimi përfundoi instalimin e [name] në kompjuterin tuaj. Aplikacionin mund ta nisni duke zgjedhur shkurtoret e instaluara.
ClickFinish=Shtypni «Përfundo» për të dalë nga instalimi.
FinishedRestartLabel=Për të përfunduar instalimin e [name], instalimi duhet ta rinisë kompjuterin. Dëshironi ta rinisni tani?
FinishedRestartMessage=Për të përfunduar instalimin e [name], instalimi duhet ta rinisë kompjuterin.%n%nDëshironi ta rinisni tani?
ShowReadmeCheck=Po, dua të shoh skedarin README
YesRadio=Po, rinise kompjuterin tani
NoRadio=Jo, do ta rinis kompjuterin më vonë
; used for example as 'Run MyProg.exe'
RunEntryExec=Ekzekuto %1
; used for example as 'View Readme.txt'
RunEntryShellExec=Shiko %1

; *** "Setup Needs the Next Disk" stuff
ChangeDiskTitle=Instalimi kërkon diskun tjetër
SelectDiskLabel2=Vendosni diskun %1 dhe shtypni «OK».%n%nNëse skedarët e këtij disku gjenden në një dosje tjetër nga ajo e shfaqur më poshtë, shkruani shtegun e saktë ose shtypni «Shfleto».
PathLabel=Shtegu:
FileNotInDir2=Skedari "%1" nuk u gjet në "%2". Vendosni diskun e duhur ose zgjidhni një dosje tjetër.
SelectDirectoryLabel=Tregoni vendndodhjen e diskut tjetër.

; *** Installation phase messages
SetupAborted=Instalimi nuk përfundoi.%n%nKorrigjoni problemin dhe ekzekutoni instalimin sërish.
AbortRetryIgnoreSelectAction=Zgjidhni veprimin
AbortRetryIgnoreRetry=Provo sërish
AbortRetryIgnoreIgnore=Shpërfille gabimin dhe vazhdo
AbortRetryIgnoreCancel=Anulo instalimin
RetryCancelSelectAction=Zgjidhni veprimin
RetryCancelRetry=Provo sërish
RetryCancelCancel=Anulo

; *** Installation status messages
StatusClosingApplications=Po mbyllen aplikacionet...
StatusCreateDirs=Po krijohen dosjet...
StatusExtractFiles=Po nxirren skedarët...
StatusDownloadFiles=Po shkarkohen skedarët...
StatusCreateIcons=Po krijohen shkurtoret...
StatusCreateIniEntries=Po krijohen shënimet INI...
StatusCreateRegistryEntries=Po krijohen shënimet në regjistër...
StatusRegisterFiles=Po regjistrohen skedarët...
StatusSavingUninstall=Po ruhen të dhënat e çinstalimit...
StatusRunProgram=Po përfundon instalimi...
StatusRestartingApplications=Po nisen sërish aplikacionet...
StatusRollback=Po zhbëhen ndryshimet...

; *** Misc. errors
ErrorInternal2=Gabim i brendshëm: %1
ErrorFunctionFailedNoCode=%1 dështoi
ErrorFunctionFailed=%1 dështoi; kodi %2
ErrorFunctionFailedWithMessage=%1 dështoi; kodi %2.%n%3
ErrorExecutingProgram=Skedari nuk u ekzekutua dot:%n%1

; *** Registry errors
ErrorRegOpenKey=Gabim gjatë hapjes së çelësit të regjistrit:%n%1\%2
ErrorRegCreateKey=Gabim gjatë krijimit të çelësit të regjistrit:%n%1\%2
ErrorRegWriteKey=Gabim gjatë shkrimit në çelësin e regjistrit:%n%1\%2

; *** INI errors
ErrorIniEntry=Gabim gjatë krijimit të shënimit INI në skedarin "%1".

; *** File copying errors
FileAbortRetryIgnoreSkipNotRecommended=Kapërce këtë skedar (nuk rekomandohet)
FileAbortRetryIgnoreIgnoreNotRecommended=Shpërfille gabimin dhe vazhdo (nuk rekomandohet)
SourceIsCorrupted=Skedari burim është i dëmtuar
SourceDoesntExist=Skedari burim "%1" nuk ekziston
SourceVerificationFailed=Verifikimi i skedarit burim dështoi: %1
VerificationSignatureDoesntExist=Skedari i nënshkrimit "%1" nuk ekziston
VerificationSignatureInvalid=Skedari i nënshkrimit "%1" është i pavlefshëm
VerificationKeyNotFound=Skedari i nënshkrimit "%1" përdor një çelës të panjohur
VerificationFileNameIncorrect=Emri i skedarit është i pasaktë
VerificationFileTagIncorrect=Etiketa e skedarit është e pasaktë
VerificationFileSizeIncorrect=Madhësia e skedarit është e pasaktë
VerificationFileHashIncorrect=Hash-i i skedarit është i pasaktë
ExistingFileReadOnly2=Skedari ekzistues nuk u zëvendësua dot, sepse është shënuar vetëm për lexim.
ExistingFileReadOnlyRetry=Hiq atributin vetëm për lexim dhe provo sërish
ExistingFileReadOnlyKeepExisting=Mbaj skedarin ekzistues
ErrorReadingExistingDest=Ndodhi një gabim gjatë leximit të skedarit ekzistues:
FileExistsSelectAction=Zgjidhni veprimin
FileExists2=Skedari ekziston tashmë.
FileExistsOverwriteExisting=Mbishkruaj skedarin ekzistues
FileExistsKeepExisting=Mbaj skedarin ekzistues
FileExistsOverwriteOrKeepAll=Bëj të njëjtën gjë për konfliktet e ardhshme
ExistingFileNewerSelectAction=Zgjidhni veprimin
ExistingFileNewer2=Skedari ekzistues është më i ri se ai që po provon të instalojë instalimi.
ExistingFileNewerOverwriteExisting=Mbishkruaj skedarin ekzistues
ExistingFileNewerKeepExisting=Mbaj skedarin ekzistues (e rekomanduar)
ExistingFileNewerOverwriteOrKeepAll=Bëj të njëjtën gjë për konfliktet e ardhshme
ErrorChangingAttr=Ndodhi një gabim gjatë ndryshimit të atributeve të skedarit ekzistues:
ErrorCreatingTemp=Ndodhi një gabim gjatë krijimit të një skedari në dosjen e instalimit:
ErrorReadingSource=Ndodhi një gabim gjatë leximit të skedarit burim:
ErrorCopying=Ndodhi një gabim gjatë kopjimit të një skedari:
ErrorDownloading=Ndodhi një gabim gjatë shkarkimit të një skedari:
ErrorExtracting=Ndodhi një gabim gjatë nxjerrjes së një arkivi:
ErrorReplacingExistingFile=Ndodhi një gabim gjatë zëvendësimit të skedarit ekzistues:
ErrorRestartReplace=RestartReplace dështoi:
ErrorRenamingTemp=Ndodhi një gabim gjatë riemërtimit të një skedari në dosjen e instalimit:
ErrorRegisterServer=DLL/OCX nuk u regjistrua dot: %1
ErrorRegSvr32Failed=RegSvr32 dështoi me kodin e daljes %1
ErrorRegisterTypeLib=Biblioteka e llojeve nuk u regjistrua dot: %1

; *** Uninstall display name markings
; used for example as 'My Program (32-bit)'
UninstallDisplayNameMark=%1 (%2)
; used for example as 'My Program (32-bit, All users)'
UninstallDisplayNameMarks=%1 (%2, %3)
UninstallDisplayNameMark32Bit=32-bit
UninstallDisplayNameMark64Bit=64-bit
UninstallDisplayNameMarkAllUsers=Të gjithë përdoruesit
UninstallDisplayNameMarkCurrentUser=Përdoruesi aktual

; *** Post-installation errors
ErrorOpeningReadme=Ndodhi një gabim gjatë hapjes së skedarit README.
ErrorRestartingComputer=Instalimi nuk e rinisi dot kompjuterin. Bëjeni me dorë.

; *** Uninstaller messages
UninstallNotFound=Skedari "%1" nuk ekziston. Çinstalimi nuk është i mundur.
UninstallOpenError=Skedari "%1" nuk u hap dot. Çinstalimi nuk është i mundur
UninstallUnsupportedVer=Skedari i ditarit të çinstalimit "%1" ka një format që nuk njihet nga ky version i çinstaluesit. Çinstalimi nuk është i mundur
UninstallUnknownEntry=Në ditarin e çinstalimit u has një shënim i panjohur (%1)
ConfirmUninstall=Dëshironi vërtet ta hiqni plotësisht %1 dhe të gjithë përbërësit e tij?
UninstallOnlyOnWin64=Ky instalim mund të çinstalohet vetëm në Windows 64-bit.
OnlyAdminCanUninstall=Ky instalim mund të çinstalohet vetëm nga një përdorues me të drejta administratori.
UninstallStatusLabel=Prisni ndërsa %1 hiqet nga kompjuteri juaj.
UninstalledAll=%1 u hoq me sukses nga kompjuteri juaj.
UninstalledMost=Çinstalimi i %1 përfundoi.%n%nDisa elemente nuk u hoqën dot. Ato mund të hiqen me dorë.
UninstalledAndNeedsRestart=Për të përfunduar çinstalimin e %1, kompjuteri juaj duhet rinisur.%n%nDëshironi ta rinisni tani?
UninstallDataCorrupted=Skedari "%1" është i dëmtuar. Çinstalimi nuk është i mundur

; *** Uninstallation phase messages
ConfirmDeleteSharedFileTitle=Të hiqet skedari i përbashkët?
ConfirmDeleteSharedFile2=Sistemi tregon se skedari i mëposhtëm i përbashkët nuk përdoret më nga asnjë program. Dëshironi që çinstalimi ta heqë këtë skedar?%n%nNëse ka ende programe që e përdorin këtë skedar dhe ai hiqet, ato programe mund të mos funksionojnë si duhet. Nëse keni dyshime, zgjidhni «Jo». Lënia e skedarit në sistem nuk do të shkaktojë dëm.
SharedFileNameLabel=Emri i skedarit:
SharedFileLocationLabel=Vendndodhja:
WizardUninstalling=Gjendja e çinstalimit
StatusUninstalling=Po çinstalohet %1...

; *** Shutdown block reasons
ShutdownBlockReasonInstallingApp=Po instalohet %1.
ShutdownBlockReasonUninstallingApp=Po çinstalohet %1.

; The custom messages below aren't used by Setup itself, but if you make
; use of them in your scripts, you'll want to translate them.

[CustomMessages]

NameAndVersion=%1 versioni %2
AdditionalIcons=Shkurtore shtesë:
CreateDesktopIcon=Krijo një shkurtore në desktop
CreateQuickLaunchIcon=Krijo një shkurtore në «Quick Launch»
ProgramOnTheWeb=%1 në internet
UninstallProgram=Çinstalo %1
LaunchProgram=Nis %1
AssocFileExtension=Shoqëro %1 me shtrirjen e skedarëve %2
AssocingFileExtension=Po shoqërohet %1 me shtrirjen e skedarëve %2...
AutoStartProgramGroupDescription=Nisja:
AutoStartProgram=Nis %1 automatikisht
AddonHostProgramNotFound=%1 nuk u gjet në dosjen që zgjodhët.%n%nDëshironi të vazhdoni gjithsesi?

; *** The messages of Lang Switcher itself (the sample is English.isl)
LocalCertReady=Instalimi do të krijojë një certifikatë në këtë kompjuter dhe do ta nënshkruajë Lang Switcher me të, që programi të funksionojë edhe në dritaret e programeve të ekzekutuara si administrator.
LocalCertFailed=Lang Switcher u instalua, por nuk do të funksionojë në dritaret e programeve të ekzekutuara si administrator: Windows nuk e pranoi certifikatën e krijuar në këtë kompjuter. Detajet gjenden në ditarin e instalimit.
RemoveSettingsQuestion=Të fshihet edhe skedari i cilësimeve të Lang Switcher?
RemoveSettingsKeep=Zgjidhni «Jo» për t'i ruajtur cilësimet për një instalim të mëvonshëm.
AuthorSlogan=programe të bëra me zemër
