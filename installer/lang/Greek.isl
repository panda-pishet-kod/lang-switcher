; Lang Switcher -- the messages of the installer in Greek (stage E114, decision 208.6, 5b; 155.11 lifted).
;
; Inno Setup has no official Greek translation (inno_official = false in res\lang\languages.toml),
; so this file carries the whole of it: [LangOptions], every message of the compiler:Default.isl
; (6.5.0+), and the messages of the installer that are Lang Switcher's own. It is read after
; compiler:Default.isl as the second MessagesFile of the "greek" entry of [Languages] in
; ..\LangSwitcher.iss, and so replaces the English text name for name.
; English.isl is the sample the own messages translate; the words shared with the program are the
; ones of res\lang\el.rc. The name of the program is not translated; it is neuter, "το Lang Switcher".
; The tone is the one of Greek Windows: the polite plural ("Κάντε κλικ", "Επιλέξτε").
; As in Default.isl: a message that has no full stop there has none here (Setup adds its own).
; No accelerators (&): in a file without an official Inno translation they are left out, so that
; one window never shows two equal letters (RULES 16). The names of the buttons inside a message
; are in the quotation marks of the language, « », after the word κουμπί (RULES 16, 17).

[LangOptions]
LanguageName=Ελληνικά
LanguageID=$0408
LanguageCodePage=0

[Messages]

; *** Application titles
SetupAppTitle=Εγκατάσταση
SetupWindowTitle=Εγκατάσταση - %1
UninstallAppTitle=Κατάργηση εγκατάστασης
UninstallAppFullTitle=%1 — κατάργηση εγκατάστασης

; *** Misc. common
InformationTitle=Πληροφορίες
ConfirmTitle=Επιβεβαίωση
ErrorTitle=Σφάλμα

; *** SetupLdr messages
SetupLdrStartupMessage=Θα γίνει εγκατάσταση του %1. Θέλετε να συνεχίσετε;
LdrCannotCreateTemp=Δεν ήταν δυνατή η δημιουργία προσωρινού αρχείου. Η εγκατάσταση ματαιώθηκε
LdrCannotExecTemp=Δεν ήταν δυνατή η εκτέλεση αρχείου στον προσωρινό φάκελο. Η εγκατάσταση ματαιώθηκε
HelpTextNote=

; *** Startup error messages
LastErrorMessage=%1.%n%nΣφάλμα %2: %3
SetupFileMissing=Το αρχείο %1 λείπει από τον φάκελο εγκατάστασης. Διορθώστε το πρόβλημα ή αποκτήστε νέο αντίγραφο του προγράμματος.
SetupFileCorrupt=Τα αρχεία εγκατάστασης είναι κατεστραμμένα. Αποκτήστε νέο αντίγραφο του προγράμματος.
SetupFileCorruptOrWrongVer=Τα αρχεία εγκατάστασης είναι κατεστραμμένα ή δεν είναι συμβατά με αυτήν την έκδοση του προγράμματος εγκατάστασης. Διορθώστε το πρόβλημα ή αποκτήστε νέο αντίγραφο του προγράμματος.
InvalidParameter=Στη γραμμή εντολών δόθηκε μη έγκυρη παράμετρος:%n%n%1
SetupAlreadyRunning=Το πρόγραμμα εγκατάστασης εκτελείται ήδη.
WindowsVersionNotSupported=Αυτό το πρόγραμμα δεν υποστηρίζει την έκδοση των Windows που εκτελείται στον υπολογιστή σας.
WindowsServicePackRequired=Αυτό το πρόγραμμα απαιτεί %1 Service Pack %2 ή νεότερο.
NotOnThisPlatform=Αυτό το πρόγραμμα δεν εκτελείται σε %1.
OnlyOnThisPlatform=Αυτό το πρόγραμμα πρέπει να εκτελείται σε %1.
OnlyOnTheseArchitectures=Αυτό το πρόγραμμα μπορεί να εγκατασταθεί μόνο σε εκδόσεις των Windows που έχουν σχεδιαστεί για τις εξής αρχιτεκτονικές επεξεργαστή:%n%n%1
WinVersionTooLowError=Αυτό το πρόγραμμα απαιτεί %1 έκδοση %2 ή νεότερη.
WinVersionTooHighError=Αυτό το πρόγραμμα δεν μπορεί να εγκατασταθεί σε %1 έκδοση %2 ή νεότερη.
AdminPrivilegesRequired=Για την εγκατάσταση αυτού του προγράμματος πρέπει να έχετε συνδεθεί ως διαχειριστής.
PowerUserPrivilegesRequired=Για την εγκατάσταση αυτού του προγράμματος πρέπει να έχετε συνδεθεί ως διαχειριστής ή ως μέλος της ομάδας Power Users.
SetupAppRunningError=Το πρόγραμμα εγκατάστασης διαπίστωσε ότι το %1 εκτελείται αυτήν τη στιγμή.%n%nΚλείστε όλα τα ανοιχτά αντίγραφά του και, στη συνέχεια, κάντε κλικ στο κουμπί «OK» για να συνεχίσετε ή στο «Άκυρο» για έξοδο.
UninstallAppRunningError=Το πρόγραμμα κατάργησης εγκατάστασης διαπίστωσε ότι το %1 εκτελείται αυτήν τη στιγμή.%n%nΚλείστε όλα τα ανοιχτά αντίγραφά του και, στη συνέχεια, κάντε κλικ στο κουμπί «OK» για να συνεχίσετε ή στο «Άκυρο» για έξοδο.

; *** Startup questions
PrivilegesRequiredOverrideTitle=Επιλογή τρόπου εγκατάστασης
PrivilegesRequiredOverrideInstruction=Επιλέξτε τρόπο εγκατάστασης
PrivilegesRequiredOverrideText1=Το %1 μπορεί να εγκατασταθεί για όλους τους χρήστες (απαιτούνται δικαιώματα διαχειριστή) ή μόνο για εσάς.
PrivilegesRequiredOverrideText2=Το %1 μπορεί να εγκατασταθεί μόνο για εσάς ή για όλους τους χρήστες (απαιτούνται δικαιώματα διαχειριστή).
PrivilegesRequiredOverrideAllUsers=Εγκατάσταση για όλους τους χρήστες
PrivilegesRequiredOverrideAllUsersRecommended=Εγκατάσταση για όλους τους χρήστες (συνιστάται)
PrivilegesRequiredOverrideCurrentUser=Εγκατάσταση μόνο για εμένα
PrivilegesRequiredOverrideCurrentUserRecommended=Εγκατάσταση μόνο για εμένα (συνιστάται)

; *** Misc. errors
ErrorCreatingDir=Το πρόγραμμα εγκατάστασης δεν μπόρεσε να δημιουργήσει τον φάκελο «%1»
ErrorTooManyFilesInDir=Δεν είναι δυνατή η δημιουργία αρχείου στον φάκελο «%1», επειδή περιέχει πάρα πολλά αρχεία

; *** Setup common messages
ExitSetupTitle=Έξοδος από την εγκατάσταση
ExitSetupMessage=Η εγκατάσταση δεν ολοκληρώθηκε. Αν βγείτε τώρα, το πρόγραμμα δεν θα εγκατασταθεί.%n%nΜπορείτε να εκτελέσετε ξανά το πρόγραμμα εγκατάστασης άλλη στιγμή για να ολοκληρώσετε την εγκατάσταση.%n%nΈξοδος από την εγκατάσταση;
AboutSetupMenuItem=Πληροφορίες για την εγκατάσταση...
AboutSetupTitle=Πληροφορίες για την εγκατάσταση
AboutSetupMessage=%1 έκδοση %2%n%3%n%nΑρχική σελίδα του %1:%n%4
AboutSetupNote=
TranslatorNote=

; *** Buttons
ButtonBack=< Πίσω
ButtonNext=Επόμενο >
ButtonInstall=Εγκατάσταση
ButtonOK=OK
ButtonCancel=Άκυρο
ButtonYes=Ναι
ButtonYesToAll=Ναι σε όλα
ButtonNo=Όχι
ButtonNoToAll=Όχι σε όλα
ButtonFinish=Τέλος
ButtonBrowse=Αναζήτηση...
ButtonWizardBrowse=Αναζήτηση...
ButtonNewFolder=Δημιουργία νέου φακέλου

; *** "Select Language" dialog messages
SelectLanguageTitle=Επιλογή γλώσσας εγκατάστασης
SelectLanguageLabel=Επιλέξτε τη γλώσσα που θα χρησιμοποιηθεί κατά την εγκατάσταση.

; *** Common wizard text
ClickNext=Κάντε κλικ στο κουμπί «Επόμενο» για να συνεχίσετε ή στο «Άκυρο» για έξοδο από την εγκατάσταση.
BeveledLabel=
BrowseDialogTitle=Αναζήτηση φακέλου
BrowseDialogLabel=Επιλέξτε έναν φάκελο από την παρακάτω λίστα και, στη συνέχεια, κάντε κλικ στο κουμπί «OK».
NewFolderName=Νέος φάκελος

; *** "Welcome" wizard page
WelcomeLabel1=Καλώς ορίσατε στον οδηγό εγκατάστασης του [name]
WelcomeLabel2=Θα γίνει εγκατάσταση του [name/ver] στον υπολογιστή σας.%n%nΣυνιστάται να κλείσετε όλες τις άλλες εφαρμογές πριν συνεχίσετε.

; *** "Password" wizard page
WizardPassword=Κωδικός πρόσβασης
PasswordLabel1=Αυτή η εγκατάσταση προστατεύεται με κωδικό πρόσβασης.
PasswordLabel3=Πληκτρολογήστε τον κωδικό πρόσβασης και, στη συνέχεια, κάντε κλικ στο κουμπί «Επόμενο» για να συνεχίσετε. Στους κωδικούς πρόσβασης γίνεται διάκριση πεζών-κεφαλαίων.
PasswordEditLabel=Κωδικός πρόσβασης:
IncorrectPassword=Ο κωδικός πρόσβασης που πληκτρολογήσατε δεν είναι σωστός. Δοκιμάστε ξανά.

; *** "License Agreement" wizard page
WizardLicense=Άδεια χρήσης
LicenseLabel=Διαβάστε τις παρακάτω σημαντικές πληροφορίες πριν συνεχίσετε.
LicenseLabel3=Διαβάστε την παρακάτω άδεια χρήσης. Πρέπει να αποδεχτείτε τους όρους της πριν συνεχίσετε την εγκατάσταση.
LicenseAccepted=Αποδέχομαι τους όρους της άδειας
LicenseNotAccepted=Δεν αποδέχομαι τους όρους της άδειας

; *** "Information" wizard pages
WizardInfoBefore=Πληροφορίες
InfoBeforeLabel=Διαβάστε τις παρακάτω σημαντικές πληροφορίες πριν συνεχίσετε.
InfoBeforeClickLabel=Όταν είστε έτοιμοι να συνεχίσετε την εγκατάσταση, κάντε κλικ στο κουμπί «Επόμενο».
WizardInfoAfter=Πληροφορίες
InfoAfterLabel=Διαβάστε τις παρακάτω σημαντικές πληροφορίες πριν συνεχίσετε.
InfoAfterClickLabel=Όταν είστε έτοιμοι να συνεχίσετε την εγκατάσταση, κάντε κλικ στο κουμπί «Επόμενο».

; *** "User Information" wizard page
WizardUserInfo=Πληροφορίες χρήστη
UserInfoDesc=Πληκτρολογήστε τα στοιχεία σας.
UserInfoName=Όνομα χρήστη:
UserInfoOrg=Οργανισμός:
UserInfoSerial=Σειριακός αριθμός:
UserInfoNameRequired=Πρέπει να πληκτρολογήσετε ένα όνομα.

; *** "Select Destination Location" wizard page
WizardSelectDir=Επιλογή θέσης προορισμού
SelectDirDesc=Πού θα εγκατασταθεί το [name];
SelectDirLabel3=Το πρόγραμμα εγκατάστασης θα εγκαταστήσει το [name] στον παρακάτω φάκελο.
SelectDirBrowseLabel=Για να συνεχίσετε, κάντε κλικ στο κουμπί «Επόμενο». Αν θέλετε να επιλέξετε άλλον φάκελο, κάντε κλικ στο κουμπί «Αναζήτηση».
DiskSpaceGBLabel=Απαιτούνται τουλάχιστον [gb] GB ελεύθερου χώρου στον δίσκο.
DiskSpaceMBLabel=Απαιτούνται τουλάχιστον [mb] MB ελεύθερου χώρου στον δίσκο.
CannotInstallToNetworkDrive=Το πρόγραμμα εγκατάστασης δεν μπορεί να κάνει εγκατάσταση σε μονάδα δικτύου.
CannotInstallToUNCPath=Το πρόγραμμα εγκατάστασης δεν μπορεί να κάνει εγκατάσταση σε διαδρομή UNC.
InvalidPath=Πρέπει να πληκτρολογήσετε μια πλήρη διαδρομή με γράμμα μονάδας δίσκου, για παράδειγμα:%n%nC:\APP%n%nή μια διαδρομή UNC της μορφής:%n%n\\server\share
InvalidDrive=Η μονάδα δίσκου ή ο κοινόχρηστος πόρος UNC που επιλέξατε δεν υπάρχει ή δεν είναι προσβάσιμος. Επιλέξτε κάποιον άλλον.
DiskSpaceWarningTitle=Δεν υπάρχει αρκετός χώρος στον δίσκο
DiskSpaceWarning=Το πρόγραμμα εγκατάστασης χρειάζεται τουλάχιστον %1 KB ελεύθερου χώρου για την εγκατάσταση, αλλά η επιλεγμένη μονάδα δίσκου έχει διαθέσιμα μόνο %2 KB.%n%nΘέλετε να συνεχίσετε παρ’ όλα αυτά;
DirNameTooLong=Το όνομα του φακέλου ή η διαδρομή είναι πολύ μεγάλη.
InvalidDirName=Το όνομα του φακέλου δεν είναι έγκυρο.
BadDirName32=Τα ονόματα φακέλων δεν μπορούν να περιέχουν κανέναν από τους εξής χαρακτήρες:%n%n%1
DirExistsTitle=Ο φάκελος υπάρχει
DirExists=Ο φάκελος:%n%n%1%n%nυπάρχει ήδη. Θέλετε να γίνει η εγκατάσταση σε αυτόν τον φάκελο παρ’ όλα αυτά;
DirDoesntExistTitle=Ο φάκελος δεν υπάρχει
DirDoesntExist=Ο φάκελος:%n%n%1%n%nδεν υπάρχει. Θέλετε να δημιουργηθεί;

; *** "Select Components" wizard page
WizardSelectComponents=Επιλογή στοιχείων
SelectComponentsDesc=Ποια στοιχεία θα εγκατασταθούν;
SelectComponentsLabel2=Επιλέξτε τα στοιχεία που θέλετε να εγκαταστήσετε και καταργήστε την επιλογή όσων δεν θέλετε να εγκαταστήσετε. Κάντε κλικ στο κουμπί «Επόμενο» όταν είστε έτοιμοι να συνεχίσετε.
FullInstallation=Πλήρης εγκατάσταση
; if possible don't translate 'Compact' as 'Minimal' (I mean 'Minimal' in your language)
CompactInstallation=Συμπαγής εγκατάσταση
CustomInstallation=Προσαρμοσμένη εγκατάσταση
NoUninstallWarningTitle=Τα στοιχεία υπάρχουν
NoUninstallWarning=Το πρόγραμμα εγκατάστασης διαπίστωσε ότι τα παρακάτω στοιχεία είναι ήδη εγκατεστημένα στον υπολογιστή σας:%n%n%1%n%nΑν καταργήσετε την επιλογή τους, η εγκατάστασή τους δεν θα καταργηθεί.%n%nΘέλετε να συνεχίσετε παρ’ όλα αυτά;
ComponentSize1=%1 KB
ComponentSize2=%1 MB
ComponentsDiskSpaceGBLabel=Η τρέχουσα επιλογή απαιτεί τουλάχιστον [gb] GB χώρου στον δίσκο.
ComponentsDiskSpaceMBLabel=Η τρέχουσα επιλογή απαιτεί τουλάχιστον [mb] MB χώρου στον δίσκο.

; *** "Select Additional Tasks" wizard page
WizardSelectTasks=Επιλογή πρόσθετων εργασιών
SelectTasksDesc=Ποιες πρόσθετες εργασίες θα εκτελεστούν;
SelectTasksLabel2=Επιλέξτε τις πρόσθετες εργασίες που θέλετε να εκτελέσει το πρόγραμμα εγκατάστασης κατά την εγκατάσταση του [name] και, στη συνέχεια, κάντε κλικ στο κουμπί «Επόμενο».

; *** "Select Start Menu Folder" wizard page
WizardSelectProgramGroup=Επιλογή φακέλου στο μενού «Έναρξη»
SelectStartMenuFolderDesc=Πού θα τοποθετήσει το πρόγραμμα εγκατάστασης τις συντομεύσεις του προγράμματος;
SelectStartMenuFolderLabel3=Το πρόγραμμα εγκατάστασης θα δημιουργήσει τις συντομεύσεις του προγράμματος στον παρακάτω φάκελο του μενού «Έναρξη».
SelectStartMenuFolderBrowseLabel=Για να συνεχίσετε, κάντε κλικ στο κουμπί «Επόμενο». Αν θέλετε να επιλέξετε άλλον φάκελο, κάντε κλικ στο κουμπί «Αναζήτηση».
MustEnterGroupName=Πρέπει να πληκτρολογήσετε ένα όνομα φακέλου.
GroupNameTooLong=Το όνομα του φακέλου ή η διαδρομή είναι πολύ μεγάλη.
InvalidGroupName=Το όνομα του φακέλου δεν είναι έγκυρο.
BadGroupName=Το όνομα του φακέλου δεν μπορεί να περιέχει κανέναν από τους εξής χαρακτήρες:%n%n%1
NoProgramGroupCheck2=Να μη δημιουργηθεί φάκελος στο μενού «Έναρξη»

; *** "Ready to Install" wizard page
WizardReady=Έτοιμο για εγκατάσταση
ReadyLabel1=Το πρόγραμμα εγκατάστασης είναι έτοιμο να ξεκινήσει την εγκατάσταση του [name] στον υπολογιστή σας.
ReadyLabel2a=Κάντε κλικ στο κουμπί «Εγκατάσταση» για να συνεχίσετε την εγκατάσταση ή στο «Πίσω» αν θέλετε να ελέγξετε ή να αλλάξετε κάποιες ρυθμίσεις.
ReadyLabel2b=Κάντε κλικ στο κουμπί «Εγκατάσταση» για να συνεχίσετε την εγκατάσταση.
ReadyMemoUserInfo=Πληροφορίες χρήστη:
ReadyMemoDir=Θέση προορισμού:
ReadyMemoType=Τύπος εγκατάστασης:
ReadyMemoComponents=Επιλεγμένα στοιχεία:
ReadyMemoGroup=Φάκελος στο μενού «Έναρξη»:
ReadyMemoTasks=Πρόσθετες εργασίες:

; *** TDownloadWizardPage wizard page and DownloadTemporaryFile
DownloadingLabel2=Λήψη αρχείων...
ButtonStopDownload=Διακοπή λήψης
StopDownload=Είστε βέβαιοι ότι θέλετε να διακόψετε τη λήψη;
ErrorDownloadAborted=Η λήψη ματαιώθηκε
ErrorDownloadFailed=Η λήψη απέτυχε: %1 %2
ErrorDownloadSizeFailed=Η λήψη του μεγέθους απέτυχε: %1 %2
ErrorProgress=Μη έγκυρη πρόοδος: %1 από %2
ErrorFileSize=Μη έγκυρο μέγεθος αρχείου: αναμενόταν %1, βρέθηκε %2

; *** TExtractionWizardPage wizard page and ExtractArchive
ExtractingLabel=Αποσυμπίεση αρχείων...
ButtonStopExtraction=Διακοπή αποσυμπίεσης
StopExtraction=Είστε βέβαιοι ότι θέλετε να διακόψετε την αποσυμπίεση;
ErrorExtractionAborted=Η αποσυμπίεση ματαιώθηκε
ErrorExtractionFailed=Η αποσυμπίεση απέτυχε: %1

; *** Archive extraction failure details
ArchiveIncorrectPassword=Ο κωδικός πρόσβασης είναι λανθασμένος
ArchiveIsCorrupted=Το συμπιεσμένο αρχείο είναι κατεστραμμένο
ArchiveUnsupportedFormat=Η μορφή του συμπιεσμένου αρχείου δεν υποστηρίζεται

; *** "Preparing to Install" wizard page
WizardPreparing=Προετοιμασία εγκατάστασης
PreparingDesc=Το πρόγραμμα εγκατάστασης προετοιμάζει την εγκατάσταση του [name] στον υπολογιστή σας.
PreviousInstallNotCompleted=Η εγκατάσταση ή η κατάργηση ενός προηγούμενου προγράμματος δεν ολοκληρώθηκε. Πρέπει να επανεκκινήσετε τον υπολογιστή σας για να ολοκληρωθεί.%n%nΜετά την επανεκκίνηση του υπολογιστή, εκτελέστε ξανά το πρόγραμμα εγκατάστασης για να ολοκληρώσετε την εγκατάσταση του [name].
CannotContinue=Το πρόγραμμα εγκατάστασης δεν μπορεί να συνεχίσει. Κάντε κλικ στο κουμπί «Άκυρο» για έξοδο.
ApplicationsFound=Οι παρακάτω εφαρμογές χρησιμοποιούν αρχεία που πρέπει να ενημερωθούν από το πρόγραμμα εγκατάστασης. Συνιστάται να επιτρέψετε στο πρόγραμμα εγκατάστασης να κλείσει αυτόματα αυτές τις εφαρμογές.
ApplicationsFound2=Οι παρακάτω εφαρμογές χρησιμοποιούν αρχεία που πρέπει να ενημερωθούν από το πρόγραμμα εγκατάστασης. Συνιστάται να επιτρέψετε στο πρόγραμμα εγκατάστασης να κλείσει αυτόματα αυτές τις εφαρμογές. Μετά την ολοκλήρωση της εγκατάστασης, το πρόγραμμα εγκατάστασης θα προσπαθήσει να τις επανεκκινήσει.
CloseApplications=Αυτόματο κλείσιμο των εφαρμογών
DontCloseApplications=Να μην κλείσουν οι εφαρμογές
ErrorCloseApplications=Το πρόγραμμα εγκατάστασης δεν μπόρεσε να κλείσει αυτόματα όλες τις εφαρμογές. Πριν συνεχίσετε, συνιστάται να κλείσετε όλες τις εφαρμογές που χρησιμοποιούν αρχεία τα οποία πρέπει να ενημερωθούν από το πρόγραμμα εγκατάστασης.
PrepareToInstallNeedsRestart=Το πρόγραμμα εγκατάστασης πρέπει να επανεκκινήσει τον υπολογιστή σας. Μετά την επανεκκίνηση, εκτελέστε ξανά το πρόγραμμα εγκατάστασης για να ολοκληρώσετε την εγκατάσταση του [name].%n%nΘέλετε να γίνει επανεκκίνηση τώρα;

; *** "Installing" wizard page
WizardInstalling=Εγκατάσταση
InstallingLabel=Περιμένετε όσο το πρόγραμμα εγκατάστασης εγκαθιστά το [name] στον υπολογιστή σας.

; *** "Setup Completed" wizard page
FinishedHeadingLabel=Ολοκλήρωση του οδηγού εγκατάστασης του [name]
FinishedLabelNoIcons=Το πρόγραμμα εγκατάστασης ολοκλήρωσε την εγκατάσταση του [name] στον υπολογιστή σας.
FinishedLabel=Το πρόγραμμα εγκατάστασης ολοκλήρωσε την εγκατάσταση του [name] στον υπολογιστή σας. Μπορείτε να εκκινήσετε την εφαρμογή επιλέγοντας τις συντομεύσεις που εγκαταστάθηκαν.
ClickFinish=Κάντε κλικ στο κουμπί «Τέλος» για έξοδο από την εγκατάσταση.
FinishedRestartLabel=Για να ολοκληρωθεί η εγκατάσταση του [name], το πρόγραμμα εγκατάστασης πρέπει να επανεκκινήσει τον υπολογιστή σας. Θέλετε να γίνει επανεκκίνηση τώρα;
FinishedRestartMessage=Για να ολοκληρωθεί η εγκατάσταση του [name], το πρόγραμμα εγκατάστασης πρέπει να επανεκκινήσει τον υπολογιστή σας.%n%nΘέλετε να γίνει επανεκκίνηση τώρα;
ShowReadmeCheck=Ναι, θέλω να δω το αρχείο README
YesRadio=Ναι, να γίνει επανεκκίνηση του υπολογιστή τώρα
NoRadio=Όχι, θα κάνω επανεκκίνηση του υπολογιστή αργότερα
; used for example as 'Run MyProg.exe'
RunEntryExec=Εκτέλεση του %1
; used for example as 'View Readme.txt'
RunEntryShellExec=Προβολή του %1

; *** "Setup Needs the Next Disk" stuff
ChangeDiskTitle=Το πρόγραμμα εγκατάστασης χρειάζεται τον επόμενο δίσκο
SelectDiskLabel2=Τοποθετήστε τον δίσκο %1 και κάντε κλικ στο κουμπί «OK».%n%nΑν τα αρχεία αυτού του δίσκου βρίσκονται σε άλλον φάκελο από αυτόν που εμφανίζεται παρακάτω, πληκτρολογήστε τη σωστή διαδρομή ή κάντε κλικ στο κουμπί «Αναζήτηση».
PathLabel=Διαδρομή:
FileNotInDir2=Το αρχείο «%1» δεν βρέθηκε στο «%2». Τοποθετήστε τον σωστό δίσκο ή επιλέξτε άλλον φάκελο.
SelectDirectoryLabel=Καθορίστε τη θέση του επόμενου δίσκου.

; *** Installation phase messages
SetupAborted=Η εγκατάσταση δεν ολοκληρώθηκε.%n%nΔιορθώστε το πρόβλημα και εκτελέστε ξανά το πρόγραμμα εγκατάστασης.
AbortRetryIgnoreSelectAction=Επιλέξτε ενέργεια
AbortRetryIgnoreRetry=Νέα προσπάθεια
AbortRetryIgnoreIgnore=Παράβλεψη του σφάλματος και συνέχεια
AbortRetryIgnoreCancel=Ακύρωση της εγκατάστασης
RetryCancelSelectAction=Επιλέξτε ενέργεια
RetryCancelRetry=Νέα προσπάθεια
RetryCancelCancel=Άκυρο

; *** Installation status messages
StatusClosingApplications=Κλείσιμο εφαρμογών...
StatusCreateDirs=Δημιουργία φακέλων...
StatusExtractFiles=Αποσυμπίεση αρχείων...
StatusDownloadFiles=Λήψη αρχείων...
StatusCreateIcons=Δημιουργία συντομεύσεων...
StatusCreateIniEntries=Δημιουργία καταχωρίσεων INI...
StatusCreateRegistryEntries=Δημιουργία καταχωρίσεων μητρώου...
StatusRegisterFiles=Καταχώριση αρχείων...
StatusSavingUninstall=Αποθήκευση πληροφοριών κατάργησης εγκατάστασης...
StatusRunProgram=Ολοκλήρωση εγκατάστασης...
StatusRestartingApplications=Επανεκκίνηση εφαρμογών...
StatusRollback=Αναίρεση αλλαγών...

; *** Misc. errors
ErrorInternal2=Εσωτερικό σφάλμα: %1
ErrorFunctionFailedNoCode=Αποτυχία %1
ErrorFunctionFailed=Αποτυχία %1, κωδικός %2
ErrorFunctionFailedWithMessage=Αποτυχία %1, κωδικός %2.%n%3
ErrorExecutingProgram=Δεν είναι δυνατή η εκτέλεση του αρχείου:%n%1

; *** Registry errors
ErrorRegOpenKey=Σφάλμα κατά το άνοιγμα του κλειδιού μητρώου:%n%1\%2
ErrorRegCreateKey=Σφάλμα κατά τη δημιουργία του κλειδιού μητρώου:%n%1\%2
ErrorRegWriteKey=Σφάλμα κατά την εγγραφή στο κλειδί μητρώου:%n%1\%2

; *** INI errors
ErrorIniEntry=Σφάλμα κατά τη δημιουργία καταχώρισης INI στο αρχείο «%1».

; *** File copying errors
FileAbortRetryIgnoreSkipNotRecommended=Παράλειψη αυτού του αρχείου (δεν συνιστάται)
FileAbortRetryIgnoreIgnoreNotRecommended=Παράβλεψη του σφάλματος και συνέχεια (δεν συνιστάται)
SourceIsCorrupted=Το αρχείο προέλευσης είναι κατεστραμμένο
SourceDoesntExist=Το αρχείο προέλευσης «%1» δεν υπάρχει
SourceVerificationFailed=Η επαλήθευση του αρχείου προέλευσης απέτυχε: %1
VerificationSignatureDoesntExist=Το αρχείο υπογραφής «%1» δεν υπάρχει
VerificationSignatureInvalid=Το αρχείο υπογραφής «%1» δεν είναι έγκυρο
VerificationKeyNotFound=Το αρχείο υπογραφής «%1» χρησιμοποιεί άγνωστο κλειδί
VerificationFileNameIncorrect=Το όνομα του αρχείου είναι λανθασμένο
VerificationFileTagIncorrect=Η ετικέτα του αρχείου είναι λανθασμένη
VerificationFileSizeIncorrect=Το μέγεθος του αρχείου είναι λανθασμένο
VerificationFileHashIncorrect=Η τιμή κατακερματισμού του αρχείου είναι λανθασμένη
ExistingFileReadOnly2=Δεν ήταν δυνατή η αντικατάσταση του υπάρχοντος αρχείου, επειδή έχει επισημανθεί ως «μόνο για ανάγνωση».
ExistingFileReadOnlyRetry=Κατάργηση του χαρακτηριστικού «μόνο για ανάγνωση» και νέα προσπάθεια
ExistingFileReadOnlyKeepExisting=Διατήρηση του υπάρχοντος αρχείου
ErrorReadingExistingDest=Παρουσιάστηκε σφάλμα κατά την ανάγνωση του υπάρχοντος αρχείου:
FileExistsSelectAction=Επιλέξτε ενέργεια
FileExists2=Το αρχείο υπάρχει ήδη.
FileExistsOverwriteExisting=Αντικατάσταση του υπάρχοντος αρχείου
FileExistsKeepExisting=Διατήρηση του υπάρχοντος αρχείου
FileExistsOverwriteOrKeepAll=Να γίνει το ίδιο και στις επόμενες διενέξεις
ExistingFileNewerSelectAction=Επιλέξτε ενέργεια
ExistingFileNewer2=Το υπάρχον αρχείο είναι νεότερο από αυτό που προσπαθεί να εγκαταστήσει το πρόγραμμα εγκατάστασης.
ExistingFileNewerOverwriteExisting=Αντικατάσταση του υπάρχοντος αρχείου
ExistingFileNewerKeepExisting=Διατήρηση του υπάρχοντος αρχείου (συνιστάται)
ExistingFileNewerOverwriteOrKeepAll=Να γίνει το ίδιο και στις επόμενες διενέξεις
ErrorChangingAttr=Παρουσιάστηκε σφάλμα κατά την αλλαγή των χαρακτηριστικών του υπάρχοντος αρχείου:
ErrorCreatingTemp=Παρουσιάστηκε σφάλμα κατά τη δημιουργία αρχείου στον φάκελο προορισμού:
ErrorReadingSource=Παρουσιάστηκε σφάλμα κατά την ανάγνωση του αρχείου προέλευσης:
ErrorCopying=Παρουσιάστηκε σφάλμα κατά την αντιγραφή αρχείου:
ErrorDownloading=Παρουσιάστηκε σφάλμα κατά τη λήψη αρχείου:
ErrorExtracting=Παρουσιάστηκε σφάλμα κατά την αποσυμπίεση συμπιεσμένου αρχείου:
ErrorReplacingExistingFile=Παρουσιάστηκε σφάλμα κατά την αντικατάσταση του υπάρχοντος αρχείου:
ErrorRestartReplace=Η λειτουργία RestartReplace απέτυχε:
ErrorRenamingTemp=Παρουσιάστηκε σφάλμα κατά τη μετονομασία αρχείου στον φάκελο προορισμού:
ErrorRegisterServer=Δεν είναι δυνατή η καταχώριση του DLL/OCX: %1
ErrorRegSvr32Failed=Το RegSvr32 απέτυχε με κωδικό εξόδου %1
ErrorRegisterTypeLib=Δεν είναι δυνατή η καταχώριση της βιβλιοθήκης τύπων: %1

; *** Uninstall display name markings
; used for example as 'My Program (32-bit)'
UninstallDisplayNameMark=%1 (%2)
; used for example as 'My Program (32-bit, All users)'
UninstallDisplayNameMarks=%1 (%2, %3)
UninstallDisplayNameMark32Bit=32 bit
UninstallDisplayNameMark64Bit=64 bit
UninstallDisplayNameMarkAllUsers=Όλοι οι χρήστες
UninstallDisplayNameMarkCurrentUser=Τρέχων χρήστης

; *** Post-installation errors
ErrorOpeningReadme=Παρουσιάστηκε σφάλμα κατά το άνοιγμα του αρχείου README.
ErrorRestartingComputer=Το πρόγραμμα εγκατάστασης δεν μπόρεσε να επανεκκινήσει τον υπολογιστή. Επανεκκινήστε τον με μη αυτόματο τρόπο.

; *** Uninstaller messages
UninstallNotFound=Το αρχείο «%1» δεν υπάρχει. Δεν είναι δυνατή η κατάργηση της εγκατάστασης.
UninstallOpenError=Δεν ήταν δυνατό το άνοιγμα του αρχείου «%1». Δεν είναι δυνατή η κατάργηση της εγκατάστασης
UninstallUnsupportedVer=Το αρχείο καταγραφής κατάργησης εγκατάστασης «%1» έχει μορφή που δεν αναγνωρίζεται από αυτήν την έκδοση του προγράμματος κατάργησης εγκατάστασης. Δεν είναι δυνατή η κατάργηση της εγκατάστασης
UninstallUnknownEntry=Βρέθηκε άγνωστη καταχώριση (%1) στο αρχείο καταγραφής κατάργησης εγκατάστασης
ConfirmUninstall=Είστε βέβαιοι ότι θέλετε να καταργήσετε πλήρως το %1 και όλα τα στοιχεία του;
UninstallOnlyOnWin64=Η κατάργηση αυτής της εγκατάστασης είναι δυνατή μόνο σε Windows 64 bit.
OnlyAdminCanUninstall=Η κατάργηση αυτής της εγκατάστασης είναι δυνατή μόνο από χρήστη με δικαιώματα διαχειριστή.
UninstallStatusLabel=Περιμένετε όσο το %1 καταργείται από τον υπολογιστή σας.
UninstalledAll=Το %1 καταργήθηκε επιτυχώς από τον υπολογιστή σας.
UninstalledMost=Η κατάργηση της εγκατάστασης του %1 ολοκληρώθηκε.%n%nΟρισμένα στοιχεία δεν ήταν δυνατό να καταργηθούν. Μπορείτε να τα καταργήσετε με μη αυτόματο τρόπο.
UninstalledAndNeedsRestart=Για να ολοκληρωθεί η κατάργηση της εγκατάστασης του %1, πρέπει να γίνει επανεκκίνηση του υπολογιστή σας.%n%nΘέλετε να γίνει επανεκκίνηση τώρα;
UninstallDataCorrupted=Το αρχείο «%1» είναι κατεστραμμένο. Δεν είναι δυνατή η κατάργηση της εγκατάστασης

; *** Uninstallation phase messages
ConfirmDeleteSharedFileTitle=Κατάργηση κοινόχρηστου αρχείου;
ConfirmDeleteSharedFile2=Το σύστημα δείχνει ότι το παρακάτω κοινόχρηστο αρχείο δεν χρησιμοποιείται πλέον από κανένα πρόγραμμα. Θέλετε το πρόγραμμα κατάργησης εγκατάστασης να καταργήσει αυτό το κοινόχρηστο αρχείο;%n%nΑν κάποια προγράμματα εξακολουθούν να χρησιμοποιούν αυτό το αρχείο και αυτό καταργηθεί, τα προγράμματα αυτά ενδέχεται να μη λειτουργούν σωστά. Αν δεν είστε βέβαιοι, επιλέξτε «Όχι». Η διατήρηση του αρχείου στο σύστημά σας δεν θα προκαλέσει καμία βλάβη.
SharedFileNameLabel=Όνομα αρχείου:
SharedFileLocationLabel=Θέση:
WizardUninstalling=Κατάσταση κατάργησης εγκατάστασης
StatusUninstalling=Κατάργηση εγκατάστασης του %1...

; *** Shutdown block reasons
ShutdownBlockReasonInstallingApp=Εγκατάσταση του %1.
ShutdownBlockReasonUninstallingApp=Κατάργηση εγκατάστασης του %1.

; The custom messages below aren't used by Setup itself, but if you make
; use of them in your scripts, you'll want to translate them.

[CustomMessages]

NameAndVersion=%1 έκδοση %2
AdditionalIcons=Πρόσθετες συντομεύσεις:
CreateDesktopIcon=Δημιουργία συντόμευσης στην επιφάνεια εργασίας
CreateQuickLaunchIcon=Δημιουργία συντόμευσης στη Γρήγορη εκκίνηση
ProgramOnTheWeb=Το %1 στο Web
UninstallProgram=Κατάργηση εγκατάστασης του %1
LaunchProgram=Εκκίνηση του %1
AssocFileExtension=Συσχέτιση του %1 με την επέκταση αρχείου %2
AssocingFileExtension=Συσχέτιση του %1 με την επέκταση αρχείου %2...
AutoStartProgramGroupDescription=Εκκίνηση:
AutoStartProgram=Αυτόματη εκκίνηση του %1
AddonHostProgramNotFound=Το %1 δεν βρέθηκε στον φάκελο που επιλέξατε.%n%nΘέλετε να συνεχίσετε παρ’ όλα αυτά;

; The messages that are Lang Switcher's own (the sample is English.isl).
LocalCertReady=Το πρόγραμμα εγκατάστασης θα δημιουργήσει ένα πιστοποιητικό σε αυτόν τον υπολογιστή και θα υπογράψει με αυτό το Lang Switcher, ώστε να μπορεί να λειτουργεί και στα παράθυρα προγραμμάτων που εκτελούνται ως διαχειριστής.
LocalCertFailed=Το Lang Switcher εγκαταστάθηκε, αλλά δεν θα λειτουργεί στα παράθυρα προγραμμάτων που εκτελούνται ως διαχειριστής: τα Windows δεν αποδέχτηκαν το πιστοποιητικό που δημιουργήθηκε σε αυτόν τον υπολογιστή. Λεπτομέρειες θα βρείτε στο αρχείο καταγραφής της εγκατάστασης.
RemoveSettingsQuestion=Να καταργηθεί και το αρχείο ρυθμίσεων του Lang Switcher;
RemoveSettingsKeep=Επιλέξτε «Όχι» για να διατηρήσετε τις ρυθμίσεις σας για μια μελλοντική επανεγκατάσταση.
AuthorSlogan=προγράμματα φτιαγμένα με καρδιά
