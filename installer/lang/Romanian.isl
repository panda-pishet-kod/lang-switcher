; Lang Switcher -- the messages of the installer in Romanian (stage E109, group B).
;
; Inno Setup has no official Romanian translation (inno_official = false in res\lang\languages.toml),
; so this file carries the whole of it: [LangOptions], every message of the compiler:Default.isl
; (6.5.0+), and the messages of the installer that are Lang Switcher's own. It is read after
; compiler:Default.isl as the second MessagesFile of the "romanian" entry of [Languages] in
; ..\LangSwitcher.iss, and so replaces the English text name for name.
; English.isl is the sample the own messages translate; the glossary of the language is
; l10n\glossary\ro.tsv of the control contour. The name of the program is not translated.
; As in Default.isl: a message that has no full stop there has none here (Setup adds its own).
; No "&" accelerators: Romanian has no official file, so one window never carries two equal
; letters; the names of buttons inside a message stand in the quotation marks of the language,
; „ ”, and are written as the buttons say them (RULES 16).

[LangOptions]
LanguageName=Română
LanguageID=$0418
LanguageCodePage=0

[Messages]

; *** Application titles
SetupAppTitle=Instalare
SetupWindowTitle=Instalare - %1
UninstallAppTitle=Dezinstalare
UninstallAppFullTitle=Dezinstalare %1

; *** Misc. common
InformationTitle=Informații
ConfirmTitle=Confirmare
ErrorTitle=Eroare

; *** SetupLdr messages
SetupLdrStartupMessage=Se va instala %1. Doriți să continuați?
LdrCannotCreateTemp=Nu s-a putut crea un fișier temporar. Instalarea a fost întreruptă
LdrCannotExecTemp=Nu s-a putut rula un fișier din directorul temporar. Instalarea a fost întreruptă
HelpTextNote=

; *** Startup error messages
LastErrorMessage=%1.%n%nEroare %2: %3
SetupFileMissing=Fișierul %1 lipsește din directorul de instalare. Remediați problema sau obțineți o copie nouă a programului.
SetupFileCorrupt=Fișierele de instalare sunt deteriorate. Obțineți o copie nouă a programului.
SetupFileCorruptOrWrongVer=Fișierele de instalare sunt deteriorate sau nu sunt compatibile cu această versiune a programului de instalare. Remediați problema sau obțineți o copie nouă a programului.
InvalidParameter=În linia de comandă a fost transmis un parametru nevalid:%n%n%1
SetupAlreadyRunning=Programul de instalare rulează deja.
WindowsVersionNotSupported=Acest program nu acceptă versiunea de Windows de pe computerul dvs.
WindowsServicePackRequired=Acest program necesită %1 Service Pack %2 sau o versiune ulterioară.
NotOnThisPlatform=Acest program nu va rula pe %1.
OnlyOnThisPlatform=Acest program trebuie rulat pe %1.
OnlyOnTheseArchitectures=Acest program poate fi instalat doar pe versiuni de Windows proiectate pentru următoarele arhitecturi de procesor:%n%n%1
WinVersionTooLowError=Acest program necesită %1 versiunea %2 sau o versiune ulterioară.
WinVersionTooHighError=Acest program nu poate fi instalat pe %1 versiunea %2 sau pe o versiune ulterioară.
AdminPrivilegesRequired=Pentru a instala acest program, trebuie să vă conectați ca administrator.
PowerUserPrivilegesRequired=Pentru a instala acest program, trebuie să vă conectați ca administrator sau ca membru al grupului Power Users.
SetupAppRunningError=Programul de instalare a detectat că %1 rulează în acest moment.%n%nÎnchideți acum toate instanțele lui, apoi faceți clic pe „OK” pentru a continua sau pe „Anulare” pentru a ieși.
UninstallAppRunningError=Programul de dezinstalare a detectat că %1 rulează în acest moment.%n%nÎnchideți acum toate instanțele lui, apoi faceți clic pe „OK” pentru a continua sau pe „Anulare” pentru a ieși.

; *** Startup questions
PrivilegesRequiredOverrideTitle=Alegerea modului de instalare
PrivilegesRequiredOverrideInstruction=Alegeți modul de instalare
PrivilegesRequiredOverrideText1=%1 poate fi instalat pentru toți utilizatorii (necesită drepturi de administrator) sau doar pentru dvs.
PrivilegesRequiredOverrideText2=%1 poate fi instalat doar pentru dvs. sau pentru toți utilizatorii (necesită drepturi de administrator).
PrivilegesRequiredOverrideAllUsers=Instalare pentru toți utilizatorii
PrivilegesRequiredOverrideAllUsersRecommended=Instalare pentru toți utilizatorii (recomandat)
PrivilegesRequiredOverrideCurrentUser=Instalare doar pentru mine
PrivilegesRequiredOverrideCurrentUserRecommended=Instalare doar pentru mine (recomandat)

; *** Misc. errors
ErrorCreatingDir=Programul de instalare nu a putut crea directorul „%1”
ErrorTooManyFilesInDir=Nu s-a putut crea un fișier în directorul „%1”, deoarece acesta conține prea multe fișiere

; *** Setup common messages
ExitSetupTitle=Ieșire din instalare
ExitSetupMessage=Instalarea nu este terminată. Dacă ieșiți acum, programul nu va fi instalat.%n%nPuteți rula din nou programul de instalare mai târziu pentru a finaliza instalarea.%n%nIeșiți din instalare?
AboutSetupMenuItem=Despre programul de instalare...
AboutSetupTitle=Despre programul de instalare
AboutSetupMessage=%1 versiunea %2%n%3%n%nPagina web %1:%n%4
AboutSetupNote=
TranslatorNote=

; *** Buttons
ButtonBack=< Înapoi
ButtonNext=Următorul >
ButtonInstall=Instalare
ButtonOK=OK
ButtonCancel=Anulare
ButtonYes=Da
ButtonYesToAll=Da pentru toate
ButtonNo=Nu
ButtonNoToAll=Nu pentru toate
ButtonFinish=Terminare
ButtonBrowse=Răsfoire...
ButtonWizardBrowse=Răsfoire...
ButtonNewFolder=Creare folder nou

; *** "Select Language" dialog messages
SelectLanguageTitle=Alegerea limbii de instalare
SelectLanguageLabel=Alegeți limba folosită în timpul instalării.

; *** Common wizard text
ClickNext=Faceți clic pe „Următorul” pentru a continua sau pe „Anulare” pentru a ieși din instalare.
BeveledLabel=
BrowseDialogTitle=Răsfoire după folder
BrowseDialogLabel=Selectați un folder din lista de mai jos, apoi faceți clic pe „OK”.
NewFolderName=Folder nou

; *** "Welcome" wizard page
WelcomeLabel1=Bun venit la expertul de instalare pentru [name]
WelcomeLabel2=Se va instala [name/ver] pe computerul dvs.%n%nVă recomandăm să închideți toate celelalte aplicații înainte de a continua.

; *** "Password" wizard page
WizardPassword=Parolă
PasswordLabel1=Această instalare este protejată cu parolă.
PasswordLabel3=Introduceți parola, apoi faceți clic pe „Următorul” pentru a continua. Parolele țin cont de majuscule și minuscule.
PasswordEditLabel=Parolă:
IncorrectPassword=Parola introdusă nu este corectă. Încercați din nou.

; *** "License Agreement" wizard page
WizardLicense=Contract de licență
LicenseLabel=Citiți următoarele informații importante înainte de a continua.
LicenseLabel3=Citiți următorul contract de licență. Trebuie să acceptați termenii acestui contract înainte de a continua instalarea.
LicenseAccepted=Accept contractul
LicenseNotAccepted=Nu accept contractul

; *** "Information" wizard pages
WizardInfoBefore=Informații
InfoBeforeLabel=Citiți următoarele informații importante înainte de a continua.
InfoBeforeClickLabel=Când sunteți gata să continuați instalarea, faceți clic pe „Următorul”.
WizardInfoAfter=Informații
InfoAfterLabel=Citiți următoarele informații importante înainte de a continua.
InfoAfterClickLabel=Când sunteți gata să continuați instalarea, faceți clic pe „Următorul”.

; *** "User Information" wizard page
WizardUserInfo=Informații despre utilizator
UserInfoDesc=Introduceți informațiile dvs.
UserInfoName=Nume utilizator:
UserInfoOrg=Organizație:
UserInfoSerial=Număr de serie:
UserInfoNameRequired=Trebuie să introduceți un nume.

; *** "Select Destination Location" wizard page
WizardSelectDir=Alegerea locației de destinație
SelectDirDesc=Unde trebuie instalat [name]?
SelectDirLabel3=Programul de instalare va instala [name] în folderul următor.
SelectDirBrowseLabel=Pentru a continua, faceți clic pe „Următorul”. Dacă doriți să alegeți un alt folder, faceți clic pe „Răsfoire”.
DiskSpaceGBLabel=Este necesar cel puțin [gb] GB de spațiu liber pe disc.
DiskSpaceMBLabel=Este necesar cel puțin [mb] MB de spațiu liber pe disc.
CannotInstallToNetworkDrive=Programul de instalare nu poate instala pe o unitate de rețea.
CannotInstallToUNCPath=Programul de instalare nu poate instala într-o cale UNC.
InvalidPath=Trebuie să introduceți o cale completă cu litera unității; de exemplu:%n%nC:\APP%n%nsau o cale UNC de forma:%n%n\\server\share
InvalidDrive=Unitatea sau resursa UNC aleasă nu există sau nu este accesibilă. Alegeți alta.
DiskSpaceWarningTitle=Spațiu insuficient pe disc
DiskSpaceWarning=Instalarea necesită cel puțin %1 KB de spațiu liber, dar unitatea aleasă are doar %2 KB disponibili.%n%nDoriți să continuați oricum?
DirNameTooLong=Numele folderului sau calea sunt prea lungi.
InvalidDirName=Numele folderului nu este valid.
BadDirName32=Numele folderelor nu pot conține niciunul dintre următoarele caractere:%n%n%1
DirExistsTitle=Folderul există
DirExists=Folderul:%n%n%1%n%nexistă deja. Doriți să instalați oricum în acest folder?
DirDoesntExistTitle=Folderul nu există
DirDoesntExist=Folderul:%n%n%1%n%nnu există. Doriți ca folderul să fie creat?

; *** "Select Components" wizard page
WizardSelectComponents=Alegerea componentelor
SelectComponentsDesc=Ce componente trebuie instalate?
SelectComponentsLabel2=Bifați componentele pe care doriți să le instalați și debifați-le pe cele pe care nu doriți să le instalați. Faceți clic pe „Următorul” când sunteți gata să continuați.
FullInstallation=Instalare completă
; if possible don't translate 'Compact' as 'Minimal' (I mean 'Minimal' in your language)
CompactInstallation=Instalare compactă
CustomInstallation=Instalare particularizată
NoUninstallWarningTitle=Componentele există
NoUninstallWarning=Programul de instalare a detectat că următoarele componente sunt deja instalate pe computerul dvs.:%n%n%1%n%nDebifarea acestor componente nu le va dezinstala.%n%nDoriți să continuați oricum?
ComponentSize1=%1 KB
ComponentSize2=%1 MB
ComponentsDiskSpaceGBLabel=Selecția curentă necesită cel puțin [gb] GB de spațiu pe disc.
ComponentsDiskSpaceMBLabel=Selecția curentă necesită cel puțin [mb] MB de spațiu pe disc.

; *** "Select Additional Tasks" wizard page
WizardSelectTasks=Alegerea activităților suplimentare
SelectTasksDesc=Ce activități suplimentare trebuie efectuate?
SelectTasksLabel2=Alegeți activitățile suplimentare pe care doriți să le efectueze programul de instalare în timpul instalării [name], apoi faceți clic pe „Următorul”.

; *** "Select Start Menu Folder" wizard page
WizardSelectProgramGroup=Alegerea folderului din meniul Start
SelectStartMenuFolderDesc=Unde trebuie să plaseze programul de instalare comenzile rapide ale programului?
SelectStartMenuFolderLabel3=Programul de instalare va crea comenzile rapide ale programului în următorul folder din meniul Start.
SelectStartMenuFolderBrowseLabel=Pentru a continua, faceți clic pe „Următorul”. Dacă doriți să alegeți un alt folder, faceți clic pe „Răsfoire”.
MustEnterGroupName=Trebuie să introduceți un nume de folder.
GroupNameTooLong=Numele folderului sau calea sunt prea lungi.
InvalidGroupName=Numele folderului nu este valid.
BadGroupName=Numele folderului nu poate conține niciunul dintre următoarele caractere:%n%n%1
NoProgramGroupCheck2=Nu se creează un folder în meniul Start

; *** "Ready to Install" wizard page
WizardReady=Gata de instalare
ReadyLabel1=Programul de instalare este gata să înceapă instalarea [name] pe computerul dvs.
ReadyLabel2a=Faceți clic pe „Instalare” pentru a continua instalarea sau pe „Înapoi” dacă doriți să examinați sau să modificați setările.
ReadyLabel2b=Faceți clic pe „Instalare” pentru a continua instalarea.
ReadyMemoUserInfo=Informații despre utilizator:
ReadyMemoDir=Locația de destinație:
ReadyMemoType=Tipul de instalare:
ReadyMemoComponents=Componente alese:
ReadyMemoGroup=Folder în meniul Start:
ReadyMemoTasks=Activități suplimentare:

; *** TDownloadWizardPage wizard page and DownloadTemporaryFile
DownloadingLabel2=Se descarcă fișierele...
ButtonStopDownload=Oprire descărcare
StopDownload=Sigur doriți să opriți descărcarea?
ErrorDownloadAborted=Descărcarea a fost întreruptă
ErrorDownloadFailed=Descărcarea a eșuat: %1 %2
ErrorDownloadSizeFailed=Obținerea dimensiunii a eșuat: %1 %2
ErrorProgress=Progres nevalid: %1 din %2
ErrorFileSize=Dimensiune de fișier nevalidă: așteptat %1, găsit %2

; *** TExtractionWizardPage wizard page and ExtractArchive
ExtractingLabel=Se extrag fișierele...
ButtonStopExtraction=Oprire extragere
StopExtraction=Sigur doriți să opriți extragerea?
ErrorExtractionAborted=Extragerea a fost întreruptă
ErrorExtractionFailed=Extragerea a eșuat: %1

; *** Archive extraction failure details
ArchiveIncorrectPassword=Parola este incorectă
ArchiveIsCorrupted=Arhiva este deteriorată
ArchiveUnsupportedFormat=Formatul arhivei nu este acceptat

; *** "Preparing to Install" wizard page
WizardPreparing=Pregătire pentru instalare
PreparingDesc=Programul de instalare se pregătește să instaleze [name] pe computerul dvs.
PreviousInstallNotCompleted=Instalarea/eliminarea unui program anterior nu a fost finalizată. Trebuie să reporniți computerul pentru a finaliza acea instalare.%n%nDupă repornirea computerului, rulați din nou programul de instalare pentru a finaliza instalarea [name].
CannotContinue=Programul de instalare nu poate continua. Faceți clic pe „Anulare” pentru a ieși.
ApplicationsFound=Următoarele aplicații folosesc fișiere care trebuie actualizate de programul de instalare. Vă recomandăm să permiteți programului de instalare să închidă automat aceste aplicații.
ApplicationsFound2=Următoarele aplicații folosesc fișiere care trebuie actualizate de programul de instalare. Vă recomandăm să permiteți programului de instalare să închidă automat aceste aplicații. După finalizarea instalării, programul de instalare va încerca să repornească aplicațiile.
CloseApplications=Închide automat aplicațiile
DontCloseApplications=Nu închide aplicațiile
ErrorCloseApplications=Programul de instalare nu a putut închide automat toate aplicațiile. Vă recomandăm să închideți, înainte de a continua, toate aplicațiile care folosesc fișiere ce trebuie actualizate de programul de instalare.
PrepareToInstallNeedsRestart=Programul de instalare trebuie să repornească computerul. După repornire, rulați din nou programul de instalare pentru a finaliza instalarea [name].%n%nDoriți să reporniți acum?

; *** "Installing" wizard page
WizardInstalling=Se instalează
InstallingLabel=Așteptați până când programul de instalare instalează [name] pe computerul dvs.

; *** "Setup Completed" wizard page
FinishedHeadingLabel=Finalizarea expertului de instalare pentru [name]
FinishedLabelNoIcons=Programul de instalare a terminat instalarea [name] pe computerul dvs.
FinishedLabel=Programul de instalare a terminat instalarea [name] pe computerul dvs. Aplicația poate fi lansată folosind comenzile rapide instalate.
ClickFinish=Faceți clic pe „Terminare” pentru a ieși din instalare.
FinishedRestartLabel=Pentru a finaliza instalarea [name], programul de instalare trebuie să repornească computerul. Doriți să reporniți acum?
FinishedRestartMessage=Pentru a finaliza instalarea [name], programul de instalare trebuie să repornească computerul.%n%nDoriți să reporniți acum?
ShowReadmeCheck=Da, doresc să văd fișierul README
YesRadio=Da, repornește computerul acum
NoRadio=Nu, voi reporni computerul mai târziu
; used for example as 'Run MyProg.exe'
RunEntryExec=Rulare %1
; used for example as 'View Readme.txt'
RunEntryShellExec=Vizualizare %1

; *** "Setup Needs the Next Disk" stuff
ChangeDiskTitle=Programul de instalare are nevoie de discul următor
SelectDiskLabel2=Introduceți discul %1 și faceți clic pe „OK”.%n%nDacă fișierele de pe acest disc se află într-un alt folder decât cel afișat mai jos, introduceți calea corectă sau faceți clic pe „Răsfoire”.
PathLabel=Cale:
FileNotInDir2=Fișierul „%1” nu a putut fi găsit în „%2”. Introduceți discul corect sau alegeți alt folder.
SelectDirectoryLabel=Specificați locația discului următor.

; *** Installation phase messages
SetupAborted=Instalarea nu a fost finalizată.%n%nRemediați problema și rulați din nou programul de instalare.
AbortRetryIgnoreSelectAction=Alegeți acțiunea
AbortRetryIgnoreRetry=Încercare din nou
AbortRetryIgnoreIgnore=Ignorarea erorii și continuare
AbortRetryIgnoreCancel=Anularea instalării
RetryCancelSelectAction=Alegeți acțiunea
RetryCancelRetry=Încercare din nou
RetryCancelCancel=Anulare

; *** Installation status messages
StatusClosingApplications=Se închid aplicațiile...
StatusCreateDirs=Se creează directoarele...
StatusExtractFiles=Se extrag fișierele...
StatusDownloadFiles=Se descarcă fișierele...
StatusCreateIcons=Se creează comenzile rapide...
StatusCreateIniEntries=Se creează intrările INI...
StatusCreateRegistryEntries=Se creează intrările din registry...
StatusRegisterFiles=Se înregistrează fișierele...
StatusSavingUninstall=Se salvează informațiile de dezinstalare...
StatusRunProgram=Se finalizează instalarea...
StatusRestartingApplications=Se repornesc aplicațiile...
StatusRollback=Se anulează modificările...

; *** Misc. errors
ErrorInternal2=Eroare internă: %1
ErrorFunctionFailedNoCode=%1 a eșuat
ErrorFunctionFailed=%1 a eșuat; cod %2
ErrorFunctionFailedWithMessage=%1 a eșuat; cod %2.%n%3
ErrorExecutingProgram=Nu s-a putut executa fișierul:%n%1

; *** Registry errors
ErrorRegOpenKey=Eroare la deschiderea cheii de registry:%n%1\%2
ErrorRegCreateKey=Eroare la crearea cheii de registry:%n%1\%2
ErrorRegWriteKey=Eroare la scrierea în cheia de registry:%n%1\%2

; *** INI errors
ErrorIniEntry=Eroare la crearea intrării INI în fișierul „%1”.

; *** File copying errors
FileAbortRetryIgnoreSkipNotRecommended=Omiterea acestui fișier (nerecomandat)
FileAbortRetryIgnoreIgnoreNotRecommended=Ignorarea erorii și continuare (nerecomandat)
SourceIsCorrupted=Fișierul sursă este deteriorat
SourceDoesntExist=Fișierul sursă „%1” nu există
SourceVerificationFailed=Verificarea fișierului sursă a eșuat: %1
VerificationSignatureDoesntExist=Fișierul de semnătură „%1” nu există
VerificationSignatureInvalid=Fișierul de semnătură „%1” nu este valid
VerificationKeyNotFound=Fișierul de semnătură „%1” folosește o cheie necunoscută
VerificationFileNameIncorrect=Numele fișierului este incorect
VerificationFileTagIncorrect=Eticheta fișierului este incorectă
VerificationFileSizeIncorrect=Dimensiunea fișierului este incorectă
VerificationFileHashIncorrect=Valoarea hash a fișierului este incorectă
ExistingFileReadOnly2=Fișierul existent nu a putut fi înlocuit deoarece este marcat doar în citire.
ExistingFileReadOnlyRetry=Eliminarea atributului doar în citire și încercare din nou
ExistingFileReadOnlyKeepExisting=Păstrarea fișierului existent
ErrorReadingExistingDest=A apărut o eroare la citirea fișierului existent:
FileExistsSelectAction=Alegeți acțiunea
FileExists2=Fișierul există deja.
FileExistsOverwriteExisting=Suprascrierea fișierului existent
FileExistsKeepExisting=Păstrarea fișierului existent
FileExistsOverwriteOrKeepAll=Aceeași acțiune pentru conflictele următoare
ExistingFileNewerSelectAction=Alegeți acțiunea
ExistingFileNewer2=Fișierul existent este mai nou decât cel pe care încearcă să-l instaleze programul de instalare.
ExistingFileNewerOverwriteExisting=Suprascrierea fișierului existent
ExistingFileNewerKeepExisting=Păstrarea fișierului existent (recomandat)
ExistingFileNewerOverwriteOrKeepAll=Aceeași acțiune pentru conflictele următoare
ErrorChangingAttr=A apărut o eroare la modificarea atributelor fișierului existent:
ErrorCreatingTemp=A apărut o eroare la crearea unui fișier în directorul de destinație:
ErrorReadingSource=A apărut o eroare la citirea fișierului sursă:
ErrorCopying=A apărut o eroare la copierea unui fișier:
ErrorDownloading=A apărut o eroare la descărcarea unui fișier:
ErrorExtracting=A apărut o eroare la extragerea unei arhive:
ErrorReplacingExistingFile=A apărut o eroare la înlocuirea fișierului existent:
ErrorRestartReplace=RestartReplace a eșuat:
ErrorRenamingTemp=A apărut o eroare la redenumirea unui fișier în directorul de destinație:
ErrorRegisterServer=Nu s-a putut înregistra DLL/OCX: %1
ErrorRegSvr32Failed=RegSvr32 a eșuat cu codul de ieșire %1
ErrorRegisterTypeLib=Nu s-a putut înregistra biblioteca de tipuri: %1

; *** Uninstall display name markings
; used for example as 'My Program (32-bit)'
UninstallDisplayNameMark=%1 (%2)
; used for example as 'My Program (32-bit, All users)'
UninstallDisplayNameMarks=%1 (%2, %3)
UninstallDisplayNameMark32Bit=32 de biți
UninstallDisplayNameMark64Bit=64 de biți
UninstallDisplayNameMarkAllUsers=Toți utilizatorii
UninstallDisplayNameMarkCurrentUser=Utilizatorul curent

; *** Post-installation errors
ErrorOpeningReadme=A apărut o eroare la deschiderea fișierului README.
ErrorRestartingComputer=Programul de instalare nu a putut reporni computerul. Reporniți-l manual.

; *** Uninstaller messages
UninstallNotFound=Fișierul „%1” nu există. Dezinstalarea nu este posibilă.
UninstallOpenError=Fișierul „%1” nu a putut fi deschis. Dezinstalarea nu este posibilă
UninstallUnsupportedVer=Fișierul jurnal de dezinstalare „%1” are un format nerecunoscut de această versiune a programului de dezinstalare. Dezinstalarea nu este posibilă
UninstallUnknownEntry=În jurnalul de dezinstalare a fost întâlnită o intrare necunoscută (%1)
ConfirmUninstall=Sigur doriți să eliminați complet %1 și toate componentele sale?
UninstallOnlyOnWin64=Această instalare poate fi dezinstalată doar pe Windows pe 64 de biți.
OnlyAdminCanUninstall=Această instalare poate fi dezinstalată doar de un utilizator cu drepturi de administrator.
UninstallStatusLabel=Așteptați până când %1 este eliminat de pe computerul dvs.
UninstalledAll=%1 a fost eliminat cu succes de pe computerul dvs.
UninstalledMost=Dezinstalarea %1 s-a terminat.%n%nUnele elemente nu au putut fi eliminate. Le puteți elimina manual.
UninstalledAndNeedsRestart=Pentru a finaliza dezinstalarea %1, computerul trebuie repornit.%n%nDoriți să reporniți acum?
UninstallDataCorrupted=Fișierul „%1” este deteriorat. Dezinstalarea nu este posibilă

; *** Uninstallation phase messages
ConfirmDeleteSharedFileTitle=Eliminați fișierul partajat?
ConfirmDeleteSharedFile2=Sistemul indică faptul că următorul fișier partajat nu mai este folosit de niciun program. Doriți ca programul de dezinstalare să elimine acest fișier partajat?%n%nDacă mai există programe care folosesc acest fișier și acesta este eliminat, acele programe pot să nu mai funcționeze corect. Dacă nu sunteți sigur, alegeți „Nu”. Păstrarea fișierului în sistem nu produce niciun rău.
SharedFileNameLabel=Nume fișier:
SharedFileLocationLabel=Locație:
WizardUninstalling=Starea dezinstalării
StatusUninstalling=Se dezinstalează %1...

; *** Shutdown block reasons
ShutdownBlockReasonInstallingApp=Se instalează %1.
ShutdownBlockReasonUninstallingApp=Se dezinstalează %1.

; The custom messages below aren't used by Setup itself, but if you make
; use of them in your scripts, you'll want to translate them.

[CustomMessages]

NameAndVersion=%1 versiunea %2
AdditionalIcons=Comenzi rapide suplimentare:
CreateDesktopIcon=Creare comandă rapidă pe desktop
CreateQuickLaunchIcon=Creare comandă rapidă în Lansare rapidă
ProgramOnTheWeb=%1 pe web
UninstallProgram=Dezinstalare %1
LaunchProgram=Lansare %1
AssocFileExtension=Asocierea %1 cu extensia de fișier %2
AssocingFileExtension=Se asociază %1 cu extensia de fișier %2...
AutoStartProgramGroupDescription=Pornire:
AutoStartProgram=Pornire automată %1
AddonHostProgramNotFound=%1 nu a putut fi găsit în folderul ales.%n%nDoriți să continuați oricum?

; *** The messages of the installer that are Lang Switcher's own
LocalCertReady=Programul de instalare va crea pe acest computer un certificat și va semna cu el Lang Switcher, astfel încât programul să funcționeze și în ferestrele programelor rulate ca administrator.
LocalCertFailed=Lang Switcher a fost instalat, dar nu va funcționa în ferestrele programelor rulate ca administrator: Windows nu a acceptat certificatul creat pe acest computer. Detalii găsiți în jurnalul de instalare.
RemoveSettingsQuestion=Eliminați și fișierul de setări Lang Switcher?
RemoveSettingsKeep=Alegeți „Nu” pentru a păstra setările pentru o reinstalare ulterioară.
AuthorSlogan=programe făcute cu suflet
