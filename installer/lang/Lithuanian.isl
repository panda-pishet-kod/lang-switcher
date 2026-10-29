; Lang Switcher -- the messages of the installer in Lithuanian (stage E109, group B).
;
; Inno Setup has no official Lithuanian translation (inno_official = false in res\lang\languages.toml),
; so this file carries the whole of it: [LangOptions], every message of the compiler:Default.isl
; (6.5.0+), and the messages of the installer that are Lang Switcher's own. It is read after
; compiler:Default.isl as the second MessagesFile of the "lithuanian" entry of [Languages] in
; ..\LangSwitcher.iss, and so replaces the English text name for name.
; English.isl is the sample the own messages translate; the glossary of the language is
; l10n\glossary\lt.tsv of the control contour. The name of the program is not translated.
; As in Default.isl: a message that has no full stop there has no full stop here (Setup adds its own).
; No "&" accelerators: Lithuanian has no official file, so one window never carries two equal letters;
; the names of buttons inside a message stand in the quotation marks of the language and are written
; as the buttons say them. The name of the program, taken as a bare name, is masculine; with the word
; "programa" it is feminine (one rule for this file, the strings and the feed).

[LangOptions]
LanguageName=Lietuvių
LanguageID=$0427
LanguageCodePage=0

[Messages]

; *** Application titles
SetupAppTitle=Diegimas
SetupWindowTitle=Diegimas - %1
UninstallAppTitle=Šalinimas
UninstallAppFullTitle=%1 šalinimas

; *** Misc. common
InformationTitle=Informacija
ConfirmTitle=Patvirtinimas
ErrorTitle=Klaida

; *** SetupLdr messages
SetupLdrStartupMessage=Bus įdiegta programa %1. Ar norite tęsti?
LdrCannotCreateTemp=Nepavyko sukurti laikinojo failo. Diegimas nutrauktas
LdrCannotExecTemp=Nepavyko paleisti failo laikinajame aplanke. Diegimas nutrauktas
HelpTextNote=

; *** Startup error messages
LastErrorMessage=%1.%n%nKlaida %2: %3
SetupFileMissing=Diegimo aplanke trūksta failo %1. Ištaisykite problemą arba gaukite naują programos kopiją.
SetupFileCorrupt=Diegimo failai sugadinti. Gaukite naują programos kopiją.
SetupFileCorruptOrWrongVer=Diegimo failai sugadinti arba nesuderinami su šia diegimo programos versija. Ištaisykite problemą arba gaukite naują programos kopiją.
InvalidParameter=Komandų eilutėje perduotas netinkamas parametras:%n%n%1
SetupAlreadyRunning=Diegimo programa jau veikia.
WindowsVersionNotSupported=Ši programa nepalaiko jūsų kompiuteryje veikiančios Windows versijos.
WindowsServicePackRequired=Šiai programai reikia %1 su Service Pack %2 arba naujesnės versijos.
NotOnThisPlatform=Ši programa neveiks sistemoje %1.
OnlyOnThisPlatform=Šią programą reikia paleisti sistemoje %1.
OnlyOnTheseArchitectures=Šią programą galima įdiegti tik Windows versijose, skirtose šioms procesoriaus architektūroms:%n%n%1
WinVersionTooLowError=Šiai programai reikia %1 versijos %2 arba naujesnės.
WinVersionTooHighError=Šios programos negalima įdiegti %1 versijoje %2 ar naujesnėje.
AdminPrivilegesRequired=Diegdami šią programą turite būti prisijungę kaip administratorius.
PowerUserPrivilegesRequired=Diegdami šią programą turite būti prisijungę kaip administratorius arba kaip grupės Power Users narys.
SetupAppRunningError=Diegimo programa aptiko, kad šiuo metu veikia programa %1.%n%nUždarykite visus jos egzempliorius, tada spustelėkite „Gerai“, kad tęstumėte, arba „Atšaukti“, kad išeitumėte.
UninstallAppRunningError=Šalinimo programa aptiko, kad šiuo metu veikia programa %1.%n%nUždarykite visus jos egzempliorius, tada spustelėkite „Gerai“, kad tęstumėte, arba „Atšaukti“, kad išeitumėte.

; *** Startup questions
PrivilegesRequiredOverrideTitle=Diegimo režimo pasirinkimas
PrivilegesRequiredOverrideInstruction=Pasirinkite diegimo režimą
PrivilegesRequiredOverrideText1=Programą %1 galima įdiegti visiems naudotojams (reikia administratoriaus teisių) arba tik jums.
PrivilegesRequiredOverrideText2=Programą %1 galima įdiegti tik jums arba visiems naudotojams (reikia administratoriaus teisių).
PrivilegesRequiredOverrideAllUsers=Įdiegti visiems naudotojams
PrivilegesRequiredOverrideAllUsersRecommended=Įdiegti visiems naudotojams (rekomenduojama)
PrivilegesRequiredOverrideCurrentUser=Įdiegti tik man
PrivilegesRequiredOverrideCurrentUserRecommended=Įdiegti tik man (rekomenduojama)

; *** Misc. errors
ErrorCreatingDir=Diegimo programai nepavyko sukurti aplanko „%1“
ErrorTooManyFilesInDir=Aplanke „%1“ nepavyko sukurti failo, nes jame per daug failų

; *** Setup common messages
ExitSetupTitle=Išeiti iš diegimo
ExitSetupMessage=Diegimas neužbaigtas. Jei išeisite dabar, programa nebus įdiegta.%n%nDiegimo programą galėsite paleisti dar kartą kitu metu, kad užbaigtumėte diegimą.%n%nIšeiti iš diegimo?
AboutSetupMenuItem=Apie diegimo programą...
AboutSetupTitle=Apie diegimo programą
AboutSetupMessage=%1 versija %2%n%3%n%n%1 pagrindinis puslapis:%n%4
AboutSetupNote=
TranslatorNote=

; *** Buttons
ButtonBack=< Atgal
ButtonNext=Toliau >
ButtonInstall=Įdiegti
ButtonOK=Gerai
ButtonCancel=Atšaukti
ButtonYes=Taip
ButtonYesToAll=Taip viskam
ButtonNo=Ne
ButtonNoToAll=Ne viskam
ButtonFinish=Baigti
ButtonBrowse=Naršyti...
ButtonWizardBrowse=Naršyti...
ButtonNewFolder=Sukurti naują aplanką

; *** "Select Language" dialog messages
SelectLanguageTitle=Diegimo kalbos pasirinkimas
SelectLanguageLabel=Pasirinkite kalbą, kuri bus naudojama diegiant.

; *** Common wizard text
ClickNext=Spustelėkite „Toliau“, kad tęstumėte, arba „Atšaukti“, kad išeitumėte iš diegimo.
BeveledLabel=
BrowseDialogTitle=Aplanko pasirinkimas
BrowseDialogLabel=Pasirinkite aplanką sąraše, tada spustelėkite „Gerai“.
NewFolderName=Naujas aplankas

; *** "Welcome" wizard page
WelcomeLabel1=Jus sveikina programos [name] diegimo vedlys
WelcomeLabel2=Jūsų kompiuteryje bus įdiegta programa [name/ver].%n%nPrieš tęsiant rekomenduojama uždaryti visas kitas programas.

; *** "Password" wizard page
WizardPassword=Slaptažodis
PasswordLabel1=Šis diegimas apsaugotas slaptažodžiu.
PasswordLabel3=Įveskite slaptažodį ir spustelėkite „Toliau“, kad tęstumėte. Slaptažodžiai skiria didžiąsias ir mažąsias raides.
PasswordEditLabel=Slaptažodis:
IncorrectPassword=Įvestas slaptažodis neteisingas. Bandykite dar kartą.

; *** "License Agreement" wizard page
WizardLicense=Licencijos sutartis
LicenseLabel=Prieš tęsdami perskaitykite šią svarbią informaciją.
LicenseLabel3=Perskaitykite toliau pateiktą licencijos sutartį. Prieš tęsdami diegimą turite sutikti su šios sutarties sąlygomis.
LicenseAccepted=Sutinku su sutartimi
LicenseNotAccepted=Nesutinku su sutartimi

; *** "Information" wizard pages
WizardInfoBefore=Informacija
InfoBeforeLabel=Prieš tęsdami perskaitykite šią svarbią informaciją.
InfoBeforeClickLabel=Kai būsite pasirengę tęsti diegimą, spustelėkite „Toliau“.
WizardInfoAfter=Informacija
InfoAfterLabel=Prieš tęsdami perskaitykite šią svarbią informaciją.
InfoAfterClickLabel=Kai būsite pasirengę tęsti diegimą, spustelėkite „Toliau“.

; *** "User Information" wizard page
WizardUserInfo=Naudotojo informacija
UserInfoDesc=Įveskite savo duomenis.
UserInfoName=Naudotojo vardas:
UserInfoOrg=Organizacija:
UserInfoSerial=Serijos numeris:
UserInfoNameRequired=Turite įvesti vardą.

; *** "Select Destination Location" wizard page
WizardSelectDir=Paskirties vietos pasirinkimas
SelectDirDesc=Kur turi būti įdiegta programa [name]?
SelectDirLabel3=Programa [name] bus įdiegta į šį aplanką.
SelectDirBrowseLabel=Norėdami tęsti, spustelėkite „Toliau“. Jei norite pasirinkti kitą aplanką, spustelėkite „Naršyti“.
DiskSpaceGBLabel=Reikia bent [gb] GB laisvos disko vietos.
DiskSpaceMBLabel=Reikia bent [mb] MB laisvos disko vietos.
CannotInstallToNetworkDrive=Diegimo programa negali diegti į tinklo diską.
CannotInstallToUNCPath=Diegimo programa negali diegti į UNC kelią.
InvalidPath=Turite įvesti pilną kelią su disko raide, pavyzdžiui:%n%nC:\APP%n%narba UNC kelią tokiu pavidalu:%n%n\\serveris\bendras
InvalidDrive=Pasirinktas diskas arba UNC išteklius neegzistuoja arba yra nepasiekiamas. Pasirinkite kitą.
DiskSpaceWarningTitle=Nepakanka disko vietos
DiskSpaceWarning=Diegimui reikia bent %1 KB laisvos vietos, bet pasirinktame diske yra tik %2 KB.%n%nVis tiek tęsti?
DirNameTooLong=Aplanko pavadinimas arba kelias per ilgas.
InvalidDirName=Aplanko pavadinimas netinkamas.
BadDirName32=Aplanko pavadinime negali būti nė vieno iš šių simbolių:%n%n%1
DirExistsTitle=Aplankas jau yra
DirExists=Aplankas:%n%n%1%n%njau yra. Ar vis tiek norite diegti į šį aplanką?
DirDoesntExistTitle=Aplanko nėra
DirDoesntExist=Aplanko:%n%n%1%n%nnėra. Ar norite jį sukurti?

; *** "Select Components" wizard page
WizardSelectComponents=Komponentų pasirinkimas
SelectComponentsDesc=Kuriuos komponentus įdiegti?
SelectComponentsLabel2=Pažymėkite komponentus, kuriuos norite įdiegti; nuimkite žymes nuo komponentų, kurių įdiegti nenorite. Kai būsite pasirengę tęsti, spustelėkite „Toliau“.
FullInstallation=Pilnas diegimas
; if possible don't translate 'Compact' as 'Minimal' (I mean 'Minimal' in your language)
CompactInstallation=Kompaktiškas diegimas
CustomInstallation=Pasirinktinis diegimas
NoUninstallWarningTitle=Komponentai jau yra
NoUninstallWarning=Diegimo programa aptiko, kad jūsų kompiuteryje jau įdiegti šie komponentai:%n%n%1%n%nJei nuimsite jų žymes, jie nebus pašalinti.%n%nVis tiek tęsti?
ComponentSize1=%1 KB
ComponentSize2=%1 MB
ComponentsDiskSpaceGBLabel=Dabartiniam pasirinkimui reikia bent [gb] GB disko vietos.
ComponentsDiskSpaceMBLabel=Dabartiniam pasirinkimui reikia bent [mb] MB disko vietos.

; *** "Select Additional Tasks" wizard page
WizardSelectTasks=Papildomų užduočių pasirinkimas
SelectTasksDesc=Kurias papildomas užduotis atlikti?
SelectTasksLabel2=Pasirinkite papildomas užduotis, kurias diegimo programa turi atlikti diegdama programą [name], tada spustelėkite „Toliau“.

; *** "Select Start Menu Folder" wizard page
WizardSelectProgramGroup=Meniu „Pradėti“ aplanko pasirinkimas
SelectStartMenuFolderDesc=Kur diegimo programa turi sukurti programos nuorodas?
SelectStartMenuFolderLabel3=Diegimo programa sukurs programos nuorodas šiame meniu „Pradėti“ aplanke.
SelectStartMenuFolderBrowseLabel=Norėdami tęsti, spustelėkite „Toliau“. Jei norite pasirinkti kitą aplanką, spustelėkite „Naršyti“.
MustEnterGroupName=Turite įvesti aplanko pavadinimą.
GroupNameTooLong=Aplanko pavadinimas arba kelias per ilgas.
InvalidGroupName=Aplanko pavadinimas netinkamas.
BadGroupName=Aplanko pavadinime negali būti nė vieno iš šių simbolių:%n%n%1
NoProgramGroupCheck2=Nekurti meniu „Pradėti“ aplanko

; *** "Ready to Install" wizard page
WizardReady=Pasirengta diegti
ReadyLabel1=Diegimo programa pasirengusi įdiegti programą [name] jūsų kompiuteryje.
ReadyLabel2a=Spustelėkite „Įdiegti“, kad tęstumėte diegimą, arba „Atgal“, jei norite peržiūrėti ar pakeisti parametrus.
ReadyLabel2b=Spustelėkite „Įdiegti“, kad tęstumėte diegimą.
ReadyMemoUserInfo=Naudotojo informacija:
ReadyMemoDir=Paskirties vieta:
ReadyMemoType=Diegimo tipas:
ReadyMemoComponents=Pasirinkti komponentai:
ReadyMemoGroup=Meniu „Pradėti“ aplankas:
ReadyMemoTasks=Papildomos užduotys:

; *** TDownloadWizardPage wizard page and DownloadTemporaryFile
DownloadingLabel2=Atsisiunčiami failai...
ButtonStopDownload=Stabdyti atsisiuntimą
StopDownload=Ar tikrai norite sustabdyti atsisiuntimą?
ErrorDownloadAborted=Atsisiuntimas nutrauktas
ErrorDownloadFailed=Atsisiuntimas nepavyko: %1 %2
ErrorDownloadSizeFailed=Nepavyko gauti dydžio: %1 %2
ErrorProgress=Netinkama eiga: %1 iš %2
ErrorFileSize=Netinkamas failo dydis: tikėtasi %1, rasta %2

; *** TExtractionWizardPage wizard page and ExtractArchive
ExtractingLabel=Išskleidžiami failai...
ButtonStopExtraction=Stabdyti išskleidimą
StopExtraction=Ar tikrai norite sustabdyti išskleidimą?
ErrorExtractionAborted=Išskleidimas nutrauktas
ErrorExtractionFailed=Išskleidimas nepavyko: %1

; *** Archive extraction failure details
ArchiveIncorrectPassword=Slaptažodis neteisingas
ArchiveIsCorrupted=Archyvas sugadintas
ArchiveUnsupportedFormat=Archyvo formatas nepalaikomas

; *** "Preparing to Install" wizard page
WizardPreparing=Pasirengimas diegti
PreparingDesc=Diegimo programa ruošiasi įdiegti programą [name] jūsų kompiuteryje.
PreviousInstallNotCompleted=Ankstesnės programos diegimas arba šalinimas nebuvo užbaigtas. Kad užbaigtumėte tą diegimą, reikia iš naujo paleisti kompiuterį.%n%nPaleidę kompiuterį iš naujo, dar kartą paleiskite diegimo programą, kad užbaigtumėte programos [name] diegimą.
CannotContinue=Diegimo programa negali tęsti. Spustelėkite „Atšaukti“, kad išeitumėte.
ApplicationsFound=Šios programos naudoja failus, kuriuos diegimo programai reikia atnaujinti. Rekomenduojama leisti diegimo programai automatiškai uždaryti šias programas.
ApplicationsFound2=Šios programos naudoja failus, kuriuos diegimo programai reikia atnaujinti. Rekomenduojama leisti diegimo programai automatiškai uždaryti šias programas. Užbaigus diegimą diegimo programa bandys paleisti tas programas iš naujo.
CloseApplications=Automatiškai uždaryti programas
DontCloseApplications=Neuždaryti programų
ErrorCloseApplications=Diegimo programai nepavyko automatiškai uždaryti visų programų. Prieš tęsiant rekomenduojama uždaryti visas programas, naudojančias failus, kuriuos diegimo programai reikia atnaujinti.
PrepareToInstallNeedsRestart=Diegimo programa turi iš naujo paleisti jūsų kompiuterį. Paleidę kompiuterį iš naujo, dar kartą paleiskite diegimo programą, kad užbaigtumėte programos [name] diegimą.%n%nAr norite paleisti iš naujo dabar?

; *** "Installing" wizard page
WizardInstalling=Diegiama
InstallingLabel=Palaukite, kol diegimo programa įdiegs programą [name] jūsų kompiuteryje.

; *** "Setup Completed" wizard page
FinishedHeadingLabel=Baigiamas programos [name] diegimo vedlys
FinishedLabelNoIcons=Diegimo programa baigė diegti programą [name] jūsų kompiuteryje.
FinishedLabel=Diegimo programa baigė diegti programą [name] jūsų kompiuteryje. Programą galima paleisti naudojant įdiegtas nuorodas.
ClickFinish=Spustelėkite „Baigti“, kad išeitumėte iš diegimo.
FinishedRestartLabel=Kad užbaigtumėte programos [name] diegimą, diegimo programa turi iš naujo paleisti jūsų kompiuterį. Ar norite paleisti iš naujo dabar?
FinishedRestartMessage=Kad užbaigtumėte programos [name] diegimą, diegimo programa turi iš naujo paleisti jūsų kompiuterį.%n%nAr norite paleisti iš naujo dabar?
ShowReadmeCheck=Taip, noriu peržiūrėti failą README
YesRadio=Taip, paleisti kompiuterį iš naujo dabar
NoRadio=Ne, kompiuterį iš naujo paleisiu vėliau
; used for example as 'Run MyProg.exe'
RunEntryExec=Paleisti %1
; used for example as 'View Readme.txt'
RunEntryShellExec=Peržiūrėti %1

; *** "Setup Needs the Next Disk" stuff
ChangeDiskTitle=Diegimo programai reikia kito disko
SelectDiskLabel2=Įdėkite diską %1 ir spustelėkite „Gerai“.%n%nJei šio disko failų galima rasti kitame aplanke nei rodomas žemiau, įveskite teisingą kelią arba spustelėkite „Naršyti“.
PathLabel=Kelias:
FileNotInDir2=Failo „%1“ nepavyko rasti aplanke „%2“. Įdėkite tinkamą diską arba pasirinkite kitą aplanką.
SelectDirectoryLabel=Nurodykite kito disko vietą.

; *** Installation phase messages
SetupAborted=Diegimas neužbaigtas.%n%nIštaisykite problemą ir dar kartą paleiskite diegimo programą.
AbortRetryIgnoreSelectAction=Pasirinkite veiksmą
AbortRetryIgnoreRetry=Bandyti dar kartą
AbortRetryIgnoreIgnore=Ignoruoti klaidą ir tęsti
AbortRetryIgnoreCancel=Atšaukti diegimą
RetryCancelSelectAction=Pasirinkite veiksmą
RetryCancelRetry=Bandyti dar kartą
RetryCancelCancel=Atšaukti

; *** Installation status messages
StatusClosingApplications=Uždaromos programos...
StatusCreateDirs=Kuriami aplankai...
StatusExtractFiles=Išskleidžiami failai...
StatusDownloadFiles=Atsisiunčiami failai...
StatusCreateIcons=Kuriamos nuorodos...
StatusCreateIniEntries=Kuriami INI įrašai...
StatusCreateRegistryEntries=Kuriami registro įrašai...
StatusRegisterFiles=Registruojami failai...
StatusSavingUninstall=Įrašoma šalinimo informacija...
StatusRunProgram=Baigiamas diegimas...
StatusRestartingApplications=Programos paleidžiamos iš naujo...
StatusRollback=Atšaukiami pakeitimai...

; *** Misc. errors
ErrorInternal2=Vidinė klaida: %1
ErrorFunctionFailedNoCode=%1 nepavyko
ErrorFunctionFailed=%1 nepavyko; kodas %2
ErrorFunctionFailedWithMessage=%1 nepavyko; kodas %2.%n%3
ErrorExecutingProgram=Nepavyko paleisti failo:%n%1

; *** Registry errors
ErrorRegOpenKey=Klaida atveriant registro raktą:%n%1\%2
ErrorRegCreateKey=Klaida kuriant registro raktą:%n%1\%2
ErrorRegWriteKey=Klaida įrašant į registro raktą:%n%1\%2

; *** INI errors
ErrorIniEntry=Klaida kuriant INI įrašą faile „%1“.

; *** File copying errors
FileAbortRetryIgnoreSkipNotRecommended=Praleisti šį failą (nerekomenduojama)
FileAbortRetryIgnoreIgnoreNotRecommended=Ignoruoti klaidą ir tęsti (nerekomenduojama)
SourceIsCorrupted=Šaltinio failas sugadintas
SourceDoesntExist=Šaltinio failo „%1“ nėra
SourceVerificationFailed=Šaltinio failo patikra nepavyko: %1
VerificationSignatureDoesntExist=Parašo failo „%1“ nėra
VerificationSignatureInvalid=Parašo failas „%1“ netinkamas
VerificationKeyNotFound=Parašo faile „%1“ naudojamas nežinomas raktas
VerificationFileNameIncorrect=Failo pavadinimas neteisingas
VerificationFileTagIncorrect=Failo žyma neteisinga
VerificationFileSizeIncorrect=Failo dydis neteisingas
VerificationFileHashIncorrect=Failo maiša neteisinga
ExistingFileReadOnly2=Esamo failo nepavyko pakeisti, nes jis pažymėtas kaip tik skaitomas.
ExistingFileReadOnlyRetry=Pašalinti tik skaitomo failo atributą ir bandyti dar kartą
ExistingFileReadOnlyKeepExisting=Palikti esamą failą
ErrorReadingExistingDest=Bandant perskaityti esamą failą įvyko klaida:
FileExistsSelectAction=Pasirinkite veiksmą
FileExists2=Failas jau yra.
FileExistsOverwriteExisting=Perrašyti esamą failą
FileExistsKeepExisting=Palikti esamą failą
FileExistsOverwriteOrKeepAll=Taip daryti ir su kitais konfliktais
ExistingFileNewerSelectAction=Pasirinkite veiksmą
ExistingFileNewer2=Esamas failas yra naujesnis nei tas, kurį bando įdiegti diegimo programa.
ExistingFileNewerOverwriteExisting=Perrašyti esamą failą
ExistingFileNewerKeepExisting=Palikti esamą failą (rekomenduojama)
ExistingFileNewerOverwriteOrKeepAll=Taip daryti ir su kitais konfliktais
ErrorChangingAttr=Bandant pakeisti esamo failo atributus įvyko klaida:
ErrorCreatingTemp=Bandant sukurti failą paskirties aplanke įvyko klaida:
ErrorReadingSource=Bandant perskaityti šaltinio failą įvyko klaida:
ErrorCopying=Bandant nukopijuoti failą įvyko klaida:
ErrorDownloading=Bandant atsisiųsti failą įvyko klaida:
ErrorExtracting=Bandant išskleisti archyvą įvyko klaida:
ErrorReplacingExistingFile=Bandant pakeisti esamą failą įvyko klaida:
ErrorRestartReplace=RestartReplace nepavyko:
ErrorRenamingTemp=Bandant pervadinti failą paskirties aplanke įvyko klaida:
ErrorRegisterServer=Nepavyko užregistruoti DLL/OCX: %1
ErrorRegSvr32Failed=RegSvr32 baigėsi klaidos kodu %1
ErrorRegisterTypeLib=Nepavyko užregistruoti tipų bibliotekos: %1

; *** Uninstall display name markings
; used for example as 'My Program (32-bit)'
UninstallDisplayNameMark=%1 (%2)
; used for example as 'My Program (32-bit, All users)'
UninstallDisplayNameMarks=%1 (%2, %3)
UninstallDisplayNameMark32Bit=32 bitų
UninstallDisplayNameMark64Bit=64 bitų
UninstallDisplayNameMarkAllUsers=Visi naudotojai
UninstallDisplayNameMarkCurrentUser=Dabartinis naudotojas

; *** Post-installation errors
ErrorOpeningReadme=Bandant atidaryti failą README įvyko klaida.
ErrorRestartingComputer=Diegimo programai nepavyko iš naujo paleisti kompiuterio. Padarykite tai rankiniu būdu.

; *** Uninstaller messages
UninstallNotFound=Failo „%1“ nėra. Pašalinti negalima.
UninstallOpenError=Failo „%1“ nepavyko atidaryti. Pašalinti negalima
UninstallUnsupportedVer=Šalinimo žurnalo failas „%1“ yra formato, kurio ši šalinimo programos versija neatpažįsta. Pašalinti negalima
UninstallUnknownEntry=Šalinimo žurnale rastas nežinomas įrašas (%1)
ConfirmUninstall=Ar tikrai norite visiškai pašalinti programą %1 ir visus jos komponentus?
UninstallOnlyOnWin64=Šį diegimą galima pašalinti tik 64 bitų Windows sistemoje.
OnlyAdminCanUninstall=Šį diegimą gali pašalinti tik naudotojas, turintis administratoriaus teises.
UninstallStatusLabel=Palaukite, kol programa %1 šalinama iš jūsų kompiuterio.
UninstalledAll=Programa %1 sėkmingai pašalinta iš jūsų kompiuterio.
UninstalledMost=Programos %1 šalinimas baigtas.%n%nKai kurių elementų nepavyko pašalinti. Juos galima pašalinti rankiniu būdu.
UninstalledAndNeedsRestart=Kad užbaigtumėte programos %1 šalinimą, reikia iš naujo paleisti kompiuterį.%n%nAr norite paleisti iš naujo dabar?
UninstallDataCorrupted=Failas „%1“ sugadintas. Pašalinti negalima

; *** Uninstallation phase messages
ConfirmDeleteSharedFileTitle=Pašalinti bendrinamą failą?
ConfirmDeleteSharedFile2=Sistema rodo, kad šio bendrinamo failo nebenaudoja jokia programa. Ar norite, kad šalinimo programa pašalintų šį bendrinamą failą?%n%nJei kuri nors programa vis dar naudoja šį failą ir jis bus pašalintas, ta programa gali veikti netinkamai. Jei nesate tikri, pasirinkite „Ne“. Palikus failą sistemoje žalos nebus.
SharedFileNameLabel=Failo pavadinimas:
SharedFileLocationLabel=Vieta:
WizardUninstalling=Šalinimo būsena
StatusUninstalling=Šalinama %1...

; *** Shutdown block reasons
ShutdownBlockReasonInstallingApp=Diegiama %1.
ShutdownBlockReasonUninstallingApp=Šalinama %1.

; The custom messages below aren't used by Setup itself, but if you make
; use of them in your scripts, you'll want to translate them.

[CustomMessages]

NameAndVersion=%1 versija %2
AdditionalIcons=Papildomos nuorodos:
CreateDesktopIcon=Sukurti nuorodą darbalaukyje
CreateQuickLaunchIcon=Sukurti spartaus paleidimo nuorodą
ProgramOnTheWeb=%1 internete
UninstallProgram=Pašalinti %1
LaunchProgram=Paleisti %1
AssocFileExtension=Susieti %1 su failų plėtiniu %2
AssocingFileExtension=%1 susiejama su failų plėtiniu %2...
AutoStartProgramGroupDescription=Paleistis:
AutoStartProgram=Automatiškai paleisti %1
AddonHostProgramNotFound=Pasirinktame aplanke nepavyko rasti %1.%n%nVis tiek tęsti?

; The messages that are Lang Switcher's own (the sample is English.isl).
LocalCertReady=Diegimo programa šiame kompiuteryje sukurs sertifikatą ir juo pasirašys Lang Switcher, kad programa veiktų ir administratoriaus teisėmis paleistų programų languose.
LocalCertFailed=Lang Switcher įdiegtas, bet neveiks administratoriaus teisėmis paleistų programų languose: Windows nepriėmė šiame kompiuteryje sukurto sertifikato. Išsamiau — diegimo žurnale.
RemoveSettingsQuestion=Pašalinti ir Lang Switcher parametrų failą?
RemoveSettingsKeep=Pasirinkite „Ne“, kad parametrai būtų išsaugoti būsimam diegimui iš naujo.
AuthorSlogan=programos, sukurtos su širdimi
