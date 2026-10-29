; Lang Switcher -- the messages of the installer in Khmer (stage E108, group A).
;
; Inno Setup has no official Khmer translation (inno_official = false in res\lang\languages.toml),
; so this file carries the whole of it: [LangOptions], every message of the compiler:Default.isl
; (6.5.0+), and the messages of the installer that are Lang Switcher's own. It is read after
; compiler:Default.isl as the second MessagesFile of the "khmer" entry of [Languages] in
; ..\LangSwitcher.iss, and so replaces the English text name for name.
; English.isl is the sample the own messages translate; the glossary of the language is
; l10n\glossary\km.tsv of the control contour. The name of the program is not translated.
; As in Default.isl: a message that has no full stop there has no khan here (Setup adds its own).
; No "&" accelerators: Khmer has no official file, so one window never carries two equal letters;
; the names of buttons inside a message stand in «» (they are written as the buttons say them).

[LangOptions]
LanguageName=ខ្មែរ
LanguageID=$0453
LanguageCodePage=0

[Messages]

; *** Application titles
SetupAppTitle=ការដំឡើង
SetupWindowTitle=ការដំឡើង - %1
UninstallAppTitle=ការលុបកម្មវិធី
UninstallAppFullTitle=ការលុប %1

; *** Misc. common
InformationTitle=ព័ត៌មាន
ConfirmTitle=បញ្ជាក់
ErrorTitle=កំហុស

; *** SetupLdr messages
SetupLdrStartupMessage=វានឹងដំឡើង %1។ តើអ្នកចង់បន្តទេ?
LdrCannotCreateTemp=មិនអាចបង្កើតឯកសារបណ្ដោះអាសន្នបានទេ។ ការដំឡើងត្រូវបានបោះបង់
LdrCannotExecTemp=មិនអាចដំណើរការឯកសារក្នុងថតបណ្ដោះអាសន្នបានទេ។ ការដំឡើងត្រូវបានបោះបង់
HelpTextNote=

; *** Startup error messages
LastErrorMessage=%1.%n%nកំហុស %2: %3
SetupFileMissing=ឯកសារ %1 បាត់ពីថតដំឡើង។ សូមដោះស្រាយបញ្ហា ឬទទួលយកច្បាប់ចម្លងថ្មីនៃកម្មវិធី។
SetupFileCorrupt=ឯកសារដំឡើងខូច។ សូមទទួលយកច្បាប់ចម្លងថ្មីនៃកម្មវិធី។
SetupFileCorruptOrWrongVer=ឯកសារដំឡើងខូច ឬមិនត្រូវគ្នាជាមួយកំណែនៃការដំឡើងនេះ។ សូមដោះស្រាយបញ្ហា ឬទទួលយកច្បាប់ចម្លងថ្មីនៃកម្មវិធី។
InvalidParameter=ប៉ារ៉ាម៉ែត្រមិនត្រឹមត្រូវត្រូវបានបញ្ជូនក្នុងបន្ទាត់ពាក្យបញ្ជា:%n%n%1
SetupAlreadyRunning=ការដំឡើងកំពុងដំណើរការរួចហើយ។
WindowsVersionNotSupported=កម្មវិធីនេះមិនគាំទ្រកំណែ Windows ដែលកុំព្យូទ័ររបស់អ្នកកំពុងប្រើទេ។
WindowsServicePackRequired=កម្មវិធីនេះត្រូវការ %1 Service Pack %2 ឬថ្មីជាងនេះ។
NotOnThisPlatform=កម្មវិធីនេះនឹងមិនដំណើរការលើ %1 ទេ។
OnlyOnThisPlatform=កម្មវិធីនេះត្រូវតែដំណើរការលើ %1។
OnlyOnTheseArchitectures=កម្មវិធីនេះអាចដំឡើងបានតែលើកំណែ Windows ដែលរចនាសម្រាប់ស្ថាបត្យកម្មប្រូសេសឺរខាងក្រោម:%n%n%1
WinVersionTooLowError=កម្មវិធីនេះត្រូវការ %1 កំណែ %2 ឬថ្មីជាងនេះ។
WinVersionTooHighError=កម្មវិធីនេះមិនអាចដំឡើងលើ %1 កំណែ %2 ឬថ្មីជាងនេះបានទេ។
AdminPrivilegesRequired=អ្នកត្រូវតែចូលគណនីជាអ្នកគ្រប់គ្រង នៅពេលដំឡើងកម្មវិធីនេះ។
PowerUserPrivilegesRequired=អ្នកត្រូវតែចូលគណនីជាអ្នកគ្រប់គ្រង ឬជាសមាជិកនៃក្រុម Power Users នៅពេលដំឡើងកម្មវិធីនេះ។
SetupAppRunningError=ការដំឡើងបានរកឃើញថា %1 កំពុងដំណើរការ។%n%nសូមបិទវាទាំងអស់ឥឡូវនេះ រួចចុច «យល់ព្រម» ដើម្បីបន្ត ឬ «បោះបង់» ដើម្បីចាកចេញ។
UninstallAppRunningError=ការលុបកម្មវិធីបានរកឃើញថា %1 កំពុងដំណើរការ។%n%nសូមបិទវាទាំងអស់ឥឡូវនេះ រួចចុច «យល់ព្រម» ដើម្បីបន្ត ឬ «បោះបង់» ដើម្បីចាកចេញ។

; *** Startup questions
PrivilegesRequiredOverrideTitle=ជ្រើសរើសរបៀបដំឡើង
PrivilegesRequiredOverrideInstruction=ជ្រើសរើសរបៀបដំឡើង
PrivilegesRequiredOverrideText1=%1 អាចដំឡើងសម្រាប់អ្នកប្រើទាំងអស់ (ត្រូវការសិទ្ធិអ្នកគ្រប់គ្រង) ឬសម្រាប់តែអ្នក។
PrivilegesRequiredOverrideText2=%1 អាចដំឡើងសម្រាប់តែអ្នក ឬសម្រាប់អ្នកប្រើទាំងអស់ (ត្រូវការសិទ្ធិអ្នកគ្រប់គ្រង)។
PrivilegesRequiredOverrideAllUsers=ដំឡើងសម្រាប់អ្នកប្រើទាំងអស់
PrivilegesRequiredOverrideAllUsersRecommended=ដំឡើងសម្រាប់អ្នកប្រើទាំងអស់ (បានណែនាំ)
PrivilegesRequiredOverrideCurrentUser=ដំឡើងសម្រាប់តែខ្ញុំ
PrivilegesRequiredOverrideCurrentUserRecommended=ដំឡើងសម្រាប់តែខ្ញុំ (បានណែនាំ)

; *** Misc. errors
ErrorCreatingDir=ការដំឡើងមិនអាចបង្កើតថត "%1" បានទេ
ErrorTooManyFilesInDir=មិនអាចបង្កើតឯកសារក្នុងថត "%1" បានទេ ព្រោះវាមានឯកសារច្រើនពេក

; *** Setup common messages
ExitSetupTitle=ចាកចេញពីការដំឡើង
ExitSetupMessage=ការដំឡើងមិនទាន់ពេញលេញទេ។ បើអ្នកចាកចេញឥឡូវនេះ កម្មវិធីនឹងមិនត្រូវបានដំឡើងទេ។%n%nអ្នកអាចដំណើរការការដំឡើងម្ដងទៀតនៅពេលផ្សេង ដើម្បីបញ្ចប់ការដំឡើង។%n%nចាកចេញពីការដំឡើងឬ?
AboutSetupMenuItem=អំពីការដំឡើង...
AboutSetupTitle=អំពីការដំឡើង
AboutSetupMessage=%1 កំណែ %2%n%3%n%nគេហទំព័រ %1:%n%4
AboutSetupNote=
TranslatorNote=

; *** Buttons
ButtonBack=< ថយក្រោយ
ButtonNext=បន្ទាប់ >
ButtonInstall=ដំឡើង
ButtonOK=យល់ព្រម
ButtonCancel=បោះបង់
ButtonYes=បាទ/ចាស
ButtonYesToAll=បាទ/ចាសទាំងអស់
ButtonNo=ទេ
ButtonNoToAll=ទេទាំងអស់
ButtonFinish=បញ្ចប់
ButtonBrowse=រកមើល...
ButtonWizardBrowse=រកមើល...
ButtonNewFolder=បង្កើតថតថ្មី

; *** "Select Language" dialog messages
SelectLanguageTitle=ជ្រើសរើសភាសាដំឡើង
SelectLanguageLabel=ជ្រើសរើសភាសាដែលត្រូវប្រើអំឡុងពេលដំឡើង។

; *** Common wizard text
ClickNext=ចុច «បន្ទាប់» ដើម្បីបន្ត ឬ «បោះបង់» ដើម្បីចាកចេញពីការដំឡើង។
BeveledLabel=
BrowseDialogTitle=រកមើលថត
BrowseDialogLabel=ជ្រើសរើសថតក្នុងបញ្ជីខាងក្រោម រួចចុច «យល់ព្រម»។
NewFolderName=ថតថ្មី

; *** "Welcome" wizard page
WelcomeLabel1=ស្វាគមន៍មកកាន់អ្នកជំនួយដំឡើង [name]
WelcomeLabel2=វានឹងដំឡើង [name/ver] លើកុំព្យូទ័ររបស់អ្នក។%n%nត្រូវបានណែនាំឱ្យបិទកម្មវិធីផ្សេងទាំងអស់មុនបន្ត។

; *** "Password" wizard page
WizardPassword=ពាក្យសម្ងាត់
PasswordLabel1=ការដំឡើងនេះត្រូវបានការពារដោយពាក្យសម្ងាត់។
PasswordLabel3=សូមផ្ដល់ពាក្យសម្ងាត់ រួចចុច «បន្ទាប់» ដើម្បីបន្ត។ ពាក្យសម្ងាត់ញែកអក្សរធំអក្សរតូច។
PasswordEditLabel=ពាក្យសម្ងាត់:
IncorrectPassword=ពាក្យសម្ងាត់ដែលអ្នកបានបញ្ចូលមិនត្រឹមត្រូវទេ។ សូមព្យាយាមម្ដងទៀត។

; *** "License Agreement" wizard page
WizardLicense=កិច្ចព្រមព្រៀងអាជ្ញាប័ណ្ណ
LicenseLabel=សូមអានព័ត៌មានសំខាន់ខាងក្រោមមុនបន្ត។
LicenseLabel3=សូមអានកិច្ចព្រមព្រៀងអាជ្ញាប័ណ្ណខាងក្រោម។ អ្នកត្រូវតែទទួលយកលក្ខខណ្ឌនៃកិច្ចព្រមព្រៀងនេះមុននឹងបន្តការដំឡើង។
LicenseAccepted=ខ្ញុំទទួលយកកិច្ចព្រមព្រៀង
LicenseNotAccepted=ខ្ញុំមិនទទួលយកកិច្ចព្រមព្រៀងទេ

; *** "Information" wizard pages
WizardInfoBefore=ព័ត៌មាន
InfoBeforeLabel=សូមអានព័ត៌មានសំខាន់ខាងក្រោមមុនបន្ត។
InfoBeforeClickLabel=នៅពេលអ្នកត្រៀមបន្តការដំឡើង សូមចុច «បន្ទាប់»។
WizardInfoAfter=ព័ត៌មាន
InfoAfterLabel=សូមអានព័ត៌មានសំខាន់ខាងក្រោមមុនបន្ត។
InfoAfterClickLabel=នៅពេលអ្នកត្រៀមបន្តការដំឡើង សូមចុច «បន្ទាប់»។

; *** "User Information" wizard page
WizardUserInfo=ព័ត៌មានអ្នកប្រើ
UserInfoDesc=សូមបញ្ចូលព័ត៌មានរបស់អ្នក។
UserInfoName=ឈ្មោះអ្នកប្រើ:
UserInfoOrg=ស្ថាប័ន:
UserInfoSerial=លេខស៊េរី:
UserInfoNameRequired=អ្នកត្រូវតែបញ្ចូលឈ្មោះ។

; *** "Select Destination Location" wizard page
WizardSelectDir=ជ្រើសរើសទីតាំងគោលដៅ
SelectDirDesc=តើគួរដំឡើង [name] នៅឯណា?
SelectDirLabel3=ការដំឡើងនឹងដំឡើង [name] ទៅក្នុងថតខាងក្រោម។
SelectDirBrowseLabel=ដើម្បីបន្ត សូមចុច «បន្ទាប់»។ ប្រសិនបើអ្នកចង់ជ្រើសរើសថតផ្សេង សូមចុច «រកមើល»។
DiskSpaceGBLabel=ត្រូវការទំហំថាសទំនេរយ៉ាងតិច [gb] GB។
DiskSpaceMBLabel=ត្រូវការទំហំថាសទំនេរយ៉ាងតិច [mb] MB។
CannotInstallToNetworkDrive=ការដំឡើងមិនអាចដំឡើងទៅដ្រាយបណ្ដាញបានទេ។
CannotInstallToUNCPath=ការដំឡើងមិនអាចដំឡើងទៅផ្លូវ UNC បានទេ។
InvalidPath=អ្នកត្រូវតែបញ្ចូលផ្លូវពេញលេញដែលមានអក្សរដ្រាយ ឧទាហរណ៍:%n%nC:\APP%n%nឬផ្លូវ UNC ក្នុងទម្រង់:%n%n\\server\share
InvalidDrive=ដ្រាយ ឬការចែករំលែក UNC ដែលអ្នកបានជ្រើសរើសមិនមាន ឬមិនអាចចូលដំណើរការបានទេ។ សូមជ្រើសរើសផ្សេង។
DiskSpaceWarningTitle=ទំហំថាសមិនគ្រប់គ្រាន់
DiskSpaceWarning=ការដំឡើងត្រូវការទំហំទំនេរយ៉ាងតិច %1 KB ប៉ុន្តែដ្រាយដែលបានជ្រើសរើសមានតែ %2 KB ប៉ុណ្ណោះ។%n%nតើអ្នកចង់បន្តទោះយ៉ាងណាក៏ដោយឬ?
DirNameTooLong=ឈ្មោះថត ឬផ្លូវវែងពេក។
InvalidDirName=ឈ្មោះថតមិនត្រឹមត្រូវ។
BadDirName32=ឈ្មោះថតមិនអាចមានតួអក្សរណាមួយខាងក្រោមទេ:%n%n%1
DirExistsTitle=មានថតរួចហើយ
DirExists=ថត:%n%n%1%n%nមានរួចហើយ។ តើអ្នកចង់ដំឡើងទៅក្នុងថតនោះទោះយ៉ាងណាក៏ដោយឬ?
DirDoesntExistTitle=មិនមានថត
DirDoesntExist=ថត:%n%n%1%n%nមិនមានទេ។ តើអ្នកចង់បង្កើតថតនោះឬ?

; *** "Select Components" wizard page
WizardSelectComponents=ជ្រើសរើសសមាសភាគ
SelectComponentsDesc=តើគួរដំឡើងសមាសភាគអ្វីខ្លះ?
SelectComponentsLabel2=ជ្រើសរើសសមាសភាគដែលអ្នកចង់ដំឡើង; ដោះធីកសមាសភាគដែលអ្នកមិនចង់ដំឡើង។ ចុច «បន្ទាប់» នៅពេលអ្នកត្រៀមបន្ត។
FullInstallation=ការដំឡើងពេញលេញ
; if possible don't translate 'Compact' as 'Minimal' (I mean 'Minimal' in your language)
CompactInstallation=ការដំឡើងខ្លី
CustomInstallation=ការដំឡើងតាមបំណង
NoUninstallWarningTitle=មានសមាសភាគរួចហើយ
NoUninstallWarning=ការដំឡើងបានរកឃើញថាសមាសភាគខាងក្រោមត្រូវបានដំឡើងរួចហើយលើកុំព្យូទ័ររបស់អ្នក:%n%n%1%n%nការដោះធីកសមាសភាគទាំងនេះនឹងមិនលុបពួកវាទេ។%n%nតើអ្នកចង់បន្តទោះយ៉ាងណាក៏ដោយឬ?
ComponentSize1=%1 KB
ComponentSize2=%1 MB
ComponentsDiskSpaceGBLabel=ជម្រើសបច្ចុប្បន្នត្រូវការទំហំថាសយ៉ាងតិច [gb] GB។
ComponentsDiskSpaceMBLabel=ជម្រើសបច្ចុប្បន្នត្រូវការទំហំថាសយ៉ាងតិច [mb] MB។

; *** "Select Additional Tasks" wizard page
WizardSelectTasks=ជ្រើសរើសកិច្ចការបន្ថែម
SelectTasksDesc=តើគួរធ្វើកិច្ចការបន្ថែមអ្វីខ្លះ?
SelectTasksLabel2=ជ្រើសរើសកិច្ចការបន្ថែមដែលអ្នកចង់ឱ្យការដំឡើងធ្វើអំឡុងពេលដំឡើង [name] រួចចុច «បន្ទាប់»។

; *** "Select Start Menu Folder" wizard page
WizardSelectProgramGroup=ជ្រើសរើសថតម៉ឺនុយចាប់ផ្ដើម
SelectStartMenuFolderDesc=តើគួរដាក់ផ្លូវកាត់របស់កម្មវិធីនៅឯណា?
SelectStartMenuFolderLabel3=ការដំឡើងនឹងបង្កើតផ្លូវកាត់របស់កម្មវិធីក្នុងថតម៉ឺនុយចាប់ផ្ដើមខាងក្រោម។
SelectStartMenuFolderBrowseLabel=ដើម្បីបន្ត សូមចុច «បន្ទាប់»។ ប្រសិនបើអ្នកចង់ជ្រើសរើសថតផ្សេង សូមចុច «រកមើល»។
MustEnterGroupName=អ្នកត្រូវតែបញ្ចូលឈ្មោះថត។
GroupNameTooLong=ឈ្មោះថត ឬផ្លូវវែងពេក។
InvalidGroupName=ឈ្មោះថតមិនត្រឹមត្រូវ។
BadGroupName=ឈ្មោះថតមិនអាចមានតួអក្សរណាមួយខាងក្រោមទេ:%n%n%1
NoProgramGroupCheck2=កុំបង្កើតថតម៉ឺនុយចាប់ផ្ដើម

; *** "Ready to Install" wizard page
WizardReady=ត្រៀមដំឡើង
ReadyLabel1=ការដំឡើងត្រៀមរួចរាល់ដើម្បីចាប់ផ្ដើមដំឡើង [name] លើកុំព្យូទ័ររបស់អ្នក។
ReadyLabel2a=ចុច «ដំឡើង» ដើម្បីបន្តការដំឡើង ឬចុច «ថយក្រោយ» បើអ្នកចង់ពិនិត្យ ឬផ្លាស់ប្ដូរការកំណត់ណាមួយ។
ReadyLabel2b=ចុច «ដំឡើង» ដើម្បីបន្តការដំឡើង។
ReadyMemoUserInfo=ព័ត៌មានអ្នកប្រើ:
ReadyMemoDir=ទីតាំងគោលដៅ:
ReadyMemoType=ប្រភេទការដំឡើង:
ReadyMemoComponents=សមាសភាគដែលបានជ្រើសរើស:
ReadyMemoGroup=ថតម៉ឺនុយចាប់ផ្ដើម:
ReadyMemoTasks=កិច្ចការបន្ថែម:

; *** TDownloadWizardPage wizard page and DownloadTemporaryFile
DownloadingLabel2=កំពុងទាញយកឯកសារ...
ButtonStopDownload=បញ្ឈប់ការទាញយក
StopDownload=តើអ្នកប្រាកដថាចង់បញ្ឈប់ការទាញយកឬ?
ErrorDownloadAborted=ការទាញយកត្រូវបានបោះបង់
ErrorDownloadFailed=ការទាញយកបរាជ័យ: %1 %2
ErrorDownloadSizeFailed=ការទទួលទំហំបរាជ័យ: %1 %2
ErrorProgress=វឌ្ឍនភាពមិនត្រឹមត្រូវ: %1 នៃ %2
ErrorFileSize=ទំហំឯកសារមិនត្រឹមត្រូវ: រំពឹងថា %1 រកឃើញ %2

; *** TExtractionWizardPage wizard page and ExtractArchive
ExtractingLabel=កំពុងស្រង់ឯកសារ...
ButtonStopExtraction=បញ្ឈប់ការស្រង់
StopExtraction=តើអ្នកប្រាកដថាចង់បញ្ឈប់ការស្រង់ឬ?
ErrorExtractionAborted=ការស្រង់ត្រូវបានបោះបង់
ErrorExtractionFailed=ការស្រង់បរាជ័យ: %1

; *** Archive extraction failure details
ArchiveIncorrectPassword=ពាក្យសម្ងាត់មិនត្រឹមត្រូវ
ArchiveIsCorrupted=បណ្ណសារខូច
ArchiveUnsupportedFormat=មិនគាំទ្រទម្រង់បណ្ណសារ

; *** "Preparing to Install" wizard page
WizardPreparing=កំពុងត្រៀមដំឡើង
PreparingDesc=ការដំឡើងកំពុងត្រៀមដំឡើង [name] លើកុំព្យូទ័ររបស់អ្នក។
PreviousInstallNotCompleted=ការដំឡើង ឬការលុបកម្មវិធីមុនមិនទាន់បញ្ចប់ទេ។ អ្នកត្រូវចាប់ផ្ដើមកុំព្យូទ័រឡើងវិញដើម្បីបញ្ចប់ការដំឡើងនោះ។%n%nបន្ទាប់ពីចាប់ផ្ដើមកុំព្យូទ័រឡើងវិញ សូមដំណើរការការដំឡើងម្ដងទៀតដើម្បីបញ្ចប់ការដំឡើង [name]។
CannotContinue=ការដំឡើងមិនអាចបន្តបានទេ។ សូមចុច «បោះបង់» ដើម្បីចាកចេញ។
ApplicationsFound=កម្មវិធីខាងក្រោមកំពុងប្រើឯកសារដែលការដំឡើងត្រូវធ្វើបច្ចុប្បន្នភាព។ ត្រូវបានណែនាំឱ្យអនុញ្ញាតឱ្យការដំឡើងបិទកម្មវិធីទាំងនេះដោយស្វ័យប្រវត្តិ។
ApplicationsFound2=កម្មវិធីខាងក្រោមកំពុងប្រើឯកសារដែលការដំឡើងត្រូវធ្វើបច្ចុប្បន្នភាព។ ត្រូវបានណែនាំឱ្យអនុញ្ញាតឱ្យការដំឡើងបិទកម្មវិធីទាំងនេះដោយស្វ័យប្រវត្តិ។ បន្ទាប់ពីការដំឡើងបញ្ចប់ ការដំឡើងនឹងព្យាយាមចាប់ផ្ដើមកម្មវិធីទាំងនេះឡើងវិញ។
CloseApplications=បិទកម្មវិធីដោយស្វ័យប្រវត្តិ
DontCloseApplications=កុំបិទកម្មវិធី
ErrorCloseApplications=ការដំឡើងមិនអាចបិទកម្មវិធីទាំងអស់ដោយស្វ័យប្រវត្តិបានទេ។ ត្រូវបានណែនាំឱ្យបិទកម្មវិធីទាំងអស់ដែលកំពុងប្រើឯកសារដែលការដំឡើងត្រូវធ្វើបច្ចុប្បន្នភាពមុនបន្ត។
PrepareToInstallNeedsRestart=ការដំឡើងត្រូវចាប់ផ្ដើមកុំព្យូទ័ររបស់អ្នកឡើងវិញ។ បន្ទាប់ពីចាប់ផ្ដើមកុំព្យូទ័រឡើងវិញ សូមដំណើរការការដំឡើងម្ដងទៀតដើម្បីបញ្ចប់ការដំឡើង [name]។%n%nតើអ្នកចង់ចាប់ផ្ដើមឡើងវិញឥឡូវនេះទេ?

; *** "Installing" wizard page
WizardInstalling=កំពុងដំឡើង
InstallingLabel=សូមរង់ចាំខណៈពេលការដំឡើងកំពុងដំឡើង [name] លើកុំព្យូទ័ររបស់អ្នក។

; *** "Setup Completed" wizard page
FinishedHeadingLabel=កំពុងបញ្ចប់អ្នកជំនួយដំឡើង [name]
FinishedLabelNoIcons=ការដំឡើងបានដំឡើង [name] លើកុំព្យូទ័ររបស់អ្នករួចរាល់។
FinishedLabel=ការដំឡើងបានដំឡើង [name] លើកុំព្យូទ័ររបស់អ្នករួចរាល់។ អ្នកអាចបើកកម្មវិធីបានដោយជ្រើសរើសផ្លូវកាត់ដែលបានដំឡើង។
ClickFinish=ចុច «បញ្ចប់» ដើម្បីចាកចេញពីការដំឡើង។
FinishedRestartLabel=ដើម្បីបញ្ចប់ការដំឡើង [name] ការដំឡើងត្រូវចាប់ផ្ដើមកុំព្យូទ័ររបស់អ្នកឡើងវិញ។ តើអ្នកចង់ចាប់ផ្ដើមឡើងវិញឥឡូវនេះទេ?
FinishedRestartMessage=ដើម្បីបញ្ចប់ការដំឡើង [name] ការដំឡើងត្រូវចាប់ផ្ដើមកុំព្យូទ័ររបស់អ្នកឡើងវិញ។%n%nតើអ្នកចង់ចាប់ផ្ដើមឡើងវិញឥឡូវនេះទេ?
ShowReadmeCheck=បាទ/ចាស ខ្ញុំចង់មើលឯកសារ README
YesRadio=បាទ/ចាស ចាប់ផ្ដើមកុំព្យូទ័រឡើងវិញឥឡូវនេះ
NoRadio=ទេ ខ្ញុំនឹងចាប់ផ្ដើមកុំព្យូទ័រឡើងវិញនៅពេលក្រោយ
; used for example as 'Run MyProg.exe'
RunEntryExec=ដំណើរការ %1
; used for example as 'View Readme.txt'
RunEntryShellExec=មើល %1

; *** "Setup Needs the Next Disk" stuff
ChangeDiskTitle=ការដំឡើងត្រូវការថាសបន្ទាប់
SelectDiskLabel2=សូមបញ្ចូលថាស %1 រួចចុច «យល់ព្រម»។%n%nបើឯកសារលើថាសនេះអាចរកឃើញក្នុងថតផ្សេងពីថតដែលបង្ហាញខាងក្រោម សូមបញ្ចូលផ្លូវត្រឹមត្រូវ ឬចុច «រកមើល»។
PathLabel=ផ្លូវ:
FileNotInDir2=មិនអាចរកឃើញឯកសារ "%1" ក្នុង "%2" ទេ។ សូមបញ្ចូលថាសត្រឹមត្រូវ ឬជ្រើសរើសថតផ្សេង។
SelectDirectoryLabel=សូមបញ្ជាក់ទីតាំងនៃថាសបន្ទាប់។

; *** Installation phase messages
SetupAborted=ការដំឡើងមិនបានបញ្ចប់ទេ។%n%nសូមដោះស្រាយបញ្ហា រួចដំណើរការការដំឡើងម្ដងទៀត។
AbortRetryIgnoreSelectAction=ជ្រើសរើសសកម្មភាព
AbortRetryIgnoreRetry=ព្យាយាមម្ដងទៀត
AbortRetryIgnoreIgnore=មិនអើពើកំហុស ហើយបន្ត
AbortRetryIgnoreCancel=បោះបង់ការដំឡើង
RetryCancelSelectAction=ជ្រើសរើសសកម្មភាព
RetryCancelRetry=ព្យាយាមម្ដងទៀត
RetryCancelCancel=បោះបង់

; *** Installation status messages
StatusClosingApplications=កំពុងបិទកម្មវិធី...
StatusCreateDirs=កំពុងបង្កើតថត...
StatusExtractFiles=កំពុងស្រង់ឯកសារ...
StatusDownloadFiles=កំពុងទាញយកឯកសារ...
StatusCreateIcons=កំពុងបង្កើតផ្លូវកាត់...
StatusCreateIniEntries=កំពុងបង្កើតធាតុ INI...
StatusCreateRegistryEntries=កំពុងបង្កើតធាតុ Registry...
StatusRegisterFiles=កំពុងចុះឈ្មោះឯកសារ...
StatusSavingUninstall=កំពុងរក្សាទុកព័ត៌មានលុបកម្មវិធី...
StatusRunProgram=កំពុងបញ្ចប់ការដំឡើង...
StatusRestartingApplications=កំពុងចាប់ផ្ដើមកម្មវិធីឡើងវិញ...
StatusRollback=កំពុងបកការផ្លាស់ប្ដូរ...

; *** Misc. errors
ErrorInternal2=កំហុសខាងក្នុង: %1
ErrorFunctionFailedNoCode=%1 បរាជ័យ
ErrorFunctionFailed=%1 បរាជ័យ; កូដ %2
ErrorFunctionFailedWithMessage=%1 បរាជ័យ; កូដ %2.%n%3
ErrorExecutingProgram=មិនអាចដំណើរការឯកសារ:%n%1

; *** Registry errors
ErrorRegOpenKey=កំហុសក្នុងការបើកគន្លឹះ Registry:%n%1\%2
ErrorRegCreateKey=កំហុសក្នុងការបង្កើតគន្លឹះ Registry:%n%1\%2
ErrorRegWriteKey=កំហុសក្នុងការសរសេរទៅគន្លឹះ Registry:%n%1\%2

; *** INI errors
ErrorIniEntry=កំហុសក្នុងការបង្កើតធាតុ INI ក្នុងឯកសារ "%1"។

; *** File copying errors
FileAbortRetryIgnoreSkipNotRecommended=រំលងឯកសារនេះ (មិនបានណែនាំ)
FileAbortRetryIgnoreIgnoreNotRecommended=មិនអើពើកំហុស ហើយបន្ត (មិនបានណែនាំ)
SourceIsCorrupted=ឯកសារប្រភពខូច
SourceDoesntExist=ឯកសារប្រភព "%1" មិនមានទេ
SourceVerificationFailed=ការផ្ទៀងផ្ទាត់ឯកសារប្រភពបរាជ័យ: %1
VerificationSignatureDoesntExist=ឯកសារហត្ថលេខា "%1" មិនមានទេ
VerificationSignatureInvalid=ឯកសារហត្ថលេខា "%1" មិនត្រឹមត្រូវ
VerificationKeyNotFound=ឯកសារហត្ថលេខា "%1" ប្រើគន្លឹះដែលមិនស្គាល់
VerificationFileNameIncorrect=ឈ្មោះឯកសារមិនត្រឹមត្រូវ
VerificationFileTagIncorrect=ស្លាកឯកសារមិនត្រឹមត្រូវ
VerificationFileSizeIncorrect=ទំហំឯកសារមិនត្រឹមត្រូវ
VerificationFileHashIncorrect=ហាសឯកសារមិនត្រឹមត្រូវ
ExistingFileReadOnly2=មិនអាចជំនួសឯកសារដែលមានស្រាប់បានទេ ព្រោះវាត្រូវបានកំណត់ជាបានតែអានប៉ុណ្ណោះ។
ExistingFileReadOnlyRetry=ដកគុណលក្ខណៈអានតែប៉ុណ្ណោះចេញ ហើយព្យាយាមម្ដងទៀត
ExistingFileReadOnlyKeepExisting=រក្សាឯកសារដែលមានស្រាប់
ErrorReadingExistingDest=មានកំហុសកើតឡើងពេលព្យាយាមអានឯកសារដែលមានស្រាប់:
FileExistsSelectAction=ជ្រើសរើសសកម្មភាព
FileExists2=ឯកសារមានរួចហើយ។
FileExistsOverwriteExisting=សរសេរជាន់លើឯកសារដែលមានស្រាប់
FileExistsKeepExisting=រក្សាឯកសារដែលមានស្រាប់
FileExistsOverwriteOrKeepAll=ធ្វើដូចនេះសម្រាប់ជម្លោះបន្ទាប់
ExistingFileNewerSelectAction=ជ្រើសរើសសកម្មភាព
ExistingFileNewer2=ឯកសារដែលមានស្រាប់ថ្មីជាងឯកសារដែលការដំឡើងកំពុងព្យាយាមដំឡើង។
ExistingFileNewerOverwriteExisting=សរសេរជាន់លើឯកសារដែលមានស្រាប់
ExistingFileNewerKeepExisting=រក្សាឯកសារដែលមានស្រាប់ (បានណែនាំ)
ExistingFileNewerOverwriteOrKeepAll=ធ្វើដូចនេះសម្រាប់ជម្លោះបន្ទាប់
ErrorChangingAttr=មានកំហុសកើតឡើងពេលព្យាយាមផ្លាស់ប្ដូរគុណលក្ខណៈនៃឯកសារដែលមានស្រាប់:
ErrorCreatingTemp=មានកំហុសកើតឡើងពេលព្យាយាមបង្កើតឯកសារក្នុងថតគោលដៅ:
ErrorReadingSource=មានកំហុសកើតឡើងពេលព្យាយាមអានឯកសារប្រភព:
ErrorCopying=មានកំហុសកើតឡើងពេលព្យាយាមចម្លងឯកសារ:
ErrorDownloading=មានកំហុសកើតឡើងពេលព្យាយាមទាញយកឯកសារ:
ErrorExtracting=មានកំហុសកើតឡើងពេលព្យាយាមស្រង់បណ្ណសារ:
ErrorReplacingExistingFile=មានកំហុសកើតឡើងពេលព្យាយាមជំនួសឯកសារដែលមានស្រាប់:
ErrorRestartReplace=RestartReplace បរាជ័យ:
ErrorRenamingTemp=មានកំហុសកើតឡើងពេលព្យាយាមប្ដូរឈ្មោះឯកសារក្នុងថតគោលដៅ:
ErrorRegisterServer=មិនអាចចុះឈ្មោះ DLL/OCX បានទេ: %1
ErrorRegSvr32Failed=RegSvr32 បរាជ័យដោយមានកូដចេញ %1
ErrorRegisterTypeLib=មិនអាចចុះឈ្មោះបណ្ណាល័យប្រភេទបានទេ: %1

; *** Uninstall display name markings
; used for example as 'My Program (32-bit)'
UninstallDisplayNameMark=%1 (%2)
; used for example as 'My Program (32-bit, All users)'
UninstallDisplayNameMarks=%1 (%2, %3)
UninstallDisplayNameMark32Bit=32-bit
UninstallDisplayNameMark64Bit=64-bit
UninstallDisplayNameMarkAllUsers=អ្នកប្រើទាំងអស់
UninstallDisplayNameMarkCurrentUser=អ្នកប្រើបច្ចុប្បន្ន

; *** Post-installation errors
ErrorOpeningReadme=មានកំហុសកើតឡើងពេលព្យាយាមបើកឯកសារ README។
ErrorRestartingComputer=ការដំឡើងមិនអាចចាប់ផ្ដើមកុំព្យូទ័រឡើងវិញបានទេ។ សូមធ្វើដោយដៃ។

; *** Uninstaller messages
UninstallNotFound=ឯកសារ "%1" មិនមានទេ។ មិនអាចលុបបានទេ។
UninstallOpenError=មិនអាចបើកឯកសារ "%1" បានទេ។ មិនអាចលុបបានទេ
UninstallUnsupportedVer=ឯកសារកំណត់ហេតុលុប "%1" មានទម្រង់ដែលកំណែនៃកម្មវិធីលុបនេះមិនស្គាល់។ មិនអាចលុបបានទេ
UninstallUnknownEntry=បានជួបធាតុមិនស្គាល់ (%1) ក្នុងកំណត់ហេតុលុប
ConfirmUninstall=តើអ្នកប្រាកដថាចង់លុប %1 និងសមាសភាគទាំងអស់របស់វាចេញទាំងស្រុងឬ?
UninstallOnlyOnWin64=ការដំឡើងនេះអាចលុបបានតែលើ Windows 64-bit ប៉ុណ្ណោះ។
OnlyAdminCanUninstall=ការដំឡើងនេះអាចលុបបានដោយអ្នកប្រើដែលមានសិទ្ធិអ្នកគ្រប់គ្រងប៉ុណ្ណោះ។
UninstallStatusLabel=សូមរង់ចាំខណៈ %1 កំពុងត្រូវបានលុបចេញពីកុំព្យូទ័ររបស់អ្នក។
UninstalledAll=%1 ត្រូវបានលុបចេញពីកុំព្យូទ័ររបស់អ្នកដោយជោគជ័យ។
UninstalledMost=ការលុប %1 បានបញ្ចប់។%n%nធាតុមួយចំនួនមិនអាចលុបបានទេ។ អ្នកអាចលុបវាដោយដៃ។
UninstalledAndNeedsRestart=ដើម្បីបញ្ចប់ការលុប %1 កុំព្យូទ័ររបស់អ្នកត្រូវចាប់ផ្ដើមឡើងវិញ។%n%nតើអ្នកចង់ចាប់ផ្ដើមឡើងវិញឥឡូវនេះទេ?
UninstallDataCorrupted=ឯកសារ "%1" ខូច។ មិនអាចលុបបានទេ

; *** Uninstallation phase messages
ConfirmDeleteSharedFileTitle=លុបឯកសារចែករំលែកឬ?
ConfirmDeleteSharedFile2=ប្រព័ន្ធបង្ហាញថាឯកសារចែករំលែកខាងក្រោមលែងត្រូវបានកម្មវិធីណាមួយប្រើទៀតហើយ។ តើអ្នកចង់ឱ្យការលុបកម្មវិធីលុបឯកសារចែករំលែកនេះទេ?%n%nបើមានកម្មវិធីណាមួយនៅតែប្រើឯកសារនេះ ហើយវាត្រូវបានលុប កម្មវិធីទាំងនោះអាចនឹងដំណើរការមិនត្រឹមត្រូវ។ បើអ្នកមិនប្រាកដ សូមជ្រើសរើស «ទេ»។ ការទុកឯកសារនៅលើប្រព័ន្ធរបស់អ្នកនឹងមិនបង្កគ្រោះថ្នាក់ទេ។
SharedFileNameLabel=ឈ្មោះឯកសារ:
SharedFileLocationLabel=ទីតាំង:
WizardUninstalling=ស្ថានភាពលុបកម្មវិធី
StatusUninstalling=កំពុងលុប %1...

; *** Shutdown block reasons
ShutdownBlockReasonInstallingApp=កំពុងដំឡើង %1។
ShutdownBlockReasonUninstallingApp=កំពុងលុប %1។

; The custom messages below aren't used by Setup itself, but if you make
; use of them in your scripts, you'll want to translate them.

[CustomMessages]

NameAndVersion=%1 កំណែ %2
AdditionalIcons=ផ្លូវកាត់បន្ថែម:
CreateDesktopIcon=បង្កើតផ្លូវកាត់លើផ្ទៃតុ
CreateQuickLaunchIcon=បង្កើតផ្លូវកាត់ Quick Launch
ProgramOnTheWeb=%1 លើបណ្ដាញ
UninstallProgram=លុប %1
LaunchProgram=ដំណើរការ %1
AssocFileExtension=ភ្ជាប់ %1 ជាមួយនឹងកន្ទុយឯកសារ %2
AssocingFileExtension=កំពុងភ្ជាប់ %1 ជាមួយនឹងកន្ទុយឯកសារ %2...
AutoStartProgramGroupDescription=ចាប់ផ្ដើម:
AutoStartProgram=ចាប់ផ្ដើម %1 ដោយស្វ័យប្រវត្តិ
AddonHostProgramNotFound=មិនអាចរកឃើញ %1 ក្នុងថតដែលអ្នកបានជ្រើសរើសទេ។%n%nតើអ្នកចង់បន្តទោះយ៉ាងណាក៏ដោយឬ?

; The messages that are Lang Switcher's own (the sample is English.isl).
LocalCertReady=ការដំឡើងនឹងបង្កើតវិញ្ញាបនបត្រលើកុំព្យូទ័រនេះ ហើយចុះហត្ថលេខា Lang Switcher ជាមួយវា ដើម្បីឱ្យវាអាចដំណើរការក្នុងបង្អួចកម្មវិធីដែលដំណើរការជាអ្នកគ្រប់គ្រង។
LocalCertFailed=Lang Switcher ត្រូវបានដំឡើង ប៉ុន្តែវានឹងមិនដំណើរការក្នុងបង្អួចកម្មវិធីដែលដំណើរការជាអ្នកគ្រប់គ្រងទេ: Windows មិនទទួលយកវិញ្ញាបនបត្រដែលបានបង្កើតលើកុំព្យូទ័រនេះ។ សូមមើលកំណត់ហេតុដំឡើងសម្រាប់ព័ត៌មានលម្អិត។
RemoveSettingsQuestion=លុបឯកសារការកំណត់របស់ Lang Switcher ផងដែរឬទេ?
RemoveSettingsKeep=ជ្រើសរើស «ទេ» ដើម្បីរក្សាការកំណត់របស់អ្នកសម្រាប់ការដំឡើងឡើងវិញនៅពេលក្រោយ។
AuthorSlogan=កម្មវិធីដែលបង្កើតដោយបេះដូង
