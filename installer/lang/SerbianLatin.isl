; Lang Switcher -- the messages of the installer in Serbian, Latin (stage E108, group A).
;
; Inno Setup has no official Serbian translation (inno_official = false in res\lang\languages.toml),
; so this file carries the whole of it: [LangOptions], every message of the compiler:Default.isl
; (6.5.0+), and the messages of the installer that are Lang Switcher's own. It is read after
; compiler:Default.isl as the second MessagesFile of the "serbianlatin" entry of [Languages] in
; ..\LangSwitcher.iss, and so replaces the English text name for name.
; English.isl is the sample the own messages translate; the glossary of the language is
; l10n\glossary\sr-Latn.tsv of the control contour. The name of the program is not translated.
; The same text in Cyrillic letters is SerbianCyrillic.isl (one dictionary for both, letter for letter).
; As in Default.isl: a message that has no full stop there has none here (Setup adds its own).
; No accelerators (&): in a file without an official Inno translation they are left out, so that
; one window never shows two equal letters (RULES 16). The names of the buttons inside a message
; are in the quotation marks of the language, „ “ (RULES 16).

[LangOptions]
LanguageName=Srpski
LanguageID=$241A
LanguageCodePage=0

[Messages]

; *** Application titles
SetupAppTitle=Instalacija
SetupWindowTitle=Instalacija - %1
UninstallAppTitle=Deinstalacija
UninstallAppFullTitle=%1 — deinstalacija

; *** Misc. common
InformationTitle=Informacija
ConfirmTitle=Potvrda
ErrorTitle=Greška

; *** SetupLdr messages
SetupLdrStartupMessage=Ovo će instalirati %1. Želite li da nastavite?
LdrCannotCreateTemp=Nije moguće napraviti privremenu datoteku. Instalacija je prekinuta
LdrCannotExecTemp=Nije moguće pokrenuti datoteku u privremenoj fascikli. Instalacija je prekinuta
HelpTextNote=

; *** Startup error messages
LastErrorMessage=%1.%n%nGreška %2: %3
SetupFileMissing=Datoteka %1 nedostaje u fascikli instalacije. Otklonite problem ili nabavite novu kopiju programa.
SetupFileCorrupt=Datoteke instalacije su oštećene. Nabavite novu kopiju programa.
SetupFileCorruptOrWrongVer=Datoteke instalacije su oštećene ili nisu u skladu sa ovom verzijom instalacije. Otklonite problem ili nabavite novu kopiju programa.
InvalidParameter=U komandnoj liniji je prosleđen neispravan parametar:%n%n%1
SetupAlreadyRunning=Instalacija je već pokrenuta.
WindowsVersionNotSupported=Ovaj program ne podržava verziju sistema Windows koja je pokrenuta na vašem računaru.
WindowsServicePackRequired=Ovaj program zahteva %1 Service Pack %2 ili noviji.
NotOnThisPlatform=Ovaj program se neće pokrenuti na sistemu %1.
OnlyOnThisPlatform=Ovaj program mora da se pokrene na sistemu %1.
OnlyOnTheseArchitectures=Ovaj program se može instalirati samo na verzijama sistema Windows namenjenim sledećim arhitekturama procesora:%n%n%1
WinVersionTooLowError=Ovaj program zahteva %1 verzije %2 ili novije.
WinVersionTooHighError=Ovaj program se ne može instalirati na %1 verzije %2 ili novije.
AdminPrivilegesRequired=Prilikom instalacije ovog programa morate biti prijavljeni kao administrator.
PowerUserPrivilegesRequired=Prilikom instalacije ovog programa morate biti prijavljeni kao administrator ili kao član grupe Power Users.
SetupAppRunningError=Instalacija je otkrila da je %1 trenutno pokrenut.%n%nZatvorite sve njegove pokrenute primerke, pa kliknite na „U redu“ da nastavite ili na „Otkaži“ da izađete.
UninstallAppRunningError=Deinstalacija je otkrila da je %1 trenutno pokrenut.%n%nZatvorite sve njegove pokrenute primerke, pa kliknite na „U redu“ da nastavite ili na „Otkaži“ da izađete.

; *** Startup questions
PrivilegesRequiredOverrideTitle=Izbor režima instalacije
PrivilegesRequiredOverrideInstruction=Izaberite režim instalacije
PrivilegesRequiredOverrideText1=%1 se može instalirati za sve korisnike (potrebna su administratorska prava) ili samo za vas.
PrivilegesRequiredOverrideText2=%1 se može instalirati samo za vas ili za sve korisnike (potrebna su administratorska prava).
PrivilegesRequiredOverrideAllUsers=Instaliraj za sve korisnike
PrivilegesRequiredOverrideAllUsersRecommended=Instaliraj za sve korisnike (preporučuje se)
PrivilegesRequiredOverrideCurrentUser=Instaliraj samo za mene
PrivilegesRequiredOverrideCurrentUserRecommended=Instaliraj samo za mene (preporučuje se)

; *** Misc. errors
ErrorCreatingDir=Instalacija nije mogla da napravi fasciklu „%1“
ErrorTooManyFilesInDir=Nije moguće napraviti datoteku u fascikli „%1“ jer sadrži previše datoteka

; *** Setup common messages
ExitSetupTitle=Izlaz iz instalacije
ExitSetupMessage=Instalacija nije završena. Ako sada izađete, program neće biti instaliran.%n%nInstalaciju možete ponovo pokrenuti kasnije da biste je završili.%n%nIzaći iz instalacije?
AboutSetupMenuItem=O instalaciji...
AboutSetupTitle=O instalaciji
AboutSetupMessage=%1 verzija %2%n%3%n%n%1 — početna stranica:%n%4
AboutSetupNote=
TranslatorNote=

; *** Buttons
ButtonBack=< Nazad
ButtonNext=Dalje >
ButtonInstall=Instaliraj
ButtonOK=U redu
ButtonCancel=Otkaži
ButtonYes=Da
ButtonYesToAll=Da za sve
ButtonNo=Ne
ButtonNoToAll=Ne za sve
ButtonFinish=Završi
ButtonBrowse=Pregledaj...
ButtonWizardBrowse=Pregledaj...
ButtonNewFolder=Nova fascikla

; *** "Select Language" dialog messages
SelectLanguageTitle=Izbor jezika instalacije
SelectLanguageLabel=Izaberite jezik koji će se koristiti tokom instalacije.

; *** Common wizard text
ClickNext=Kliknite na „Dalje“ da nastavite ili na „Otkaži“ da izađete iz instalacije.
BeveledLabel=
BrowseDialogTitle=Izbor fascikle
BrowseDialogLabel=Izaberite fasciklu sa spiska ispod, a zatim kliknite na „U redu“.
NewFolderName=Nova fascikla

; *** "Welcome" wizard page
WelcomeLabel1=Dobro došli u čarobnjak za instalaciju programa [name]
WelcomeLabel2=Ovo će instalirati [name/ver] na vaš računar.%n%nPreporučuje se da zatvorite sve ostale programe pre nego što nastavite.

; *** "Password" wizard page
WizardPassword=Lozinka
PasswordLabel1=Ova instalacija je zaštićena lozinkom.
PasswordLabel3=Unesite lozinku, pa kliknite na „Dalje“ da nastavite. Lozinke razlikuju velika i mala slova.
PasswordEditLabel=Lozinka:
IncorrectPassword=Uneta lozinka nije tačna. Pokušajte ponovo.

; *** "License Agreement" wizard page
WizardLicense=Ugovor o licenci
LicenseLabel=Pažljivo pročitajte sledeće važne informacije pre nego što nastavite.
LicenseLabel3=Pročitajte sledeći ugovor o licenci. Morate prihvatiti uslove ovog ugovora da biste nastavili instalaciju.
LicenseAccepted=Prihvatam ugovor
LicenseNotAccepted=Ne prihvatam ugovor

; *** "Information" wizard pages
WizardInfoBefore=Informacije
InfoBeforeLabel=Pažljivo pročitajte sledeće važne informacije pre nego što nastavite.
InfoBeforeClickLabel=Kada budete spremni da nastavite instalaciju, kliknite na „Dalje“.
WizardInfoAfter=Informacije
InfoAfterLabel=Pažljivo pročitajte sledeće važne informacije pre nego što nastavite.
InfoAfterClickLabel=Kada budete spremni da nastavite instalaciju, kliknite na „Dalje“.

; *** "User Information" wizard page
WizardUserInfo=Podaci o korisniku
UserInfoDesc=Unesite svoje podatke.
UserInfoName=Korisničko ime:
UserInfoOrg=Organizacija:
UserInfoSerial=Serijski broj:
UserInfoNameRequired=Morate uneti ime.

; *** "Select Destination Location" wizard page
WizardSelectDir=Izbor odredišne lokacije
SelectDirDesc=Gde treba instalirati program [name]?
SelectDirLabel3=Instalacija će instalirati program [name] u sledeću fasciklu.
SelectDirBrowseLabel=Kliknite na „Dalje“ da nastavite. Ako želite da izaberete drugu fasciklu, kliknite na „Pregledaj“.
DiskSpaceGBLabel=Potrebno je najmanje [gb] GB slobodnog prostora na disku.
DiskSpaceMBLabel=Potrebno je najmanje [mb] MB slobodnog prostora na disku.
CannotInstallToNetworkDrive=Instalacija ne može da instalira na mrežni disk.
CannotInstallToUNCPath=Instalacija ne može da instalira na UNC putanju.
InvalidPath=Morate uneti punu putanju sa oznakom diska, na primer:%n%nC:\APP%n%nili UNC putanju u obliku:%n%n\\server\share
InvalidDrive=Izabrani disk ili UNC deljena fascikla ne postoji ili nije dostupna. Izaberite drugi.
DiskSpaceWarningTitle=Nema dovoljno prostora na disku
DiskSpaceWarning=Instalacija zahteva najmanje %1 KB slobodnog prostora, a izabrani disk ima samo %2 KB.%n%nŽelite li ipak da nastavite?
DirNameTooLong=Naziv fascikle ili putanja je predugačka.
InvalidDirName=Naziv fascikle nije ispravan.
BadDirName32=Nazivi fascikli ne mogu sadržati nijedan od sledećih znakova:%n%n%1
DirExistsTitle=Fascikla već postoji
DirExists=Fascikla:%n%n%1%n%nveć postoji. Želite li ipak da instalirate u tu fasciklu?
DirDoesntExistTitle=Fascikla ne postoji
DirDoesntExist=Fascikla:%n%n%1%n%nne postoji. Želite li da se napravi?

; *** "Select Components" wizard page
WizardSelectComponents=Izbor komponenti
SelectComponentsDesc=Koje komponente treba instalirati?
SelectComponentsLabel2=Označite komponente koje želite da instalirate, a uklonite oznaku sa onih koje ne želite. Kliknite na „Dalje“ kada budete spremni da nastavite.
FullInstallation=Potpuna instalacija
; if possible don't translate 'Compact' as 'Minimal' (I mean 'Minimal' in your language)
CompactInstallation=Kompaktna instalacija
CustomInstallation=Prilagođena instalacija
NoUninstallWarningTitle=Komponente već postoje
NoUninstallWarning=Instalacija je otkrila da su sledeće komponente već instalirane na vašem računaru:%n%n%1%n%nAko uklonite oznaku sa njih, one neće biti deinstalirane.%n%nŽelite li ipak da nastavite?
ComponentSize1=%1 KB
ComponentSize2=%1 MB
ComponentsDiskSpaceGBLabel=Trenutni izbor zahteva najmanje [gb] GB prostora na disku.
ComponentsDiskSpaceMBLabel=Trenutni izbor zahteva najmanje [mb] MB prostora na disku.

; *** "Select Additional Tasks" wizard page
WizardSelectTasks=Izbor dodatnih zadataka
SelectTasksDesc=Koje dodatne zadatke treba izvršiti?
SelectTasksLabel2=Izaberite dodatne zadatke koje instalacija treba da izvrši tokom instaliranja programa [name], pa kliknite na „Dalje“.

; *** "Select Start Menu Folder" wizard page
WizardSelectProgramGroup=Izbor fascikle u meniju „Start“
SelectStartMenuFolderDesc=Gde instalacija treba da postavi prečice programa?
SelectStartMenuFolderLabel3=Instalacija će napraviti prečice programa u sledećoj fascikli menija „Start“.
SelectStartMenuFolderBrowseLabel=Kliknite na „Dalje“ da nastavite. Ako želite da izaberete drugu fasciklu, kliknite na „Pregledaj“.
MustEnterGroupName=Morate uneti naziv fascikle.
GroupNameTooLong=Naziv fascikle ili putanja je predugačka.
InvalidGroupName=Naziv fascikle nije ispravan.
BadGroupName=Naziv fascikle ne može sadržati nijedan od sledećih znakova:%n%n%1
NoProgramGroupCheck2=Ne pravi fasciklu u meniju „Start“

; *** "Ready to Install" wizard page
WizardReady=Spremno za instalaciju
ReadyLabel1=Instalacija je spremna da započne instaliranje programa [name] na vaš računar.
ReadyLabel2a=Kliknite na „Instaliraj“ da nastavite instalaciju ili na „Nazad“ ako želite da pregledate ili promenite podešavanja.
ReadyLabel2b=Kliknite na „Instaliraj“ da nastavite instalaciju.
ReadyMemoUserInfo=Podaci o korisniku:
ReadyMemoDir=Odredišna lokacija:
ReadyMemoType=Vrsta instalacije:
ReadyMemoComponents=Izabrane komponente:
ReadyMemoGroup=Fascikla u meniju „Start“:
ReadyMemoTasks=Dodatni zadaci:

; *** TDownloadWizardPage wizard page and DownloadTemporaryFile
DownloadingLabel2=Preuzimanje datoteka...
ButtonStopDownload=Zaustavi preuzimanje
StopDownload=Da li ste sigurni da želite da zaustavite preuzimanje?
ErrorDownloadAborted=Preuzimanje je prekinuto
ErrorDownloadFailed=Preuzimanje nije uspelo: %1 %2
ErrorDownloadSizeFailed=Dobavljanje veličine nije uspelo: %1 %2
ErrorProgress=Neispravan napredak: %1 od %2
ErrorFileSize=Neispravna veličina datoteke: očekivano %1, pronađeno %2

; *** TExtractionWizardPage wizard page and ExtractArchive
ExtractingLabel=Raspakivanje datoteka...
ButtonStopExtraction=Zaustavi raspakivanje
StopExtraction=Da li ste sigurni da želite da zaustavite raspakivanje?
ErrorExtractionAborted=Raspakivanje je prekinuto
ErrorExtractionFailed=Raspakivanje nije uspelo: %1

; *** Archive extraction failure details
ArchiveIncorrectPassword=Lozinka nije tačna
ArchiveIsCorrupted=Arhiva je oštećena
ArchiveUnsupportedFormat=Format arhive nije podržan

; *** "Preparing to Install" wizard page
WizardPreparing=Priprema za instalaciju
PreparingDesc=Instalacija se priprema da instalira program [name] na vaš računar.
PreviousInstallNotCompleted=Instalacija ili uklanjanje prethodnog programa nije završeno. Morate ponovo pokrenuti računar da biste završili tu instalaciju.%n%nNakon ponovnog pokretanja računara ponovo pokrenite instalaciju da biste završili instaliranje programa [name].
CannotContinue=Instalacija ne može da nastavi. Kliknite na „Otkaži“ da izađete.
ApplicationsFound=Sledeći programi koriste datoteke koje instalacija treba da ažurira. Preporučuje se da dozvolite instalaciji da automatski zatvori te programe.
ApplicationsFound2=Sledeći programi koriste datoteke koje instalacija treba da ažurira. Preporučuje se da dozvolite instalaciji da automatski zatvori te programe. Kada se instaliranje završi, instalacija će pokušati da ponovo pokrene te programe.
CloseApplications=Automatski zatvori programe
DontCloseApplications=Ne zatvaraj programe
ErrorCloseApplications=Instalacija nije mogla automatski da zatvori sve programe. Preporučuje se da zatvorite sve programe koji koriste datoteke koje instalacija treba da ažurira, pre nego što nastavite.
PrepareToInstallNeedsRestart=Instalacija mora ponovo da pokrene računar. Nakon ponovnog pokretanja računara ponovo pokrenite instalaciju da biste završili instaliranje programa [name].%n%nŽelite li da ponovo pokrenete računar sada?

; *** "Installing" wizard page
WizardInstalling=Instaliranje
InstallingLabel=Sačekajte dok instalacija instalira program [name] na vaš računar.

; *** "Setup Completed" wizard page
FinishedHeadingLabel=Završetak čarobnjaka za instalaciju programa [name]
FinishedLabelNoIcons=Instalacija je završila instaliranje programa [name] na vaš računar.
FinishedLabel=Instalacija je završila instaliranje programa [name] na vaš računar. Program možete pokrenuti pomoću instaliranih prečica.
ClickFinish=Kliknite na „Završi“ da izađete iz instalacije.
FinishedRestartLabel=Da bi instaliranje programa [name] bilo završeno, instalacija mora ponovo da pokrene računar. Želite li da to uradite sada?
FinishedRestartMessage=Da bi instaliranje programa [name] bilo završeno, instalacija mora ponovo da pokrene računar.%n%nŽelite li da to uradite sada?
ShowReadmeCheck=Da, želim da pregledam datoteku README
YesRadio=Da, ponovo pokreni računar sada
NoRadio=Ne, računar ću ponovo pokrenuti kasnije
; used for example as 'Run MyProg.exe'
RunEntryExec=Pokreni %1
; used for example as 'View Readme.txt'
RunEntryShellExec=Pregledaj %1

; *** "Setup Needs the Next Disk" stuff
ChangeDiskTitle=Instalacija traži sledeći disk
SelectDiskLabel2=Ubacite disk %1 i kliknite na „U redu“.%n%nAko se datoteke sa ovog diska nalaze u drugoj fascikli od prikazane ispod, unesite ispravnu putanju ili kliknite na „Pregledaj“.
PathLabel=Putanja:
FileNotInDir2=Datoteka „%1“ nije pronađena u „%2“. Ubacite ispravan disk ili izaberite drugu fasciklu.
SelectDirectoryLabel=Navedite lokaciju sledećeg diska.

; *** Installation phase messages
SetupAborted=Instalacija nije završena.%n%nOtklonite problem i ponovo pokrenite instalaciju.
AbortRetryIgnoreSelectAction=Izaberite radnju
AbortRetryIgnoreRetry=Pokušaj ponovo
AbortRetryIgnoreIgnore=Zanemari grešku i nastavi
AbortRetryIgnoreCancel=Otkaži instalaciju
RetryCancelSelectAction=Izaberite radnju
RetryCancelRetry=Pokušaj ponovo
RetryCancelCancel=Otkaži

; *** Installation status messages
StatusClosingApplications=Zatvaranje programa...
StatusCreateDirs=Pravljenje fascikli...
StatusExtractFiles=Raspakivanje datoteka...
StatusDownloadFiles=Preuzimanje datoteka...
StatusCreateIcons=Pravljenje prečica...
StatusCreateIniEntries=Pravljenje INI unosa...
StatusCreateRegistryEntries=Pravljenje unosa u registru...
StatusRegisterFiles=Registrovanje datoteka...
StatusSavingUninstall=Čuvanje podataka za deinstalaciju...
StatusRunProgram=Završavanje instalacije...
StatusRestartingApplications=Ponovno pokretanje programa...
StatusRollback=Poništavanje promena...

; *** Misc. errors
ErrorInternal2=Interna greška: %1
ErrorFunctionFailedNoCode=%1 nije uspelo
ErrorFunctionFailed=%1 nije uspelo; kod %2
ErrorFunctionFailedWithMessage=%1 nije uspelo; kod %2.%n%3
ErrorExecutingProgram=Nije moguće pokrenuti datoteku:%n%1

; *** Registry errors
ErrorRegOpenKey=Greška pri otvaranju ključa registra:%n%1\%2
ErrorRegCreateKey=Greška pri pravljenju ključa registra:%n%1\%2
ErrorRegWriteKey=Greška pri upisivanju u ključ registra:%n%1\%2

; *** INI errors
ErrorIniEntry=Greška pri pravljenju INI unosa u datoteci „%1“.

; *** File copying errors
FileAbortRetryIgnoreSkipNotRecommended=Preskoči ovu datoteku (ne preporučuje se)
FileAbortRetryIgnoreIgnoreNotRecommended=Zanemari grešku i nastavi (ne preporučuje se)
SourceIsCorrupted=Izvorna datoteka je oštećena
SourceDoesntExist=Izvorna datoteka „%1“ ne postoji
SourceVerificationFailed=Provera izvorne datoteke nije uspela: %1
VerificationSignatureDoesntExist=Datoteka potpisa „%1“ ne postoji
VerificationSignatureInvalid=Datoteka potpisa „%1“ je neispravna
VerificationKeyNotFound=Datoteka potpisa „%1“ koristi nepoznat ključ
VerificationFileNameIncorrect=Naziv datoteke nije ispravan
VerificationFileTagIncorrect=Oznaka datoteke nije ispravna
VerificationFileSizeIncorrect=Veličina datoteke nije ispravna
VerificationFileHashIncorrect=Heš datoteke nije ispravan
ExistingFileReadOnly2=Postojeća datoteka nije mogla biti zamenjena jer je označena kao „samo za čitanje“.
ExistingFileReadOnlyRetry=Ukloni oznaku „samo za čitanje“ i pokušaj ponovo
ExistingFileReadOnlyKeepExisting=Zadrži postojeću datoteku
ErrorReadingExistingDest=Došlo je do greške pri pokušaju čitanja postojeće datoteke:
FileExistsSelectAction=Izaberite radnju
FileExists2=Datoteka već postoji.
FileExistsOverwriteExisting=Zameni postojeću datoteku
FileExistsKeepExisting=Zadrži postojeću datoteku
FileExistsOverwriteOrKeepAll=Uradi to i za sledeće sukobe
ExistingFileNewerSelectAction=Izaberite radnju
ExistingFileNewer2=Postojeća datoteka je novija od one koju instalacija pokušava da instalira.
ExistingFileNewerOverwriteExisting=Zameni postojeću datoteku
ExistingFileNewerKeepExisting=Zadrži postojeću datoteku (preporučuje se)
ExistingFileNewerOverwriteOrKeepAll=Uradi to i za sledeće sukobe
ErrorChangingAttr=Došlo je do greške pri pokušaju promene atributa postojeće datoteke:
ErrorCreatingTemp=Došlo je do greške pri pokušaju pravljenja datoteke u odredišnoj fascikli:
ErrorReadingSource=Došlo je do greške pri pokušaju čitanja izvorne datoteke:
ErrorCopying=Došlo je do greške pri pokušaju kopiranja datoteke:
ErrorDownloading=Došlo je do greške pri pokušaju preuzimanja datoteke:
ErrorExtracting=Došlo je do greške pri pokušaju raspakivanja arhive:
ErrorReplacingExistingFile=Došlo je do greške pri pokušaju zamene postojeće datoteke:
ErrorRestartReplace=RestartReplace nije uspeo:
ErrorRenamingTemp=Došlo je do greške pri pokušaju promene naziva datoteke u odredišnoj fascikli:
ErrorRegisterServer=Nije moguće registrovati DLL/OCX: %1
ErrorRegSvr32Failed=RegSvr32 nije uspeo, izlazni kod %1
ErrorRegisterTypeLib=Nije moguće registrovati biblioteku tipova: %1

; *** Uninstall display name markings
; used for example as 'My Program (32-bit)'
UninstallDisplayNameMark=%1 (%2)
; used for example as 'My Program (32-bit, All users)'
UninstallDisplayNameMarks=%1 (%2, %3)
UninstallDisplayNameMark32Bit=32-bitna
UninstallDisplayNameMark64Bit=64-bitna
UninstallDisplayNameMarkAllUsers=Svi korisnici
UninstallDisplayNameMarkCurrentUser=Trenutni korisnik

; *** Post-installation errors
ErrorOpeningReadme=Došlo je do greške pri pokušaju otvaranja datoteke README.
ErrorRestartingComputer=Instalacija nije mogla da ponovo pokrene računar. Uradite to ručno.

; *** Uninstaller messages
UninstallNotFound=Datoteka „%1“ ne postoji. Deinstalacija nije moguća.
UninstallOpenError=Datoteka „%1“ nije mogla da se otvori. Deinstalacija nije moguća
UninstallUnsupportedVer=Datoteka dnevnika deinstalacije „%1“ je u formatu koji ova verzija programa za deinstalaciju ne prepoznaje. Deinstalacija nije moguća
UninstallUnknownEntry=U dnevniku deinstalacije pronađen je nepoznat unos (%1)
ConfirmUninstall=Da li ste sigurni da želite potpuno da uklonite %1 i sve njegove komponente?
UninstallOnlyOnWin64=Ova instalacija se može deinstalirati samo na 64-bitnom sistemu Windows.
OnlyAdminCanUninstall=Ovu instalaciju može deinstalirati samo korisnik sa administratorskim pravima.
UninstallStatusLabel=Sačekajte dok se %1 uklanja sa vašeg računara.
UninstalledAll=%1 je uspešno uklonjen sa vašeg računara.
UninstalledMost=Deinstalacija programa %1 je završena.%n%nNeke stavke nisu mogle biti uklonjene. Možete ih ukloniti ručno.
UninstalledAndNeedsRestart=Da bi deinstalacija programa %1 bila završena, računar mora ponovo da se pokrene.%n%nŽelite li da to uradite sada?
UninstallDataCorrupted=Datoteka „%1“ je oštećena. Deinstalacija nije moguća

; *** Uninstallation phase messages
ConfirmDeleteSharedFileTitle=Ukloniti deljenu datoteku?
ConfirmDeleteSharedFile2=Sistem pokazuje da nijedan program više ne koristi sledeću deljenu datoteku. Želite li da deinstalacija ukloni tu deljenu datoteku?%n%nAko neki programi još uvek koriste tu datoteku, a ona bude uklonjena, ti programi možda neće raditi ispravno. Ako niste sigurni, izaberite „Ne“. Ostavljanje datoteke na sistemu ne može da naškodi.
SharedFileNameLabel=Naziv datoteke:
SharedFileLocationLabel=Lokacija:
WizardUninstalling=Stanje deinstalacije
StatusUninstalling=Deinstaliranje programa %1...

; *** Shutdown block reasons
ShutdownBlockReasonInstallingApp=Instaliranje programa %1.
ShutdownBlockReasonUninstallingApp=Deinstaliranje programa %1.

; The custom messages below aren't used by Setup itself, but if you make
; use of them in your scripts, you'll want to translate them.

[CustomMessages]

NameAndVersion=%1 verzija %2
AdditionalIcons=Dodatne prečice:
CreateDesktopIcon=Napravi prečicu na radnoj površini
CreateQuickLaunchIcon=Napravi prečicu za brzo pokretanje
ProgramOnTheWeb=%1 na vebu
UninstallProgram=Deinstaliraj %1
LaunchProgram=Pokreni %1
AssocFileExtension=Poveži %1 sa ekstenzijom datoteke %2
AssocingFileExtension=Povezivanje programa %1 sa ekstenzijom datoteke %2...
AutoStartProgramGroupDescription=Pokretanje sa sistemom:
AutoStartProgram=Automatski pokreni %1
AddonHostProgramNotFound=%1 nije pronađen u izabranoj fascikli.%n%nŽelite li ipak da nastavite?

; The messages that are Lang Switcher's own (the sample is English.isl).
LocalCertReady=Instalacija će na ovom računaru napraviti sertifikat i njime potpisati Lang Switcher, kako bi program radio i u prozorima programa pokrenutih kao administrator.
LocalCertFailed=Lang Switcher je instaliran, ali neće raditi u prozorima programa pokrenutih kao administrator: Windows nije prihvatio sertifikat napravljen na ovom računaru. Detalje potražite u dnevniku instalacije.
RemoveSettingsQuestion=Želite li da uklonite i datoteku podešavanja programa Lang Switcher?
RemoveSettingsKeep=Izaberite „Ne“ da zadržite podešavanja za ponovnu instalaciju.
AuthorSlogan=programi napravljeni od srca
