; Lang Switcher -- the messages of the installer in Icelandic (stage E109, group B).
;
; Inno Setup has no official Icelandic file, so this file is whole: [LangOptions], all the messages of
; compiler:Default.isl (version 6.5.0 sample) translated, then the stock [CustomMessages] of that
; file and the messages of the installer that are its own (the sample is English.isl). Read as the
; only MessagesFile of the "icelandic" entry of [Languages] in ..\LangSwitcher.iss; Default.isl may
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
LanguageName=Íslenska
LanguageID=$040F
LanguageCodePage=1252
;DialogFontName=
;DialogFontSize=9
;DialogFontBaseScaleWidth=7
;DialogFontBaseScaleHeight=15
;WelcomeFontName=Segoe UI
;WelcomeFontSize=14

[Messages]

; *** Application titles
SetupAppTitle=Uppsetning
SetupWindowTitle=Uppsetning - %1
UninstallAppTitle=Fjarlæging
UninstallAppFullTitle=Fjarlæging %1

; *** Misc. common
InformationTitle=Upplýsingar
ConfirmTitle=Staðfesta
ErrorTitle=Villa

; *** SetupLdr messages
SetupLdrStartupMessage=Þetta mun setja upp %1. Viltu halda áfram?
LdrCannotCreateTemp=Ekki tókst að búa til tímabundna skrá. Uppsetningu hætt
LdrCannotExecTemp=Ekki tókst að keyra skrá í tímabundinni möppu. Uppsetningu hætt
HelpTextNote=

; *** Startup error messages
LastErrorMessage=%1.%n%nVilla %2: %3
SetupFileMissing=Skrána %1 vantar í uppsetningarmöppuna. Leiðréttu vandann eða fáðu nýtt eintak af forritinu.
SetupFileCorrupt=Uppsetningarskrárnar eru skemmdar. Fáðu nýtt eintak af forritinu.
SetupFileCorruptOrWrongVer=Uppsetningarskrárnar eru skemmdar eða passa ekki við þessa útgáfu uppsetningarforritsins. Leiðréttu vandann eða fáðu nýtt eintak af forritinu.
InvalidParameter=Ógild færibreyta var send í skipanalínunni:%n%n%1
SetupAlreadyRunning=Uppsetningarforritið er þegar í gangi.
WindowsVersionNotSupported=Þetta forrit styður ekki þá útgáfu af Windows sem tölvan þín keyrir.
WindowsServicePackRequired=Þetta forrit krefst %1 Service Pack %2 eða nýrra.
NotOnThisPlatform=Þetta forrit keyrir ekki á %1.
OnlyOnThisPlatform=Þetta forrit verður að keyra á %1.
OnlyOnTheseArchitectures=Aðeins er hægt að setja þetta forrit upp á útgáfum af Windows sem eru hannaðar fyrir eftirfarandi gerðir örgjörva:%n%n%1
WinVersionTooLowError=Þetta forrit krefst %1 útgáfu %2 eða nýrri.
WinVersionTooHighError=Ekki er hægt að setja þetta forrit upp á %1 útgáfu %2 eða nýrri.
AdminPrivilegesRequired=Þú verður að skrá þig inn sem kerfisstjóri til að setja þetta forrit upp.
PowerUserPrivilegesRequired=Þú verður að skrá þig inn sem kerfisstjóri eða sem meðlimur í hópnum Power Users til að setja þetta forrit upp.
SetupAppRunningError=Uppsetningarforritið hefur greint að %1 er í gangi núna.%n%nLokaðu öllum tilvikum þess núna og smelltu síðan á „Í lagi“ til að halda áfram eða „Hætta við“ til að hætta.
UninstallAppRunningError=Fjarlægingarforritið hefur greint að %1 er í gangi núna.%n%nLokaðu öllum tilvikum þess núna og smelltu síðan á „Í lagi“ til að halda áfram eða „Hætta við“ til að hætta.

; *** Startup questions
PrivilegesRequiredOverrideTitle=Veldu uppsetningarham
PrivilegesRequiredOverrideInstruction=Veldu uppsetningarham
PrivilegesRequiredOverrideText1=Hægt er að setja %1 upp fyrir alla notendur (krefst stjórnandaréttinda) eða aðeins fyrir þig.
PrivilegesRequiredOverrideText2=Hægt er að setja %1 upp aðeins fyrir þig eða fyrir alla notendur (krefst stjórnandaréttinda).
PrivilegesRequiredOverrideAllUsers=Setja upp fyrir alla notendur
PrivilegesRequiredOverrideAllUsersRecommended=Setja upp fyrir alla notendur (mælt með)
PrivilegesRequiredOverrideCurrentUser=Setja upp aðeins fyrir mig
PrivilegesRequiredOverrideCurrentUserRecommended=Setja upp aðeins fyrir mig (mælt með)

; *** Misc. errors
ErrorCreatingDir=Uppsetningarforritinu tókst ekki að búa til möppuna „%1“
ErrorTooManyFilesInDir=Ekki tókst að búa til skrá í möppunni „%1“ vegna þess að hún inniheldur of margar skrár

; *** Setup common messages
ExitSetupTitle=Hætta í uppsetningu
ExitSetupMessage=Uppsetningu er ekki lokið. Ef þú hættir núna verður forritið ekki sett upp.%n%nÞú getur keyrt uppsetningarforritið aftur síðar til að ljúka uppsetningunni.%n%nHætta í uppsetningu?
AboutSetupMenuItem=Um uppsetningarforritið...
AboutSetupTitle=Um uppsetningarforritið
AboutSetupMessage=%1 útgáfa %2%n%3%n%nVefsíða %1:%n%4
AboutSetupNote=
TranslatorNote=

; *** Buttons
ButtonBack=< Til baka
ButtonNext=Áfram >
ButtonInstall=Setja upp
ButtonOK=Í lagi
ButtonCancel=Hætta við
ButtonYes=Já
ButtonYesToAll=Já við öllu
ButtonNo=Nei
ButtonNoToAll=Nei við öllu
ButtonFinish=Ljúka
ButtonBrowse=Skoða...
ButtonWizardBrowse=Skoða...
ButtonNewFolder=Búa til nýja möppu

; *** "Select Language" dialog messages
SelectLanguageTitle=Veldu tungumál uppsetningar
SelectLanguageLabel=Veldu tungumálið sem nota á við uppsetninguna.

; *** Common wizard text
ClickNext=Smelltu á „Áfram“ til að halda áfram eða „Hætta við“ til að hætta í uppsetningu.
BeveledLabel=
BrowseDialogTitle=Skoða möppur
BrowseDialogLabel=Veldu möppu á listanum hér að neðan og smelltu síðan á „Í lagi“.
NewFolderName=Ný mappa

; *** "Welcome" wizard page
WelcomeLabel1=Velkomin í uppsetningarleiðarvísi [name]
WelcomeLabel2=Þetta setur [name/ver] upp á tölvunni þinni.%n%nMælt er með að þú lokir öllum öðrum forritum áður en þú heldur áfram.

; *** "Password" wizard page
WizardPassword=Lykilorð
PasswordLabel1=Þessi uppsetning er varin með lykilorði.
PasswordLabel3=Sláðu inn lykilorðið og smelltu síðan á „Áfram“ til að halda áfram. Lykilorð eru há- og lágstafanæm.
PasswordEditLabel=Lykilorð:
IncorrectPassword=Lykilorðið sem þú slóst inn er ekki rétt. Reyndu aftur.

; *** "License Agreement" wizard page
WizardLicense=Leyfissamningur
LicenseLabel=Lestu eftirfarandi mikilvægar upplýsingar áður en þú heldur áfram.
LicenseLabel3=Lestu eftirfarandi leyfissamning. Þú verður að samþykkja skilmála samningsins áður en uppsetningu er haldið áfram.
LicenseAccepted=Ég samþykki samninginn
LicenseNotAccepted=Ég samþykki ekki samninginn

; *** "Information" wizard pages
WizardInfoBefore=Upplýsingar
InfoBeforeLabel=Lestu eftirfarandi mikilvægar upplýsingar áður en þú heldur áfram.
InfoBeforeClickLabel=Smelltu á „Áfram“ þegar þú vilt halda uppsetningu áfram.
WizardInfoAfter=Upplýsingar
InfoAfterLabel=Lestu eftirfarandi mikilvægar upplýsingar áður en þú heldur áfram.
InfoAfterClickLabel=Smelltu á „Áfram“ þegar þú vilt halda uppsetningu áfram.

; *** "User Information" wizard page
WizardUserInfo=Notandaupplýsingar
UserInfoDesc=Sláðu inn upplýsingarnar þínar.
UserInfoName=Notandanafn:
UserInfoOrg=Stofnun:
UserInfoSerial=Raðnúmer:
UserInfoNameRequired=Þú verður að slá inn nafn.

; *** "Select Destination Location" wizard page
WizardSelectDir=Veldu staðsetningu
SelectDirDesc=Hvar á að setja [name] upp?
SelectDirLabel3=Uppsetningarforritið setur [name] upp í eftirfarandi möppu.
SelectDirBrowseLabel=Smelltu á „Áfram“ til að halda áfram. Ef þú vilt velja aðra möppu skaltu smella á „Skoða“.
DiskSpaceGBLabel=Að minnsta kosti [gb] GB af lausu diskplássi þarf.
DiskSpaceMBLabel=Að minnsta kosti [mb] MB af lausu diskplássi þarf.
CannotInstallToNetworkDrive=Ekki er hægt að setja upp á netdrif.
CannotInstallToUNCPath=Ekki er hægt að setja upp á UNC-slóð.
InvalidPath=Þú verður að slá inn heila slóð með drifstaf; til dæmis:%n%nC:\APP%n%neða UNC-slóð á forminu:%n%n\\þjónn\deiling
InvalidDrive=Drifið eða UNC-deilingin sem þú valdir er ekki til eða er ekki aðgengileg. Veldu annað.
DiskSpaceWarningTitle=Ekki nóg diskpláss
DiskSpaceWarning=Uppsetningin krefst að minnsta kosti %1 KB af lausu plássi, en aðeins %2 KB eru laus á völdu drifi.%n%nViltu halda áfram samt sem áður?
DirNameTooLong=Heiti möppunnar eða slóðin er of löng.
InvalidDirName=Heiti möppunnar er ekki gilt.
BadDirName32=Möppuheiti mega ekki innihalda neina af eftirfarandi stöfum:%n%n%1
DirExistsTitle=Mappan er til
DirExists=Mappan:%n%n%1%n%ner þegar til. Viltu setja upp í þeirri möppu samt sem áður?
DirDoesntExistTitle=Mappan er ekki til
DirDoesntExist=Mappan:%n%n%1%n%ner ekki til. Viltu að hún verði búin til?

; *** "Select Components" wizard page
WizardSelectComponents=Veldu íhluti
SelectComponentsDesc=Hvaða íhluti á að setja upp?
SelectComponentsLabel2=Veldu íhlutina sem þú vilt setja upp; hreinsaðu hakið við íhluti sem þú vilt ekki setja upp. Smelltu á „Áfram“ þegar þú vilt halda áfram.
FullInstallation=Full uppsetning
; if possible don't translate 'Compact' as 'Minimal' (I mean 'Minimal' in your language)
CompactInstallation=Þétt uppsetning
CustomInstallation=Sérsniðin uppsetning
NoUninstallWarningTitle=Íhlutir eru til
NoUninstallWarning=Uppsetningarforritið hefur greint að eftirfarandi íhlutir eru þegar uppsettir á tölvunni þinni:%n%n%1%n%nEf hakið er tekið af þessum íhlutum eru þeir ekki fjarlægðir.%n%nViltu halda áfram samt sem áður?
ComponentSize1=%1 KB
ComponentSize2=%1 MB
ComponentsDiskSpaceGBLabel=Núverandi val krefst að minnsta kosti [gb] GB af diskplássi.
ComponentsDiskSpaceMBLabel=Núverandi val krefst að minnsta kosti [mb] MB af diskplássi.

; *** "Select Additional Tasks" wizard page
WizardSelectTasks=Veldu viðbótarverk
SelectTasksDesc=Hvaða viðbótarverk á að framkvæma?
SelectTasksLabel2=Veldu viðbótarverkin sem uppsetningarforritið á að framkvæma við uppsetningu [name] og smelltu síðan á „Áfram“.

; *** "Select Start Menu Folder" wizard page
WizardSelectProgramGroup=Veldu möppu í Upphafsvalmynd
SelectStartMenuFolderDesc=Hvar á uppsetningarforritið að setja flýtivísa forritsins?
SelectStartMenuFolderLabel3=Uppsetningarforritið býr til flýtivísa forritsins í eftirfarandi möppu í Upphafsvalmynd.
SelectStartMenuFolderBrowseLabel=Smelltu á „Áfram“ til að halda áfram. Ef þú vilt velja aðra möppu skaltu smella á „Skoða“.
MustEnterGroupName=Þú verður að slá inn heiti möppu.
GroupNameTooLong=Heiti möppunnar eða slóðin er of löng.
InvalidGroupName=Heiti möppunnar er ekki gilt.
BadGroupName=Heiti möppunnar má ekki innihalda neina af eftirfarandi stöfum:%n%n%1
NoProgramGroupCheck2=Ekki búa til möppu í Upphafsvalmynd

; *** "Ready to Install" wizard page
WizardReady=Tilbúið til uppsetningar
ReadyLabel1=Uppsetningarforritið er nú tilbúið að byrja að setja [name] upp á tölvunni þinni.
ReadyLabel2a=Smelltu á „Setja upp“ til að halda uppsetningu áfram eða á „Til baka“ ef þú vilt skoða eða breyta stillingum.
ReadyLabel2b=Smelltu á „Setja upp“ til að halda uppsetningu áfram.
ReadyMemoUserInfo=Notandaupplýsingar:
ReadyMemoDir=Staðsetning:
ReadyMemoType=Gerð uppsetningar:
ReadyMemoComponents=Valdir íhlutir:
ReadyMemoGroup=Mappa í Upphafsvalmynd:
ReadyMemoTasks=Viðbótarverk:

; *** TDownloadWizardPage wizard page and DownloadTemporaryFile
DownloadingLabel2=Sæki skrár...
ButtonStopDownload=Stöðva niðurhal
StopDownload=Ertu viss um að þú viljir stöðva niðurhalið?
ErrorDownloadAborted=Niðurhali hætt
ErrorDownloadFailed=Niðurhal mistókst: %1 %2
ErrorDownloadSizeFailed=Ekki tókst að fá stærð: %1 %2
ErrorProgress=Ógild framvinda: %1 af %2
ErrorFileSize=Ógild skráarstærð: búist var við %1, fannst %2

; *** TExtractionWizardPage wizard page and ExtractArchive
ExtractingLabel=Pakka út skrám...
ButtonStopExtraction=Stöðva útpökkun
StopExtraction=Ertu viss um að þú viljir stöðva útpökkunina?
ErrorExtractionAborted=Útpökkun hætt
ErrorExtractionFailed=Útpökkun mistókst: %1

; *** Archive extraction failure details
ArchiveIncorrectPassword=Lykilorðið er rangt
ArchiveIsCorrupted=Safnskráin er skemmd
ArchiveUnsupportedFormat=Safnskrársniðið er ekki stutt

; *** "Preparing to Install" wizard page
WizardPreparing=Undirbý uppsetningu
PreparingDesc=Uppsetningarforritið undirbýr uppsetningu [name] á tölvunni þinni.
PreviousInstallNotCompleted=Uppsetningu/fjarlægingu fyrra forrits var ekki lokið. Endurræsa þarf tölvuna til að ljúka þeirri uppsetningu.%n%nKeyrðu uppsetningarforritið aftur eftir endurræsingu til að ljúka uppsetningu [name].
CannotContinue=Uppsetningarforritið getur ekki haldið áfram. Smelltu á „Hætta við“ til að hætta.
ApplicationsFound=Eftirfarandi forrit nota skrár sem uppsetningarforritið þarf að uppfæra. Mælt er með að þú leyfir uppsetningarforritinu að loka þessum forritum sjálfvirkt.
ApplicationsFound2=Eftirfarandi forrit nota skrár sem uppsetningarforritið þarf að uppfæra. Mælt er með að þú leyfir uppsetningarforritinu að loka þessum forritum sjálfvirkt. Eftir að uppsetningu lýkur reynir uppsetningarforritið að ræsa forritin aftur.
CloseApplications=Loka forritunum sjálfvirkt
DontCloseApplications=Ekki loka forritunum
ErrorCloseApplications=Uppsetningarforritinu tókst ekki að loka öllum forritum sjálfvirkt. Mælt er með að þú lokir öllum forritum sem nota skrár sem uppsetningarforritið þarf að uppfæra áður en þú heldur áfram.
PrepareToInstallNeedsRestart=Uppsetningarforritið þarf að endurræsa tölvuna. Keyrðu uppsetningarforritið aftur eftir endurræsingu til að ljúka uppsetningu [name].%n%nViltu endurræsa núna?

; *** "Installing" wizard page
WizardInstalling=Set upp
InstallingLabel=Bíddu á meðan uppsetningarforritið setur [name] upp á tölvunni þinni.

; *** "Setup Completed" wizard page
FinishedHeadingLabel=Uppsetningu [name] er að ljúka
FinishedLabelNoIcons=Uppsetningarforritið hefur lokið við að setja [name] upp á tölvunni þinni.
FinishedLabel=Uppsetningarforritið hefur lokið við að setja [name] upp á tölvunni þinni. Hægt er að ræsa forritið með því að velja flýtivísana sem voru settir upp.
ClickFinish=Smelltu á „Ljúka“ til að hætta í uppsetningu.
FinishedRestartLabel=Til að ljúka uppsetningu [name] þarf uppsetningarforritið að endurræsa tölvuna. Viltu endurræsa núna?
FinishedRestartMessage=Til að ljúka uppsetningu [name] þarf uppsetningarforritið að endurræsa tölvuna.%n%nViltu endurræsa núna?
ShowReadmeCheck=Já, ég vil skoða README-skrána
YesRadio=Já, endurræsa tölvuna núna
NoRadio=Nei, ég endurræsi tölvuna síðar
; used for example as 'Run MyProg.exe'
RunEntryExec=Keyra %1
; used for example as 'View Readme.txt'
RunEntryShellExec=Skoða %1

; *** "Setup Needs the Next Disk" stuff
ChangeDiskTitle=Uppsetningarforritið þarf næsta disk
SelectDiskLabel2=Settu disk %1 í og smelltu á „Í lagi“.%n%nEf skrárnar á þessum diski eru í annarri möppu en þeirri sem sýnd er hér að neðan skaltu slá inn rétta slóð eða smella á „Skoða“.
PathLabel=Slóð:
FileNotInDir2=Skráin „%1“ fannst ekki í „%2“. Settu réttan disk í eða veldu aðra möppu.
SelectDirectoryLabel=Tilgreindu staðsetningu næsta disks.

; *** Installation phase messages
SetupAborted=Uppsetningu var ekki lokið.%n%nLeiðréttu vandann og keyrðu uppsetningarforritið aftur.
AbortRetryIgnoreSelectAction=Veldu aðgerð
AbortRetryIgnoreRetry=Reyna aftur
AbortRetryIgnoreIgnore=Hunsa villuna og halda áfram
AbortRetryIgnoreCancel=Hætta við uppsetningu
RetryCancelSelectAction=Veldu aðgerð
RetryCancelRetry=Reyna aftur
RetryCancelCancel=Hætta við

; *** Installation status messages
StatusClosingApplications=Loka forritum...
StatusCreateDirs=Bý til möppur...
StatusExtractFiles=Pakka út skrám...
StatusDownloadFiles=Sæki skrár...
StatusCreateIcons=Bý til flýtivísa...
StatusCreateIniEntries=Bý til INI-færslur...
StatusCreateRegistryEntries=Bý til færslur í skrásetningu...
StatusRegisterFiles=Skrái skrár...
StatusSavingUninstall=Vista upplýsingar um fjarlægingu...
StatusRunProgram=Lýk uppsetningu...
StatusRestartingApplications=Ræsi forrit aftur...
StatusRollback=Afturkalla breytingar...

; *** Misc. errors
ErrorInternal2=Innri villa: %1
ErrorFunctionFailedNoCode=%1 mistókst
ErrorFunctionFailed=%1 mistókst; kóði %2
ErrorFunctionFailedWithMessage=%1 mistókst; kóði %2.%n%3
ErrorExecutingProgram=Ekki tókst að keyra skrána:%n%1

; *** Registry errors
ErrorRegOpenKey=Villa við opnun skrásetningarlykils:%n%1\%2
ErrorRegCreateKey=Villa við gerð skrásetningarlykils:%n%1\%2
ErrorRegWriteKey=Villa við skrif í skrásetningarlykil:%n%1\%2

; *** INI errors
ErrorIniEntry=Villa við gerð INI-færslu í skránni „%1“.

; *** File copying errors
FileAbortRetryIgnoreSkipNotRecommended=Sleppa þessari skrá (ekki mælt með)
FileAbortRetryIgnoreIgnoreNotRecommended=Hunsa villuna og halda áfram (ekki mælt með)
SourceIsCorrupted=Upprunaskráin er skemmd
SourceDoesntExist=Upprunaskráin „%1“ er ekki til
SourceVerificationFailed=Sannprófun upprunaskrár mistókst: %1
VerificationSignatureDoesntExist=Undirskriftarskráin „%1“ er ekki til
VerificationSignatureInvalid=Undirskriftarskráin „%1“ er ógild
VerificationKeyNotFound=Undirskriftarskráin „%1“ notar óþekktan lykil
VerificationFileNameIncorrect=Heiti skrárinnar er rangt
VerificationFileTagIncorrect=Merki skrárinnar er rangt
VerificationFileSizeIncorrect=Stærð skrárinnar er röng
VerificationFileHashIncorrect=Tætigildi skrárinnar er rangt
ExistingFileReadOnly2=Ekki var hægt að skipta út núverandi skrá þar sem hún er merkt skrifvarin.
ExistingFileReadOnlyRetry=Fjarlægja skrifvörnina og reyna aftur
ExistingFileReadOnlyKeepExisting=Halda núverandi skrá
ErrorReadingExistingDest=Villa kom upp við að lesa núverandi skrá:
FileExistsSelectAction=Veldu aðgerð
FileExists2=Skráin er þegar til.
FileExistsOverwriteExisting=Skrifa yfir núverandi skrá
FileExistsKeepExisting=Halda núverandi skrá
FileExistsOverwriteOrKeepAll=Gera þetta við næstu árekstra
ExistingFileNewerSelectAction=Veldu aðgerð
ExistingFileNewer2=Núverandi skrá er nýrri en sú sem uppsetningarforritið reynir að setja upp.
ExistingFileNewerOverwriteExisting=Skrifa yfir núverandi skrá
ExistingFileNewerKeepExisting=Halda núverandi skrá (mælt með)
ExistingFileNewerOverwriteOrKeepAll=Gera þetta við næstu árekstra
ErrorChangingAttr=Villa kom upp við að breyta eigindum núverandi skrár:
ErrorCreatingTemp=Villa kom upp við að búa til skrá í áfangamöppunni:
ErrorReadingSource=Villa kom upp við að lesa upprunaskrána:
ErrorCopying=Villa kom upp við að afrita skrá:
ErrorDownloading=Villa kom upp við að sækja skrá:
ErrorExtracting=Villa kom upp við að pakka út safnskrá:
ErrorReplacingExistingFile=Villa kom upp við að skipta út núverandi skrá:
ErrorRestartReplace=RestartReplace mistókst:
ErrorRenamingTemp=Villa kom upp við að endurnefna skrá í áfangamöppunni:
ErrorRegisterServer=Ekki tókst að skrá DLL/OCX: %1
ErrorRegSvr32Failed=RegSvr32 mistókst með skilakóðann %1
ErrorRegisterTypeLib=Ekki tókst að skrá gerðasafnið: %1

; *** Uninstall display name markings
; used for example as 'My Program (32-bit)'
UninstallDisplayNameMark=%1 (%2)
; used for example as 'My Program (32-bit, All users)'
UninstallDisplayNameMarks=%1 (%2, %3)
UninstallDisplayNameMark32Bit=32-bita
UninstallDisplayNameMark64Bit=64-bita
UninstallDisplayNameMarkAllUsers=Allir notendur
UninstallDisplayNameMarkCurrentUser=Núverandi notandi

; *** Post-installation errors
ErrorOpeningReadme=Villa kom upp við að opna README-skrána.
ErrorRestartingComputer=Uppsetningarforritinu tókst ekki að endurræsa tölvuna. Gerðu það handvirkt.

; *** Uninstaller messages
UninstallNotFound=Skráin „%1“ er ekki til. Ekki er hægt að fjarlægja.
UninstallOpenError=Ekki tókst að opna skrána „%1“. Ekki er hægt að fjarlægja
UninstallUnsupportedVer=Fjarlægingarannállinn „%1“ er á sniði sem þessi útgáfa fjarlægingarforritsins þekkir ekki. Ekki er hægt að fjarlægja
UninstallUnknownEntry=Óþekkt færsla (%1) fannst í fjarlægingarannálnum
ConfirmUninstall=Ertu viss um að þú viljir fjarlægja %1 og alla íhluti þess alveg?
UninstallOnlyOnWin64=Aðeins er hægt að fjarlægja þessa uppsetningu á 64 bita Windows.
OnlyAdminCanUninstall=Aðeins notandi með stjórnandaréttindi getur fjarlægt þessa uppsetningu.
UninstallStatusLabel=Bíddu á meðan %1 er fjarlægt af tölvunni þinni.
UninstalledAll=%1 var fjarlægt af tölvunni þinni.
UninstalledMost=Fjarlægingu %1 er lokið.%n%nEkki tókst að fjarlægja suma hluti. Hægt er að fjarlægja þá handvirkt.
UninstalledAndNeedsRestart=Til að ljúka fjarlægingu %1 þarf að endurræsa tölvuna.%n%nViltu endurræsa núna?
UninstallDataCorrupted=Skráin „%1“ er skemmd. Ekki er hægt að fjarlægja

; *** Uninstallation phase messages
ConfirmDeleteSharedFileTitle=Fjarlægja sameiginlega skrá?
ConfirmDeleteSharedFile2=Kerfið gefur til kynna að ekkert forrit noti lengur eftirfarandi sameiginlega skrá. Viltu að fjarlægingarforritið fjarlægi þessa sameiginlegu skrá?%n%nEf einhver forrit nota enn þessa skrá og hún er fjarlægð gætu þau forrit hætt að virka rétt. Ef þú ert ekki viss skaltu velja „Nei“. Skráin skaðar ekkert þótt hún sé áfram á kerfinu.
SharedFileNameLabel=Heiti skráar:
SharedFileLocationLabel=Staðsetning:
WizardUninstalling=Staða fjarlægingar
StatusUninstalling=Fjarlægi %1...

; *** Shutdown block reasons
ShutdownBlockReasonInstallingApp=Set upp %1.
ShutdownBlockReasonUninstallingApp=Fjarlægi %1.

; The custom messages below aren't used by Setup itself, but if you make
; use of them in your scripts, you'll want to translate them.

[CustomMessages]

NameAndVersion=%1 útgáfa %2
AdditionalIcons=Viðbótarflýtivísar:
CreateDesktopIcon=Búa til flýtivísi á skjáborði
CreateQuickLaunchIcon=Búa til flýtivísi í Quick Launch
ProgramOnTheWeb=%1 á vefnum
UninstallProgram=Fjarlægja %1
LaunchProgram=Ræsa %1
AssocFileExtension=Tengja %1 við skráarendinguna %2
AssocingFileExtension=Tengi %1 við skráarendinguna %2...
AutoStartProgramGroupDescription=Ræsing:
AutoStartProgram=Ræsa %1 sjálfvirkt
AddonHostProgramNotFound=%1 fannst ekki í möppunni sem þú valdir.%n%nViltu halda áfram samt sem áður?

; The messages that are Lang Switcher's own (the sample is English.isl).
LocalCertReady=Uppsetningarforritið býr til skilríki á þessari tölvu og undirritar Lang Switcher með því svo forritið virki í gluggum forrita sem keyrð eru sem kerfisstjóri.
LocalCertFailed=Lang Switcher var sett upp en virkar ekki í gluggum forrita sem keyrð eru sem kerfisstjóri: Windows tók ekki við skilríkinu sem búið var til á þessari tölvu. Sjá nánar í uppsetningarannálnum.
RemoveSettingsQuestion=Fjarlægja einnig stillingaskrá Lang Switcher?
RemoveSettingsKeep=Veldu „Nei“ til að halda stillingunum þínum fyrir enduruppsetningu síðar.
AuthorSlogan=forrit gerð af heilum hug
