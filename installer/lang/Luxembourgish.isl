; Lang Switcher -- the messages of the installer in Luxembourgish (stage E109, group B).
;
; Inno Setup has no official Luxembourgish file, so this file is whole: [LangOptions], all the messages of
; compiler:Default.isl (version 6.5.0 sample) translated, then the stock [CustomMessages] of that
; file and the messages of the installer that are its own (the sample is English.isl). Read as the
; only MessagesFile of the "luxembourgish" entry of [Languages] in ..\LangSwitcher.iss; Default.isl may
; stand before it: what is missing here falls back to English.
; No accelerators (&): a file without an official translation has none, so that one window never
; shows two equal letters (decision of the E108 trial, RULES 16). Messages that end without a
; full stop in Default.isl end without one here: Setup adds a Latin full stop itself. The names of
; the buttons inside a message stand in the quotes of the language. The name of the program is
; not translated; the installer calls itself "Setup", as the Dutch and German official files do.
; The form of address is the polite "Dir" (the plural imperative), as in the .rc of the language.
; AuthorSlogan (stage E99, decision 167.1): the slogan of the signature of the author at the
; bottom of every page of the wizard -- after "Panda Koder" and a middle dot; light, without
; pathos. The name of the author and the address of the site stay as they are in every language.

[LangOptions]
LanguageName=Lëtzebuergesch
LanguageID=$046E
LanguageCodePage=0
;DialogFontName=
;DialogFontSize=9
;DialogFontBaseScaleWidth=7
;DialogFontBaseScaleHeight=15
;WelcomeFontName=Segoe UI
;WelcomeFontSize=14

[Messages]

; *** Application titles
SetupAppTitle=Setup
SetupWindowTitle=Setup - %1
UninstallAppTitle=Desinstalléieren
UninstallAppFullTitle=%1 desinstalléieren

; *** Misc. common
InformationTitle=Informatioun
ConfirmTitle=Bestätegung
ErrorTitle=Feeler

; *** SetupLdr messages
SetupLdrStartupMessage=Dëst installéiert %1. Wëllt Dir weiderfueren?
LdrCannotCreateTemp=Eng temporär Datei konnt net erstellt ginn. Setup gouf ofgebrach
LdrCannotExecTemp=D'Datei am temporären Dossier konnt net ausgefouert ginn. Setup gouf ofgebrach
HelpTextNote=

; *** Startup error messages
LastErrorMessage=%1.%n%nFeeler %2: %3
SetupFileMissing=D'Datei %1 feelt am Installatiounsdossier. Léist de Problem oder kritt eng nei Kopie vum Programm.
SetupFileCorrupt=D'Setup-Dateie si beschiedegt. Kritt eng nei Kopie vum Programm.
SetupFileCorruptOrWrongVer=D'Setup-Dateie si beschiedegt oder net mat dëser Versioun vu Setup kompatibel. Léist de Problem oder kritt eng nei Kopie vum Programm.
InvalidParameter=An der Kommandozeil gouf en ongültege Parameter iwwerginn:%n%n%1
SetupAlreadyRunning=Setup leeft schonn.
WindowsVersionNotSupported=Dëse Programm ënnerstëtzt d'Windows-Versioun net, déi op Ärem Computer leeft.
WindowsServicePackRequired=Dëse Programm brauch %1 Service Pack %2 oder méi nei.
NotOnThisPlatform=Dëse Programm leeft net ënner %1.
OnlyOnThisPlatform=Dëse Programm muss ënner %1 lafen.
OnlyOnTheseArchitectures=Dëse Programm kann nëmmen op Windows-Versioune installéiert ginn, déi fir folgend Prozessorarchitekturen entwéckelt goufen:%n%n%1
WinVersionTooLowError=Dëse Programm brauch %1 Versioun %2 oder méi nei.
WinVersionTooHighError=Dëse Programm kann net op %1 Versioun %2 oder méi nei installéiert ginn.
AdminPrivilegesRequired=Dir musst als Administrateur ugemellt sinn, wann Dir dëse Programm installéiert.
PowerUserPrivilegesRequired=Dir musst als Administrateur oder als Member vun der Grupp „Power Users“ ugemellt sinn, wann Dir dëse Programm installéiert.
SetupAppRunningError=Setup huet festgestallt, datt %1 de Moment leeft.%n%nMaacht elo all Instanze dovunner zou a klickt dann op „OK“, fir weiderzefueren, oder op „Ofbriechen“, fir ze verloossen.
UninstallAppRunningError=Den Desinstallateur huet festgestallt, datt %1 de Moment leeft.%n%nMaacht elo all Instanze dovunner zou a klickt dann op „OK“, fir weiderzefueren, oder op „Ofbriechen“, fir ze verloossen.

; *** Startup questions
PrivilegesRequiredOverrideTitle=Installatiounsmodus fir Setup auswielen
PrivilegesRequiredOverrideInstruction=Installatiounsmodus auswielen
PrivilegesRequiredOverrideText1=%1 ka fir all Benotzer (Administratorrechter néideg) oder just fir Iech installéiert ginn.
PrivilegesRequiredOverrideText2=%1 ka just fir Iech oder fir all Benotzer (Administratorrechter néideg) installéiert ginn.
PrivilegesRequiredOverrideAllUsers=Fir all Benotzer installéieren
PrivilegesRequiredOverrideAllUsersRecommended=Fir all Benotzer installéieren (unzeroden)
PrivilegesRequiredOverrideCurrentUser=Just fir mech installéieren
PrivilegesRequiredOverrideCurrentUserRecommended=Just fir mech installéieren (unzeroden)

; *** Misc. errors
ErrorCreatingDir=Setup konnt den Dossier „%1“ net erstellen
ErrorTooManyFilesInDir=Et konnt keng Datei am Dossier „%1“ erstellt ginn, well en ze vill Dateien enthält

; *** Setup common messages
ExitSetupTitle=Setup verloossen
ExitSetupMessage=Setup ass net komplett. Wann Dir elo verloosst, gëtt de Programm net installéiert.%n%nDir kënnt Setup spéider nach eng Kéier starten, fir d'Installatioun ofzeschléissen.%n%nSetup verloossen?
AboutSetupMenuItem=Iwwer Setup...
AboutSetupTitle=Iwwer Setup
AboutSetupMessage=%1 Versioun %2%n%3%n%n%1 Websäit:%n%4
AboutSetupNote=
TranslatorNote=

; *** Buttons
ButtonBack=< Zréck
ButtonNext=Weider >
ButtonInstall=Installéieren
ButtonOK=OK
ButtonCancel=Ofbriechen
ButtonYes=Jo
ButtonYesToAll=Jo fir all
ButtonNo=Neen
ButtonNoToAll=Nee fir all
ButtonFinish=Ofschléissen
ButtonBrowse=Duerchsichen...
ButtonWizardBrowse=Duerchsichen...
ButtonNewFolder=Neien Dossier erstellen

; *** "Select Language" dialog messages
SelectLanguageTitle=Sprooch vum Setup auswielen
SelectLanguageLabel=Wielt d'Sprooch fir d'Installatioun aus.

; *** Common wizard text
ClickNext=Klickt op „Weider“, fir weiderzefueren, oder op „Ofbriechen“, fir Setup ze verloossen.
BeveledLabel=
BrowseDialogTitle=Dossier sichen
BrowseDialogLabel=Wielt an der Lëscht hei ënnendrënner en Dossier aus a klickt dann op „OK“.
NewFolderName=Neien Dossier

; *** "Welcome" wizard page
WelcomeLabel1=Wëllkomm beim Setup-Assistent vu [name]
WelcomeLabel2=Dëst installéiert [name/ver] op Ärem Computer.%n%nEt ass unzeroden, all aner Programmer zouzemaachen, ier Dir weiderfuert.

; *** "Password" wizard page
WizardPassword=Passwuert
PasswordLabel1=Dës Installatioun ass mat engem Passwuert geschützt.
PasswordLabel3=Gitt d'Passwuert an a klickt dann op „Weider“, fir weiderzefueren. Tëscht Groussbuschtawen a Klengbuschtawe gëtt ënnerscheet.
PasswordEditLabel=Passwuert:
IncorrectPassword=D'Passwuert, dat Dir aginn hutt, ass net korrekt. Probéiert et nach eng Kéier.

; *** "License Agreement" wizard page
WizardLicense=Lizenzvertrag
LicenseLabel=Liest folgend wichteg Informatiounen, ier Dir weiderfuert.
LicenseLabel3=Liest de folgende Lizenzvertrag. Dir musst d'Bedéngunge vun dësem Vertrag akzeptéieren, ier Dir mat der Installatioun weiderfuert.
LicenseAccepted=Ech akzeptéieren de Vertrag
LicenseNotAccepted=Ech akzeptéieren de Vertrag net

; *** "Information" wizard pages
WizardInfoBefore=Informatioun
InfoBeforeLabel=Liest folgend wichteg Informatiounen, ier Dir weiderfuert.
InfoBeforeClickLabel=Wann Dir prett sidd, fir mat Setup weiderzefueren, klickt op „Weider“.
WizardInfoAfter=Informatioun
InfoAfterLabel=Liest folgend wichteg Informatiounen, ier Dir weiderfuert.
InfoAfterClickLabel=Wann Dir prett sidd, fir mat Setup weiderzefueren, klickt op „Weider“.

; *** "User Information" wizard page
WizardUserInfo=Benotzerinformatioun
UserInfoDesc=Gitt Är Informatiounen an.
UserInfoName=Benotzernumm:
UserInfoOrg=Organisatioun:
UserInfoSerial=Seriennummer:
UserInfoNameRequired=Dir musst en Numm aginn.

; *** "Select Destination Location" wizard page
WizardSelectDir=Zildossier auswielen
SelectDirDesc=Wou soll [name] installéiert ginn?
SelectDirLabel3=Setup installéiert [name] an de folgenden Dossier.
SelectDirBrowseLabel=Fir weiderzefueren, klickt op „Weider“. Wann Dir en aneren Dossier wiele wëllt, klickt op „Duerchsichen“.
DiskSpaceGBLabel=Op d'mannst [gb] GB fräi Späicherplaz ass néideg.
DiskSpaceMBLabel=Op d'mannst [mb] MB fräi Späicherplaz ass néideg.
CannotInstallToNetworkDrive=Setup kann net op en Netzwierklaufwierk installéieren.
CannotInstallToUNCPath=Setup kann net op en UNC-Pad installéieren.
InvalidPath=Dir musst e komplette Pad mat Laufwierksbuschtaf aginn; zum Beispill:%n%nC:\APP%n%noder en UNC-Pad an dëser Form:%n%n\\server\share
InvalidDrive=Dat gewielt Laufwierk oder den UNC-Share existéiert net oder ass net accessibel. Wielt eppes anescht aus.
DiskSpaceWarningTitle=Net genuch Späicherplaz
DiskSpaceWarning=Setup brauch op d'mannst %1 KB fräi Späicherplaz fir d'Installatioun, mä um gewielte Laufwierk sinn nëmmen %2 KB disponibel.%n%nWëllt Dir trotzdem weiderfueren?
DirNameTooLong=Den Numm vum Dossier oder de Pad ass ze laang.
InvalidDirName=Den Numm vum Dossier ass net gëlteg.
BadDirName32=Dossiernimm däerfen déi folgend Zeechen net enthalen:%n%n%1
DirExistsTitle=Dossier existéiert
DirExists=Den Dossier:%n%n%1%n%nexistéiert schonn. Wëllt Dir trotzdem an dësen Dossier installéieren?
DirDoesntExistTitle=Dossier existéiert net
DirDoesntExist=Den Dossier:%n%n%1%n%nexistéiert net. Soll den Dossier erstellt ginn?

; *** "Select Components" wizard page
WizardSelectComponents=Komponenten auswielen
SelectComponentsDesc=Wéi eng Komponente sollen installéiert ginn?
SelectComponentsLabel2=Wielt d'Komponenten aus, déi Dir installéiere wëllt; huelt d'Hake bei deenen ewech, déi Dir net installéiere wëllt. Klickt op „Weider“, wann Dir prett sidd.
FullInstallation=Voll Installatioun
; if possible don't translate 'Compact' as 'Minimal' (I mean 'Minimal' in your language)
CompactInstallation=Kompakt Installatioun
CustomInstallation=Benotzerdefinéiert Installatioun
NoUninstallWarningTitle=Komponente si schonn do
NoUninstallWarning=Setup huet festgestallt, datt folgend Komponente schonn op Ärem Computer installéiert sinn:%n%n%1%n%nWann Dir dës Komponenten ofwielt, gi se net desinstalléiert.%n%nWëllt Dir trotzdem weiderfueren?
ComponentSize1=%1 KB
ComponentSize2=%1 MB
ComponentsDiskSpaceGBLabel=Déi aktuell Auswiel brauch op d'mannst [gb] GB Späicherplaz.
ComponentsDiskSpaceMBLabel=Déi aktuell Auswiel brauch op d'mannst [mb] MB Späicherplaz.

; *** "Select Additional Tasks" wizard page
WizardSelectTasks=Zousätzlech Aufgaben auswielen
SelectTasksDesc=Wéi eng zousätzlech Aufgabe sollen duerchgefouert ginn?
SelectTasksLabel2=Wielt déi zousätzlech Aufgaben aus, déi Setup wärend der Installatioun vu [name] duerchféiere soll, a klickt dann op „Weider“.

; *** "Select Start Menu Folder" wizard page
WizardSelectProgramGroup=Startmenü-Dossier auswielen
SelectStartMenuFolderDesc=Wou soll Setup d'Ofkierzunge vum Programm uleeën?
SelectStartMenuFolderLabel3=Setup leet d'Ofkierzunge vum Programm am folgende Startmenü-Dossier un.
SelectStartMenuFolderBrowseLabel=Fir weiderzefueren, klickt op „Weider“. Wann Dir en aneren Dossier wiele wëllt, klickt op „Duerchsichen“.
MustEnterGroupName=Dir musst en Dossiernumm aginn.
GroupNameTooLong=Den Numm vum Dossier oder de Pad ass ze laang.
InvalidGroupName=Den Numm vum Dossier ass net gëlteg.
BadGroupName=Den Dossiernumm däerf keent vun de folgende Zeechen enthalen:%n%n%1
NoProgramGroupCheck2=Kee Startmenü-Dossier erstellen

; *** "Ready to Install" wizard page
WizardReady=Prett fir d'Installatioun
ReadyLabel1=Setup ass elo prett, fir [name] op Ärem Computer ze installéieren.
ReadyLabel2a=Klickt op „Installéieren“, fir mat der Installatioun weiderzefueren, oder op „Zréck“, wann Dir Astellunge kontrolléiere oder ännere wëllt.
ReadyLabel2b=Klickt op „Installéieren“, fir mat der Installatioun weiderzefueren.
ReadyMemoUserInfo=Benotzerinformatioun:
ReadyMemoDir=Zilplaz:
ReadyMemoType=Installatiounstyp:
ReadyMemoComponents=Gewielt Komponenten:
ReadyMemoGroup=Startmenü-Dossier:
ReadyMemoTasks=Zousätzlech Aufgaben:

; *** TDownloadWizardPage wizard page and DownloadTemporaryFile
DownloadingLabel2=Dateie ginn erofgelueden...
ButtonStopDownload=Download ofbriechen
StopDownload=Sidd Dir sécher, datt Dir den Download ofbrieche wëllt?
ErrorDownloadAborted=Download ofgebrach
ErrorDownloadFailed=Download feelgeschloen: %1 %2
ErrorDownloadSizeFailed=D'Gréisst konnt net ofgefrot ginn: %1 %2
ErrorProgress=Ongültege Fortschrëtt: %1 vun %2
ErrorFileSize=Ongülteg Dateigréisst: %1 erwaart, %2 fonnt

; *** TExtractionWizardPage wizard page and ExtractArchive
ExtractingLabel=Dateie ginn extrahéiert...
ButtonStopExtraction=Extraktioun ofbriechen
StopExtraction=Sidd Dir sécher, datt Dir d'Extraktioun ofbrieche wëllt?
ErrorExtractionAborted=Extraktioun ofgebrach
ErrorExtractionFailed=Extraktioun feelgeschloen: %1

; *** Archive extraction failure details
ArchiveIncorrectPassword=D'Passwuert ass falsch
ArchiveIsCorrupted=D'Archiv ass beschiedegt
ArchiveUnsupportedFormat=D'Archivformat gëtt net ënnerstëtzt

; *** "Preparing to Install" wizard page
WizardPreparing=Installatioun gëtt virbereet
PreparingDesc=Setup bereet d'Installatioun vu [name] op Ärem Computer vir.
PreviousInstallNotCompleted=D'Installatioun oder d'Ewechhuele vun engem fréiere Programm gouf net ofgeschloss. Dir musst Äre Computer nei starten, fir déi Installatioun ofzeschléissen.%n%nNodeems Dir Äre Computer nei gestart hutt, start Setup nach eng Kéier, fir d'Installatioun vu [name] ofzeschléissen.
CannotContinue=Setup kann net weiderfueren. Klickt op „Ofbriechen“, fir ze verloossen.
ApplicationsFound=Folgend Programmer benotze Dateien, déi vu Setup aktualiséiert musse ginn. Et ass unzeroden, datt Setup dës Programmer automatesch zoumaache däerf.
ApplicationsFound2=Folgend Programmer benotze Dateien, déi vu Setup aktualiséiert musse ginn. Et ass unzeroden, datt Setup dës Programmer automatesch zoumaache däerf. Nodeems d'Installatioun ofgeschloss ass, probéiert Setup d'Programmer nei ze starten.
CloseApplications=Programmer automatesch zoumaachen
DontCloseApplications=Programmer net zoumaachen
ErrorCloseApplications=Setup konnt net all Programmer automatesch zoumaachen. Et ass unzeroden, all Programmer zouzemaachen, déi Dateie benotzen, déi vu Setup aktualiséiert musse ginn, ier Dir weiderfuert.
PrepareToInstallNeedsRestart=Setup muss Äre Computer nei starten. Nodeems Dir Äre Computer nei gestart hutt, start Setup nach eng Kéier, fir d'Installatioun vu [name] ofzeschléissen.%n%nWëllt Dir elo nei starten?

; *** "Installing" wizard page
WizardInstalling=Installéieren
InstallingLabel=Waart, wärend Setup [name] op Ärem Computer installéiert.

; *** "Setup Completed" wizard page
FinishedHeadingLabel=Setup-Assistent vu [name] ofschléissen
FinishedLabelNoIcons=Setup huet d'Installatioun vu [name] op Ärem Computer ofgeschloss.
FinishedLabel=Setup huet d'Installatioun vu [name] op Ärem Computer ofgeschloss. Dir kënnt de Programm iwwer déi installéiert Ofkierzunge starten.
ClickFinish=Klickt op „Ofschléissen“, fir Setup ze verloossen.
FinishedRestartLabel=Fir d'Installatioun vu [name] ofzeschléissen, muss Setup Äre Computer nei starten. Wëllt Dir elo nei starten?
FinishedRestartMessage=Fir d'Installatioun vu [name] ofzeschléissen, muss Setup Äre Computer nei starten.%n%nWëllt Dir elo nei starten?
ShowReadmeCheck=Jo, ech wëll d'README-Datei kucken
YesRadio=Jo, de Computer elo nei starten
NoRadio=Neen, ech starte de Computer méi spéit nei
; used for example as 'Run MyProg.exe'
RunEntryExec=%1 starten
; used for example as 'View Readme.txt'
RunEntryShellExec=%1 kucken

; *** "Setup Needs the Next Disk" stuff
ChangeDiskTitle=Setup brauch déi nächst Disk
SelectDiskLabel2=Leet Disk %1 an a klickt op „OK“.%n%nWann d'Dateie vun dëser Disk an engem aneren Dossier ze fanne sinn, wéi deen hei ënnendrënner, gitt de richtege Pad an oder klickt op „Duerchsichen“.
PathLabel=Pad:
FileNotInDir2=D'Datei „%1“ konnt net a „%2“ fonnt ginn. Leet déi richteg Disk an oder wielt en aneren Dossier.
SelectDirectoryLabel=Gitt d'Plaz vun der nächster Disk un.

; *** Installation phase messages
SetupAborted=Setup gouf net ofgeschloss.%n%nLéist de Problem a start Setup nach eng Kéier.
AbortRetryIgnoreSelectAction=Aktioun auswielen
AbortRetryIgnoreRetry=Nach eng Kéier probéieren
AbortRetryIgnoreIgnore=De Feeler ignoréieren a weiderfueren
AbortRetryIgnoreCancel=Installatioun ofbriechen
RetryCancelSelectAction=Aktioun auswielen
RetryCancelRetry=Nach eng Kéier probéieren
RetryCancelCancel=Ofbriechen

; *** Installation status messages
StatusClosingApplications=Programmer ginn zougemaach...
StatusCreateDirs=Dossiere ginn erstellt...
StatusExtractFiles=Dateie ginn extrahéiert...
StatusDownloadFiles=Dateie ginn erofgelueden...
StatusCreateIcons=Ofkierzunge ginn erstellt...
StatusCreateIniEntries=INI-Aschreiwunge ginn erstellt...
StatusCreateRegistryEntries=Registry-Aschreiwunge ginn erstellt...
StatusRegisterFiles=Dateie gi registréiert...
StatusSavingUninstall=Desinstallatiounsinformatioune gi gespäichert...
StatusRunProgram=Installatioun gëtt ofgeschloss...
StatusRestartingApplications=Programmer ginn nei gestart...
StatusRollback=Ännerunge ginn zréckgesat...

; *** Misc. errors
ErrorInternal2=Interne Feeler: %1
ErrorFunctionFailedNoCode=%1 ass feelgeschloen
ErrorFunctionFailed=%1 ass feelgeschloen; Code %2
ErrorFunctionFailedWithMessage=%1 ass feelgeschloen; Code %2.%n%3
ErrorExecutingProgram=D'Datei konnt net ausgefouert ginn:%n%1

; *** Registry errors
ErrorRegOpenKey=Feeler beim Opmaache vum Registry-Schlëssel:%n%1\%2
ErrorRegCreateKey=Feeler beim Erstelle vum Registry-Schlëssel:%n%1\%2
ErrorRegWriteKey=Feeler beim Schreiwe vum Registry-Schlëssel:%n%1\%2

; *** INI errors
ErrorIniEntry=Feeler beim Erstelle vun enger INI-Aschreiwung an der Datei „%1“.

; *** File copying errors
FileAbortRetryIgnoreSkipNotRecommended=Dës Datei iwwersprangen (net unzeroden)
FileAbortRetryIgnoreIgnoreNotRecommended=De Feeler ignoréieren a weiderfueren (net unzeroden)
SourceIsCorrupted=D'Quelldatei ass beschiedegt
SourceDoesntExist=D'Quelldatei „%1“ gëtt et net
SourceVerificationFailed=D'Verifikatioun vun der Quelldatei ass feelgeschloen: %1
VerificationSignatureDoesntExist=D'Ënnerschrëftdatei „%1“ gëtt et net
VerificationSignatureInvalid=D'Ënnerschrëftdatei „%1“ ass ongülteg
VerificationKeyNotFound=D'Ënnerschrëftdatei „%1“ benotzt en onbekannte Schlëssel
VerificationFileNameIncorrect=Den Numm vun der Datei ass falsch
VerificationFileTagIncorrect=D'Tag vun der Datei ass falsch
VerificationFileSizeIncorrect=D'Gréisst vun der Datei ass falsch
VerificationFileHashIncorrect=Den Hash vun der Datei ass falsch
ExistingFileReadOnly2=Déi existent Datei konnt net ersat ginn, well se als schreifgeschützt markéiert ass.
ExistingFileReadOnlyRetry=Schreifschutz ewechhuelen an nach eng Kéier probéieren
ExistingFileReadOnlyKeepExisting=Déi existent Datei behalen
ErrorReadingExistingDest=Beim Liese vun der existenter Datei ass e Feeler opgetrueden:
FileExistsSelectAction=Aktioun auswielen
FileExists2=D'Datei gëtt et schonn.
FileExistsOverwriteExisting=Déi existent Datei iwwerschreiwen
FileExistsKeepExisting=Déi existent Datei behalen
FileExistsOverwriteOrKeepAll=Dat bei den nächste Konflikter och maachen
ExistingFileNewerSelectAction=Aktioun auswielen
ExistingFileNewer2=Déi existent Datei ass méi nei wéi déi, déi Setup probéiert ze installéieren.
ExistingFileNewerOverwriteExisting=Déi existent Datei iwwerschreiwen
ExistingFileNewerKeepExisting=Déi existent Datei behalen (unzeroden)
ExistingFileNewerOverwriteOrKeepAll=Dat bei den nächste Konflikter och maachen
ErrorChangingAttr=Beim Ännere vun den Attributer vun der existenter Datei ass e Feeler opgetrueden:
ErrorCreatingTemp=Beim Erstelle vun enger Datei am Zildossier ass e Feeler opgetrueden:
ErrorReadingSource=Beim Liese vun der Quelldatei ass e Feeler opgetrueden:
ErrorCopying=Beim Kopéiere vun enger Datei ass e Feeler opgetrueden:
ErrorDownloading=Beim Erofluede vun enger Datei ass e Feeler opgetrueden:
ErrorExtracting=Beim Extrahéiere vun engem Archiv ass e Feeler opgetrueden:
ErrorReplacingExistingFile=Beim Ersetze vun der existenter Datei ass e Feeler opgetrueden:
ErrorRestartReplace=RestartReplace ass feelgeschloen:
ErrorRenamingTemp=Beim Ëmbenenne vun enger Datei am Zildossier ass e Feeler opgetrueden:
ErrorRegisterServer=D'DLL/OCX konnt net registréiert ginn: %1
ErrorRegSvr32Failed=RegSvr32 ass mat Exitcode %1 feelgeschloen
ErrorRegisterTypeLib=D'Typbibliothéik konnt net registréiert ginn: %1

; *** Uninstall display name markings
; used for example as 'My Program (32-bit)'
UninstallDisplayNameMark=%1 (%2)
; used for example as 'My Program (32-bit, All users)'
UninstallDisplayNameMarks=%1 (%2, %3)
UninstallDisplayNameMark32Bit=32-Bit
UninstallDisplayNameMark64Bit=64-Bit
UninstallDisplayNameMarkAllUsers=All Benotzer
UninstallDisplayNameMarkCurrentUser=Aktuelle Benotzer

; *** Post-installation errors
ErrorOpeningReadme=Beim Opmaache vun der README-Datei ass e Feeler opgetrueden.
ErrorRestartingComputer=Setup konnt de Computer net nei starten. Maacht dat manuell.

; *** Uninstaller messages
UninstallNotFound=D'Datei „%1“ gëtt et net. Desinstalléieren net méiglech.
UninstallOpenError=D'Datei „%1“ konnt net opgemaach ginn. Desinstalléieren net méiglech
UninstallUnsupportedVer=D'Desinstallatiounsprotokolldatei „%1“ huet e Format, deen dës Versioun vum Desinstallateur net erkennt. Desinstalléieren net méiglech
UninstallUnknownEntry=Am Desinstallatiounsprotokoll gouf eng onbekannt Aschreiwung (%1) fonnt
ConfirmUninstall=Sidd Dir sécher, datt Dir %1 an all seng Komponente komplett ewechhuele wëllt?
UninstallOnlyOnWin64=Dës Installatioun kann nëmmen op 64-Bit-Windows desinstalléiert ginn.
OnlyAdminCanUninstall=Dës Installatioun kann nëmme vun engem Benotzer mat Administratorrechter desinstalléiert ginn.
UninstallStatusLabel=Waart, wärend %1 vun Ärem Computer ewechgeholl gëtt.
UninstalledAll=%1 gouf erfollegräich vun Ärem Computer ewechgeholl.
UninstalledMost=D'Desinstalléiere vu %1 ass ofgeschloss.%n%nE puer Elementer konnten net ewechgeholl ginn. Dir kënnt se manuell ewechhuelen.
UninstalledAndNeedsRestart=Fir d'Desinstalléiere vu %1 ofzeschléissen, muss Äre Computer nei gestart ginn.%n%nWëllt Dir elo nei starten?
UninstallDataCorrupted=D'Datei „%1“ ass beschiedegt. Desinstalléieren net méiglech

; *** Uninstallation phase messages
ConfirmDeleteSharedFileTitle=Gedeelt Datei ewechhuelen?
ConfirmDeleteSharedFile2=D'System weist un, datt folgend gedeelt Datei vu kengem Programm méi benotzt gëtt. Soll den Desinstallateur dës gedeelt Datei ewechhuelen?%n%nWa Programmer dës Datei nach benotzen a se ewechgeholl gëtt, funktionéieren dës Programmer vläicht net méi richteg. Wann Dir Iech net sécher sidd, wielt „Neen“. D'Datei um System ze loossen, schued net.
SharedFileNameLabel=Dateinumm:
SharedFileLocationLabel=Plaz:
WizardUninstalling=Desinstallatiounszoustand
StatusUninstalling=%1 gëtt desinstalléiert...

; *** Shutdown block reasons
ShutdownBlockReasonInstallingApp=%1 gëtt installéiert.
ShutdownBlockReasonUninstallingApp=%1 gëtt desinstalléiert.

; The custom messages below aren't used by Setup itself, but if you make
; use of them in your scripts, you'll want to translate them.

[CustomMessages]

NameAndVersion=%1 Versioun %2
AdditionalIcons=Zousätzlech Ofkierzungen:
CreateDesktopIcon=Eng Ofkierzung um Desktop erstellen
CreateQuickLaunchIcon=Eng Ofkierzung an der Schnellstartleescht erstellen
ProgramOnTheWeb=%1 um Web
UninstallProgram=%1 desinstalléieren
LaunchProgram=%1 starten
AssocFileExtension=%1 mat der Dateierweiderung %2 verknäppen
AssocingFileExtension=%1 gëtt mat der Dateierweiderung %2 verknäppt...
AutoStartProgramGroupDescription=Autostart:
AutoStartProgram=%1 automatesch starten
AddonHostProgramNotFound=%1 konnt am gewielten Dossier net fonnt ginn.%n%nWëllt Dir trotzdem weiderfueren?

LocalCertReady=Setup erstellt op dësem Computer en Zertifikat an ënnerschreift Lang Switcher domat, fir datt de Programm och a Fënstere vu Programmer funktionéiert, déi als Administrateur lafen.
LocalCertFailed=Lang Switcher gouf installéiert, funktionéiert awer net a Fënstere vu Programmer, déi als Administrateur lafen: Windows huet d'Zertifikat, dat op dësem Computer erstellt gouf, net akzeptéiert. Détailer stinn am Installatiounsjournal.
RemoveSettingsQuestion=Och d'Astellungsdatei vu Lang Switcher ewechhuelen?
RemoveSettingsKeep=Wielt „Neen“, fir d'Astellunge fir eng spéider Neiinstallatioun ze behalen.
AuthorSlogan=Programmer mat Häerz gemaach
