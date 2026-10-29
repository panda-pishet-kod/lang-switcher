; Lang Switcher -- the messages of the installer in Norwegian Nynorsk (stage E109, group B).
;
; Inno Setup has no official Nynorsk translation (inno_official = false in res\lang\languages.toml;
; the official Norwegian.isl is Bokmål), so this file carries the whole of it: [LangOptions], every
; message of the compiler:Default.isl (6.5.0+), and the messages of the installer that are Lang
; Switcher's own. It is read after compiler:Default.isl as the second MessagesFile of the
; "norwegiannynorsk" entry of [Languages] in ..\LangSwitcher.iss, and so replaces the English text
; name for name. English.isl is the sample the own messages translate; the glossary of the language
; is l10n\glossary\nn.tsv of the control contour. The name of the program is not translated.
; The name of a button inside a message is in the quotation marks of the language and is the word
; of the button here; no & accelerators in the file (RULES 16).
; As in Default.isl: a message that has no full stop there has none here (Setup adds its own).
; The forms are Nynorsk (ikkje, eg, kva, -ar in the present), not Bokmål with other endings.

[LangOptions]
LanguageName=Norsk nynorsk
LanguageID=$0814
LanguageCodePage=1252

[Messages]

; *** Application titles
SetupAppTitle=Installasjon
SetupWindowTitle=Installasjon – %1
UninstallAppTitle=Avinstallasjon
UninstallAppFullTitle=Avinstallasjon av %1

; *** Misc. common
InformationTitle=Informasjon
ConfirmTitle=Stadfest
ErrorTitle=Feil

; *** SetupLdr messages
SetupLdrStartupMessage=Dette installerer %1. Vil du halde fram?
LdrCannotCreateTemp=Kunne ikkje opprette ei mellombels fil. Installasjonen er avbroten
LdrCannotExecTemp=Kunne ikkje køyre fila i den mellombelse mappa. Installasjonen er avbroten
HelpTextNote=

; *** Startup error messages
LastErrorMessage=%1.%n%nFeil %2: %3
SetupFileMissing=Fila %1 manglar i installasjonsmappa. Rett opp problemet eller skaff ein ny kopi av programmet.
SetupFileCorrupt=Installasjonsfilene er skadde. Skaff ein ny kopi av programmet.
SetupFileCorruptOrWrongVer=Installasjonsfilene er skadde, eller dei passar ikkje til denne versjonen av installasjonsprogrammet. Rett opp problemet eller skaff ein ny kopi av programmet.
InvalidParameter=Kommandolinja inneheldt ein ugyldig parameter:%n%n%1
SetupAlreadyRunning=Installasjonsprogrammet køyrer allereie.
WindowsVersionNotSupported=Dette programmet støttar ikkje Windows-versjonen som er på datamaskinen din.
WindowsServicePackRequired=Dette programmet krev %1 Service Pack %2 eller nyare.
NotOnThisPlatform=Dette programmet køyrer ikkje på %1.
OnlyOnThisPlatform=Dette programmet må køyrast på %1.
OnlyOnTheseArchitectures=Dette programmet kan berre installerast på Windows-versjonar som er laga for desse prosessorarkitekturane:%n%n%1
WinVersionTooLowError=Dette programmet krev %1 versjon %2 eller nyare.
WinVersionTooHighError=Dette programmet kan ikkje installerast på %1 versjon %2 eller nyare.
AdminPrivilegesRequired=Du må vere logga på som administrator for å installere dette programmet.
PowerUserPrivilegesRequired=Du må vere logga på som administrator eller som medlem av gruppa Power Users for å installere dette programmet.
SetupAppRunningError=Installasjonsprogrammet har oppdaga at %1 køyrer no.%n%nLukk alle førekomstane av det, og klikk deretter «OK» for å halde fram, eller «Avbryt» for å avslutte.
UninstallAppRunningError=Avinstallasjonsprogrammet har oppdaga at %1 køyrer no.%n%nLukk alle førekomstane av det, og klikk deretter «OK» for å halde fram, eller «Avbryt» for å avslutte.

; *** Startup questions
PrivilegesRequiredOverrideTitle=Vel installasjonsmodus
PrivilegesRequiredOverrideInstruction=Vel installasjonsmodus
PrivilegesRequiredOverrideText1=%1 kan installerast for alle brukarar (krev administratorrettar) eller berre for deg.
PrivilegesRequiredOverrideText2=%1 kan installerast berre for deg eller for alle brukarar (krev administratorrettar).
PrivilegesRequiredOverrideAllUsers=Installer for alle brukarar
PrivilegesRequiredOverrideAllUsersRecommended=Installer for alle brukarar (tilrådd)
PrivilegesRequiredOverrideCurrentUser=Installer berre for meg
PrivilegesRequiredOverrideCurrentUserRecommended=Installer berre for meg (tilrådd)

; *** Misc. errors
ErrorCreatingDir=Installasjonsprogrammet kunne ikkje opprette mappa «%1»
ErrorTooManyFilesInDir=Kunne ikkje opprette ei fil i mappa «%1» fordi ho inneheld for mange filer

; *** Setup common messages
ExitSetupTitle=Avslutt installasjonen
ExitSetupMessage=Installasjonen er ikkje fullført. Viss du avsluttar no, blir ikkje programmet installert.%n%nDu kan køyre installasjonsprogrammet på nytt seinare for å fullføre installasjonen.%n%nVil du avslutte installasjonen?
AboutSetupMenuItem=Om installasjonsprogrammet...
AboutSetupTitle=Om installasjonsprogrammet
AboutSetupMessage=%1 versjon %2%n%3%n%n%1 heimeside:%n%4
AboutSetupNote=
TranslatorNote=

; *** Buttons
ButtonBack=< Tilbake
ButtonNext=Neste >
ButtonInstall=Installer
ButtonOK=OK
ButtonCancel=Avbryt
ButtonYes=Ja
ButtonYesToAll=Ja til alle
ButtonNo=Nei
ButtonNoToAll=Nei til alle
ButtonFinish=Fullfør
ButtonBrowse=Bla gjennom...
ButtonWizardBrowse=Bla gjennom...
ButtonNewFolder=Opprett ny mappe

; *** "Select Language" dialog messages
SelectLanguageTitle=Vel språk for installasjonen
SelectLanguageLabel=Vel språket som skal brukast under installasjonen.

; *** Common wizard text
ClickNext=Klikk «Neste» for å halde fram, eller «Avbryt» for å avslutte installasjonen.
BeveledLabel=
BrowseDialogTitle=Bla etter mappe
BrowseDialogLabel=Vel ei mappe i lista nedanfor, og klikk deretter «OK».
NewFolderName=Ny mappe

; *** "Welcome" wizard page
WelcomeLabel1=Velkomen til installasjonsprogrammet for [name]
WelcomeLabel2=Dette installerer [name/ver] på datamaskinen din.%n%nDet blir tilrådd at du lukkar alle andre program før du held fram.

; *** "Password" wizard page
WizardPassword=Passord
PasswordLabel1=Denne installasjonen er verna med passord.
PasswordLabel3=Skriv inn passordet, og klikk deretter «Neste» for å halde fram. Passord skil mellom store og små bokstavar.
PasswordEditLabel=Passord:
IncorrectPassword=Passordet du skreiv inn, er feil. Prøv igjen.

; *** "License Agreement" wizard page
WizardLicense=Lisensavtale
LicenseLabel=Les denne viktige informasjonen før du held fram.
LicenseLabel3=Les lisensavtalen nedanfor. Du må godta vilkåra i avtalen før du held fram med installasjonen.
LicenseAccepted=Eg godtek avtalen
LicenseNotAccepted=Eg godtek ikkje avtalen

; *** "Information" wizard pages
WizardInfoBefore=Informasjon
InfoBeforeLabel=Les denne viktige informasjonen før du held fram.
InfoBeforeClickLabel=Når du er klar til å halde fram med installasjonen, klikkar du «Neste».
WizardInfoAfter=Informasjon
InfoAfterLabel=Les denne viktige informasjonen før du held fram.
InfoAfterClickLabel=Når du er klar til å halde fram med installasjonen, klikkar du «Neste».

; *** "User Information" wizard page
WizardUserInfo=Brukarinformasjon
UserInfoDesc=Skriv inn opplysningane dine.
UserInfoName=Brukarnamn:
UserInfoOrg=Organisasjon:
UserInfoSerial=Serienummer:
UserInfoNameRequired=Du må skrive inn eit namn.

; *** "Select Destination Location" wizard page
WizardSelectDir=Vel målmappe
SelectDirDesc=Kvar skal [name] installerast?
SelectDirLabel3=Installasjonsprogrammet installerer [name] i denne mappa.
SelectDirBrowseLabel=Klikk «Neste» for å halde fram. Vil du velje ei anna mappe, klikkar du «Bla gjennom».
DiskSpaceGBLabel=Det trengst minst [gb] GB ledig diskplass.
DiskSpaceMBLabel=Det trengst minst [mb] MB ledig diskplass.
CannotInstallToNetworkDrive=Installasjonsprogrammet kan ikkje installere på ein nettverksstasjon.
CannotInstallToUNCPath=Installasjonsprogrammet kan ikkje installere på ein UNC-sti.
InvalidPath=Du må skrive inn ein full sti med stasjonsbokstav, til dømes:%n%nC:\APP%n%neller ein UNC-sti på forma:%n%n\\server\share
InvalidDrive=Stasjonen eller UNC-delinga du valde, finst ikkje eller er ikkje tilgjengeleg. Vel ein annan.
DiskSpaceWarningTitle=For lite diskplass
DiskSpaceWarning=Installasjonen krev minst %1 KB ledig plass, men den valde stasjonen har berre %2 KB ledig.%n%nVil du halde fram likevel?
DirNameTooLong=Mappenamnet eller stien er for langt.
InvalidDirName=Mappenamnet er ugyldig.
BadDirName32=Mappenamn kan ikkje innehalde nokon av desse teikna:%n%n%1
DirExistsTitle=Mappa finst
DirExists=Mappa:%n%n%1%n%nfinst allereie. Vil du installere i den mappa likevel?
DirDoesntExistTitle=Mappa finst ikkje
DirDoesntExist=Mappa:%n%n%1%n%nfinst ikkje. Vil du at mappa skal opprettast?

; *** "Select Components" wizard page
WizardSelectComponents=Vel komponentar
SelectComponentsDesc=Kva komponentar skal installerast?
SelectComponentsLabel2=Vel komponentane du vil installere, og fjern merket for dei du ikkje vil installere. Klikk «Neste» når du er klar til å halde fram.
FullInstallation=Full installasjon
; if possible don't translate 'Compact' as 'Minimal' (I mean 'Minimal' in your language)
CompactInstallation=Kompakt installasjon
CustomInstallation=Tilpassa installasjon
NoUninstallWarningTitle=Komponentane finst
NoUninstallWarning=Installasjonsprogrammet har oppdaga at desse komponentane alt er installerte på datamaskinen din:%n%n%1%n%nÅ fjerne merket for desse komponentane avinstallerer dei ikkje.%n%nVil du halde fram likevel?
ComponentSize1=%1 KB
ComponentSize2=%1 MB
ComponentsDiskSpaceGBLabel=Det gjeldande valet krev minst [gb] GB diskplass.
ComponentsDiskSpaceMBLabel=Det gjeldande valet krev minst [mb] MB diskplass.

; *** "Select Additional Tasks" wizard page
WizardSelectTasks=Vel tilleggsoppgåver
SelectTasksDesc=Kva tilleggsoppgåver skal utførast?
SelectTasksLabel2=Vel tilleggsoppgåvene du vil at installasjonsprogrammet skal utføre medan [name] blir installert, og klikk deretter «Neste».

; *** "Select Start Menu Folder" wizard page
WizardSelectProgramGroup=Vel mappe i startmenyen
SelectStartMenuFolderDesc=Kvar skal installasjonsprogrammet leggje snarvegane til programmet?
SelectStartMenuFolderLabel3=Installasjonsprogrammet opprettar snarvegane til programmet i denne mappa i startmenyen.
SelectStartMenuFolderBrowseLabel=Klikk «Neste» for å halde fram. Vil du velje ei anna mappe, klikkar du «Bla gjennom».
MustEnterGroupName=Du må skrive inn eit mappenamn.
GroupNameTooLong=Mappenamnet eller stien er for langt.
InvalidGroupName=Mappenamnet er ugyldig.
BadGroupName=Mappenamnet kan ikkje innehalde nokon av desse teikna:%n%n%1
NoProgramGroupCheck2=Ikkje opprett mappe i startmenyen

; *** "Ready to Install" wizard page
WizardReady=Klar til å installere
ReadyLabel1=Installasjonsprogrammet er no klart til å installere [name] på datamaskinen din.
ReadyLabel2a=Klikk «Installer» for å halde fram med installasjonen, eller «Tilbake» viss du vil sjå over eller endre innstillingar.
ReadyLabel2b=Klikk «Installer» for å halde fram med installasjonen.
ReadyMemoUserInfo=Brukarinformasjon:
ReadyMemoDir=Målmappe:
ReadyMemoType=Installasjonstype:
ReadyMemoComponents=Valde komponentar:
ReadyMemoGroup=Mappe i startmenyen:
ReadyMemoTasks=Tilleggsoppgåver:

; *** TDownloadWizardPage wizard page and DownloadTemporaryFile
DownloadingLabel2=Lastar ned filer...
ButtonStopDownload=Stopp nedlastinga
StopDownload=Er du sikker på at du vil stoppe nedlastinga?
ErrorDownloadAborted=Nedlastinga vart avbroten
ErrorDownloadFailed=Nedlastinga mislukkast: %1 %2
ErrorDownloadSizeFailed=Kunne ikkje hente storleiken: %1 %2
ErrorProgress=Ugyldig framdrift: %1 av %2
ErrorFileSize=Ugyldig filstorleik: venta %1, fann %2

; *** TExtractionWizardPage wizard page and ExtractArchive
ExtractingLabel=Pakkar ut filer...
ButtonStopExtraction=Stopp utpakkinga
StopExtraction=Er du sikker på at du vil stoppe utpakkinga?
ErrorExtractionAborted=Utpakkinga vart avbroten
ErrorExtractionFailed=Utpakkinga mislukkast: %1

; *** Archive extraction failure details
ArchiveIncorrectPassword=Passordet er feil
ArchiveIsCorrupted=Arkivet er skadd
ArchiveUnsupportedFormat=Arkivformatet er ikkje støtta

; *** "Preparing to Install" wizard page
WizardPreparing=Førebur installasjonen
PreparingDesc=Installasjonsprogrammet førebur installasjonen av [name] på datamaskinen din.
PreviousInstallNotCompleted=Installasjonen eller fjerninga av eit tidlegare program vart ikkje fullført. Du må starte datamaskinen på nytt for å fullføre han.%n%nEtter omstarten køyrer du installasjonsprogrammet på nytt for å fullføre installasjonen av [name].
CannotContinue=Installasjonsprogrammet kan ikkje halde fram. Klikk «Avbryt» for å avslutte.
ApplicationsFound=Desse programma bruker filer som installasjonsprogrammet må oppdatere. Det blir tilrådd at installasjonsprogrammet får lukke dei automatisk.
ApplicationsFound2=Desse programma bruker filer som installasjonsprogrammet må oppdatere. Det blir tilrådd at installasjonsprogrammet får lukke dei automatisk. Når installasjonen er ferdig, prøver installasjonsprogrammet å starte programma på nytt.
CloseApplications=Lukk programma automatisk
DontCloseApplications=Ikkje lukk programma
ErrorCloseApplications=Installasjonsprogrammet kunne ikkje lukke alle programma automatisk. Det blir tilrådd at du lukkar alle program som bruker filer installasjonsprogrammet må oppdatere, før du held fram.
PrepareToInstallNeedsRestart=Installasjonsprogrammet må starte datamaskinen på nytt. Etter omstarten køyrer du installasjonsprogrammet på nytt for å fullføre installasjonen av [name].%n%nVil du starte på nytt no?

; *** "Installing" wizard page
WizardInstalling=Installerer
InstallingLabel=Vent medan installasjonsprogrammet installerer [name] på datamaskinen din.

; *** "Setup Completed" wizard page
FinishedHeadingLabel=Fullfører installasjonsprogrammet for [name]
FinishedLabelNoIcons=Installasjonsprogrammet er ferdig med å installere [name] på datamaskinen din.
FinishedLabel=Installasjonsprogrammet er ferdig med å installere [name] på datamaskinen din. Du kan starte programmet frå snarvegane som er oppretta.
ClickFinish=Klikk «Fullfør» for å avslutte installasjonsprogrammet.
FinishedRestartLabel=For å fullføre installasjonen av [name] må installasjonsprogrammet starte datamaskinen på nytt. Vil du starte på nytt no?
FinishedRestartMessage=For å fullføre installasjonen av [name] må installasjonsprogrammet starte datamaskinen på nytt.%n%nVil du starte på nytt no?
ShowReadmeCheck=Ja, eg vil sjå LESMEG-fila
YesRadio=Ja, start datamaskinen på nytt no
NoRadio=Nei, eg startar datamaskinen på nytt seinare
; used for example as 'Run MyProg.exe'
RunEntryExec=Køyr %1
; used for example as 'View Readme.txt'
RunEntryShellExec=Vis %1

; *** "Setup Needs the Next Disk" stuff
ChangeDiskTitle=Installasjonsprogrammet treng neste disk
SelectDiskLabel2=Set inn disk %1 og klikk «OK».%n%nViss filene på denne disken finst i ei anna mappe enn den som er vist nedanfor, skriv du inn rett sti eller klikkar «Bla gjennom».
PathLabel=Sti:
FileNotInDir2=Fila «%1» vart ikkje funnen i «%2». Set inn rett disk eller vel ei anna mappe.
SelectDirectoryLabel=Oppgi kvar den neste disken finst.

; *** Installation phase messages
SetupAborted=Installasjonen vart ikkje fullført.%n%nRett opp problemet og køyr installasjonsprogrammet på nytt.
AbortRetryIgnoreSelectAction=Vel handling
AbortRetryIgnoreRetry=Prøv igjen
AbortRetryIgnoreIgnore=Ignorer feilen og hald fram
AbortRetryIgnoreCancel=Avbryt installasjonen
RetryCancelSelectAction=Vel handling
RetryCancelRetry=Prøv igjen
RetryCancelCancel=Avbryt

; *** Installation status messages
StatusClosingApplications=Lukkar program...
StatusCreateDirs=Opprettar mapper...
StatusExtractFiles=Pakkar ut filer...
StatusDownloadFiles=Lastar ned filer...
StatusCreateIcons=Opprettar snarvegar...
StatusCreateIniEntries=Opprettar INI-oppføringar...
StatusCreateRegistryEntries=Opprettar registeroppføringar...
StatusRegisterFiles=Registrerer filer...
StatusSavingUninstall=Lagrar informasjon om avinstallasjon...
StatusRunProgram=Fullfører installasjonen...
StatusRestartingApplications=Startar program på nytt...
StatusRollback=Rullar tilbake endringar...

; *** Misc. errors
ErrorInternal2=Intern feil: %1
ErrorFunctionFailedNoCode=%1 mislukkast
ErrorFunctionFailed=%1 mislukkast; kode %2
ErrorFunctionFailedWithMessage=%1 mislukkast; kode %2.%n%3
ErrorExecutingProgram=Kunne ikkje køyre fila:%n%1

; *** Registry errors
ErrorRegOpenKey=Feil ved opning av registernøkkel:%n%1\%2
ErrorRegCreateKey=Feil ved oppretting av registernøkkel:%n%1\%2
ErrorRegWriteKey=Feil ved skriving til registernøkkel:%n%1\%2

; *** INI errors
ErrorIniEntry=Feil ved oppretting av INI-oppføring i fila «%1».

; *** File copying errors
FileAbortRetryIgnoreSkipNotRecommended=Hopp over denne fila (ikkje tilrådd)
FileAbortRetryIgnoreIgnoreNotRecommended=Ignorer feilen og hald fram (ikkje tilrådd)
SourceIsCorrupted=Kjeldefila er skadd
SourceDoesntExist=Kjeldefila «%1» finst ikkje
SourceVerificationFailed=Kontrollen av kjeldefila mislukkast: %1
VerificationSignatureDoesntExist=Signaturfila «%1» finst ikkje
VerificationSignatureInvalid=Signaturfila «%1» er ugyldig
VerificationKeyNotFound=Signaturfila «%1» bruker ein ukjend nøkkel
VerificationFileNameIncorrect=Filnamnet er feil
VerificationFileTagIncorrect=Merket på fila er feil
VerificationFileSizeIncorrect=Filstorleiken er feil
VerificationFileHashIncorrect=Hash-verdien til fila er feil
ExistingFileReadOnly2=Den eksisterande fila kunne ikkje erstattast fordi ho er skriveverna.
ExistingFileReadOnlyRetry=Fjern skrivevernet og prøv igjen
ExistingFileReadOnlyKeepExisting=Behald den eksisterande fila
ErrorReadingExistingDest=Det oppstod ein feil under lesing av den eksisterande fila:
FileExistsSelectAction=Vel handling
FileExists2=Fila finst allereie.
FileExistsOverwriteExisting=Overskriv den eksisterande fila
FileExistsKeepExisting=Behald den eksisterande fila
FileExistsOverwriteOrKeepAll=Gjer det same for dei neste konfliktane
ExistingFileNewerSelectAction=Vel handling
ExistingFileNewer2=Den eksisterande fila er nyare enn fila installasjonsprogrammet prøver å installere.
ExistingFileNewerOverwriteExisting=Overskriv den eksisterande fila
ExistingFileNewerKeepExisting=Behald den eksisterande fila (tilrådd)
ExistingFileNewerOverwriteOrKeepAll=Gjer det same for dei neste konfliktane
ErrorChangingAttr=Det oppstod ein feil under endring av eigenskapane til den eksisterande fila:
ErrorCreatingTemp=Det oppstod ein feil under oppretting av ei fil i målmappa:
ErrorReadingSource=Det oppstod ein feil under lesing av kjeldefila:
ErrorCopying=Det oppstod ein feil under kopiering av ei fil:
ErrorDownloading=Det oppstod ein feil under nedlasting av ei fil:
ErrorExtracting=Det oppstod ein feil under utpakking av eit arkiv:
ErrorReplacingExistingFile=Det oppstod ein feil under erstatting av den eksisterande fila:
ErrorRestartReplace=RestartReplace mislukkast:
ErrorRenamingTemp=Det oppstod ein feil under endring av namnet på ei fil i målmappa:
ErrorRegisterServer=Kunne ikkje registrere DLL/OCX: %1
ErrorRegSvr32Failed=RegSvr32 mislukkast med avslutningskode %1
ErrorRegisterTypeLib=Kunne ikkje registrere typebiblioteket: %1

; *** Uninstall display name markings
; used for example as 'My Program (32-bit)'
UninstallDisplayNameMark=%1 (%2)
; used for example as 'My Program (32-bit, All users)'
UninstallDisplayNameMarks=%1 (%2, %3)
UninstallDisplayNameMark32Bit=32-bit
UninstallDisplayNameMark64Bit=64-bit
UninstallDisplayNameMarkAllUsers=Alle brukarar
UninstallDisplayNameMarkCurrentUser=Gjeldande brukar

; *** Post-installation errors
ErrorOpeningReadme=Det oppstod ein feil under opning av LESMEG-fila.
ErrorRestartingComputer=Installasjonsprogrammet kunne ikkje starte datamaskinen på nytt. Gjer dette manuelt.

; *** Uninstaller messages
UninstallNotFound=Fila «%1» finst ikkje. Kan ikkje avinstallere.
UninstallOpenError=Fila «%1» kunne ikkje opnast. Kan ikkje avinstallere
UninstallUnsupportedVer=Avinstallasjonsloggen «%1» har eit format som denne versjonen av avinstallasjonsprogrammet ikkje kjenner att. Kan ikkje avinstallere
UninstallUnknownEntry=Det vart funne ei ukjend oppføring (%1) i avinstallasjonsloggen
ConfirmUninstall=Er du sikker på at du vil fjerne %1 og alle komponentane fullstendig?
UninstallOnlyOnWin64=Denne installasjonen kan berre avinstallerast på 64-bit Windows.
OnlyAdminCanUninstall=Denne installasjonen kan berre avinstallerast av ein brukar med administratorrettar.
UninstallStatusLabel=Vent medan %1 blir fjerna frå datamaskinen din.
UninstalledAll=%1 vart fjerna frå datamaskinen din.
UninstalledMost=Avinstallasjonen av %1 er ferdig.%n%nNokre element kunne ikkje fjernast. Dei kan fjernast manuelt.
UninstalledAndNeedsRestart=For å fullføre avinstallasjonen av %1 må datamaskinen startast på nytt.%n%nVil du starte på nytt no?
UninstallDataCorrupted=Fila «%1» er skadd. Kan ikkje avinstallere

; *** Uninstallation phase messages
ConfirmDeleteSharedFileTitle=Fjerne delt fil?
ConfirmDeleteSharedFile2=Systemet melder at den delte fila nedanfor ikkje lenger blir brukt av noko program. Vil du at avinstallasjonsprogrammet skal fjerne ho?%n%nViss program framleis bruker fila og ho blir fjerna, er det ikkje sikkert at dei verkar som dei skal. Er du usikker, vel «Nei». Å la fila liggje på systemet gjer ingen skade.
SharedFileNameLabel=Filnamn:
SharedFileLocationLabel=Plassering:
WizardUninstalling=Status for avinstallasjonen
StatusUninstalling=Avinstallerer %1...

; *** Shutdown block reasons
ShutdownBlockReasonInstallingApp=Installerer %1.
ShutdownBlockReasonUninstallingApp=Avinstallerer %1.

; The custom messages below aren't used by Setup itself, but if you make
; use of them in your scripts, you'll want to translate them.

[CustomMessages]

NameAndVersion=%1 versjon %2
AdditionalIcons=Ekstra snarvegar:
CreateDesktopIcon=Opprett ein snarveg på skrivebordet
CreateQuickLaunchIcon=Opprett ein snarveg i Hurtigstart
ProgramOnTheWeb=%1 på nettet
UninstallProgram=Avinstaller %1
LaunchProgram=Start %1
AssocFileExtension=Knyt %1 til filtypen %2
AssocingFileExtension=Knyter %1 til filtypen %2...
AutoStartProgramGroupDescription=Oppstart:
AutoStartProgram=Start %1 automatisk
AddonHostProgramNotFound=%1 vart ikkje funnen i mappa du valde.%n%nVil du halde fram likevel?

; The messages that are Lang Switcher's own (the sample is English.isl).
LocalCertReady=Installasjonsprogrammet opprettar eit sertifikat på denne datamaskinen og signerer Lang Switcher med det, slik at programmet òg verkar i vindauga til program som køyrer som administrator.
LocalCertFailed=Lang Switcher er installert, men verkar ikkje i vindauga til program som køyrer som administrator: Windows godtok ikkje sertifikatet som vart oppretta på denne datamaskinen. Sjå installasjonsloggen for detaljar.
RemoveSettingsQuestion=Vil du òg fjerne innstillingsfila til Lang Switcher?
RemoveSettingsKeep=Vel «Nei» for å behalde innstillingane til ein seinare ny installasjon.
AuthorSlogan=program laga med hjarte
