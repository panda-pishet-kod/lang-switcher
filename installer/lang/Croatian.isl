; Lang Switcher -- the messages of the installer in Croatian (stage E109, group B).
;
; Inno Setup has no official Croatian translation (inno_official = false in res\lang\languages.toml),
; so this file carries the whole of it: [LangOptions], every message of the compiler:Default.isl
; (6.5.0+), and the messages of the installer that are Lang Switcher's own. It is read after
; compiler:Default.isl as the second MessagesFile of the "croatian" entry of [Languages] in
; ..\LangSwitcher.iss, and so replaces the English text name for name.
; English.isl is the sample the own messages translate; the glossary of the language is
; l10n\glossary\hr.tsv of the control contour. The name of the program is not translated.
; As in Default.isl: a message that has no full stop there has none here (Setup adds its own).
; No accelerators (&): in a file without an official Inno translation they are left out, so that
; one window never shows two equal letters (RULES 16). The names of the buttons inside a message
; are in the quotation marks of the language, „ ” (RULES 16). "Prečac" without "tipkovni" is the
; shortcut (.lnk) of Windows; the hotkey of the program is "tipkovni prečac" (glossary hr.tsv).

[LangOptions]
LanguageName=Hrvatski
LanguageID=$041A
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
ErrorTitle=Pogreška

; *** SetupLdr messages
SetupLdrStartupMessage=Ovime ćete instalirati program %1. Želite li nastaviti?
LdrCannotCreateTemp=Nije moguće izraditi privremenu datoteku. Instalacija je prekinuta
LdrCannotExecTemp=Nije moguće pokrenuti datoteku u privremenoj mapi. Instalacija je prekinuta
HelpTextNote=

; *** Startup error messages
LastErrorMessage=%1.%n%nPogreška %2: %3
SetupFileMissing=Datoteka %1 nedostaje u instalacijskoj mapi. Otklonite problem ili nabavite novu kopiju programa.
SetupFileCorrupt=Instalacijske datoteke su oštećene. Nabavite novu kopiju programa.
SetupFileCorruptOrWrongVer=Instalacijske datoteke su oštećene ili nisu u skladu s ovom verzijom instalacije. Otklonite problem ili nabavite novu kopiju programa.
InvalidParameter=U naredbenom retku zadan je neispravan parametar:%n%n%1
SetupAlreadyRunning=Instalacija je već pokrenuta.
WindowsVersionNotSupported=Ovaj program ne podržava verziju sustava Windows koja je pokrenuta na vašem računalu.
WindowsServicePackRequired=Ovaj program zahtijeva %1 Service Pack %2 ili noviji.
NotOnThisPlatform=Ovaj se program neće izvoditi na sustavu %1.
OnlyOnThisPlatform=Ovaj se program mora izvoditi na sustavu %1.
OnlyOnTheseArchitectures=Ovaj se program može instalirati samo na verzije sustava Windows namijenjene sljedećim arhitekturama procesora:%n%n%1
WinVersionTooLowError=Ovaj program zahtijeva %1 verzije %2 ili novije.
WinVersionTooHighError=Ovaj se program ne može instalirati na %1 verzije %2 ili novije.
AdminPrivilegesRequired=Pri instalaciji ovog programa morate biti prijavljeni kao administrator.
PowerUserPrivilegesRequired=Pri instalaciji ovog programa morate biti prijavljeni kao administrator ili kao član grupe Power Users.
SetupAppRunningError=Instalacija je otkrila da je program %1 trenutno pokrenut.%n%nZatvorite sve njegove otvorene primjerke, zatim kliknite „U redu” za nastavak ili „Odustani” za izlaz.
UninstallAppRunningError=Deinstalacija je otkrila da je program %1 trenutno pokrenut.%n%nZatvorite sve njegove otvorene primjerke, zatim kliknite „U redu” za nastavak ili „Odustani” za izlaz.

; *** Startup questions
PrivilegesRequiredOverrideTitle=Odabir načina instalacije
PrivilegesRequiredOverrideInstruction=Odaberite način instalacije
PrivilegesRequiredOverrideText1=Program %1 može se instalirati za sve korisnike (potrebna su administratorska prava) ili samo za vas.
PrivilegesRequiredOverrideText2=Program %1 može se instalirati samo za vas ili za sve korisnike (potrebna su administratorska prava).
PrivilegesRequiredOverrideAllUsers=Instaliraj za sve korisnike
PrivilegesRequiredOverrideAllUsersRecommended=Instaliraj za sve korisnike (preporučeno)
PrivilegesRequiredOverrideCurrentUser=Instaliraj samo za mene
PrivilegesRequiredOverrideCurrentUserRecommended=Instaliraj samo za mene (preporučeno)

; *** Misc. errors
ErrorCreatingDir=Instalacija nije mogla izraditi mapu „%1”
ErrorTooManyFilesInDir=Nije moguće izraditi datoteku u mapi „%1” jer ona sadrži previše datoteka

; *** Setup common messages
ExitSetupTitle=Izlaz iz instalacije
ExitSetupMessage=Instalacija nije dovršena. Ako sada izađete, program neće biti instaliran.%n%nInstalaciju možete ponovno pokrenuti kasnije kako biste je dovršili.%n%nIzaći iz instalacije?
AboutSetupMenuItem=O instalaciji...
AboutSetupTitle=O instalaciji
AboutSetupMessage=%1 verzija %2%n%3%n%n%1 – početna stranica:%n%4
AboutSetupNote=
TranslatorNote=

; *** Buttons
ButtonBack=< Natrag
ButtonNext=Dalje >
ButtonInstall=Instaliraj
ButtonOK=U redu
ButtonCancel=Odustani
ButtonYes=Da
ButtonYesToAll=Da za sve
ButtonNo=Ne
ButtonNoToAll=Ne za sve
ButtonFinish=Završi
ButtonBrowse=Pregledaj...
ButtonWizardBrowse=Pregledaj...
ButtonNewFolder=Nova mapa

; *** "Select Language" dialog messages
SelectLanguageTitle=Odabir jezika instalacije
SelectLanguageLabel=Odaberite jezik koji će se koristiti tijekom instalacije.

; *** Common wizard text
ClickNext=Kliknite „Dalje” za nastavak ili „Odustani” za izlaz iz instalacije.
BeveledLabel=
BrowseDialogTitle=Odabir mape
BrowseDialogLabel=Odaberite mapu s popisa u nastavku, zatim kliknite „U redu”.
NewFolderName=Nova mapa

; *** "Welcome" wizard page
WelcomeLabel1=Dobro došli u čarobnjak za instalaciju programa [name]
WelcomeLabel2=Ovime ćete instalirati [name/ver] na svoje računalo.%n%nPreporučuje se zatvoriti sve ostale programe prije nastavka.

; *** "Password" wizard page
WizardPassword=Lozinka
PasswordLabel1=Ova je instalacija zaštićena lozinkom.
PasswordLabel3=Unesite lozinku, zatim kliknite „Dalje” za nastavak. Lozinke razlikuju velika i mala slova.
PasswordEditLabel=Lozinka:
IncorrectPassword=Unesena lozinka nije točna. Pokušajte ponovno.

; *** "License Agreement" wizard page
WizardLicense=Licencni ugovor
LicenseLabel=Prije nastavka pažljivo pročitajte sljedeće važne informacije.
LicenseLabel3=Pročitajte sljedeći licencni ugovor. Morate prihvatiti uvjete ovog ugovora da biste nastavili instalaciju.
LicenseAccepted=Prihvaćam ugovor
LicenseNotAccepted=Ne prihvaćam ugovor

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
WizardSelectDir=Odabir odredišne mape
SelectDirDesc=Gdje želite instalirati program [name]?
SelectDirLabel3=Instalacija će program [name] instalirati u sljedeću mapu.
SelectDirBrowseLabel=Za nastavak kliknite „Dalje”. Ako želite odabrati drugu mapu, kliknite „Pregledaj”.
DiskSpaceGBLabel=Potrebno je najmanje [gb] GB slobodnog prostora na disku.
DiskSpaceMBLabel=Potrebno je najmanje [mb] MB slobodnog prostora na disku.
CannotInstallToNetworkDrive=Instalacija se ne može izvršiti na mrežni pogon.
CannotInstallToUNCPath=Instalacija se ne može izvršiti na UNC putanju.
InvalidPath=Morate unijeti punu putanju s oznakom pogona, na primjer:%n%nC:\APP%n%nili UNC putanju u obliku:%n%n\\server\share
InvalidDrive=Odabrani pogon ili UNC dijeljeni resurs ne postoji ili mu se ne može pristupiti. Odaberite drugi.
DiskSpaceWarningTitle=Nedovoljno prostora na disku
DiskSpaceWarning=Instalacija zahtijeva najmanje %1 KB slobodnog prostora, a na odabranom pogonu dostupno je samo %2 KB.%n%nŽelite li svejedno nastaviti?
DirNameTooLong=Naziv mape ili putanja predugi su.
InvalidDirName=Naziv mape nije valjan.
BadDirName32=Nazivi mapa ne smiju sadržavati nijedan od sljedećih znakova:%n%n%1
DirExistsTitle=Mapa postoji
DirExists=Mapa:%n%n%1%n%nveć postoji. Želite li svejedno instalirati u tu mapu?
DirDoesntExistTitle=Mapa ne postoji
DirDoesntExist=Mapa:%n%n%1%n%nne postoji. Želite li da se mapa izradi?

; *** "Select Components" wizard page
WizardSelectComponents=Odabir komponenti
SelectComponentsDesc=Koje komponente treba instalirati?
SelectComponentsLabel2=Odaberite komponente koje želite instalirati; poništite odabir komponenti koje ne želite instalirati. Kliknite „Dalje” kada budete spremni nastaviti.
FullInstallation=Potpuna instalacija
; if possible don't translate 'Compact' as 'Minimal' (I mean 'Minimal' in your language)
CompactInstallation=Sažeta instalacija
CustomInstallation=Prilagođena instalacija
NoUninstallWarningTitle=Komponente postoje
NoUninstallWarning=Instalacija je otkrila da su sljedeće komponente već instalirane na vašem računalu:%n%n%1%n%nPoništavanje odabira tih komponenti neće ih deinstalirati.%n%nŽelite li svejedno nastaviti?
ComponentSize1=%1 KB
ComponentSize2=%1 MB
ComponentsDiskSpaceGBLabel=Trenutni odabir zahtijeva najmanje [gb] GB prostora na disku.
ComponentsDiskSpaceMBLabel=Trenutni odabir zahtijeva najmanje [mb] MB prostora na disku.

; *** "Select Additional Tasks" wizard page
WizardSelectTasks=Odabir dodatnih zadataka
SelectTasksDesc=Koje dodatne zadatke treba izvršiti?
SelectTasksLabel2=Odaberite dodatne zadatke koje instalacija treba izvršiti tijekom instalacije programa [name], zatim kliknite „Dalje”.

; *** "Select Start Menu Folder" wizard page
WizardSelectProgramGroup=Odabir mape izbornika Start
SelectStartMenuFolderDesc=Gdje instalacija treba smjestiti prečace programa?
SelectStartMenuFolderLabel3=Instalacija će izraditi prečace programa u sljedećoj mapi izbornika Start.
SelectStartMenuFolderBrowseLabel=Za nastavak kliknite „Dalje”. Ako želite odabrati drugu mapu, kliknite „Pregledaj”.
MustEnterGroupName=Morate unijeti naziv mape.
GroupNameTooLong=Naziv mape ili putanja predugi su.
InvalidGroupName=Naziv mape nije valjan.
BadGroupName=Naziv mape ne smije sadržavati nijedan od sljedećih znakova:%n%n%1
NoProgramGroupCheck2=Ne izrađuj mapu u izborniku Start

; *** "Ready to Install" wizard page
WizardReady=Spremno za instalaciju
ReadyLabel1=Instalacija je spremna za instaliranje programa [name] na vaše računalo.
ReadyLabel2a=Kliknite „Instaliraj” za nastavak instalacije ili „Natrag” ako želite pregledati ili promijeniti neke postavke.
ReadyLabel2b=Kliknite „Instaliraj” za nastavak instalacije.
ReadyMemoUserInfo=Podaci o korisniku:
ReadyMemoDir=Odredišna mapa:
ReadyMemoType=Vrsta instalacije:
ReadyMemoComponents=Odabrane komponente:
ReadyMemoGroup=Mapa izbornika Start:
ReadyMemoTasks=Dodatni zadaci:

; *** TDownloadWizardPage wizard page and DownloadTemporaryFile
DownloadingLabel2=Preuzimanje datoteka...
ButtonStopDownload=Zaustavi preuzimanje
StopDownload=Jeste li sigurni da želite zaustaviti preuzimanje?
ErrorDownloadAborted=Preuzimanje je prekinuto
ErrorDownloadFailed=Preuzimanje nije uspjelo: %1 %2
ErrorDownloadSizeFailed=Dohvaćanje veličine nije uspjelo: %1 %2
ErrorProgress=Neispravan napredak: %1 od %2
ErrorFileSize=Neispravna veličina datoteke: očekivano %1, pronađeno %2

; *** TExtractionWizardPage wizard page and ExtractArchive
ExtractingLabel=Raspakiravanje datoteka...
ButtonStopExtraction=Zaustavi raspakiravanje
StopExtraction=Jeste li sigurni da želite zaustaviti raspakiravanje?
ErrorExtractionAborted=Raspakiravanje je prekinuto
ErrorExtractionFailed=Raspakiravanje nije uspjelo: %1

; *** Archive extraction failure details
ArchiveIncorrectPassword=Lozinka nije točna
ArchiveIsCorrupted=Arhiva je oštećena
ArchiveUnsupportedFormat=Format arhive nije podržan

; *** "Preparing to Install" wizard page
WizardPreparing=Priprema za instalaciju
PreparingDesc=Instalacija se priprema za instaliranje programa [name] na vaše računalo.
PreviousInstallNotCompleted=Instalacija ili uklanjanje prethodnog programa nije dovršeno. Morate ponovno pokrenuti računalo da biste dovršili tu instalaciju.%n%nNakon ponovnog pokretanja računala ponovno pokrenite instalaciju kako biste dovršili instalaciju programa [name].
CannotContinue=Instalacija ne može nastaviti. Kliknite „Odustani” za izlaz.
ApplicationsFound=Sljedeći programi koriste datoteke koje instalacija mora ažurirati. Preporučuje se da instalaciji dopustite da te programe automatski zatvori.
ApplicationsFound2=Sljedeći programi koriste datoteke koje instalacija mora ažurirati. Preporučuje se da instalaciji dopustite da te programe automatski zatvori. Nakon dovršetka instalacije pokušat će ih ponovno pokrenuti.
CloseApplications=Automatski zatvori programe
DontCloseApplications=Ne zatvaraj programe
ErrorCloseApplications=Instalacija nije mogla automatski zatvoriti sve programe. Prije nastavka preporučuje se zatvoriti sve programe koji koriste datoteke koje instalacija mora ažurirati.
PrepareToInstallNeedsRestart=Instalacija mora ponovno pokrenuti vaše računalo. Nakon ponovnog pokretanja ponovno pokrenite instalaciju kako biste dovršili instalaciju programa [name].%n%nŽelite li ponovno pokrenuti računalo sada?

; *** "Installing" wizard page
WizardInstalling=Instaliranje
InstallingLabel=Pričekajte dok instalacija instalira program [name] na vaše računalo.

; *** "Setup Completed" wizard page
FinishedHeadingLabel=Završetak čarobnjaka za instalaciju programa [name]
FinishedLabelNoIcons=Instalacija je dovršila instaliranje programa [name] na vaše računalo.
FinishedLabel=Instalacija je dovršila instaliranje programa [name] na vaše računalo. Program možete pokrenuti pomoću instaliranih prečaca.
ClickFinish=Kliknite „Završi” za izlaz iz instalacije.
FinishedRestartLabel=Za dovršetak instalacije programa [name] instalacija mora ponovno pokrenuti vaše računalo. Želite li to učiniti sada?
FinishedRestartMessage=Za dovršetak instalacije programa [name] instalacija mora ponovno pokrenuti vaše računalo.%n%nŽelite li to učiniti sada?
ShowReadmeCheck=Da, želim pregledati datoteku README
YesRadio=Da, ponovno pokreni računalo sada
NoRadio=Ne, računalo ću ponovno pokrenuti kasnije
; used for example as 'Run MyProg.exe'
RunEntryExec=Pokreni %1
; used for example as 'View Readme.txt'
RunEntryShellExec=Pregledaj %1

; *** "Setup Needs the Next Disk" stuff
ChangeDiskTitle=Instalacija traži sljedeći disk
SelectDiskLabel2=Umetnite disk %1 i kliknite „U redu”.%n%nAko se datoteke s ovog diska nalaze u drugoj mapi od one prikazane u nastavku, unesite ispravnu putanju ili kliknite „Pregledaj”.
PathLabel=Putanja:
FileNotInDir2=Datoteka „%1” nije pronađena u „%2”. Umetnite ispravan disk ili odaberite drugu mapu.
SelectDirectoryLabel=Odredite mjesto sljedećeg diska.

; *** Installation phase messages
SetupAborted=Instalacija nije dovršena.%n%nOtklonite problem i ponovno pokrenite instalaciju.
AbortRetryIgnoreSelectAction=Odaberite radnju
AbortRetryIgnoreRetry=Pokušaj ponovno
AbortRetryIgnoreIgnore=Zanemari pogrešku i nastavi
AbortRetryIgnoreCancel=Odustani od instalacije
RetryCancelSelectAction=Odaberite radnju
RetryCancelRetry=Pokušaj ponovno
RetryCancelCancel=Odustani

; *** Installation status messages
StatusClosingApplications=Zatvaranje programa...
StatusCreateDirs=Izrada mapa...
StatusExtractFiles=Raspakiravanje datoteka...
StatusDownloadFiles=Preuzimanje datoteka...
StatusCreateIcons=Izrada prečaca...
StatusCreateIniEntries=Izrada INI zapisa...
StatusCreateRegistryEntries=Izrada zapisa u registru...
StatusRegisterFiles=Registriranje datoteka...
StatusSavingUninstall=Spremanje podataka za deinstalaciju...
StatusRunProgram=Završavanje instalacije...
StatusRestartingApplications=Ponovno pokretanje programa...
StatusRollback=Poništavanje promjena...

; *** Misc. errors
ErrorInternal2=Interna pogreška: %1
ErrorFunctionFailedNoCode=%1 nije uspjelo
ErrorFunctionFailed=%1 nije uspjelo; šifra %2
ErrorFunctionFailedWithMessage=%1 nije uspjelo; šifra %2.%n%3
ErrorExecutingProgram=Nije moguće pokrenuti datoteku:%n%1

; *** Registry errors
ErrorRegOpenKey=Pogreška pri otvaranju ključa registra:%n%1\%2
ErrorRegCreateKey=Pogreška pri izradi ključa registra:%n%1\%2
ErrorRegWriteKey=Pogreška pri pisanju u ključ registra:%n%1\%2

; *** INI errors
ErrorIniEntry=Pogreška pri izradi INI zapisa u datoteci „%1”.

; *** File copying errors
FileAbortRetryIgnoreSkipNotRecommended=Preskoči ovu datoteku (ne preporučuje se)
FileAbortRetryIgnoreIgnoreNotRecommended=Zanemari pogrešku i nastavi (ne preporučuje se)
SourceIsCorrupted=Izvorna datoteka je oštećena
SourceDoesntExist=Izvorna datoteka „%1” ne postoji
SourceVerificationFailed=Provjera izvorne datoteke nije uspjela: %1
VerificationSignatureDoesntExist=Datoteka potpisa „%1” ne postoji
VerificationSignatureInvalid=Datoteka potpisa „%1” nije valjana
VerificationKeyNotFound=Datoteka potpisa „%1” koristi nepoznat ključ
VerificationFileNameIncorrect=Naziv datoteke nije ispravan
VerificationFileTagIncorrect=Oznaka datoteke nije ispravna
VerificationFileSizeIncorrect=Veličina datoteke nije ispravna
VerificationFileHashIncorrect=Hash datoteke nije ispravan
ExistingFileReadOnly2=Postojeću datoteku nije bilo moguće zamijeniti jer je označena samo za čitanje.
ExistingFileReadOnlyRetry=Ukloni oznaku samo za čitanje i pokušaj ponovno
ExistingFileReadOnlyKeepExisting=Zadrži postojeću datoteku
ErrorReadingExistingDest=Došlo je do pogreške pri čitanju postojeće datoteke:
FileExistsSelectAction=Odaberite radnju
FileExists2=Datoteka već postoji.
FileExistsOverwriteExisting=Prepiši postojeću datoteku
FileExistsKeepExisting=Zadrži postojeću datoteku
FileExistsOverwriteOrKeepAll=Primijeni na sljedeće sukobe
ExistingFileNewerSelectAction=Odaberite radnju
ExistingFileNewer2=Postojeća datoteka novija je od one koju instalacija pokušava instalirati.
ExistingFileNewerOverwriteExisting=Prepiši postojeću datoteku
ExistingFileNewerKeepExisting=Zadrži postojeću datoteku (preporučeno)
ExistingFileNewerOverwriteOrKeepAll=Primijeni na sljedeće sukobe
ErrorChangingAttr=Došlo je do pogreške pri promjeni atributa postojeće datoteke:
ErrorCreatingTemp=Došlo je do pogreške pri izradi datoteke u odredišnoj mapi:
ErrorReadingSource=Došlo je do pogreške pri čitanju izvorne datoteke:
ErrorCopying=Došlo je do pogreške pri kopiranju datoteke:
ErrorDownloading=Došlo je do pogreške pri preuzimanju datoteke:
ErrorExtracting=Došlo je do pogreške pri raspakiravanju arhive:
ErrorReplacingExistingFile=Došlo je do pogreške pri zamjeni postojeće datoteke:
ErrorRestartReplace=RestartReplace nije uspio:
ErrorRenamingTemp=Došlo je do pogreške pri preimenovanju datoteke u odredišnoj mapi:
ErrorRegisterServer=Nije moguće registrirati DLL/OCX: %1
ErrorRegSvr32Failed=RegSvr32 nije uspio s izlaznim kodom %1
ErrorRegisterTypeLib=Nije moguće registrirati biblioteku tipova: %1

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
ErrorOpeningReadme=Došlo je do pogreške pri otvaranju datoteke README.
ErrorRestartingComputer=Instalacija nije mogla ponovno pokrenuti računalo. Učinite to ručno.

; *** Uninstaller messages
UninstallNotFound=Datoteka „%1” ne postoji. Deinstalacija nije moguća.
UninstallOpenError=Datoteku „%1” nije moguće otvoriti. Deinstalacija nije moguća
UninstallUnsupportedVer=Datoteka dnevnika deinstalacije „%1” u formatu je koji ova verzija deinstalacijskog programa ne prepoznaje. Deinstalacija nije moguća
UninstallUnknownEntry=U dnevniku deinstalacije pronađen je nepoznat zapis (%1)
ConfirmUninstall=Jeste li sigurni da želite potpuno ukloniti program %1 i sve njegove komponente?
UninstallOnlyOnWin64=Ova se instalacija može deinstalirati samo na 64-bitnom sustavu Windows.
OnlyAdminCanUninstall=Ovu instalaciju može deinstalirati samo korisnik s administratorskim pravima.
UninstallStatusLabel=Pričekajte dok se program %1 uklanja s vašeg računala.
UninstalledAll=Program %1 uspješno je uklonjen s vašeg računala.
UninstalledMost=Deinstalacija programa %1 je dovršena.%n%nNeke elemente nije bilo moguće ukloniti. Možete ih ukloniti ručno.
UninstalledAndNeedsRestart=Za dovršetak deinstalacije programa %1 potrebno je ponovno pokrenuti računalo.%n%nŽelite li to učiniti sada?
UninstallDataCorrupted=Datoteka „%1” je oštećena. Deinstalacija nije moguća

; *** Uninstallation phase messages
ConfirmDeleteSharedFileTitle=Ukloniti dijeljenu datoteku?
ConfirmDeleteSharedFile2=Sustav pokazuje da sljedeću dijeljenu datoteku više ne koristi nijedan program. Želite li da je deinstalacija ukloni?%n%nAko neki programi još koriste tu datoteku, a ona se ukloni, ti programi možda neće ispravno raditi. Ako niste sigurni, odaberite „Ne”. Ostavljanje datoteke u sustavu neće uzrokovati štetu.
SharedFileNameLabel=Naziv datoteke:
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
AdditionalIcons=Dodatni prečaci:
CreateDesktopIcon=Izradi prečac na radnoj površini
CreateQuickLaunchIcon=Izradi prečac za brzo pokretanje
ProgramOnTheWeb=%1 na webu
UninstallProgram=Deinstaliraj %1
LaunchProgram=Pokreni %1
AssocFileExtension=Pridruži program %1 datotečnom nastavku %2
AssocingFileExtension=Pridruživanje programa %1 datotečnom nastavku %2...
AutoStartProgramGroupDescription=Pokretanje sa sustavom:
AutoStartProgram=Automatski pokreni %1
AddonHostProgramNotFound=Program %1 nije pronađen u odabranoj mapi.%n%nŽelite li svejedno nastaviti?

; *** Lang Switcher's own messages (the sample is English.isl)
LocalCertReady=Instalacija će na ovom računalu izraditi certifikat i njime potpisati Lang Switcher kako bi program radio i u prozorima programa pokrenutih kao administrator.
LocalCertFailed=Lang Switcher je instaliran, ali neće raditi u prozorima programa pokrenutih kao administrator: Windows nije prihvatio certifikat izrađen na ovom računalu. Pojedinosti potražite u dnevniku instalacije.
RemoveSettingsQuestion=Ukloniti i datoteku postavki programa Lang Switcher?
RemoveSettingsKeep=Odaberite „Ne” da biste zadržali postavke za kasniju ponovnu instalaciju.
AuthorSlogan=programi rađeni od srca
