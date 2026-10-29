; Lang Switcher -- the messages of the installer in Bosnian, Latin (stage E109, group B).
;
; Inno Setup has no official Bosnian translation (inno_official = false in res\lang\languages.toml),
; so this file carries the whole of it: [LangOptions], every message of the compiler:Default.isl
; (6.5.0+), and the messages of the installer that are Lang Switcher's own. It is read after
; compiler:Default.isl as the second MessagesFile of the "bosnian" entry of [Languages] in
; ..\LangSwitcher.iss, and so replaces the English text name for name.
; English.isl is the sample the own messages translate; the glossary of the language is
; l10n\glossary\bs.tsv of the control contour. The name of the program is not translated.
; As in Default.isl: a message that has no full stop there has none here (Setup adds its own).
; No accelerators (&): in a file without an official Inno translation they are left out, so that
; one window never shows two equal letters (RULES 16). The names of the buttons inside a message
; are in the quotation marks of the language, „ ” (RULES 16). "Prečica" without "na tastaturi" is
; the shortcut (.lnk) of Windows; the hotkey of the program is "prečica na tastaturi" (glossary
; bs.tsv).

[LangOptions]
LanguageName=Bosanski
LanguageID=$141A
LanguageCodePage=0

[Messages]

; *** Application titles
SetupAppTitle=Instalacija
SetupWindowTitle=Instalacija – %1
UninstallAppTitle=Deinstalacija
UninstallAppFullTitle=Deinstalacija programa %1

; *** Misc. common
InformationTitle=Informacija
ConfirmTitle=Potvrda
ErrorTitle=Greška

; *** SetupLdr messages
SetupLdrStartupMessage=Ovim ćete instalirati program %1. Želite li nastaviti?
LdrCannotCreateTemp=Nije moguće kreirati privremenu datoteku. Instalacija je prekinuta
LdrCannotExecTemp=Nije moguće pokrenuti datoteku u privremenom folderu. Instalacija je prekinuta
HelpTextNote=

; *** Startup error messages
LastErrorMessage=%1.%n%nGreška %2: %3
SetupFileMissing=Datoteka %1 nedostaje u instalacijskom folderu. Otklonite problem ili nabavite novu kopiju programa.
SetupFileCorrupt=Instalacijske datoteke su oštećene. Nabavite novu kopiju programa.
SetupFileCorruptOrWrongVer=Instalacijske datoteke su oštećene ili nisu kompatibilne s ovom verzijom instalacije. Otklonite problem ili nabavite novu kopiju programa.
InvalidParameter=U komandnoj liniji zadan je neispravan parametar:%n%n%1
SetupAlreadyRunning=Instalacija je već pokrenuta.
WindowsVersionNotSupported=Ovaj program ne podržava verziju sistema Windows koja je pokrenuta na vašem računaru.
WindowsServicePackRequired=Ovaj program zahtijeva %1 Service Pack %2 ili noviji.
NotOnThisPlatform=Ovaj program se neće izvoditi na sistemu %1.
OnlyOnThisPlatform=Ovaj program se mora izvoditi na sistemu %1.
OnlyOnTheseArchitectures=Ovaj program se može instalirati samo na verzije sistema Windows namijenjene sljedećim arhitekturama procesora:%n%n%1
WinVersionTooLowError=Ovaj program zahtijeva %1 verzije %2 ili novije.
WinVersionTooHighError=Ovaj program se ne može instalirati na %1 verzije %2 ili novije.
AdminPrivilegesRequired=Pri instalaciji ovog programa morate biti prijavljeni kao administrator.
PowerUserPrivilegesRequired=Pri instalaciji ovog programa morate biti prijavljeni kao administrator ili kao član grupe Power Users.
SetupAppRunningError=Instalacija je otkrila da je program %1 trenutno pokrenut.%n%nZatvorite sve njegove otvorene instance, zatim kliknite „U redu” za nastavak ili „Otkaži” za izlaz.
UninstallAppRunningError=Deinstalacija je otkrila da je program %1 trenutno pokrenut.%n%nZatvorite sve njegove otvorene instance, zatim kliknite „U redu” za nastavak ili „Otkaži” za izlaz.

; *** Startup questions
PrivilegesRequiredOverrideTitle=Izbor načina instalacije
PrivilegesRequiredOverrideInstruction=Izaberite način instalacije
PrivilegesRequiredOverrideText1=Program %1 se može instalirati za sve korisnike (potrebna su administratorska prava) ili samo za vas.
PrivilegesRequiredOverrideText2=Program %1 se može instalirati samo za vas ili za sve korisnike (potrebna su administratorska prava).
PrivilegesRequiredOverrideAllUsers=Instaliraj za sve korisnike
PrivilegesRequiredOverrideAllUsersRecommended=Instaliraj za sve korisnike (preporučeno)
PrivilegesRequiredOverrideCurrentUser=Instaliraj samo za mene
PrivilegesRequiredOverrideCurrentUserRecommended=Instaliraj samo za mene (preporučeno)

; *** Misc. errors
ErrorCreatingDir=Instalacija nije mogla kreirati folder „%1”
ErrorTooManyFilesInDir=Nije moguće kreirati datoteku u folderu „%1” jer on sadrži previše datoteka

; *** Setup common messages
ExitSetupTitle=Izlaz iz instalacije
ExitSetupMessage=Instalacija nije završena. Ako sada izađete, program neće biti instaliran.%n%nInstalaciju možete ponovo pokrenuti kasnije kako biste je završili.%n%nIzaći iz instalacije?
AboutSetupMenuItem=O instalaciji...
AboutSetupTitle=O instalaciji
AboutSetupMessage=%1 verzija %2%n%3%n%n%1 – početna stranica:%n%4
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
ButtonNewFolder=Novi folder

; *** "Select Language" dialog messages
SelectLanguageTitle=Izbor jezika instalacije
SelectLanguageLabel=Izaberite jezik koji će se koristiti tokom instalacije.

; *** Common wizard text
ClickNext=Kliknite „Dalje” za nastavak ili „Otkaži” za izlaz iz instalacije.
BeveledLabel=
BrowseDialogTitle=Izbor foldera
BrowseDialogLabel=Izaberite folder sa spiska ispod, zatim kliknite „U redu”.
NewFolderName=Novi folder

; *** "Welcome" wizard page
WelcomeLabel1=Dobro došli u čarobnjak za instalaciju programa [name]
WelcomeLabel2=Ovim ćete instalirati [name/ver] na svoj računar.%n%nPreporučuje se zatvoriti sve ostale programe prije nastavka.

; *** "Password" wizard page
WizardPassword=Lozinka
PasswordLabel1=Ova instalacija je zaštićena lozinkom.
PasswordLabel3=Unesite lozinku, zatim kliknite „Dalje” za nastavak. Lozinke razlikuju velika i mala slova.
PasswordEditLabel=Lozinka:
IncorrectPassword=Unesena lozinka nije tačna. Pokušajte ponovo.

; *** "License Agreement" wizard page
WizardLicense=Licencni ugovor
LicenseLabel=Prije nastavka pažljivo pročitajte sljedeće važne informacije.
LicenseLabel3=Pročitajte sljedeći licencni ugovor. Morate prihvatiti uslove ovog ugovora da biste nastavili instalaciju.
LicenseAccepted=Prihvatam ugovor
LicenseNotAccepted=Ne prihvatam ugovor

; *** "Information" wizard pages
WizardInfoBefore=Informacije
InfoBeforeLabel=Prije nastavka pažljivo pročitajte sljedeće važne informacije.
InfoBeforeClickLabel=Kada budete spremni nastaviti instalaciju, kliknite „Dalje”.
WizardInfoAfter=Informacije
InfoAfterLabel=Prije nastavka pažljivo pročitajte sljedeće važne informacije.
InfoAfterClickLabel=Kada budete spremni nastaviti, kliknite „Dalje”.

; *** "User Information" wizard page
WizardUserInfo=Podaci o korisniku
UserInfoDesc=Unesite svoje podatke.
UserInfoName=Korisničko ime:
UserInfoOrg=Organizacija:
UserInfoSerial=Serijski broj:
UserInfoNameRequired=Morate unijeti ime.

; *** "Select Destination Location" wizard page
WizardSelectDir=Izbor odredišnog foldera
SelectDirDesc=Gdje želite instalirati program [name]?
SelectDirLabel3=Instalacija će program [name] instalirati u sljedeći folder.
SelectDirBrowseLabel=Za nastavak kliknite „Dalje”. Ako želite izabrati drugi folder, kliknite „Pregledaj”.
DiskSpaceGBLabel=Potrebno je najmanje [gb] GB slobodnog prostora na disku.
DiskSpaceMBLabel=Potrebno je najmanje [mb] MB slobodnog prostora na disku.
CannotInstallToNetworkDrive=Instalacija se ne može izvršiti na mrežni disk.
CannotInstallToUNCPath=Instalacija se ne može izvršiti na UNC putanju.
InvalidPath=Morate unijeti punu putanju s oznakom diska, na primjer:%n%nC:\APP%n%nili UNC putanju u obliku:%n%n\\server\share
InvalidDrive=Izabrani disk ili UNC dijeljeni resurs ne postoji ili mu se ne može pristupiti. Izaberite drugi.
DiskSpaceWarningTitle=Nedovoljno prostora na disku
DiskSpaceWarning=Instalacija zahtijeva najmanje %1 KB slobodnog prostora, a na izabranom disku dostupno je samo %2 KB.%n%nŽelite li ipak nastaviti?
DirNameTooLong=Ime foldera ili putanja su predugi.
InvalidDirName=Ime foldera nije valjano.
BadDirName32=Imena foldera ne smiju sadržavati nijedan od sljedećih znakova:%n%n%1
DirExistsTitle=Folder postoji
DirExists=Folder:%n%n%1%n%nveć postoji. Želite li ipak instalirati u taj folder?
DirDoesntExistTitle=Folder ne postoji
DirDoesntExist=Folder:%n%n%1%n%nne postoji. Želite li da se folder kreira?

; *** "Select Components" wizard page
WizardSelectComponents=Izbor komponenti
SelectComponentsDesc=Koje komponente treba instalirati?
SelectComponentsLabel2=Izaberite komponente koje želite instalirati; poništite izbor komponenti koje ne želite instalirati. Kliknite „Dalje” kada budete spremni nastaviti.
FullInstallation=Potpuna instalacija
; if possible don't translate 'Compact' as 'Minimal' (I mean 'Minimal' in your language)
CompactInstallation=Sažeta instalacija
CustomInstallation=Prilagođena instalacija
NoUninstallWarningTitle=Komponente postoje
NoUninstallWarning=Instalacija je otkrila da su sljedeće komponente već instalirane na vašem računaru:%n%n%1%n%nPoništavanje izbora tih komponenti neće ih deinstalirati.%n%nŽelite li ipak nastaviti?
ComponentSize1=%1 KB
ComponentSize2=%1 MB
ComponentsDiskSpaceGBLabel=Trenutni izbor zahtijeva najmanje [gb] GB prostora na disku.
ComponentsDiskSpaceMBLabel=Trenutni izbor zahtijeva najmanje [mb] MB prostora na disku.

; *** "Select Additional Tasks" wizard page
WizardSelectTasks=Izbor dodatnih zadataka
SelectTasksDesc=Koje dodatne zadatke treba izvršiti?
SelectTasksLabel2=Izaberite dodatne zadatke koje instalacija treba izvršiti tokom instalacije programa [name], zatim kliknite „Dalje”.

; *** "Select Start Menu Folder" wizard page
WizardSelectProgramGroup=Izbor foldera menija Start
SelectStartMenuFolderDesc=Gdje instalacija treba smjestiti prečice programa?
SelectStartMenuFolderLabel3=Instalacija će kreirati prečice programa u sljedećem folderu menija Start.
SelectStartMenuFolderBrowseLabel=Za nastavak kliknite „Dalje”. Ako želite izabrati drugi folder, kliknite „Pregledaj”.
MustEnterGroupName=Morate unijeti ime foldera.
GroupNameTooLong=Ime foldera ili putanja su predugi.
InvalidGroupName=Ime foldera nije valjano.
BadGroupName=Ime foldera ne smije sadržavati nijedan od sljedećih znakova:%n%n%1
NoProgramGroupCheck2=Ne kreiraj folder u meniju Start

; *** "Ready to Install" wizard page
WizardReady=Spremno za instalaciju
ReadyLabel1=Instalacija je spremna za instaliranje programa [name] na vaš računar.
ReadyLabel2a=Kliknite „Instaliraj” za nastavak instalacije ili „Nazad” ako želite pregledati ili promijeniti neke postavke.
ReadyLabel2b=Kliknite „Instaliraj” za nastavak instalacije.
ReadyMemoUserInfo=Podaci o korisniku:
ReadyMemoDir=Odredišni folder:
ReadyMemoType=Vrsta instalacije:
ReadyMemoComponents=Izabrane komponente:
ReadyMemoGroup=Folder menija Start:
ReadyMemoTasks=Dodatni zadaci:

; *** TDownloadWizardPage wizard page and DownloadTemporaryFile
DownloadingLabel2=Preuzimanje datoteka...
ButtonStopDownload=Zaustavi preuzimanje
StopDownload=Jeste li sigurni da želite zaustaviti preuzimanje?
ErrorDownloadAborted=Preuzimanje je prekinuto
ErrorDownloadFailed=Preuzimanje nije uspjelo: %1 %2
ErrorDownloadSizeFailed=Dobavljanje veličine nije uspjelo: %1 %2
ErrorProgress=Neispravan napredak: %1 od %2
ErrorFileSize=Neispravna veličina datoteke: očekivano %1, pronađeno %2

; *** TExtractionWizardPage wizard page and ExtractArchive
ExtractingLabel=Raspakivanje datoteka...
ButtonStopExtraction=Zaustavi raspakivanje
StopExtraction=Jeste li sigurni da želite zaustaviti raspakivanje?
ErrorExtractionAborted=Raspakivanje je prekinuto
ErrorExtractionFailed=Raspakivanje nije uspjelo: %1

; *** Archive extraction failure details
ArchiveIncorrectPassword=Lozinka nije tačna
ArchiveIsCorrupted=Arhiva je oštećena
ArchiveUnsupportedFormat=Format arhive nije podržan

; *** "Preparing to Install" wizard page
WizardPreparing=Priprema za instalaciju
PreparingDesc=Instalacija se priprema za instaliranje programa [name] na vaš računar.
PreviousInstallNotCompleted=Instalacija ili uklanjanje prethodnog programa nije završeno. Morate ponovo pokrenuti računar da biste završili tu instalaciju.%n%nNakon ponovnog pokretanja računara ponovo pokrenite instalaciju kako biste završili instalaciju programa [name].
CannotContinue=Instalacija ne može nastaviti. Kliknite „Otkaži” za izlaz.
ApplicationsFound=Sljedeći programi koriste datoteke koje instalacija mora ažurirati. Preporučuje se da instalaciji dozvolite da te programe automatski zatvori.
ApplicationsFound2=Sljedeći programi koriste datoteke koje instalacija mora ažurirati. Preporučuje se da instalaciji dozvolite da te programe automatski zatvori. Nakon završetka instalacije pokušat će ih ponovo pokrenuti.
CloseApplications=Automatski zatvori programe
DontCloseApplications=Ne zatvaraj programe
ErrorCloseApplications=Instalacija nije mogla automatski zatvoriti sve programe. Prije nastavka preporučuje se zatvoriti sve programe koji koriste datoteke koje instalacija mora ažurirati.
PrepareToInstallNeedsRestart=Instalacija mora ponovo pokrenuti vaš računar. Nakon ponovnog pokretanja ponovo pokrenite instalaciju kako biste završili instalaciju programa [name].%n%nŽelite li sada ponovo pokrenuti računar?

; *** "Installing" wizard page
WizardInstalling=Instaliranje
InstallingLabel=Sačekajte dok instalacija instalira program [name] na vaš računar.

; *** "Setup Completed" wizard page
FinishedHeadingLabel=Završetak čarobnjaka za instalaciju programa [name]
FinishedLabelNoIcons=Instalacija je završila instaliranje programa [name] na vaš računar.
FinishedLabel=Instalacija je završila instaliranje programa [name] na vaš računar. Program možete pokrenuti pomoću instaliranih prečica.
ClickFinish=Kliknite „Završi” za izlaz iz instalacije.
FinishedRestartLabel=Za završetak instalacije programa [name] instalacija mora ponovo pokrenuti vaš računar. Želite li to uraditi sada?
FinishedRestartMessage=Za završetak instalacije programa [name] instalacija mora ponovo pokrenuti vaš računar.%n%nŽelite li to uraditi sada?
ShowReadmeCheck=Da, želim pregledati datoteku README
YesRadio=Da, ponovo pokreni računar sada
NoRadio=Ne, računar ću ponovo pokrenuti kasnije
; used for example as 'Run MyProg.exe'
RunEntryExec=Pokreni %1
; used for example as 'View Readme.txt'
RunEntryShellExec=Pregledaj %1

; *** "Setup Needs the Next Disk" stuff
ChangeDiskTitle=Instalacija traži sljedeći disk
SelectDiskLabel2=Umetnite disk %1 i kliknite „U redu”.%n%nAko se datoteke s ovog diska nalaze u drugom folderu od onog prikazanog ispod, unesite ispravnu putanju ili kliknite „Pregledaj”.
PathLabel=Putanja:
FileNotInDir2=Datoteka „%1” nije pronađena u „%2”. Umetnite ispravan disk ili izaberite drugi folder.
SelectDirectoryLabel=Odredite mjesto sljedećeg diska.

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
StatusCreateDirs=Kreiranje foldera...
StatusExtractFiles=Raspakivanje datoteka...
StatusDownloadFiles=Preuzimanje datoteka...
StatusCreateIcons=Kreiranje prečica...
StatusCreateIniEntries=Kreiranje INI zapisa...
StatusCreateRegistryEntries=Kreiranje zapisa u registru...
StatusRegisterFiles=Registrovanje datoteka...
StatusSavingUninstall=Čuvanje podataka za deinstalaciju...
StatusRunProgram=Završavanje instalacije...
StatusRestartingApplications=Ponovno pokretanje programa...
StatusRollback=Poništavanje promjena...

; *** Misc. errors
ErrorInternal2=Interna greška: %1
ErrorFunctionFailedNoCode=%1 nije uspjelo
ErrorFunctionFailed=%1 nije uspjelo; kod %2
ErrorFunctionFailedWithMessage=%1 nije uspjelo; kod %2.%n%3
ErrorExecutingProgram=Nije moguće pokrenuti datoteku:%n%1

; *** Registry errors
ErrorRegOpenKey=Greška pri otvaranju ključa registra:%n%1\%2
ErrorRegCreateKey=Greška pri kreiranju ključa registra:%n%1\%2
ErrorRegWriteKey=Greška pri pisanju u ključ registra:%n%1\%2

; *** INI errors
ErrorIniEntry=Greška pri kreiranju INI zapisa u datoteci „%1”.

; *** File copying errors
FileAbortRetryIgnoreSkipNotRecommended=Preskoči ovu datoteku (ne preporučuje se)
FileAbortRetryIgnoreIgnoreNotRecommended=Zanemari grešku i nastavi (ne preporučuje se)
SourceIsCorrupted=Izvorna datoteka je oštećena
SourceDoesntExist=Izvorna datoteka „%1” ne postoji
SourceVerificationFailed=Provjera izvorne datoteke nije uspjela: %1
VerificationSignatureDoesntExist=Datoteka potpisa „%1” ne postoji
VerificationSignatureInvalid=Datoteka potpisa „%1” nije valjana
VerificationKeyNotFound=Datoteka potpisa „%1” koristi nepoznat ključ
VerificationFileNameIncorrect=Ime datoteke nije ispravno
VerificationFileTagIncorrect=Oznaka datoteke nije ispravna
VerificationFileSizeIncorrect=Veličina datoteke nije ispravna
VerificationFileHashIncorrect=Hash datoteke nije ispravan
ExistingFileReadOnly2=Postojeću datoteku nije bilo moguće zamijeniti jer je označena samo za čitanje.
ExistingFileReadOnlyRetry=Ukloni oznaku samo za čitanje i pokušaj ponovo
ExistingFileReadOnlyKeepExisting=Zadrži postojeću datoteku
ErrorReadingExistingDest=Došlo je do greške pri čitanju postojeće datoteke:
FileExistsSelectAction=Izaberite radnju
FileExists2=Datoteka već postoji.
FileExistsOverwriteExisting=Prepiši postojeću datoteku
FileExistsKeepExisting=Zadrži postojeću datoteku
FileExistsOverwriteOrKeepAll=Primijeni na sljedeće sukobe
ExistingFileNewerSelectAction=Izaberite radnju
ExistingFileNewer2=Postojeća datoteka je novija od one koju instalacija pokušava instalirati.
ExistingFileNewerOverwriteExisting=Prepiši postojeću datoteku
ExistingFileNewerKeepExisting=Zadrži postojeću datoteku (preporučeno)
ExistingFileNewerOverwriteOrKeepAll=Primijeni na sljedeće sukobe
ErrorChangingAttr=Došlo je do greške pri promjeni atributa postojeće datoteke:
ErrorCreatingTemp=Došlo je do greške pri kreiranju datoteke u odredišnom folderu:
ErrorReadingSource=Došlo je do greške pri čitanju izvorne datoteke:
ErrorCopying=Došlo je do greške pri kopiranju datoteke:
ErrorDownloading=Došlo je do greške pri preuzimanju datoteke:
ErrorExtracting=Došlo je do greške pri raspakivanju arhive:
ErrorReplacingExistingFile=Došlo je do greške pri zamjeni postojeće datoteke:
ErrorRestartReplace=RestartReplace nije uspio:
ErrorRenamingTemp=Došlo je do greške pri preimenovanju datoteke u odredišnom folderu:
ErrorRegisterServer=Nije moguće registrovati DLL/OCX: %1
ErrorRegSvr32Failed=RegSvr32 nije uspio s izlaznim kodom %1
ErrorRegisterTypeLib=Nije moguće registrovati biblioteku tipova: %1

; *** Uninstall display name markings
; used for example as 'My Program (32-bit)'
UninstallDisplayNameMark=%1 (%2)
; used for example as 'My Program (32-bit, All users)'
UninstallDisplayNameMarks=%1 (%2, %3)
UninstallDisplayNameMark32Bit=32-bitno
UninstallDisplayNameMark64Bit=64-bitno
UninstallDisplayNameMarkAllUsers=Svi korisnici
UninstallDisplayNameMarkCurrentUser=Trenutni korisnik

; *** Post-installation errors
ErrorOpeningReadme=Došlo je do greške pri otvaranju datoteke README.
ErrorRestartingComputer=Instalacija nije mogla ponovo pokrenuti računar. Uradite to ručno.

; *** Uninstaller messages
UninstallNotFound=Datoteka „%1” ne postoji. Deinstalacija nije moguća.
UninstallOpenError=Datoteku „%1” nije moguće otvoriti. Deinstalacija nije moguća
UninstallUnsupportedVer=Datoteka dnevnika deinstalacije „%1” je u formatu koji ova verzija deinstalacijskog programa ne prepoznaje. Deinstalacija nije moguća
UninstallUnknownEntry=U dnevniku deinstalacije pronađen je nepoznat zapis (%1)
ConfirmUninstall=Jeste li sigurni da želite potpuno ukloniti program %1 i sve njegove komponente?
UninstallOnlyOnWin64=Ova instalacija se može deinstalirati samo na 64-bitnom sistemu Windows.
OnlyAdminCanUninstall=Ovu instalaciju može deinstalirati samo korisnik s administratorskim pravima.
UninstallStatusLabel=Sačekajte dok se program %1 uklanja s vašeg računara.
UninstalledAll=Program %1 je uspješno uklonjen s vašeg računara.
UninstalledMost=Deinstalacija programa %1 je završena.%n%nNeke elemente nije bilo moguće ukloniti. Možete ih ukloniti ručno.
UninstalledAndNeedsRestart=Za završetak deinstalacije programa %1 potrebno je ponovo pokrenuti računar.%n%nŽelite li to uraditi sada?
UninstallDataCorrupted=Datoteka „%1” je oštećena. Deinstalacija nije moguća

; *** Uninstallation phase messages
ConfirmDeleteSharedFileTitle=Ukloniti dijeljenu datoteku?
ConfirmDeleteSharedFile2=Sistem pokazuje da sljedeću dijeljenu datoteku više ne koristi nijedan program. Želite li da je deinstalacija ukloni?%n%nAko neki programi još koriste tu datoteku, a ona se ukloni, ti programi možda neće ispravno raditi. Ako niste sigurni, izaberite „Ne”. Ostavljanje datoteke u sistemu neće uzrokovati štetu.
SharedFileNameLabel=Ime datoteke:
SharedFileLocationLabel=Mjesto:
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
CreateDesktopIcon=Kreiraj prečicu na radnoj površini
CreateQuickLaunchIcon=Kreiraj prečicu za brzo pokretanje
ProgramOnTheWeb=%1 na webu
UninstallProgram=Deinstaliraj %1
LaunchProgram=Pokreni %1
AssocFileExtension=Pridruži program %1 ekstenziji datoteke %2
AssocingFileExtension=Pridruživanje programa %1 ekstenziji datoteke %2...
AutoStartProgramGroupDescription=Pokretanje sa sistemom:
AutoStartProgram=Automatski pokreni %1
AddonHostProgramNotFound=Program %1 nije pronađen u izabranom folderu.%n%nŽelite li ipak nastaviti?

; *** Lang Switcher's own messages (the sample is English.isl)
LocalCertReady=Instalacija će na ovom računaru kreirati certifikat i njime potpisati Lang Switcher kako bi program radio i u prozorima programa pokrenutih kao administrator.
LocalCertFailed=Lang Switcher je instaliran, ali neće raditi u prozorima programa pokrenutih kao administrator: Windows nije prihvatio certifikat kreiran na ovom računaru. Detalje potražite u dnevniku instalacije.
RemoveSettingsQuestion=Ukloniti i datoteku postavki programa Lang Switcher?
RemoveSettingsKeep=Izaberite „Ne” da biste zadržali postavke za kasniju ponovnu instalaciju.
AuthorSlogan=programi rađeni od srca
