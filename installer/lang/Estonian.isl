; Lang Switcher -- the messages of the installer in Estonian (stage E109, group B).
;
; Inno Setup has no official Estonian translation (inno_official = false in res\lang\languages.toml),
; so this file carries the whole of it: [LangOptions], every message of the compiler:Default.isl
; (6.5.0+), and the messages of the installer that are Lang Switcher's own. It is read after
; compiler:Default.isl as the second MessagesFile of the "estonian" entry of [Languages] in
; ..\LangSwitcher.iss, and so replaces the English text name for name.
; English.isl is the sample the own messages translate; the glossary of the language is
; l10n\glossary\et.tsv of the control contour. The name of the program is not translated.
; As in Default.isl: a message that has no full stop there has none here (Setup adds its own).
; No "&" accelerators (RULES 16): the names of the buttons inside a text stand in the quotes „ “.
; The tone is the one of Windows in Estonian: an instruction or a question addresses the reader
; politely in the plural (te: "Valige", "Kas soovite"); the names of buttons are the imperative of
; the singular or a plain noun ("Paigalda", "Loobu").

[LangOptions]
LanguageName=Eesti
LanguageID=$0425
LanguageCodePage=0

[Messages]

; *** Application titles
SetupAppTitle=Paigaldaja
SetupWindowTitle=Paigaldaja - %1
UninstallAppTitle=Eemaldaja
UninstallAppFullTitle=Programmi %1 eemaldaja

; *** Misc. common
InformationTitle=Teave
ConfirmTitle=Kinnitus
ErrorTitle=Viga

; *** SetupLdr messages
SetupLdrStartupMessage=Paigaldatakse %1. Kas soovite jätkata?
LdrCannotCreateTemp=Ajutist faili ei saanud luua. Paigaldamine katkestati
LdrCannotExecTemp=Ajutises kaustas ei saanud faili käivitada. Paigaldamine katkestati
HelpTextNote=

; *** Startup error messages
LastErrorMessage=%1.%n%nViga %2: %3
SetupFileMissing=Paigalduskaustast puudub fail %1. Parandage probleem või hankige programmi uus koopia.
SetupFileCorrupt=Paigaldusfailid on rikutud. Hankige programmi uus koopia.
SetupFileCorruptOrWrongVer=Paigaldusfailid on rikutud või ühildumatud selle paigaldaja versiooniga. Parandage probleem või hankige programmi uus koopia.
InvalidParameter=Käsureal anti sobimatu parameeter:%n%n%1
SetupAlreadyRunning=Paigaldaja juba töötab.
WindowsVersionNotSupported=See programm ei toeta teie arvutis töötavat Windowsi versiooni.
WindowsServicePackRequired=See programm nõuab süsteemi %1 Service Pack %2 või uuemat versiooni.
NotOnThisPlatform=See programm ei tööta süsteemis %1.
OnlyOnThisPlatform=See programm tuleb käivitada süsteemis %1.
OnlyOnTheseArchitectures=Seda programmi saab paigaldada ainult Windowsi versioonidele, mis on mõeldud järgmistele protsessoriarhitektuuridele:%n%n%1
WinVersionTooLowError=See programm nõuab süsteemi %1 versiooni %2 või uuemat.
WinVersionTooHighError=Seda programmi ei saa paigaldada süsteemi %1 versioonile %2 ega uuemale.
AdminPrivilegesRequired=Selle programmi paigaldamiseks peate olema sisse logitud administraatorina.
PowerUserPrivilegesRequired=Selle programmi paigaldamiseks peate olema sisse logitud administraatorina või rühma Power Users liikmena.
SetupAppRunningError=Paigaldaja tuvastas, et %1 töötab praegu.%n%nSulgege kõik selle aknad ja jätkamiseks klõpsake nuppu „OK“ või väljumiseks nuppu „Loobu“.
UninstallAppRunningError=Eemaldaja tuvastas, et %1 töötab praegu.%n%nSulgege kõik selle aknad ja jätkamiseks klõpsake nuppu „OK“ või väljumiseks nuppu „Loobu“.

; *** Startup questions
PrivilegesRequiredOverrideTitle=Valige paigaldusrežiim
PrivilegesRequiredOverrideInstruction=Valige paigaldusrežiim
PrivilegesRequiredOverrideText1=%1 saab paigaldada kõigile kasutajatele (vajab administraatoriõigusi) või ainult teile.
PrivilegesRequiredOverrideText2=%1 saab paigaldada ainult teile või kõigile kasutajatele (vajab administraatoriõigusi).
PrivilegesRequiredOverrideAllUsers=Paigalda kõigile kasutajatele
PrivilegesRequiredOverrideAllUsersRecommended=Paigalda kõigile kasutajatele (soovitatav)
PrivilegesRequiredOverrideCurrentUser=Paigalda ainult mulle
PrivilegesRequiredOverrideCurrentUserRecommended=Paigalda ainult mulle (soovitatav)

; *** Misc. errors
ErrorCreatingDir=Paigaldaja ei saanud luua kausta „%1“
ErrorTooManyFilesInDir=Kausta „%1“ ei saa faili luua, sest selles on liiga palju faile

; *** Setup common messages
ExitSetupTitle=Paigaldajast väljumine
ExitSetupMessage=Paigaldus pole lõpule viidud. Kui väljute nüüd, siis programmi ei paigaldata.%n%nPaigalduse lõpuleviimiseks võite paigaldaja hiljem uuesti käivitada.%n%nKas väljuda paigaldajast?
AboutSetupMenuItem=Teave paigaldaja kohta...
AboutSetupTitle=Teave paigaldaja kohta
AboutSetupMessage=%1 versioon %2%n%3%n%n%1 koduleht:%n%4
AboutSetupNote=
TranslatorNote=

; *** Buttons
ButtonBack=< Tagasi
ButtonNext=Edasi >
ButtonInstall=Paigalda
ButtonOK=OK
ButtonCancel=Loobu
ButtonYes=Jah
ButtonYesToAll=Jah, kõigile
ButtonNo=Ei
ButtonNoToAll=Ei, kõigile
ButtonFinish=Lõpeta
ButtonBrowse=Sirvi...
ButtonWizardBrowse=Sirvi...
ButtonNewFolder=Loo uus kaust

; *** "Select Language" dialog messages
SelectLanguageTitle=Paigaldaja keele valimine
SelectLanguageLabel=Valige paigalduse ajal kasutatav keel.

; *** Common wizard text
ClickNext=Jätkamiseks klõpsake nuppu „Edasi“, väljumiseks nuppu „Loobu“.
BeveledLabel=
BrowseDialogTitle=Kausta valimine
BrowseDialogLabel=Valige allolevast loendist kaust ja klõpsake nuppu „OK“.
NewFolderName=Uus kaust

; *** "Welcome" wizard page
WelcomeLabel1=Tere tulemast programmi [name] paigaldusviisardisse
WelcomeLabel2=Teie arvutisse paigaldatakse [name/ver].%n%nEnne jätkamist on soovitatav sulgeda kõik muud rakendused.

; *** "Password" wizard page
WizardPassword=Parool
PasswordLabel1=See paigaldus on parooliga kaitstud.
PasswordLabel3=Sisestage parool ja jätkamiseks klõpsake nuppu „Edasi“. Paroolid on tõstutundlikud.
PasswordEditLabel=Parool:
IncorrectPassword=Sisestatud parool pole õige. Proovige uuesti.

; *** "License Agreement" wizard page
WizardLicense=Litsentsileping
LicenseLabel=Enne jätkamist lugege läbi järgmine oluline teave.
LicenseLabel3=Lugege läbi järgmine litsentsileping. Enne paigaldamise jätkamist peate lepingu tingimustega nõustuma.
LicenseAccepted=Nõustun lepinguga
LicenseNotAccepted=Ma ei nõustu lepinguga

; *** "Information" wizard pages
WizardInfoBefore=Teave
InfoBeforeLabel=Enne jätkamist lugege läbi järgmine oluline teave.
InfoBeforeClickLabel=Kui olete valmis paigaldamist jätkama, klõpsake nuppu „Edasi“.
WizardInfoAfter=Teave
InfoAfterLabel=Enne jätkamist lugege läbi järgmine oluline teave.
InfoAfterClickLabel=Kui olete valmis paigaldamist jätkama, klõpsake nuppu „Edasi“.

; *** "User Information" wizard page
WizardUserInfo=Kasutaja andmed
UserInfoDesc=Sisestage oma andmed.
UserInfoName=Kasutajanimi:
UserInfoOrg=Organisatsioon:
UserInfoSerial=Seerianumber:
UserInfoNameRequired=Nimi tuleb sisestada.

; *** "Select Destination Location" wizard page
WizardSelectDir=Sihtkoha valimine
SelectDirDesc=Kuhu programm [name] paigaldada?
SelectDirLabel3=Paigaldaja paigaldab programmi [name] järgmisse kausta.
SelectDirBrowseLabel=Jätkamiseks klõpsake nuppu „Edasi“. Kui soovite valida mõne teise kausta, klõpsake nuppu „Sirvi“.
DiskSpaceGBLabel=Vaja on vähemalt [gb] GB vaba kettaruumi.
DiskSpaceMBLabel=Vaja on vähemalt [mb] MB vaba kettaruumi.
CannotInstallToNetworkDrive=Paigaldaja ei saa paigaldada võrgukettale.
CannotInstallToUNCPath=Paigaldaja ei saa paigaldada UNC-teele.
InvalidPath=Sisestage täielik tee koos draivitähega, näiteks:%n%nC:\APP%n%nvõi UNC-tee kujul:%n%n\\server\share
InvalidDrive=Valitud draivi või UNC-jagamist pole olemas või see pole juurdepääsetav. Valige mõni teine.
DiskSpaceWarningTitle=Kettaruumi pole piisavalt
DiskSpaceWarning=Paigaldamiseks on vaja vähemalt %1 KB vaba ruumi, kuid valitud draivil on saadaval ainult %2 KB.%n%nKas soovite sellegipoolest jätkata?
DirNameTooLong=Kausta nimi või tee on liiga pikk.
InvalidDirName=Kausta nimi pole kehtiv.
BadDirName32=Kaustanimed ei tohi sisaldada ühtegi järgmistest märkidest:%n%n%1
DirExistsTitle=Kaust on olemas
DirExists=Kaust:%n%n%1%n%non juba olemas. Kas soovite sellegipoolest sellesse kausta paigaldada?
DirDoesntExistTitle=Kausta pole olemas
DirDoesntExist=Kausta:%n%n%1%n%npole olemas. Kas soovite kausta luua?

; *** "Select Components" wizard page
WizardSelectComponents=Komponentide valimine
SelectComponentsDesc=Milliseid komponente paigaldada?
SelectComponentsLabel2=Märkige paigaldatavad komponendid; eemaldage märge komponentidelt, mida te paigaldada ei soovi. Jätkamiseks klõpsake nuppu „Edasi“.
FullInstallation=Täielik paigaldus
; if possible don't translate 'Compact' as 'Minimal' (I mean 'Minimal' in your language)
CompactInstallation=Kompaktne paigaldus
CustomInstallation=Kohandatud paigaldus
NoUninstallWarningTitle=Komponendid on olemas
NoUninstallWarning=Paigaldaja tuvastas, et järgmised komponendid on teie arvutisse juba paigaldatud:%n%n%1%n%nNende komponentide märke eemaldamine neid ei eemalda.%n%nKas soovite sellegipoolest jätkata?
ComponentSize1=%1 KB
ComponentSize2=%1 MB
ComponentsDiskSpaceGBLabel=Praegune valik vajab vähemalt [gb] GB kettaruumi.
ComponentsDiskSpaceMBLabel=Praegune valik vajab vähemalt [mb] MB kettaruumi.

; *** "Select Additional Tasks" wizard page
WizardSelectTasks=Lisatoimingute valimine
SelectTasksDesc=Milliseid lisatoiminguid teha?
SelectTasksLabel2=Valige lisatoimingud, mida paigaldaja programmi [name] paigaldamise ajal tegema peaks, ja klõpsake nuppu „Edasi“.

; *** "Select Start Menu Folder" wizard page
WizardSelectProgramGroup=Menüü Start kausta valimine
SelectStartMenuFolderDesc=Kuhu paigaldaja peaks programmi otseteed paigutama?
SelectStartMenuFolderLabel3=Paigaldaja loob programmi otseteed järgmisse menüü Start kausta.
SelectStartMenuFolderBrowseLabel=Jätkamiseks klõpsake nuppu „Edasi“. Kui soovite valida mõne teise kausta, klõpsake nuppu „Sirvi“.
MustEnterGroupName=Sisestage kausta nimi.
GroupNameTooLong=Kausta nimi või tee on liiga pikk.
InvalidGroupName=Kausta nimi pole kehtiv.
BadGroupName=Kausta nimi ei tohi sisaldada ühtegi järgmistest märkidest:%n%n%1
NoProgramGroupCheck2=Ära loo menüü Start kausta

; *** "Ready to Install" wizard page
WizardReady=Valmis paigaldama
ReadyLabel1=Paigaldaja on nüüd valmis programmi [name] teie arvutisse paigaldama.
ReadyLabel2a=Paigaldamise jätkamiseks klõpsake nuppu „Paigalda“ või klõpsake nuppu „Tagasi“, kui soovite sätteid üle vaadata või muuta.
ReadyLabel2b=Paigaldamise jätkamiseks klõpsake nuppu „Paigalda“.
ReadyMemoUserInfo=Kasutaja andmed:
ReadyMemoDir=Sihtkoht:
ReadyMemoType=Paigaldustüüp:
ReadyMemoComponents=Valitud komponendid:
ReadyMemoGroup=Menüü Start kaust:
ReadyMemoTasks=Lisatoimingud:

; *** TDownloadWizardPage wizard page and DownloadTemporaryFile
DownloadingLabel2=Failide allalaadimine...
ButtonStopDownload=Peata allalaadimine
StopDownload=Kas olete kindel, et soovite allalaadimise peatada?
ErrorDownloadAborted=Allalaadimine katkestati
ErrorDownloadFailed=Allalaadimine ebaõnnestus: %1 %2
ErrorDownloadSizeFailed=Suuruse tuvastamine ebaõnnestus: %1 %2
ErrorProgress=Vigane edenemine: %1 / %2
ErrorFileSize=Vigane failisuurus: eeldati %1, leiti %2

; *** TExtractionWizardPage wizard page and ExtractArchive
ExtractingLabel=Failide lahtipakkimine...
ButtonStopExtraction=Peata lahtipakkimine
StopExtraction=Kas olete kindel, et soovite lahtipakkimise peatada?
ErrorExtractionAborted=Lahtipakkimine katkestati
ErrorExtractionFailed=Lahtipakkimine ebaõnnestus: %1

; *** Archive extraction failure details
ArchiveIncorrectPassword=Parool on vale
ArchiveIsCorrupted=Arhiiv on rikutud
ArchiveUnsupportedFormat=Arhiivivormingut ei toetata

; *** "Preparing to Install" wizard page
WizardPreparing=Paigaldamiseks valmistumine
PreparingDesc=Paigaldaja valmistub programmi [name] teie arvutisse paigaldama.
PreviousInstallNotCompleted=Eelmise programmi paigaldamine või eemaldamine jäi pooleli. Selle lõpuleviimiseks tuleb arvuti taaskäivitada.%n%nPärast arvuti taaskäivitamist käivitage paigaldaja uuesti, et lõpule viia programmi [name] paigaldamine.
CannotContinue=Paigaldaja ei saa jätkata. Väljumiseks klõpsake nuppu „Loobu“.
ApplicationsFound=Järgmised rakendused kasutavad faile, mida paigaldaja peab värskendama. On soovitatav lubada paigaldajal need rakendused automaatselt sulgeda.
ApplicationsFound2=Järgmised rakendused kasutavad faile, mida paigaldaja peab värskendama. On soovitatav lubada paigaldajal need rakendused automaatselt sulgeda. Pärast paigaldamise lõppu proovib paigaldaja rakendused uuesti käivitada.
CloseApplications=Sulge rakendused automaatselt
DontCloseApplications=Ära sulge rakendusi
ErrorCloseApplications=Paigaldaja ei suutnud kõiki rakendusi automaatselt sulgeda. Enne jätkamist on soovitatav sulgeda kõik rakendused, mis kasutavad faile, mida paigaldaja peab värskendama.
PrepareToInstallNeedsRestart=Paigaldaja peab teie arvuti taaskäivitama. Pärast arvuti taaskäivitamist käivitage paigaldaja uuesti, et lõpule viia programmi [name] paigaldamine.%n%nKas soovite arvuti kohe taaskäivitada?

; *** "Installing" wizard page
WizardInstalling=Paigaldamine
InstallingLabel=Oodake, kuni paigaldaja paigaldab programmi [name] teie arvutisse.

; *** "Setup Completed" wizard page
FinishedHeadingLabel=Programmi [name] paigaldusviisardi lõpetamine
FinishedLabelNoIcons=Paigaldaja lõpetas programmi [name] paigaldamise teie arvutisse.
FinishedLabel=Paigaldaja lõpetas programmi [name] paigaldamise teie arvutisse. Programmi saab käivitada paigaldatud otseteede kaudu.
ClickFinish=Paigaldajast väljumiseks klõpsake nuppu „Lõpeta“.
FinishedRestartLabel=Programmi [name] paigaldamise lõpuleviimiseks peab paigaldaja teie arvuti taaskäivitama. Kas soovite arvuti kohe taaskäivitada?
FinishedRestartMessage=Programmi [name] paigaldamise lõpuleviimiseks peab paigaldaja teie arvuti taaskäivitama.%n%nKas soovite arvuti kohe taaskäivitada?
ShowReadmeCheck=Jah, soovin näha README-faili
YesRadio=Jah, taaskäivita arvuti kohe
NoRadio=Ei, taaskäivitan arvuti hiljem
; used for example as 'Run MyProg.exe'
RunEntryExec=Käivita %1
; used for example as 'View Readme.txt'
RunEntryShellExec=Vaata faili %1

; *** "Setup Needs the Next Disk" stuff
ChangeDiskTitle=Paigaldaja vajab järgmist ketast
SelectDiskLabel2=Sisestage ketas %1 ja klõpsake nuppu „OK“.%n%nKui selle ketta failid asuvad mõnes muus kaustas kui allpool kuvatud kaust, sisestage õige tee või klõpsake nuppu „Sirvi“.
PathLabel=Tee:
FileNotInDir2=Faili „%1“ ei leitud asukohast „%2“. Sisestage õige ketas või valige mõni teine kaust.
SelectDirectoryLabel=Määrake järgmise ketta asukoht.

; *** Installation phase messages
SetupAborted=Paigaldamist ei viidud lõpule.%n%nParandage probleem ja käivitage paigaldaja uuesti.
AbortRetryIgnoreSelectAction=Valige toiming
AbortRetryIgnoreRetry=Proovi uuesti
AbortRetryIgnoreIgnore=Ignoreeri viga ja jätka
AbortRetryIgnoreCancel=Tühista paigaldamine
RetryCancelSelectAction=Valige toiming
RetryCancelRetry=Proovi uuesti
RetryCancelCancel=Loobu

; *** Installation status messages
StatusClosingApplications=Rakenduste sulgemine...
StatusCreateDirs=Kaustade loomine...
StatusExtractFiles=Failide lahtipakkimine...
StatusDownloadFiles=Failide allalaadimine...
StatusCreateIcons=Otseteede loomine...
StatusCreateIniEntries=INI-kirjete loomine...
StatusCreateRegistryEntries=Registrikirjete loomine...
StatusRegisterFiles=Failide registreerimine...
StatusSavingUninstall=Eemaldamisteabe salvestamine...
StatusRunProgram=Paigaldamise lõpetamine...
StatusRestartingApplications=Rakenduste taaskäivitamine...
StatusRollback=Muudatuste tagasivõtmine...

; *** Misc. errors
ErrorInternal2=Sisemine viga: %1
ErrorFunctionFailedNoCode=%1 ebaõnnestus
ErrorFunctionFailed=%1 ebaõnnestus; kood %2
ErrorFunctionFailedWithMessage=%1 ebaõnnestus; kood %2.%n%3
ErrorExecutingProgram=Faili ei saanud käivitada:%n%1

; *** Registry errors
ErrorRegOpenKey=Viga registrivõtme avamisel:%n%1\%2
ErrorRegCreateKey=Viga registrivõtme loomisel:%n%1\%2
ErrorRegWriteKey=Viga registrivõtmesse kirjutamisel:%n%1\%2

; *** INI errors
ErrorIniEntry=Viga INI-kirje loomisel failis „%1“.

; *** File copying errors
FileAbortRetryIgnoreSkipNotRecommended=Jäta see fail vahele (ei soovitata)
FileAbortRetryIgnoreIgnoreNotRecommended=Ignoreeri viga ja jätka (ei soovitata)
SourceIsCorrupted=Lähtefail on rikutud
SourceDoesntExist=Lähtefaili „%1“ pole olemas
SourceVerificationFailed=Lähtefaili kontrollimine ebaõnnestus: %1
VerificationSignatureDoesntExist=Allkirjafaili „%1“ pole olemas
VerificationSignatureInvalid=Allkirjafail „%1“ on kehtetu
VerificationKeyNotFound=Allkirjafail „%1“ kasutab tundmatut võtit
VerificationFileNameIncorrect=Faili nimi on vale
VerificationFileTagIncorrect=Faili silt on vale
VerificationFileSizeIncorrect=Faili suurus on vale
VerificationFileHashIncorrect=Faili räsi on vale
ExistingFileReadOnly2=Olemasolevat faili ei saanud asendada, sest see on märgitud kirjutuskaitstuks.
ExistingFileReadOnlyRetry=Eemalda kirjutuskaitse ja proovi uuesti
ExistingFileReadOnlyKeepExisting=Jäta olemasolev fail alles
ErrorReadingExistingDest=Olemasoleva faili lugemisel tekkis viga:
FileExistsSelectAction=Valige toiming
FileExists2=Fail on juba olemas.
FileExistsOverwriteExisting=Kirjuta olemasolev fail üle
FileExistsKeepExisting=Jäta olemasolev fail alles
FileExistsOverwriteOrKeepAll=Tee nii ka järgmiste konfliktide puhul
ExistingFileNewerSelectAction=Valige toiming
ExistingFileNewer2=Olemasolev fail on uuem kui see, mida paigaldaja proovib paigaldada.
ExistingFileNewerOverwriteExisting=Kirjuta olemasolev fail üle
ExistingFileNewerKeepExisting=Jäta olemasolev fail alles (soovitatav)
ExistingFileNewerOverwriteOrKeepAll=Tee nii ka järgmiste konfliktide puhul
ErrorChangingAttr=Olemasoleva faili atribuutide muutmisel tekkis viga:
ErrorCreatingTemp=Sihtkausta faili loomisel tekkis viga:
ErrorReadingSource=Lähtefaili lugemisel tekkis viga:
ErrorCopying=Faili kopeerimisel tekkis viga:
ErrorDownloading=Faili allalaadimisel tekkis viga:
ErrorExtracting=Arhiivi lahtipakkimisel tekkis viga:
ErrorReplacingExistingFile=Olemasoleva faili asendamisel tekkis viga:
ErrorRestartReplace=RestartReplace ebaõnnestus:
ErrorRenamingTemp=Sihtkausta faili ümbernimetamisel tekkis viga:
ErrorRegisterServer=DLL/OCX-i registreerimine ebaõnnestus: %1
ErrorRegSvr32Failed=RegSvr32 ebaõnnestus väljumiskoodiga %1
ErrorRegisterTypeLib=Tüübiteegi registreerimine ebaõnnestus: %1

; *** Uninstall display name markings
; used for example as 'My Program (32-bit)'
UninstallDisplayNameMark=%1 (%2)
; used for example as 'My Program (32-bit, All users)'
UninstallDisplayNameMarks=%1 (%2, %3)
UninstallDisplayNameMark32Bit=32-bitine
UninstallDisplayNameMark64Bit=64-bitine
UninstallDisplayNameMarkAllUsers=Kõik kasutajad
UninstallDisplayNameMarkCurrentUser=Praegune kasutaja

; *** Post-installation errors
ErrorOpeningReadme=README-faili avamisel tekkis viga.
ErrorRestartingComputer=Paigaldaja ei suutnud arvutit taaskäivitada. Tehke seda käsitsi.

; *** Uninstaller messages
UninstallNotFound=Faili „%1“ pole olemas. Eemaldamine pole võimalik.
UninstallOpenError=Faili „%1“ ei saanud avada. Eemaldamine pole võimalik
UninstallUnsupportedVer=Eemaldamislogi fail „%1“ on vormingus, mida see eemaldaja versioon ei tunne. Eemaldamine pole võimalik
UninstallUnknownEntry=Eemaldamislogis leiti tundmatu kirje (%1)
ConfirmUninstall=Kas olete kindel, et soovite programmi %1 ja kõik selle komponendid täielikult eemaldada?
UninstallOnlyOnWin64=Seda paigaldust saab eemaldada ainult 64-bitises Windowsis.
OnlyAdminCanUninstall=Seda paigaldust saab eemaldada ainult administraatoriõigustega kasutaja.
UninstallStatusLabel=Oodake, kuni programm %1 teie arvutist eemaldatakse.
UninstalledAll=Programm %1 eemaldati teie arvutist edukalt.
UninstalledMost=Programmi %1 eemaldamine on lõpetatud.%n%nMõnda elementi ei saanud eemaldada. Need saab käsitsi eemaldada.
UninstalledAndNeedsRestart=Programmi %1 eemaldamise lõpuleviimiseks tuleb arvuti taaskäivitada.%n%nKas soovite arvuti kohe taaskäivitada?
UninstallDataCorrupted=Fail „%1“ on rikutud. Eemaldamine pole võimalik

; *** Uninstallation phase messages
ConfirmDeleteSharedFileTitle=Kas eemaldada jagatud fail?
ConfirmDeleteSharedFile2=Süsteem teatab, et järgmist jagatud faili ei kasuta enam ükski programm. Kas soovite, et eemaldaja selle jagatud faili eemaldaks?%n%nKui mõni programm seda faili siiski kasutab ja see eemaldatakse, ei pruugi need programmid korralikult töötada. Kui te pole kindel, valige „Ei“. Faili süsteemi jätmine ei tee kahju.
SharedFileNameLabel=Faili nimi:
SharedFileLocationLabel=Asukoht:
WizardUninstalling=Eemaldamise olek
StatusUninstalling=Programmi %1 eemaldamine...

; *** Shutdown block reasons
ShutdownBlockReasonInstallingApp=Programmi %1 paigaldamine.
ShutdownBlockReasonUninstallingApp=Programmi %1 eemaldamine.

; The custom messages below aren't used by Setup itself, but if you make
; use of them in your scripts, you'll want to translate them.

[CustomMessages]

NameAndVersion=%1 versioon %2
AdditionalIcons=Lisaotseteed:
CreateDesktopIcon=Loo töölauaotsetee
CreateQuickLaunchIcon=Loo kiirkäivituse otsetee
ProgramOnTheWeb=%1 veebis
UninstallProgram=Eemalda %1
LaunchProgram=Käivita %1
AssocFileExtension=Seosta programm %1 faililaiendiga %2
AssocingFileExtension=Programmi %1 seostamine faililaiendiga %2...
AutoStartProgramGroupDescription=Käivitus:
AutoStartProgram=Käivita %1 automaatselt
AddonHostProgramNotFound=Programmi %1 ei leitud valitud kaustast.%n%nKas soovite sellegipoolest jätkata?

; The messages that are Lang Switcher's own (the sample is English.isl).
LocalCertReady=Paigaldaja loob selles arvutis sertifikaadi ja allkirjastab sellega programmi Lang Switcher, et see töötaks ka administraatorina käivitatud programmide akendes.
LocalCertFailed=Programm Lang Switcher paigaldati, kuid see ei tööta administraatorina käivitatud programmide akendes: Windows ei võtnud selles arvutis loodud sertifikaati vastu. Üksikasjad on paigaldamise logis.
RemoveSettingsQuestion=Kas eemaldada ka programmi Lang Switcher sätete fail?
RemoveSettingsKeep=Valige „Ei“, et säilitada sätted hilisemaks uuesti paigaldamiseks.
AuthorSlogan=südamega tehtud programmid
