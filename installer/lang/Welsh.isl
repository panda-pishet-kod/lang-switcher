; Lang Switcher -- the messages of the installer in Welsh (stage E109, group B).
;
; Inno Setup has no official Welsh file, so this file is whole: [LangOptions], all the messages of
; compiler:Default.isl (version 6.5.0 sample) translated, then the stock [CustomMessages] of that
; file and the messages of the installer that are its own (the sample is English.isl). Read as the
; only MessagesFile of the "welsh" entry of [Languages] in ..\LangSwitcher.iss; Default.isl may
; stand before it: what is missing here falls back to English.
; No accelerators (&): a file without an official translation has none, so that one window never
; shows two equal letters (decision of the E108 trial, RULES 16). Messages that end without a
; full stop in Default.isl end without one here: Setup adds a Latin full stop itself. The names of
; the buttons inside a message stand in the quotes of the language. The name of the program is
; not translated.
; AuthorSlogan (stage E99, decision 167.1): the slogan of the signature of the author at the
; bottom of every page of the wizard -- after "Panda Koder" and a middle dot; light, without
; pathos. The name of the author and the address of the site stay as they are in every language.

[LangOptions]
LanguageName=Cymraeg
LanguageID=$0452
LanguageCodePage=0
;DialogFontName=
;DialogFontSize=9
;DialogFontBaseScaleWidth=7
;DialogFontBaseScaleHeight=15
;WelcomeFontName=Segoe UI
;WelcomeFontSize=14

[Messages]

; *** Application titles
SetupAppTitle=Gosod
SetupWindowTitle=Gosod - %1
UninstallAppTitle=Dadosod
UninstallAppFullTitle=Dadosod %1

; *** Misc. common
InformationTitle=Gwybodaeth
ConfirmTitle=Cadarnhau
ErrorTitle=Gwall

; *** SetupLdr messages
SetupLdrStartupMessage=Bydd hyn yn gosod %1. Hoffech chi barhau?
LdrCannotCreateTemp=Methu creu ffeil dros dro. Rhoddwyd y gorau i'r gosod
LdrCannotExecTemp=Methu gweithredu ffeil yn y ffolder dros dro. Rhoddwyd y gorau i'r gosod
HelpTextNote=

; *** Startup error messages
LastErrorMessage=%1.%n%nGwall %2: %3
SetupFileMissing=Mae'r ffeil %1 ar goll o'r ffolder gosod. Cywirwch y broblem neu sicrhewch gopi newydd o'r rhaglen.
SetupFileCorrupt=Mae'r ffeiliau gosod wedi'u llygru. Sicrhewch gopi newydd o'r rhaglen.
SetupFileCorruptOrWrongVer=Mae'r ffeiliau gosod wedi'u llygru, neu nid ydynt yn gydnaws â'r fersiwn hon o'r gosodwr. Cywirwch y broblem neu sicrhewch gopi newydd o'r rhaglen.
InvalidParameter=Pasiwyd paramedr annilys ar y llinell orchymyn:%n%n%1
SetupAlreadyRunning=Mae'r gosodwr eisoes yn rhedeg.
WindowsVersionNotSupported=Nid yw'r rhaglen hon yn cefnogi'r fersiwn o Windows sy'n rhedeg ar eich cyfrifiadur.
WindowsServicePackRequired=Mae'r rhaglen hon angen %1 Service Pack %2 neu'n hwyrach.
NotOnThisPlatform=Ni fydd y rhaglen hon yn rhedeg ar %1.
OnlyOnThisPlatform=Rhaid rhedeg y rhaglen hon ar %1.
OnlyOnTheseArchitectures=Dim ond ar fersiynau o Windows a ddyluniwyd ar gyfer y pensaernïaethau prosesydd canlynol y gellir gosod y rhaglen hon:%n%n%1
WinVersionTooLowError=Mae'r rhaglen hon angen %1 fersiwn %2 neu'n hwyrach.
WinVersionTooHighError=Ni ellir gosod y rhaglen hon ar %1 fersiwn %2 neu'n hwyrach.
AdminPrivilegesRequired=Rhaid i chi fod wedi mewngofnodi fel gweinyddwr wrth osod y rhaglen hon.
PowerUserPrivilegesRequired=Rhaid i chi fod wedi mewngofnodi fel gweinyddwr neu fel aelod o'r grŵp Power Users wrth osod y rhaglen hon.
SetupAppRunningError=Mae'r gosodwr wedi canfod bod %1 yn rhedeg ar hyn o bryd.%n%nCaewch bob enghraifft ohoni nawr, yna cliciwch “Iawn” i barhau, neu “Canslo” i ymadael.
UninstallAppRunningError=Mae'r dadosodwr wedi canfod bod %1 yn rhedeg ar hyn o bryd.%n%nCaewch bob enghraifft ohoni nawr, yna cliciwch “Iawn” i barhau, neu “Canslo” i ymadael.

; *** Startup questions
PrivilegesRequiredOverrideTitle=Dewis modd gosod
PrivilegesRequiredOverrideInstruction=Dewiswch y modd gosod
PrivilegesRequiredOverrideText1=Gellir gosod %1 ar gyfer pob defnyddiwr (mae angen breintiau gweinyddol), neu i chi yn unig.
PrivilegesRequiredOverrideText2=Gellir gosod %1 i chi yn unig, neu ar gyfer pob defnyddiwr (mae angen breintiau gweinyddol).
PrivilegesRequiredOverrideAllUsers=Gosod ar gyfer pob defnyddiwr
PrivilegesRequiredOverrideAllUsersRecommended=Gosod ar gyfer pob defnyddiwr (argymhellir)
PrivilegesRequiredOverrideCurrentUser=Gosod i mi yn unig
PrivilegesRequiredOverrideCurrentUserRecommended=Gosod i mi yn unig (argymhellir)

; *** Misc. errors
ErrorCreatingDir=Methodd y gosodwr greu'r ffolder “%1”
ErrorTooManyFilesInDir=Methu creu ffeil yn y ffolder “%1” oherwydd ei fod yn cynnwys gormod o ffeiliau

; *** Setup common messages
ExitSetupTitle=Ymadael â'r gosodwr
ExitSetupMessage=Nid yw'r gosod wedi'i gwblhau. Os ymadewch nawr, ni fydd y rhaglen yn cael ei gosod.%n%nGallwch redeg y gosodwr eto rywbryd arall i gwblhau'r gosod.%n%nYmadael â'r gosodwr?
AboutSetupMenuItem=Ynghylch y gosodwr...
AboutSetupTitle=Ynghylch y gosodwr
AboutSetupMessage=%1 fersiwn %2%n%3%n%nCartref %1 ar y we:%n%4
AboutSetupNote=
TranslatorNote=

; *** Buttons
ButtonBack=< Yn ôl
ButtonNext=Nesaf >
ButtonInstall=Gosod
ButtonOK=Iawn
ButtonCancel=Canslo
ButtonYes=Ie
ButtonYesToAll=Ie i bawb
ButtonNo=Na
ButtonNoToAll=Na i bawb
ButtonFinish=Gorffen
ButtonBrowse=Pori...
ButtonWizardBrowse=Pori...
ButtonNewFolder=Gwneud ffolder newydd

; *** "Select Language" dialog messages
SelectLanguageTitle=Dewis iaith y gosodwr
SelectLanguageLabel=Dewiswch yr iaith i'w defnyddio yn ystod y gosod.

; *** Common wizard text
ClickNext=Cliciwch “Nesaf” i barhau, neu “Canslo” i ymadael â'r gosodwr.
BeveledLabel=
BrowseDialogTitle=Pori am ffolder
BrowseDialogLabel=Dewiswch ffolder yn y rhestr isod, yna cliciwch “Iawn”.
NewFolderName=Ffolder newydd

; *** "Welcome" wizard page
WelcomeLabel1=Croeso i Ddewin Gosod [name]
WelcomeLabel2=Bydd hyn yn gosod [name/ver] ar eich cyfrifiadur.%n%nArgymhellir eich bod yn cau pob rhaglen arall cyn parhau.

; *** "Password" wizard page
WizardPassword=Cyfrinair
PasswordLabel1=Mae'r gosod hwn wedi'i warchod â chyfrinair.
PasswordLabel3=Rhowch y cyfrinair, yna cliciwch “Nesaf” i barhau. Mae cyfrineiriau'n sensitif i briflythrennau.
PasswordEditLabel=Cyfrinair:
IncorrectPassword=Nid yw'r cyfrinair a roesoch yn gywir. Ceisiwch eto.

; *** "License Agreement" wizard page
WizardLicense=Cytundeb trwydded
LicenseLabel=Darllenwch y wybodaeth bwysig ganlynol cyn parhau.
LicenseLabel3=Darllenwch y Cytundeb Trwydded canlynol. Rhaid i chi dderbyn telerau'r cytundeb hwn cyn parhau â'r gosod.
LicenseAccepted=Rwy'n derbyn y cytundeb
LicenseNotAccepted=Nid wyf yn derbyn y cytundeb

; *** "Information" wizard pages
WizardInfoBefore=Gwybodaeth
InfoBeforeLabel=Darllenwch y wybodaeth bwysig ganlynol cyn parhau.
InfoBeforeClickLabel=Pan fyddwch yn barod i barhau â'r gosod, cliciwch “Nesaf”.
WizardInfoAfter=Gwybodaeth
InfoAfterLabel=Darllenwch y wybodaeth bwysig ganlynol cyn parhau.
InfoAfterClickLabel=Pan fyddwch yn barod i barhau â'r gosod, cliciwch “Nesaf”.

; *** "User Information" wizard page
WizardUserInfo=Gwybodaeth defnyddiwr
UserInfoDesc=Rhowch eich gwybodaeth.
UserInfoName=Enw'r defnyddiwr:
UserInfoOrg=Sefydliad:
UserInfoSerial=Rhif cyfresol:
UserInfoNameRequired=Rhaid i chi roi enw.

; *** "Select Destination Location" wizard page
WizardSelectDir=Dewis lleoliad y gosod
SelectDirDesc=Ymhle y dylid gosod [name]?
SelectDirLabel3=Bydd y gosodwr yn gosod [name] yn y ffolder canlynol.
SelectDirBrowseLabel=I barhau, cliciwch “Nesaf”. Os hoffech ddewis ffolder gwahanol, cliciwch “Pori”.
DiskSpaceGBLabel=Mae angen o leiaf [gb] GB o le rhydd ar y ddisg.
DiskSpaceMBLabel=Mae angen o leiaf [mb] MB o le rhydd ar y ddisg.
CannotInstallToNetworkDrive=Ni all y gosodwr osod i yriant rhwydwaith.
CannotInstallToUNCPath=Ni all y gosodwr osod i lwybr UNC.
InvalidPath=Rhaid i chi roi llwybr llawn gyda llythyren gyriant; er enghraifft:%n%nC:\APP%n%nneu lwybr UNC yn y ffurf:%n%n\\gweinydd\rhannu
InvalidDrive=Nid yw'r gyriant neu'r rhaniad UNC a ddewisoch yn bodoli neu nid oes modd cael mynediad ato. Dewiswch un arall.
DiskSpaceWarningTitle=Dim digon o le ar y ddisg
DiskSpaceWarning=Mae'r gosodwr angen o leiaf %1 KB o le rhydd i osod, ond dim ond %2 KB sydd ar gael ar y gyriant a ddewiswyd.%n%nHoffech chi barhau beth bynnag?
DirNameTooLong=Mae enw neu lwybr y ffolder yn rhy hir.
InvalidDirName=Nid yw enw'r ffolder yn ddilys.
BadDirName32=Ni all enwau ffolder gynnwys unrhyw un o'r nodau canlynol:%n%n%1
DirExistsTitle=Mae'r ffolder yn bodoli
DirExists=Mae'r ffolder:%n%n%1%n%neisoes yn bodoli. Hoffech chi osod yn y ffolder hwnnw beth bynnag?
DirDoesntExistTitle=Nid yw'r ffolder yn bodoli
DirDoesntExist=Nid yw'r ffolder:%n%n%1%n%nyn bodoli. Hoffech chi greu'r ffolder?

; *** "Select Components" wizard page
WizardSelectComponents=Dewis cydrannau
SelectComponentsDesc=Pa gydrannau ddylid eu gosod?
SelectComponentsLabel2=Dewiswch y cydrannau yr hoffech eu gosod; dad-ddewiswch y cydrannau nad ydych eu heisiau. Cliciwch “Nesaf” pan fyddwch yn barod i barhau.
FullInstallation=Gosod llawn
; if possible don't translate 'Compact' as 'Minimal' (I mean 'Minimal' in your language)
CompactInstallation=Gosod cryno
CustomInstallation=Gosod wedi'i deilwra
NoUninstallWarningTitle=Mae cydrannau'n bodoli
NoUninstallWarning=Mae'r gosodwr wedi canfod bod y cydrannau canlynol eisoes wedi'u gosod ar eich cyfrifiadur:%n%n%1%n%nNi fydd dad-ddewis y cydrannau hyn yn eu dadosod.%n%nHoffech chi barhau beth bynnag?
ComponentSize1=%1 KB
ComponentSize2=%1 MB
ComponentsDiskSpaceGBLabel=Mae'r dewis presennol angen o leiaf [gb] GB o le ar y ddisg.
ComponentsDiskSpaceMBLabel=Mae'r dewis presennol angen o leiaf [mb] MB o le ar y ddisg.

; *** "Select Additional Tasks" wizard page
WizardSelectTasks=Dewis tasgau ychwanegol
SelectTasksDesc=Pa dasgau ychwanegol ddylid eu cyflawni?
SelectTasksLabel2=Dewiswch y tasgau ychwanegol yr hoffech i'r gosodwr eu cyflawni wrth osod [name], yna cliciwch “Nesaf”.

; *** "Select Start Menu Folder" wizard page
WizardSelectProgramGroup=Dewis ffolder y ddewislen Cychwyn
SelectStartMenuFolderDesc=Ymhle y dylai'r gosodwr osod llwybrau byr y rhaglen?
SelectStartMenuFolderLabel3=Bydd y gosodwr yn creu llwybrau byr y rhaglen yn ffolder y ddewislen Cychwyn canlynol.
SelectStartMenuFolderBrowseLabel=I barhau, cliciwch “Nesaf”. Os hoffech ddewis ffolder gwahanol, cliciwch “Pori”.
MustEnterGroupName=Rhaid i chi roi enw ffolder.
GroupNameTooLong=Mae enw neu lwybr y ffolder yn rhy hir.
InvalidGroupName=Nid yw enw'r ffolder yn ddilys.
BadGroupName=Ni all enw'r ffolder gynnwys unrhyw un o'r nodau canlynol:%n%n%1
NoProgramGroupCheck2=Peidio â chreu ffolder yn y ddewislen Cychwyn

; *** "Ready to Install" wizard page
WizardReady=Barod i osod
ReadyLabel1=Mae'r gosodwr bellach yn barod i ddechrau gosod [name] ar eich cyfrifiadur.
ReadyLabel2a=Cliciwch “Gosod” i barhau â'r gosod, neu gliciwch “Yn ôl” os hoffech adolygu neu newid unrhyw osodiadau.
ReadyLabel2b=Cliciwch “Gosod” i barhau â'r gosod.
ReadyMemoUserInfo=Gwybodaeth defnyddiwr:
ReadyMemoDir=Lleoliad y gosod:
ReadyMemoType=Math o osod:
ReadyMemoComponents=Cydrannau a ddewiswyd:
ReadyMemoGroup=Ffolder y ddewislen Cychwyn:
ReadyMemoTasks=Tasgau ychwanegol:

; *** TDownloadWizardPage wizard page and DownloadTemporaryFile
DownloadingLabel2=Yn lawrlwytho ffeiliau...
ButtonStopDownload=Atal y lawrlwytho
StopDownload=Ydych chi'n siŵr eich bod am atal y lawrlwytho?
ErrorDownloadAborted=Rhoddwyd y gorau i'r lawrlwytho
ErrorDownloadFailed=Methodd y lawrlwytho: %1 %2
ErrorDownloadSizeFailed=Methodd cael y maint: %1 %2
ErrorProgress=Cynnydd annilys: %1 o %2
ErrorFileSize=Maint ffeil annilys: disgwyliwyd %1, cafwyd %2

; *** TExtractionWizardPage wizard page and ExtractArchive
ExtractingLabel=Yn echdynnu ffeiliau...
ButtonStopExtraction=Atal yr echdynnu
StopExtraction=Ydych chi'n siŵr eich bod am atal yr echdynnu?
ErrorExtractionAborted=Rhoddwyd y gorau i'r echdynnu
ErrorExtractionFailed=Methodd yr echdynnu: %1

; *** Archive extraction failure details
ArchiveIncorrectPassword=Mae'r cyfrinair yn anghywir
ArchiveIsCorrupted=Mae'r archif wedi'i llygru
ArchiveUnsupportedFormat=Nid yw fformat yr archif yn cael ei gefnogi

; *** "Preparing to Install" wizard page
WizardPreparing=Paratoi i osod
PreparingDesc=Mae'r gosodwr yn paratoi i osod [name] ar eich cyfrifiadur.
PreviousInstallNotCompleted=Ni chwblhawyd gosod/dileu rhaglen flaenorol. Bydd angen i chi ailgychwyn eich cyfrifiadur i gwblhau'r gosod hwnnw.%n%nAr ôl ailgychwyn eich cyfrifiadur, rhedwch y gosodwr eto i gwblhau gosod [name].
CannotContinue=Ni all y gosodwr barhau. Cliciwch “Canslo” i ymadael.
ApplicationsFound=Mae'r rhaglenni canlynol yn defnyddio ffeiliau y mae angen i'r gosodwr eu diweddaru. Argymhellir eich bod yn caniatáu i'r gosodwr gau'r rhaglenni hyn yn awtomatig.
ApplicationsFound2=Mae'r rhaglenni canlynol yn defnyddio ffeiliau y mae angen i'r gosodwr eu diweddaru. Argymhellir eich bod yn caniatáu i'r gosodwr gau'r rhaglenni hyn yn awtomatig. Ar ôl i'r gosod orffen, bydd y gosodwr yn ceisio ailddechrau'r rhaglenni.
CloseApplications=Cau'r rhaglenni yn awtomatig
DontCloseApplications=Peidio â chau'r rhaglenni
ErrorCloseApplications=Methodd y gosodwr gau pob rhaglen yn awtomatig. Argymhellir eich bod yn cau pob rhaglen sy'n defnyddio ffeiliau y mae angen i'r gosodwr eu diweddaru cyn parhau.
PrepareToInstallNeedsRestart=Rhaid i'r gosodwr ailgychwyn eich cyfrifiadur. Ar ôl ailgychwyn eich cyfrifiadur, rhedwch y gosodwr eto i gwblhau gosod [name].%n%nHoffech chi ailgychwyn nawr?

; *** "Installing" wizard page
WizardInstalling=Yn gosod
InstallingLabel=Arhoswch tra bo'r gosodwr yn gosod [name] ar eich cyfrifiadur.

; *** "Setup Completed" wizard page
FinishedHeadingLabel=Cwblhau Dewin Gosod [name]
FinishedLabelNoIcons=Mae'r gosodwr wedi gorffen gosod [name] ar eich cyfrifiadur.
FinishedLabel=Mae'r gosodwr wedi gorffen gosod [name] ar eich cyfrifiadur. Gellir lansio'r rhaglen drwy ddewis y llwybrau byr a osodwyd.
ClickFinish=Cliciwch “Gorffen” i ymadael â'r gosodwr.
FinishedRestartLabel=I gwblhau gosod [name], rhaid i'r gosodwr ailgychwyn eich cyfrifiadur. Hoffech chi ailgychwyn nawr?
FinishedRestartMessage=I gwblhau gosod [name], rhaid i'r gosodwr ailgychwyn eich cyfrifiadur.%n%nHoffech chi ailgychwyn nawr?
ShowReadmeCheck=Ie, hoffwn weld y ffeil README
YesRadio=Ie, ailgychwyn y cyfrifiadur nawr
NoRadio=Na, byddaf yn ailgychwyn y cyfrifiadur yn nes ymlaen
; used for example as 'Run MyProg.exe'
RunEntryExec=Rhedeg %1
; used for example as 'View Readme.txt'
RunEntryShellExec=Gweld %1

; *** "Setup Needs the Next Disk" stuff
ChangeDiskTitle=Mae'r gosodwr angen y ddisg nesaf
SelectDiskLabel2=Rhowch Ddisg %1 i mewn a chliciwch “Iawn”.%n%nOs gellir dod o hyd i'r ffeiliau ar y ddisg hon mewn ffolder heblaw'r un a ddangosir isod, rhowch y llwybr cywir neu gliciwch “Pori”.
PathLabel=Llwybr:
FileNotInDir2=Ni ellid dod o hyd i'r ffeil “%1” yn “%2”. Rhowch y ddisg gywir i mewn neu ddewiswch ffolder arall.
SelectDirectoryLabel=Nodwch leoliad y ddisg nesaf.

; *** Installation phase messages
SetupAborted=Ni chwblhawyd y gosod.%n%nCywirwch y broblem a rhedwch y gosodwr eto.
AbortRetryIgnoreSelectAction=Dewis gweithred
AbortRetryIgnoreRetry=Ceisio eto
AbortRetryIgnoreIgnore=Anwybyddu'r gwall a pharhau
AbortRetryIgnoreCancel=Canslo'r gosod
RetryCancelSelectAction=Dewis gweithred
RetryCancelRetry=Ceisio eto
RetryCancelCancel=Canslo

; *** Installation status messages
StatusClosingApplications=Yn cau rhaglenni...
StatusCreateDirs=Yn creu ffolderi...
StatusExtractFiles=Yn echdynnu ffeiliau...
StatusDownloadFiles=Yn lawrlwytho ffeiliau...
StatusCreateIcons=Yn creu llwybrau byr...
StatusCreateIniEntries=Yn creu cofnodion INI...
StatusCreateRegistryEntries=Yn creu cofnodion cofrestrfa...
StatusRegisterFiles=Yn cofrestru ffeiliau...
StatusSavingUninstall=Yn cadw gwybodaeth dadosod...
StatusRunProgram=Yn gorffen y gosod...
StatusRestartingApplications=Yn ailddechrau rhaglenni...
StatusRollback=Yn dadwneud newidiadau...

; *** Misc. errors
ErrorInternal2=Gwall mewnol: %1
ErrorFunctionFailedNoCode=Methodd %1
ErrorFunctionFailed=Methodd %1; cod %2
ErrorFunctionFailedWithMessage=Methodd %1; cod %2.%n%3
ErrorExecutingProgram=Methu gweithredu'r ffeil:%n%1

; *** Registry errors
ErrorRegOpenKey=Gwall wrth agor allwedd cofrestrfa:%n%1\%2
ErrorRegCreateKey=Gwall wrth greu allwedd cofrestrfa:%n%1\%2
ErrorRegWriteKey=Gwall wrth ysgrifennu i allwedd cofrestrfa:%n%1\%2

; *** INI errors
ErrorIniEntry=Gwall wrth greu cofnod INI yn y ffeil “%1”.

; *** File copying errors
FileAbortRetryIgnoreSkipNotRecommended=Hepgor y ffeil hon (ni argymhellir)
FileAbortRetryIgnoreIgnoreNotRecommended=Anwybyddu'r gwall a pharhau (ni argymhellir)
SourceIsCorrupted=Mae'r ffeil ffynhonnell wedi'i llygru
SourceDoesntExist=Nid yw'r ffeil ffynhonnell “%1” yn bodoli
SourceVerificationFailed=Methodd dilysu'r ffeil ffynhonnell: %1
VerificationSignatureDoesntExist=Nid yw'r ffeil llofnod “%1” yn bodoli
VerificationSignatureInvalid=Mae'r ffeil llofnod “%1” yn annilys
VerificationKeyNotFound=Mae'r ffeil llofnod “%1” yn defnyddio allwedd anhysbys
VerificationFileNameIncorrect=Mae enw'r ffeil yn anghywir
VerificationFileTagIncorrect=Mae tag y ffeil yn anghywir
VerificationFileSizeIncorrect=Mae maint y ffeil yn anghywir
VerificationFileHashIncorrect=Mae hash y ffeil yn anghywir
ExistingFileReadOnly2=Ni ellid disodli'r ffeil bresennol oherwydd ei bod wedi'i nodi'n ddarllen yn unig.
ExistingFileReadOnlyRetry=Tynnu'r priodoledd darllen yn unig a cheisio eto
ExistingFileReadOnlyKeepExisting=Cadw'r ffeil bresennol
ErrorReadingExistingDest=Digwyddodd gwall wrth geisio darllen y ffeil bresennol:
FileExistsSelectAction=Dewis gweithred
FileExists2=Mae'r ffeil eisoes yn bodoli.
FileExistsOverwriteExisting=Trosysgrifo'r ffeil bresennol
FileExistsKeepExisting=Cadw'r ffeil bresennol
FileExistsOverwriteOrKeepAll=Gwneud hyn ar gyfer y gwrthdaro nesaf
ExistingFileNewerSelectAction=Dewis gweithred
ExistingFileNewer2=Mae'r ffeil bresennol yn newyddach na'r un y mae'r gosodwr yn ceisio ei gosod.
ExistingFileNewerOverwriteExisting=Trosysgrifo'r ffeil bresennol
ExistingFileNewerKeepExisting=Cadw'r ffeil bresennol (argymhellir)
ExistingFileNewerOverwriteOrKeepAll=Gwneud hyn ar gyfer y gwrthdaro nesaf
ErrorChangingAttr=Digwyddodd gwall wrth geisio newid priodoleddau'r ffeil bresennol:
ErrorCreatingTemp=Digwyddodd gwall wrth geisio creu ffeil yn y ffolder cyrchfan:
ErrorReadingSource=Digwyddodd gwall wrth geisio darllen y ffeil ffynhonnell:
ErrorCopying=Digwyddodd gwall wrth geisio copïo ffeil:
ErrorDownloading=Digwyddodd gwall wrth geisio lawrlwytho ffeil:
ErrorExtracting=Digwyddodd gwall wrth geisio echdynnu archif:
ErrorReplacingExistingFile=Digwyddodd gwall wrth geisio disodli'r ffeil bresennol:
ErrorRestartReplace=Methodd RestartReplace:
ErrorRenamingTemp=Digwyddodd gwall wrth geisio ailenwi ffeil yn y ffolder cyrchfan:
ErrorRegisterServer=Methu cofrestru'r DLL/OCX: %1
ErrorRegSvr32Failed=Methodd RegSvr32 gyda chod ymadael %1
ErrorRegisterTypeLib=Methu cofrestru'r llyfrgell mathau: %1

; *** Uninstall display name markings
; used for example as 'My Program (32-bit)'
UninstallDisplayNameMark=%1 (%2)
; used for example as 'My Program (32-bit, All users)'
UninstallDisplayNameMarks=%1 (%2, %3)
UninstallDisplayNameMark32Bit=32-bit
UninstallDisplayNameMark64Bit=64-bit
UninstallDisplayNameMarkAllUsers=Pob defnyddiwr
UninstallDisplayNameMarkCurrentUser=Y defnyddiwr presennol

; *** Post-installation errors
ErrorOpeningReadme=Digwyddodd gwall wrth geisio agor y ffeil README.
ErrorRestartingComputer=Methodd y gosodwr ailgychwyn y cyfrifiadur. Gwnewch hyn â llaw.

; *** Uninstaller messages
UninstallNotFound=Nid yw'r ffeil “%1” yn bodoli. Methu dadosod.
UninstallOpenError=Ni ellid agor y ffeil “%1”. Methu dadosod
UninstallUnsupportedVer=Mae ffeil log dadosod “%1” mewn fformat nad yw'r fersiwn hon o'r dadosodwr yn ei adnabod. Methu dadosod
UninstallUnknownEntry=Daethpwyd ar draws cofnod anhysbys (%1) yn y log dadosod
ConfirmUninstall=Ydych chi'n siŵr eich bod am ddileu %1 a'i holl gydrannau yn llwyr?
UninstallOnlyOnWin64=Dim ond ar Windows 64-bit y gellir dadosod y gosodiad hwn.
OnlyAdminCanUninstall=Dim ond defnyddiwr â breintiau gweinyddol all ddadosod y gosodiad hwn.
UninstallStatusLabel=Arhoswch tra bo %1 yn cael ei dileu o'ch cyfrifiadur.
UninstalledAll=Dilëwyd %1 o'ch cyfrifiadur yn llwyddiannus.
UninstalledMost=Mae dadosod %1 wedi'i gwblhau.%n%nNi ellid dileu rhai elfennau. Gellir eu dileu â llaw.
UninstalledAndNeedsRestart=I gwblhau dadosod %1, rhaid ailgychwyn eich cyfrifiadur.%n%nHoffech chi ailgychwyn nawr?
UninstallDataCorrupted=Mae'r ffeil “%1” wedi'i llygru. Methu dadosod

; *** Uninstallation phase messages
ConfirmDeleteSharedFileTitle=Dileu ffeil a rennir?
ConfirmDeleteSharedFile2=Mae'r system yn nodi nad oes unrhyw raglen yn defnyddio'r ffeil a rennir ganlynol mwyach. Hoffech chi i'r dadosodwr ddileu'r ffeil a rennir hon?%n%nOs yw unrhyw raglenni'n dal i ddefnyddio'r ffeil hon a'i bod yn cael ei dileu, efallai na fydd y rhaglenni hynny'n gweithio'n iawn. Os nad ydych yn siŵr, dewiswch “Na”. Ni fydd gadael y ffeil ar eich system yn achosi unrhyw niwed.
SharedFileNameLabel=Enw'r ffeil:
SharedFileLocationLabel=Lleoliad:
WizardUninstalling=Statws dadosod
StatusUninstalling=Yn dadosod %1...

; *** Shutdown block reasons
ShutdownBlockReasonInstallingApp=Yn gosod %1.
ShutdownBlockReasonUninstallingApp=Yn dadosod %1.

; The custom messages below aren't used by Setup itself, but if you make
; use of them in your scripts, you'll want to translate them.

[CustomMessages]

NameAndVersion=%1 fersiwn %2
AdditionalIcons=Llwybrau byr ychwanegol:
CreateDesktopIcon=Creu llwybr byr ar y bwrdd gwaith
CreateQuickLaunchIcon=Creu llwybr byr Quick Launch
ProgramOnTheWeb=%1 ar y We
UninstallProgram=Dadosod %1
LaunchProgram=Lansio %1
AssocFileExtension=Cysylltu %1 â'r estyniad ffeil %2
AssocingFileExtension=Yn cysylltu %1 â'r estyniad ffeil %2...
AutoStartProgramGroupDescription=Cychwyn:
AutoStartProgram=Cychwyn %1 yn awtomatig
AddonHostProgramNotFound=Ni ellid dod o hyd i %1 yn y ffolder a ddewisoch.%n%nHoffech chi barhau beth bynnag?

; The messages that are Lang Switcher's own (the sample is English.isl).
LocalCertReady=Bydd y gosodwr yn creu tystysgrif ar y cyfrifiadur hwn ac yn llofnodi Lang Switcher â hi, fel y gall y rhaglen weithio mewn ffenestri rhaglenni a redir fel gweinyddwr.
LocalCertFailed=Gosodwyd Lang Switcher, ond ni fydd yn gweithio mewn ffenestri rhaglenni a redir fel gweinyddwr: ni dderbyniodd Windows y dystysgrif a grëwyd ar y cyfrifiadur hwn. Gweler y log gosod am fanylion.
RemoveSettingsQuestion=Dileu ffeil gosodiadau Lang Switcher hefyd?
RemoveSettingsKeep=Dewiswch “Na” i gadw eich gosodiadau ar gyfer ailosod yn nes ymlaen.
AuthorSlogan=rhaglenni a wnaed â chalon
