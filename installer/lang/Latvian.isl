; Lang Switcher -- the messages of the installer in Latvian (stage E109, group B).
;
; Inno Setup has no official Latvian translation (inno_official = false in res\lang\languages.toml),
; so this file carries the whole of it: [LangOptions], every message of the compiler:Default.isl
; (6.5.0+), and the messages of the installer that are Lang Switcher's own. It is read after
; compiler:Default.isl as the second MessagesFile of the "latvian" entry of [Languages] in
; ..\LangSwitcher.iss, and so replaces the English text name for name.
; English.isl is the sample the own messages translate; the glossary of the language is
; l10n\glossary\lv.tsv of the control contour. The name of the program is not translated.
; As in Default.isl: a message that has no full stop there has no full stop here (Setup adds its own).
; No "&" accelerators: Latvian has no official file, so one window never carries two equal letters;
; the names of buttons inside a message stand in the quotation marks of the language (« ») and are
; written as the buttons say them. The name of the program, taken as a bare name, is masculine; with
; the word "programma" it is feminine (one rule for this file, the strings and the feed).

[LangOptions]
LanguageName=Latviešu
LanguageID=$0426
LanguageCodePage=0

[Messages]

; *** Application titles
SetupAppTitle=Instalēšana
SetupWindowTitle=Instalēšana - %1
UninstallAppTitle=Atinstalēšana
UninstallAppFullTitle=%1 atinstalēšana

; *** Misc. common
InformationTitle=Informācija
ConfirmTitle=Apstiprinājums
ErrorTitle=Kļūda

; *** SetupLdr messages
SetupLdrStartupMessage=Tiks instalēta programma %1. Vai vēlaties turpināt?
LdrCannotCreateTemp=Neizdevās izveidot pagaidu failu. Instalēšana pārtraukta
LdrCannotExecTemp=Neizdevās palaist failu pagaidu mapē. Instalēšana pārtraukta
HelpTextNote=

; *** Startup error messages
LastErrorMessage=%1.%n%nKļūda %2: %3
SetupFileMissing=Instalēšanas mapē trūkst faila %1. Novērsiet problēmu vai iegūstiet jaunu programmas kopiju.
SetupFileCorrupt=Instalēšanas faili ir bojāti. Iegūstiet jaunu programmas kopiju.
SetupFileCorruptOrWrongVer=Instalēšanas faili ir bojāti vai nav saderīgi ar šo instalēšanas programmas versiju. Novērsiet problēmu vai iegūstiet jaunu programmas kopiju.
InvalidParameter=Komandrindā tika nodots nederīgs parametrs:%n%n%1
SetupAlreadyRunning=Instalēšanas programma jau darbojas.
WindowsVersionNotSupported=Šī programma neatbalsta jūsu datorā darbojošos Windows versiju.
WindowsServicePackRequired=Šai programmai nepieciešama %1 versija ar Service Pack %2 vai jaunāka.
NotOnThisPlatform=Šī programma nedarbosies sistēmā %1.
OnlyOnThisPlatform=Šī programma jāpalaiž sistēmā %1.
OnlyOnTheseArchitectures=Šo programmu var instalēt tikai tādās Windows versijās, kas paredzētas šādām procesora arhitektūrām:%n%n%1
WinVersionTooLowError=Šai programmai nepieciešama %1 versija %2 vai jaunāka.
WinVersionTooHighError=Šo programmu nevar instalēt %1 versijā %2 vai jaunākā.
AdminPrivilegesRequired=Lai instalētu šo programmu, jāpierakstās kā administratoram.
PowerUserPrivilegesRequired=Lai instalētu šo programmu, jāpierakstās kā administratoram vai kā grupas Power Users dalībniekam.
SetupAppRunningError=Instalēšanas programma konstatēja, ka pašlaik darbojas programma %1.%n%nAizveriet visas tās kopijas, pēc tam noklikšķiniet uz «Labi», lai turpinātu, vai uz «Atcelt», lai izietu.
UninstallAppRunningError=Atinstalēšanas programma konstatēja, ka pašlaik darbojas programma %1.%n%nAizveriet visas tās kopijas, pēc tam noklikšķiniet uz «Labi», lai turpinātu, vai uz «Atcelt», lai izietu.

; *** Startup questions
PrivilegesRequiredOverrideTitle=Instalēšanas režīma izvēle
PrivilegesRequiredOverrideInstruction=Izvēlieties instalēšanas režīmu
PrivilegesRequiredOverrideText1=Programmu %1 var instalēt visiem lietotājiem (nepieciešamas administratora tiesības) vai tikai jums.
PrivilegesRequiredOverrideText2=Programmu %1 var instalēt tikai jums vai visiem lietotājiem (nepieciešamas administratora tiesības).
PrivilegesRequiredOverrideAllUsers=Instalēt visiem lietotājiem
PrivilegesRequiredOverrideAllUsersRecommended=Instalēt visiem lietotājiem (ieteicams)
PrivilegesRequiredOverrideCurrentUser=Instalēt tikai man
PrivilegesRequiredOverrideCurrentUserRecommended=Instalēt tikai man (ieteicams)

; *** Misc. errors
ErrorCreatingDir=Instalēšanas programmai neizdevās izveidot mapi «%1»
ErrorTooManyFilesInDir=Neizdevās izveidot failu mapē «%1», jo tajā ir pārāk daudz failu

; *** Setup common messages
ExitSetupTitle=Iziet no instalēšanas
ExitSetupMessage=Instalēšana nav pabeigta. Ja iziesit tagad, programma netiks instalēta.%n%nInstalēšanas programmu varat palaist vēlreiz citā laikā, lai pabeigtu instalēšanu.%n%nIziet no instalēšanas?
AboutSetupMenuItem=Par instalēšanas programmu...
AboutSetupTitle=Par instalēšanas programmu
AboutSetupMessage=%1 versija %2%n%3%n%n%1 mājaslapa:%n%4
AboutSetupNote=
TranslatorNote=

; *** Buttons
ButtonBack=< Atpakaļ
ButtonNext=Tālāk >
ButtonInstall=Instalēt
ButtonOK=Labi
ButtonCancel=Atcelt
ButtonYes=Jā
ButtonYesToAll=Jā visiem
ButtonNo=Nē
ButtonNoToAll=Nē visiem
ButtonFinish=Pabeigt
ButtonBrowse=Pārlūkot...
ButtonWizardBrowse=Pārlūkot...
ButtonNewFolder=Izveidot jaunu mapi

; *** "Select Language" dialog messages
SelectLanguageTitle=Instalēšanas valodas izvēle
SelectLanguageLabel=Izvēlieties valodu, ko lietot instalēšanas laikā.

; *** Common wizard text
ClickNext=Noklikšķiniet uz «Tālāk», lai turpinātu, vai uz «Atcelt», lai izietu no instalēšanas.
BeveledLabel=
BrowseDialogTitle=Mapes izvēle
BrowseDialogLabel=Izvēlieties mapi sarakstā, pēc tam noklikšķiniet uz «Labi».
NewFolderName=Jauna mape

; *** "Welcome" wizard page
WelcomeLabel1=Laipni lūdzam programmas [name] instalēšanas vednī
WelcomeLabel2=Jūsu datorā tiks instalēta programma [name/ver].%n%nPirms turpināšanas ieteicams aizvērt visas pārējās programmas.

; *** "Password" wizard page
WizardPassword=Parole
PasswordLabel1=Šī instalēšana ir aizsargāta ar paroli.
PasswordLabel3=Ievadiet paroli, pēc tam noklikšķiniet uz «Tālāk», lai turpinātu. Parolēs tiek ņemts vērā burtu reģistrs.
PasswordEditLabel=Parole:
IncorrectPassword=Ievadītā parole nav pareiza. Mēģiniet vēlreiz.

; *** "License Agreement" wizard page
WizardLicense=Licences līgums
LicenseLabel=Pirms turpināšanas izlasiet šo svarīgo informāciju.
LicenseLabel3=Izlasiet tālāk norādīto licences līgumu. Pirms instalēšanas turpināšanas jums jāpiekrīt šī līguma noteikumiem.
LicenseAccepted=Es piekrītu līgumam
LicenseNotAccepted=Es nepiekrītu līgumam

; *** "Information" wizard pages
WizardInfoBefore=Informācija
InfoBeforeLabel=Pirms turpināšanas izlasiet šo svarīgo informāciju.
InfoBeforeClickLabel=Kad esat gatavi turpināt instalēšanu, noklikšķiniet uz «Tālāk».
WizardInfoAfter=Informācija
InfoAfterLabel=Pirms turpināšanas izlasiet šo svarīgo informāciju.
InfoAfterClickLabel=Kad esat gatavi turpināt instalēšanu, noklikšķiniet uz «Tālāk».

; *** "User Information" wizard page
WizardUserInfo=Lietotāja informācija
UserInfoDesc=Ievadiet savus datus.
UserInfoName=Lietotāja vārds:
UserInfoOrg=Organizācija:
UserInfoSerial=Sērijas numurs:
UserInfoNameRequired=Jāievada vārds.

; *** "Select Destination Location" wizard page
WizardSelectDir=Izvēlieties mērķa vietu
SelectDirDesc=Kur jāinstalē programma [name]?
SelectDirLabel3=Programma [name] tiks instalēta šajā mapē.
SelectDirBrowseLabel=Lai turpinātu, noklikšķiniet uz «Tālāk». Ja vēlaties izvēlēties citu mapi, noklikšķiniet uz «Pārlūkot».
DiskSpaceGBLabel=Nepieciešami vismaz [gb] GB brīvas vietas diskā.
DiskSpaceMBLabel=Nepieciešami vismaz [mb] MB brīvas vietas diskā.
CannotInstallToNetworkDrive=Instalēšanas programma nevar instalēt tīkla diskā.
CannotInstallToUNCPath=Instalēšanas programma nevar instalēt UNC ceļā.
InvalidPath=Jāievada pilns ceļš ar diska burtu, piemēram:%n%nC:\APP%n%nvai UNC ceļš šādā formā:%n%n\\serveris\kopigs
InvalidDrive=Izvēlētais disks vai UNC kopīgotais resurss nepastāv vai nav pieejams. Izvēlieties citu.
DiskSpaceWarningTitle=Nepietiek vietas diskā
DiskSpaceWarning=Instalēšanai nepieciešami vismaz %1 KB brīvas vietas, bet izvēlētajā diskā ir pieejami tikai %2 KB.%n%nVai tomēr turpināt?
DirNameTooLong=Mapes nosaukums vai ceļš ir pārāk garš.
InvalidDirName=Mapes nosaukums nav derīgs.
BadDirName32=Mapju nosaukumos nedrīkst būt neviena no šīm rakstzīmēm:%n%n%1
DirExistsTitle=Mape jau pastāv
DirExists=Mape:%n%n%1%n%njau pastāv. Vai tomēr vēlaties instalēt šajā mapē?
DirDoesntExistTitle=Mape nepastāv
DirDoesntExist=Mape:%n%n%1%n%nnepastāv. Vai vēlaties to izveidot?

; *** "Select Components" wizard page
WizardSelectComponents=Izvēlieties komponentus
SelectComponentsDesc=Kuri komponenti jāinstalē?
SelectComponentsLabel2=Atzīmējiet komponentus, ko vēlaties instalēt; noņemiet atzīmi komponentiem, ko nevēlaties instalēt. Kad esat gatavi turpināt, noklikšķiniet uz «Tālāk».
FullInstallation=Pilna instalēšana
; if possible don't translate 'Compact' as 'Minimal' (I mean 'Minimal' in your language)
CompactInstallation=Kompakta instalēšana
CustomInstallation=Pielāgota instalēšana
NoUninstallWarningTitle=Komponenti jau pastāv
NoUninstallWarning=Instalēšanas programma konstatēja, ka jūsu datorā jau ir instalēti šādi komponenti:%n%n%1%n%nNoņemot to atzīmes, komponenti netiks atinstalēti.%n%nVai tomēr turpināt?
ComponentSize1=%1 KB
ComponentSize2=%1 MB
ComponentsDiskSpaceGBLabel=Pašreizējai izvēlei nepieciešami vismaz [gb] GB vietas diskā.
ComponentsDiskSpaceMBLabel=Pašreizējai izvēlei nepieciešami vismaz [mb] MB vietas diskā.

; *** "Select Additional Tasks" wizard page
WizardSelectTasks=Izvēlieties papildu uzdevumus
SelectTasksDesc=Kuri papildu uzdevumi jāveic?
SelectTasksLabel2=Izvēlieties papildu uzdevumus, ko instalēšanas programma veiks, instalējot programmu [name], pēc tam noklikšķiniet uz «Tālāk».

; *** "Select Start Menu Folder" wizard page
WizardSelectProgramGroup=Izvēlieties izvēlnes Sākt mapi
SelectStartMenuFolderDesc=Kur instalēšanas programmai jāizveido programmas saīsnes?
SelectStartMenuFolderLabel3=Instalēšanas programma izveidos programmas saīsnes šajā izvēlnes Sākt mapē.
SelectStartMenuFolderBrowseLabel=Lai turpinātu, noklikšķiniet uz «Tālāk». Ja vēlaties izvēlēties citu mapi, noklikšķiniet uz «Pārlūkot».
MustEnterGroupName=Jāievada mapes nosaukums.
GroupNameTooLong=Mapes nosaukums vai ceļš ir pārāk garš.
InvalidGroupName=Mapes nosaukums nav derīgs.
BadGroupName=Mapes nosaukumā nedrīkst būt neviena no šīm rakstzīmēm:%n%n%1
NoProgramGroupCheck2=Neveidot izvēlnes Sākt mapi

; *** "Ready to Install" wizard page
WizardReady=Gatavs instalēšanai
ReadyLabel1=Instalēšanas programma ir gatava instalēt programmu [name] jūsu datorā.
ReadyLabel2a=Noklikšķiniet uz «Instalēt», lai turpinātu instalēšanu, vai uz «Atpakaļ», ja vēlaties pārskatīt vai mainīt iestatījumus.
ReadyLabel2b=Noklikšķiniet uz «Instalēt», lai turpinātu instalēšanu.
ReadyMemoUserInfo=Lietotāja informācija:
ReadyMemoDir=Mērķa vieta:
ReadyMemoType=Instalēšanas veids:
ReadyMemoComponents=Izvēlētie komponenti:
ReadyMemoGroup=Izvēlnes Sākt mape:
ReadyMemoTasks=Papildu uzdevumi:

; *** TDownloadWizardPage wizard page and DownloadTemporaryFile
DownloadingLabel2=Notiek failu lejupielāde...
ButtonStopDownload=Apturēt lejupielādi
StopDownload=Vai tiešām vēlaties apturēt lejupielādi?
ErrorDownloadAborted=Lejupielāde pārtraukta
ErrorDownloadFailed=Lejupielāde neizdevās: %1 %2
ErrorDownloadSizeFailed=Neizdevās iegūt izmēru: %1 %2
ErrorProgress=Nederīgs progress: %1 no %2
ErrorFileSize=Nederīgs faila izmērs: gaidīts %1, atrasts %2

; *** TExtractionWizardPage wizard page and ExtractArchive
ExtractingLabel=Notiek failu izvilkšana...
ButtonStopExtraction=Apturēt izvilkšanu
StopExtraction=Vai tiešām vēlaties apturēt izvilkšanu?
ErrorExtractionAborted=Izvilkšana pārtraukta
ErrorExtractionFailed=Izvilkšana neizdevās: %1

; *** Archive extraction failure details
ArchiveIncorrectPassword=Parole nav pareiza
ArchiveIsCorrupted=Arhīvs ir bojāts
ArchiveUnsupportedFormat=Arhīva formāts netiek atbalstīts

; *** "Preparing to Install" wizard page
WizardPreparing=Sagatavošanās instalēšanai
PreparingDesc=Instalēšanas programma gatavojas instalēt programmu [name] jūsu datorā.
PreviousInstallNotCompleted=Iepriekšējās programmas instalēšana vai noņemšana netika pabeigta. Lai pabeigtu šo instalēšanu, jārestartē dators.%n%nPēc datora restartēšanas vēlreiz palaidiet instalēšanas programmu, lai pabeigtu programmas [name] instalēšanu.
CannotContinue=Instalēšanas programma nevar turpināt. Noklikšķiniet uz «Atcelt», lai izietu.
ApplicationsFound=Šīs programmas izmanto failus, kas jāatjaunina instalēšanas programmai. Ieteicams ļaut instalēšanas programmai automātiski aizvērt šīs programmas.
ApplicationsFound2=Šīs programmas izmanto failus, kas jāatjaunina instalēšanas programmai. Ieteicams ļaut instalēšanas programmai automātiski aizvērt šīs programmas. Kad instalēšana būs pabeigta, instalēšanas programma mēģinās šīs programmas palaist atkārtoti.
CloseApplications=Automātiski aizvērt programmas
DontCloseApplications=Neaizvērt programmas
ErrorCloseApplications=Instalēšanas programmai neizdevās automātiski aizvērt visas programmas. Pirms turpināšanas ieteicams aizvērt visas programmas, kas izmanto failus, kas jāatjaunina instalēšanas programmai.
PrepareToInstallNeedsRestart=Instalēšanas programmai jārestartē jūsu dators. Pēc datora restartēšanas vēlreiz palaidiet instalēšanas programmu, lai pabeigtu programmas [name] instalēšanu.%n%nVai vēlaties restartēt datoru tagad?

; *** "Installing" wizard page
WizardInstalling=Notiek instalēšana
InstallingLabel=Uzgaidiet, kamēr instalēšanas programma instalē programmu [name] jūsu datorā.

; *** "Setup Completed" wizard page
FinishedHeadingLabel=Programmas [name] instalēšanas vednis tiek pabeigts
FinishedLabelNoIcons=Instalēšanas programma ir pabeigusi programmas [name] instalēšanu jūsu datorā.
FinishedLabel=Instalēšanas programma ir pabeigusi programmas [name] instalēšanu jūsu datorā. Programmu var palaist, izmantojot instalētās saīsnes.
ClickFinish=Noklikšķiniet uz «Pabeigt», lai izietu no instalēšanas.
FinishedRestartLabel=Lai pabeigtu programmas [name] instalēšanu, instalēšanas programmai jārestartē jūsu dators. Vai vēlaties restartēt datoru tagad?
FinishedRestartMessage=Lai pabeigtu programmas [name] instalēšanu, instalēšanas programmai jārestartē jūsu dators.%n%nVai vēlaties restartēt datoru tagad?
ShowReadmeCheck=Jā, vēlos apskatīt failu README
YesRadio=Jā, restartēt datoru tagad
NoRadio=Nē, datoru restartēšu vēlāk
; used for example as 'Run MyProg.exe'
RunEntryExec=Palaist %1
; used for example as 'View Readme.txt'
RunEntryShellExec=Apskatīt %1

; *** "Setup Needs the Next Disk" stuff
ChangeDiskTitle=Instalēšanas programmai nepieciešams nākamais disks
SelectDiskLabel2=Ievietojiet disku %1 un noklikšķiniet uz «Labi».%n%nJa šī diska faili atrodami citā mapē, nevis tajā, kas parādīta zemāk, ievadiet pareizo ceļu vai noklikšķiniet uz «Pārlūkot».
PathLabel=Ceļš:
FileNotInDir2=Failu «%1» neizdevās atrast mapē «%2». Ievietojiet pareizo disku vai izvēlieties citu mapi.
SelectDirectoryLabel=Norādiet nākamā diska atrašanās vietu.

; *** Installation phase messages
SetupAborted=Instalēšana nav pabeigta.%n%nNovērsiet problēmu un vēlreiz palaidiet instalēšanas programmu.
AbortRetryIgnoreSelectAction=Izvēlieties darbību
AbortRetryIgnoreRetry=Mēģināt vēlreiz
AbortRetryIgnoreIgnore=Ignorēt kļūdu un turpināt
AbortRetryIgnoreCancel=Atcelt instalēšanu
RetryCancelSelectAction=Izvēlieties darbību
RetryCancelRetry=Mēģināt vēlreiz
RetryCancelCancel=Atcelt

; *** Installation status messages
StatusClosingApplications=Notiek programmu aizvēršana...
StatusCreateDirs=Notiek mapju izveide...
StatusExtractFiles=Notiek failu izvilkšana...
StatusDownloadFiles=Notiek failu lejupielāde...
StatusCreateIcons=Notiek saīsņu izveide...
StatusCreateIniEntries=Notiek INI ierakstu izveide...
StatusCreateRegistryEntries=Notiek reģistra ierakstu izveide...
StatusRegisterFiles=Notiek failu reģistrēšana...
StatusSavingUninstall=Notiek atinstalēšanas informācijas saglabāšana...
StatusRunProgram=Notiek instalēšanas pabeigšana...
StatusRestartingApplications=Notiek programmu atkārtota palaišana...
StatusRollback=Notiek izmaiņu atcelšana...

; *** Misc. errors
ErrorInternal2=Iekšēja kļūda: %1
ErrorFunctionFailedNoCode=%1 neizdevās
ErrorFunctionFailed=%1 neizdevās; kods %2
ErrorFunctionFailedWithMessage=%1 neizdevās; kods %2.%n%3
ErrorExecutingProgram=Neizdevās palaist failu:%n%1

; *** Registry errors
ErrorRegOpenKey=Kļūda, atverot reģistra atslēgu:%n%1\%2
ErrorRegCreateKey=Kļūda, veidojot reģistra atslēgu:%n%1\%2
ErrorRegWriteKey=Kļūda, rakstot reģistra atslēgā:%n%1\%2

; *** INI errors
ErrorIniEntry=Kļūda, veidojot INI ierakstu failā «%1».

; *** File copying errors
FileAbortRetryIgnoreSkipNotRecommended=Izlaist šo failu (nav ieteicams)
FileAbortRetryIgnoreIgnoreNotRecommended=Ignorēt kļūdu un turpināt (nav ieteicams)
SourceIsCorrupted=Avota fails ir bojāts
SourceDoesntExist=Avota fails «%1» nepastāv
SourceVerificationFailed=Avota faila pārbaude neizdevās: %1
VerificationSignatureDoesntExist=Paraksta fails «%1» nepastāv
VerificationSignatureInvalid=Paraksta fails «%1» nav derīgs
VerificationKeyNotFound=Paraksta failā «%1» izmantota nezināma atslēga
VerificationFileNameIncorrect=Faila nosaukums nav pareizs
VerificationFileTagIncorrect=Faila birka nav pareiza
VerificationFileSizeIncorrect=Faila izmērs nav pareizs
VerificationFileHashIncorrect=Faila jaucējkods nav pareizs
ExistingFileReadOnly2=Esošo failu nevarēja aizstāt, jo tas ir atzīmēts kā tikai lasāms.
ExistingFileReadOnlyRetry=Noņemt tikai lasāmā faila atribūtu un mēģināt vēlreiz
ExistingFileReadOnlyKeepExisting=Paturēt esošo failu
ErrorReadingExistingDest=Mēģinot nolasīt esošo failu, radās kļūda:
FileExistsSelectAction=Izvēlieties darbību
FileExists2=Fails jau pastāv.
FileExistsOverwriteExisting=Pārrakstīt esošo failu
FileExistsKeepExisting=Paturēt esošo failu
FileExistsOverwriteOrKeepAll=Tā rīkoties ar nākamajiem konfliktiem
ExistingFileNewerSelectAction=Izvēlieties darbību
ExistingFileNewer2=Esošais fails ir jaunāks par to, ko instalēšanas programma mēģina instalēt.
ExistingFileNewerOverwriteExisting=Pārrakstīt esošo failu
ExistingFileNewerKeepExisting=Paturēt esošo failu (ieteicams)
ExistingFileNewerOverwriteOrKeepAll=Tā rīkoties ar nākamajiem konfliktiem
ErrorChangingAttr=Mēģinot mainīt esošā faila atribūtus, radās kļūda:
ErrorCreatingTemp=Mēģinot izveidot failu mērķa mapē, radās kļūda:
ErrorReadingSource=Mēģinot nolasīt avota failu, radās kļūda:
ErrorCopying=Mēģinot kopēt failu, radās kļūda:
ErrorDownloading=Mēģinot lejupielādēt failu, radās kļūda:
ErrorExtracting=Mēģinot izvilkt arhīvu, radās kļūda:
ErrorReplacingExistingFile=Mēģinot aizstāt esošo failu, radās kļūda:
ErrorRestartReplace=RestartReplace neizdevās:
ErrorRenamingTemp=Mēģinot pārdēvēt failu mērķa mapē, radās kļūda:
ErrorRegisterServer=Neizdevās reģistrēt DLL/OCX: %1
ErrorRegSvr32Failed=RegSvr32 neizdevās, izejas kods %1
ErrorRegisterTypeLib=Neizdevās reģistrēt tipu bibliotēku: %1

; *** Uninstall display name markings
; used for example as 'My Program (32-bit)'
UninstallDisplayNameMark=%1 (%2)
; used for example as 'My Program (32-bit, All users)'
UninstallDisplayNameMarks=%1 (%2, %3)
UninstallDisplayNameMark32Bit=32 bitu
UninstallDisplayNameMark64Bit=64 bitu
UninstallDisplayNameMarkAllUsers=Visi lietotāji
UninstallDisplayNameMarkCurrentUser=Pašreizējais lietotājs

; *** Post-installation errors
ErrorOpeningReadme=Mēģinot atvērt failu README, radās kļūda.
ErrorRestartingComputer=Instalēšanas programmai neizdevās restartēt datoru. Izdariet to manuāli.

; *** Uninstaller messages
UninstallNotFound=Fails «%1» nepastāv. Atinstalēšana nav iespējama.
UninstallOpenError=Failu «%1» neizdevās atvērt. Atinstalēšana nav iespējama
UninstallUnsupportedVer=Atinstalēšanas žurnāla fails «%1» ir formātā, ko šī atinstalēšanas programmas versija neatpazīst. Atinstalēšana nav iespējama
UninstallUnknownEntry=Atinstalēšanas žurnālā atrasts nezināms ieraksts (%1)
ConfirmUninstall=Vai tiešām vēlaties pilnībā noņemt programmu %1 un visus tās komponentus?
UninstallOnlyOnWin64=Šo instalāciju var atinstalēt tikai 64 bitu Windows sistēmā.
OnlyAdminCanUninstall=Šo instalāciju var atinstalēt tikai lietotājs ar administratora tiesībām.
UninstallStatusLabel=Uzgaidiet, kamēr programma %1 tiek noņemta no jūsu datora.
UninstalledAll=Programma %1 tika veiksmīgi noņemta no jūsu datora.
UninstalledMost=Programmas %1 atinstalēšana ir pabeigta.%n%nDažus elementus neizdevās noņemt. Tos var noņemt manuāli.
UninstalledAndNeedsRestart=Lai pabeigtu programmas %1 atinstalēšanu, jārestartē dators.%n%nVai vēlaties restartēt datoru tagad?
UninstallDataCorrupted=Fails «%1» ir bojāts. Atinstalēšana nav iespējama

; *** Uninstallation phase messages
ConfirmDeleteSharedFileTitle=Noņemt kopīgoto failu?
ConfirmDeleteSharedFile2=Sistēma norāda, ka šo kopīgoto failu vairs neizmanto neviena programma. Vai vēlaties, lai atinstalēšanas programma noņem šo kopīgoto failu?%n%nJa kāda programma joprojām izmanto šo failu un tas tiks noņemts, šī programma var nedarboties pareizi. Ja šaubāties, izvēlieties «Nē». Atstājot failu sistēmā, kaitējums netiks nodarīts.
SharedFileNameLabel=Faila nosaukums:
SharedFileLocationLabel=Atrašanās vieta:
WizardUninstalling=Atinstalēšanas stāvoklis
StatusUninstalling=Notiek %1 atinstalēšana...

; *** Shutdown block reasons
ShutdownBlockReasonInstallingApp=Notiek %1 instalēšana.
ShutdownBlockReasonUninstallingApp=Notiek %1 atinstalēšana.

; The custom messages below aren't used by Setup itself, but if you make
; use of them in your scripts, you'll want to translate them.

[CustomMessages]

NameAndVersion=%1 versija %2
AdditionalIcons=Papildu saīsnes:
CreateDesktopIcon=Izveidot saīsni darbvirsmā
CreateQuickLaunchIcon=Izveidot ātrās palaišanas saīsni
ProgramOnTheWeb=%1 tīmeklī
UninstallProgram=Atinstalēt %1
LaunchProgram=Palaist %1
AssocFileExtension=Saistīt %1 ar faila paplašinājumu %2
AssocingFileExtension=Notiek %1 saistīšana ar faila paplašinājumu %2...
AutoStartProgramGroupDescription=Palaišana:
AutoStartProgram=Automātiski palaist %1
AddonHostProgramNotFound=Izvēlētajā mapē neizdevās atrast %1.%n%nVai tomēr turpināt?

; The messages that are Lang Switcher's own (the sample is English.isl).
LocalCertReady=Instalēšanas programma šajā datorā izveidos sertifikātu un ar to parakstīs Lang Switcher, lai programma darbotos arī ar administratora tiesībām palaistu programmu logos.
LocalCertFailed=Lang Switcher ir instalēts, bet nedarbosies ar administratora tiesībām palaistu programmu logos: Windows nepieņēma šajā datorā izveidoto sertifikātu. Sīkāka informācija ir instalēšanas žurnālā.
RemoveSettingsQuestion=Vai noņemt arī Lang Switcher iestatījumu failu?
RemoveSettingsKeep=Izvēlieties «Nē», lai saglabātu iestatījumus nākamajai instalēšanai.
AuthorSlogan=programmas, kas veidotas ar sirdi
