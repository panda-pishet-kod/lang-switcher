; Lang Switcher -- the messages of the installer in Assamese (stage E108, group A).
;
; Inno Setup has no official Assamese translation (inno_official = false in res\lang\languages.toml),
; so this file carries the whole of it: [LangOptions], every message of the compiler:Default.isl
; (6.5.0+), and the messages of the installer that are Lang Switcher's own. It is read after
; compiler:Default.isl as the second MessagesFile of the "assamese" entry of [Languages] in
; ..\LangSwitcher.iss, and so replaces the English text name for name.
; English.isl is the sample the own messages translate; the glossary of the language is
; l10n\glossary\as.tsv of the control contour. The name of the program is not translated.
; As in Default.isl: a message that has no full stop there has no danda here (Setup adds its own).
; No accelerators (&): the file has no official Inno translation, so no letter is claimed twice
; in one window (RULES 16). Names of buttons inside a message stand in the quotes of the language.
; Assamese letters: RA is U+09F0 and WA is U+09F1 in every word below, never the Bengali RA U+09B0.

[LangOptions]
LanguageName=অসমীয়া
LanguageID=$044D
LanguageCodePage=0

[Messages]

; *** Application titles
SetupAppTitle=ছেটআপ
SetupWindowTitle=ছেটআপ - %1
UninstallAppTitle=আনইনষ্টল
UninstallAppFullTitle=%1 আনইনষ্টল

; *** Misc. common
InformationTitle=তথ্য
ConfirmTitle=নিশ্চিত কৰক
ErrorTitle=ত্ৰুটি

; *** SetupLdr messages
SetupLdrStartupMessage=ই %1 ইনষ্টল কৰিব। আপুনি আগবাঢ়িব বিচাৰেনে?
LdrCannotCreateTemp=অস্থায়ী ফাইল সৃষ্টি কৰিব নোৱাৰি। ছেটআপ বাতিল কৰা হৈছে
LdrCannotExecTemp=অস্থায়ী ফোল্ডাৰৰ ফাইল চলাব নোৱাৰি। ছেটআপ বাতিল কৰা হৈছে
HelpTextNote=

; *** Startup error messages
LastErrorMessage=%1.%n%nত্ৰুটি %2: %3
SetupFileMissing=ইনষ্টলেচন ফোল্ডাৰত %1 ফাইলটো নাই। অনুগ্ৰহ কৰি সমস্যাটো শুধৰাওক অথবা প্ৰগ্ৰামটোৰ এটা নতুন কপি সংগ্ৰহ কৰক।
SetupFileCorrupt=ছেটআপ ফাইলবোৰ নষ্ট হৈছে। অনুগ্ৰহ কৰি প্ৰগ্ৰামটোৰ এটা নতুন কপি সংগ্ৰহ কৰক।
SetupFileCorruptOrWrongVer=ছেটআপ ফাইলবোৰ নষ্ট হৈছে, অথবা ছেটআপৰ এই সংস্কৰণৰ সৈতে খাপ নাখায়। অনুগ্ৰহ কৰি সমস্যাটো শুধৰাওক অথবা প্ৰগ্ৰামটোৰ এটা নতুন কপি সংগ্ৰহ কৰক।
InvalidParameter=কমাণ্ড লাইনত এটা অবৈধ পেৰামিটাৰ দিয়া হৈছে:%n%n%1
SetupAlreadyRunning=ছেটআপ ইতিমধ্যে চলি আছে।
WindowsVersionNotSupported=আপোনাৰ কম্পিউটাৰত চলি থকা Windows-ৰ সংস্কৰণ এই প্ৰগ্ৰামে সমৰ্থন নকৰে।
WindowsServicePackRequired=এই প্ৰগ্ৰামৰ বাবে %1 ছাৰ্ভিচ পেক %2 বা তাতকৈ নতুন সংস্কৰণ লাগিব।
NotOnThisPlatform=এই প্ৰগ্ৰাম %1-ত নচলিব।
OnlyOnThisPlatform=এই প্ৰগ্ৰাম %1-ত চলাব লাগিব।
OnlyOnTheseArchitectures=এই প্ৰগ্ৰাম কেৱল তলৰ প্ৰচেছৰ আৰ্কিটেকচাৰৰ বাবে বনোৱা Windows-ত ইনষ্টল কৰিব পাৰি:%n%n%1
WinVersionTooLowError=এই প্ৰগ্ৰামৰ বাবে %1 সংস্কৰণ %2 বা তাতকৈ নতুন সংস্কৰণ লাগিব।
WinVersionTooHighError=এই প্ৰগ্ৰাম %1 সংস্কৰণ %2 বা তাতকৈ নতুন সংস্কৰণত ইনষ্টল কৰিব নোৱাৰি।
AdminPrivilegesRequired=এই প্ৰগ্ৰাম ইনষ্টল কৰোঁতে আপুনি প্ৰশাসক হিচাপে ছাইন ইন কৰি থাকিব লাগিব।
PowerUserPrivilegesRequired=এই প্ৰগ্ৰাম ইনষ্টল কৰোঁতে আপুনি প্ৰশাসক অথবা Power Users গোটৰ সদস্য হিচাপে ছাইন ইন কৰি থাকিব লাগিব।
SetupAppRunningError=ছেটআপে দেখিছে যে %1 এতিয়া চলি আছে।%n%nঅনুগ্ৰহ কৰি ইয়াৰ সকলো ৱিণ্ডো এতিয়াই বন্ধ কৰক, তাৰ পিছত আগবাঢ়িবলৈ “ঠিক আছে” বা ওলাই যাবলৈ “বাতিল” ক্লিক কৰক।
UninstallAppRunningError=আনইনষ্টলে দেখিছে যে %1 এতিয়া চলি আছে।%n%nঅনুগ্ৰহ কৰি ইয়াৰ সকলো ৱিণ্ডো এতিয়াই বন্ধ কৰক, তাৰ পিছত আগবাঢ়িবলৈ “ঠিক আছে” বা ওলাই যাবলৈ “বাতিল” ক্লিক কৰক।

; *** Startup questions
PrivilegesRequiredOverrideTitle=ছেটআপৰ ইনষ্টল মোড বাছক
PrivilegesRequiredOverrideInstruction=ইনষ্টল মোড বাছক
PrivilegesRequiredOverrideText1=%1 সকলো ব্যৱহাৰকাৰীৰ বাবে (প্ৰশাসকৰ অধিকাৰ লাগিব) অথবা কেৱল আপোনাৰ বাবে ইনষ্টল কৰিব পাৰি।
PrivilegesRequiredOverrideText2=%1 কেৱল আপোনাৰ বাবে অথবা সকলো ব্যৱহাৰকাৰীৰ বাবে (প্ৰশাসকৰ অধিকাৰ লাগিব) ইনষ্টল কৰিব পাৰি।
PrivilegesRequiredOverrideAllUsers=সকলো ব্যৱহাৰকাৰীৰ বাবে ইনষ্টল কৰক
PrivilegesRequiredOverrideAllUsersRecommended=সকলো ব্যৱহাৰকাৰীৰ বাবে ইনষ্টল কৰক (প্ৰস্তাৱিত)
PrivilegesRequiredOverrideCurrentUser=কেৱল মোৰ বাবে ইনষ্টল কৰক
PrivilegesRequiredOverrideCurrentUserRecommended=কেৱল মোৰ বাবে ইনষ্টল কৰক (প্ৰস্তাৱিত)

; *** Misc. errors
ErrorCreatingDir=ছেটআপে “%1” ফোল্ডাৰটো সৃষ্টি কৰিব পৰা নাই
ErrorTooManyFilesInDir=“%1” ফোল্ডাৰত ফাইল সৃষ্টি কৰিব নোৱাৰি, কাৰণ তাত অতি বেছি ফাইল আছে

; *** Setup common messages
ExitSetupTitle=ছেটআপৰ পৰা ওলাওক
ExitSetupMessage=ছেটআপ সম্পূৰ্ণ হোৱা নাই। এতিয়া ওলালে প্ৰগ্ৰামটো ইনষ্টল নহব।%n%nইনষ্টলেচন সম্পূৰ্ণ কৰিবলৈ আপুনি পিছত আকৌ ছেটআপ চলাব পাৰে।%n%nছেটআপৰ পৰা ওলাবনে?
AboutSetupMenuItem=ছেটআপৰ সম্পৰ্কে...
AboutSetupTitle=ছেটআপৰ সম্পৰ্কে
AboutSetupMessage=%1 সংস্কৰণ %2%n%3%n%n%1 হোম পেজ:%n%4
AboutSetupNote=
TranslatorNote=

; *** Buttons
ButtonBack=< পিছলৈ
ButtonNext=পৰৱৰ্তী >
ButtonInstall=ইনষ্টল কৰক
ButtonOK=ঠিক আছে
ButtonCancel=বাতিল
ButtonYes=হয়
ButtonYesToAll=সকলোবোৰৰ বাবে হয়
ButtonNo=নহয়
ButtonNoToAll=সকলোবোৰৰ বাবে নহয়
ButtonFinish=সমাপ্ত
ButtonBrowse=ব্ৰাউজ...
ButtonWizardBrowse=ব্ৰাউজ...
ButtonNewFolder=নতুন ফোল্ডাৰ সৃষ্টি কৰক

; *** "Select Language" dialog messages
SelectLanguageTitle=ছেটআপৰ ভাষা বাছক
SelectLanguageLabel=ইনষ্টলেচনৰ সময়ত ব্যৱহাৰ কৰিবলগীয়া ভাষা বাছক।

; *** Common wizard text
ClickNext=আগবাঢ়িবলৈ “পৰৱৰ্তী” বা ছেটআপৰ পৰা ওলাবলৈ “বাতিল” ক্লিক কৰক।
BeveledLabel=
BrowseDialogTitle=ফোল্ডাৰ বাছক
BrowseDialogLabel=তলৰ তালিকাৰ পৰা এটা ফোল্ডাৰ বাছক, তাৰ পিছত “ঠিক আছে” ক্লিক কৰক।
NewFolderName=নতুন ফোল্ডাৰ

; *** "Welcome" wizard page
WelcomeLabel1=[name] ছেটআপ ৱিজাৰ্ডলৈ স্বাগতম
WelcomeLabel2=ই আপোনাৰ কম্পিউটাৰত [name/ver] ইনষ্টল কৰিব।%n%nআগবাঢ়ি যোৱাৰ আগতে আন সকলো এপ্লিকেচন বন্ধ কৰি লোৱাটো পৰামৰ্শযোগ্য।

; *** "Password" wizard page
WizardPassword=পাছৱৰ্ড
PasswordLabel1=এই ইনষ্টলেচন পাছৱৰ্ডেৰে সুৰক্ষিত।
PasswordLabel3=অনুগ্ৰহ কৰি পাছৱৰ্ড দিয়ক, তাৰ পিছত আগবাঢ়িবলৈ “পৰৱৰ্তী” ক্লিক কৰক। পাছৱৰ্ডত ডাঙৰ আৰু সৰু আখৰ বেলেগ বেলেগ ধৰা হয়।
PasswordEditLabel=পাছৱৰ্ড:
IncorrectPassword=আপুনি দিয়া পাছৱৰ্ড শুদ্ধ নহয়। অনুগ্ৰহ কৰি আকৌ চেষ্টা কৰক।

; *** "License Agreement" wizard page
WizardLicense=লাইচেন্স চুক্তি
LicenseLabel=আগবাঢ়ি যোৱাৰ আগতে অনুগ্ৰহ কৰি তলৰ গুৰুত্বপূৰ্ণ তথ্য পঢ়ক।
LicenseLabel3=অনুগ্ৰহ কৰি তলৰ লাইচেন্স চুক্তি পঢ়ক। ইনষ্টলেচন আগবঢ়াই নিয়াৰ আগতে আপুনি এই চুক্তিৰ চৰ্তবোৰ মানি লব লাগিব।
LicenseAccepted=মই চুক্তিটো মানি লৈছোঁ
LicenseNotAccepted=মই চুক্তিটো মানি লোৱা নাই

; *** "Information" wizard pages
WizardInfoBefore=তথ্য
InfoBeforeLabel=আগবাঢ়ি যোৱাৰ আগতে অনুগ্ৰহ কৰি তলৰ গুৰুত্বপূৰ্ণ তথ্য পঢ়ক।
InfoBeforeClickLabel=ছেটআপ আগবঢ়াই নিবলৈ সাজু হলে “পৰৱৰ্তী” ক্লিক কৰক।
WizardInfoAfter=তথ্য
InfoAfterLabel=আগবাঢ়ি যোৱাৰ আগতে অনুগ্ৰহ কৰি তলৰ গুৰুত্বপূৰ্ণ তথ্য পঢ়ক।
InfoAfterClickLabel=ছেটআপ আগবঢ়াই নিবলৈ সাজু হলে “পৰৱৰ্তী” ক্লিক কৰক।

; *** "User Information" wizard page
WizardUserInfo=ব্যৱহাৰকাৰীৰ তথ্য
UserInfoDesc=অনুগ্ৰহ কৰি আপোনাৰ তথ্য দিয়ক।
UserInfoName=ব্যৱহাৰকাৰীৰ নাম:
UserInfoOrg=প্ৰতিষ্ঠান:
UserInfoSerial=ক্ৰমিক নম্বৰ:
UserInfoNameRequired=আপুনি এটা নাম দিবই লাগিব।

; *** "Select Destination Location" wizard page
WizardSelectDir=গন্তব্য স্থান বাছক
SelectDirDesc=[name] কোনঠাইত ইনষ্টল কৰা হব?
SelectDirLabel3=ছেটআপে [name] তলৰ ফোল্ডাৰত ইনষ্টল কৰিব।
SelectDirBrowseLabel=আগবাঢ়িবলৈ “পৰৱৰ্তী” ক্লিক কৰক। অন্য এটা ফোল্ডাৰ বাছিব বিচাৰিলে “ব্ৰাউজ” ক্লিক কৰক।
DiskSpaceGBLabel=কমেও [gb] GB খালী ডিস্ক ঠাই লাগিব।
DiskSpaceMBLabel=কমেও [mb] MB খালী ডিস্ক ঠাই লাগিব।
CannotInstallToNetworkDrive=ছেটআপে নেটৱৰ্ক ড্ৰাইভত ইনষ্টল কৰিব নোৱাৰে।
CannotInstallToUNCPath=ছেটআপে UNC পাথত ইনষ্টল কৰিব নোৱাৰে।
InvalidPath=ড্ৰাইভৰ আখৰসহ এটা সম্পূৰ্ণ পাথ দিব লাগিব; যেনে:%n%nC:\APP%n%nঅথবা এই আকাৰৰ এটা UNC পাথ:%n%n\\server\share
InvalidDrive=আপুনি বাছি লোৱা ড্ৰাইভ বা UNC শ্বেয়াৰ নাই অথবা পোৱা নাযায়। অনুগ্ৰহ কৰি আন এটা বাছক।
DiskSpaceWarningTitle=ডিস্কত পৰ্যাপ্ত ঠাই নাই
DiskSpaceWarning=ইনষ্টল কৰিবলৈ ছেটআপক কমেও %1 KB খালী ঠাই লাগে, কিন্তু বাছি লোৱা ড্ৰাইভত মাত্ৰ %2 KB আছে।%n%nতথাপি আগবাঢ়িবনে?
DirNameTooLong=ফোল্ডাৰৰ নাম বা পাথ অতি দীঘল।
InvalidDirName=ফোল্ডাৰৰ নাম শুদ্ধ নহয়।
BadDirName32=ফোল্ডাৰৰ নামত তলৰ কোনো আখৰ থাকিব নোৱাৰে:%n%n%1
DirExistsTitle=ফোল্ডাৰ আছে
DirExists=এই ফোল্ডাৰটো:%n%n%1%n%nআগৰে পৰা আছে। তথাপি সেই ফোল্ডাৰতে ইনষ্টল কৰিব বিচাৰেনে?
DirDoesntExistTitle=ফোল্ডাৰ নাই
DirDoesntExist=এই ফোল্ডাৰটো:%n%n%1%n%nনাই। ফোল্ডাৰটো সৃষ্টি কৰিব বিচাৰেনে?

; *** "Select Components" wizard page
WizardSelectComponents=উপাদান বাছক
SelectComponentsDesc=কোনবোৰ উপাদান ইনষ্টল কৰা হব?
SelectComponentsLabel2=যিবোৰ উপাদান ইনষ্টল কৰিব বিচাৰে সেইবোৰ বাছক; যিবোৰ বিচৰা নাই সেইবোৰৰ টিক আঁতৰাওক। সাজু হলে “পৰৱৰ্তী” ক্লিক কৰক।
FullInstallation=সম্পূৰ্ণ ইনষ্টলেচন
; if possible don't translate 'Compact' as 'Minimal' (I mean 'Minimal' in your language)
CompactInstallation=সংক্ষিপ্ত ইনষ্টলেচন
CustomInstallation=নিজৰ পছন্দৰ ইনষ্টলেচন
NoUninstallWarningTitle=উপাদান আগৰে পৰা আছে
NoUninstallWarning=ছেটআপে দেখিছে যে তলৰ উপাদানবোৰ আপোনাৰ কম্পিউটাৰত আগৰে পৰা ইনষ্টল কৰা আছে:%n%n%1%n%nএইবোৰ বাছনিৰ পৰা আঁতৰালে সেইবোৰ আনইনষ্টল নহয়।%n%nতথাপি আগবাঢ়িবনে?
ComponentSize1=%1 KB
ComponentSize2=%1 MB
ComponentsDiskSpaceGBLabel=বৰ্তমানৰ বাছনিৰ বাবে কমেও [gb] GB ডিস্ক ঠাই লাগিব।
ComponentsDiskSpaceMBLabel=বৰ্তমানৰ বাছনিৰ বাবে কমেও [mb] MB ডিস্ক ঠাই লাগিব।

; *** "Select Additional Tasks" wizard page
WizardSelectTasks=অতিৰিক্ত কাম বাছক
SelectTasksDesc=কোনবোৰ অতিৰিক্ত কাম কৰা হব?
SelectTasksLabel2=[name] ইনষ্টল কৰোঁতে ছেটআপে কৰিব লগা অতিৰিক্ত কামবোৰ বাছক, তাৰ পিছত “পৰৱৰ্তী” ক্লিক কৰক।

; *** "Select Start Menu Folder" wizard page
WizardSelectProgramGroup=আৰম্ভ মেনু ফোল্ডাৰ বাছক
SelectStartMenuFolderDesc=ছেটআপে প্ৰগ্ৰামটোৰ শৰ্টকাটবোৰ কোনঠাইত ৰাখিব?
SelectStartMenuFolderLabel3=ছেটআপে প্ৰগ্ৰামটোৰ শৰ্টকাটবোৰ তলৰ আৰম্ভ মেনু ফোল্ডাৰত সৃষ্টি কৰিব।
SelectStartMenuFolderBrowseLabel=আগবাঢ়িবলৈ “পৰৱৰ্তী” ক্লিক কৰক। অন্য এটা ফোল্ডাৰ বাছিব বিচাৰিলে “ব্ৰাউজ” ক্লিক কৰক।
MustEnterGroupName=আপুনি এটা ফোল্ডাৰৰ নাম দিবই লাগিব।
GroupNameTooLong=ফোল্ডাৰৰ নাম বা পাথ অতি দীঘল।
InvalidGroupName=ফোল্ডাৰৰ নাম শুদ্ধ নহয়।
BadGroupName=ফোল্ডাৰৰ নামত তলৰ কোনো আখৰ থাকিব নোৱাৰে:%n%n%1
NoProgramGroupCheck2=আৰম্ভ মেনু ফোল্ডাৰ সৃষ্টি নকৰিব

; *** "Ready to Install" wizard page
WizardReady=ইনষ্টল কৰিবলৈ সাজু
ReadyLabel1=ছেটআপ এতিয়া আপোনাৰ কম্পিউটাৰত [name] ইনষ্টল কৰিবলৈ সাজু।
ReadyLabel2a=ইনষ্টলেচন আগবঢ়াবলৈ “ইনষ্টল কৰক” ক্লিক কৰক, অথবা কোনো ছেটিংছ চাব বা সলনি কৰিব বিচাৰিলে “পিছলৈ” ক্লিক কৰক।
ReadyLabel2b=ইনষ্টলেচন আগবঢ়াবলৈ “ইনষ্টল কৰক” ক্লিক কৰক।
ReadyMemoUserInfo=ব্যৱহাৰকাৰীৰ তথ্য:
ReadyMemoDir=গন্তব্য স্থান:
ReadyMemoType=ছেটআপৰ প্ৰকাৰ:
ReadyMemoComponents=বাছি লোৱা উপাদান:
ReadyMemoGroup=আৰম্ভ মেনু ফোল্ডাৰ:
ReadyMemoTasks=অতিৰিক্ত কাম:

; *** TDownloadWizardPage wizard page and DownloadTemporaryFile
DownloadingLabel2=ফাইল ডাউনলোড হৈ আছে...
ButtonStopDownload=ডাউনলোড বন্ধ কৰক
StopDownload=আপুনি নিশ্চিতনে যে ডাউনলোড বন্ধ কৰিব বিচাৰে?
ErrorDownloadAborted=ডাউনলোড বাতিল কৰা হৈছে
ErrorDownloadFailed=ডাউনলোড বিফল: %1 %2
ErrorDownloadSizeFailed=আকাৰ জানিব নোৱাৰিলে: %1 %2
ErrorProgress=অবৈধ অগ্ৰগতি: %2-ৰ ভিতৰত %1
ErrorFileSize=ফাইলৰ আকাৰ অবৈধ: প্ৰত্যাশিত %1, পোৱা গৈছে %2

; *** TExtractionWizardPage wizard page and ExtractArchive
ExtractingLabel=ফাইল উলিওৱা হৈ আছে...
ButtonStopExtraction=উলিওৱা বন্ধ কৰক
StopExtraction=আপুনি নিশ্চিতনে যে উলিওৱা বন্ধ কৰিব বিচাৰে?
ErrorExtractionAborted=উলিওৱা বাতিল কৰা হৈছে
ErrorExtractionFailed=উলিওৱা বিফল: %1

; *** Archive extraction failure details
ArchiveIncorrectPassword=পাছৱৰ্ড শুদ্ধ নহয়
ArchiveIsCorrupted=আৰ্কাইভ নষ্ট হৈছে
ArchiveUnsupportedFormat=আৰ্কাইভৰ ফৰ্মেট সমৰ্থিত নহয়

; *** "Preparing to Install" wizard page
WizardPreparing=ইনষ্টলৰ প্ৰস্তুতি চলি আছে
PreparingDesc=ছেটআপে আপোনাৰ কম্পিউটাৰত [name] ইনষ্টল কৰিবলৈ প্ৰস্তুতি চলাইছে।
PreviousInstallNotCompleted=আগৰ কোনো প্ৰগ্ৰামৰ ইনষ্টলেচন বা আঁতৰোৱা সম্পূৰ্ণ হোৱা নাই। সেই ইনষ্টলেচন সম্পূৰ্ণ কৰিবলৈ আপুনি কম্পিউটাৰ ৰিষ্টাৰ্ট কৰিব লাগিব।%n%nকম্পিউটাৰ ৰিষ্টাৰ্ট কৰাৰ পিছত [name]ৰ ইনষ্টলেচন সম্পূৰ্ণ কৰিবলৈ আকৌ ছেটআপ চলাওক।
CannotContinue=ছেটআপে আগবাঢ়িব নোৱাৰে। ওলাই যাবলৈ অনুগ্ৰহ কৰি “বাতিল” ক্লিক কৰক।
ApplicationsFound=ছেটআপে আপডেট কৰিবলগীয়া ফাইলবোৰ তলৰ এপ্লিকেচনবোৰে ব্যৱহাৰ কৰি আছে। ছেটআপক এই এপ্লিকেচনবোৰ নিজে বন্ধ কৰিবলৈ দিয়াটো পৰামৰ্শযোগ্য।
ApplicationsFound2=ছেটআপে আপডেট কৰিবলগীয়া ফাইলবোৰ তলৰ এপ্লিকেচনবোৰে ব্যৱহাৰ কৰি আছে। ছেটআপক এই এপ্লিকেচনবোৰ নিজে বন্ধ কৰিবলৈ দিয়াটো পৰামৰ্শযোগ্য। ইনষ্টলেচন শেষ হলে ছেটআপে এপ্লিকেচনবোৰ আকৌ আৰম্ভ কৰিবলৈ চেষ্টা কৰিব।
CloseApplications=এপ্লিকেচনবোৰ নিজে বন্ধ কৰক
DontCloseApplications=এপ্লিকেচনবোৰ বন্ধ নকৰিব
ErrorCloseApplications=ছেটআপে সকলো এপ্লিকেচন নিজে বন্ধ কৰিব পৰা নাই। ছেটআপে আপডেট কৰিবলগীয়া ফাইলবোৰ ব্যৱহাৰ কৰি থকা সকলো এপ্লিকেচন আগবাঢ়ি যোৱাৰ আগতে বন্ধ কৰি লোৱাটো পৰামৰ্শযোগ্য।
PrepareToInstallNeedsRestart=ছেটআপে আপোনাৰ কম্পিউটাৰ ৰিষ্টাৰ্ট কৰিব লাগিব। কম্পিউটাৰ ৰিষ্টাৰ্ট কৰাৰ পিছত [name]ৰ ইনষ্টলেচন সম্পূৰ্ণ কৰিবলৈ আকৌ ছেটআপ চলাওক।%n%nএতিয়া ৰিষ্টাৰ্ট কৰিব বিচাৰেনে?

; *** "Installing" wizard page
WizardInstalling=ইনষ্টল হৈ আছে
InstallingLabel=ছেটআপে আপোনাৰ কম্পিউটাৰত [name] ইনষ্টল কৰি থকাৰ সময়ছোৱাত অনুগ্ৰহ কৰি অপেক্ষা কৰক।

; *** "Setup Completed" wizard page
FinishedHeadingLabel=[name] ছেটআপ ৱিজাৰ্ড সম্পূৰ্ণ হৈছে
FinishedLabelNoIcons=ছেটআপে আপোনাৰ কম্পিউটাৰত [name] ইনষ্টল কৰা সম্পূৰ্ণ কৰিলে।
FinishedLabel=ছেটআপে আপোনাৰ কম্পিউটাৰত [name] ইনষ্টল কৰা সম্পূৰ্ণ কৰিলে। ইনষ্টল হোৱা শৰ্টকাটবোৰ বাছি এপ্লিকেচনটো আৰম্ভ কৰিব পাৰি।
ClickFinish=ছেটআপৰ পৰা ওলাবলৈ “সমাপ্ত” ক্লিক কৰক।
FinishedRestartLabel=[name]ৰ ইনষ্টলেচন সম্পূৰ্ণ কৰিবলৈ ছেটআপে আপোনাৰ কম্পিউটাৰ ৰিষ্টাৰ্ট কৰিব লাগিব। এতিয়া ৰিষ্টাৰ্ট কৰিব বিচাৰেনে?
FinishedRestartMessage=[name]ৰ ইনষ্টলেচন সম্পূৰ্ণ কৰিবলৈ ছেটআপে আপোনাৰ কম্পিউটাৰ ৰিষ্টাৰ্ট কৰিব লাগিব।%n%nএতিয়া ৰিষ্টাৰ্ট কৰিব বিচাৰেনে?
ShowReadmeCheck=হয়, মই README ফাইলটো চাব বিচাৰোঁ
YesRadio=হয়, এতিয়াই কম্পিউটাৰ ৰিষ্টাৰ্ট কৰক
NoRadio=নহয়, মই পিছত কম্পিউটাৰ ৰিষ্টাৰ্ট কৰিম
; used for example as 'Run MyProg.exe'
RunEntryExec=%1 চলাওক
; used for example as 'View Readme.txt'
RunEntryShellExec=%1 চাওক

; *** "Setup Needs the Next Disk" stuff
ChangeDiskTitle=ছেটআপক পৰৱৰ্তী ডিস্ক লাগে
SelectDiskLabel2=অনুগ্ৰহ কৰি ডিস্ক %1 সুমুৱাওক আৰু “ঠিক আছে” ক্লিক কৰক।%n%nএই ডিস্কৰ ফাইলবোৰ তলত দেখুওৱা ফোল্ডাৰৰ বাহিৰে আন কতো থাকিলে শুদ্ধ পাথ লিখক অথবা “ব্ৰাউজ” ক্লিক কৰক।
PathLabel=পাথ:
FileNotInDir2=“%2”ত “%1” ফাইলটো পোৱা নাযায়। অনুগ্ৰহ কৰি শুদ্ধ ডিস্ক সুমুৱাওক অথবা অন্য এটা ফোল্ডাৰ বাছক।
SelectDirectoryLabel=অনুগ্ৰহ কৰি পৰৱৰ্তী ডিস্কৰ স্থান দিয়ক।

; *** Installation phase messages
SetupAborted=ছেটআপ সম্পূৰ্ণ হোৱা নাই।%n%nঅনুগ্ৰহ কৰি সমস্যাটো শুধৰাই আকৌ ছেটআপ চলাওক।
AbortRetryIgnoreSelectAction=কৰণীয় বাছক
AbortRetryIgnoreRetry=আকৌ চেষ্টা কৰক
AbortRetryIgnoreIgnore=ত্ৰুটি উপেক্ষা কৰি আগবাঢ়ক
AbortRetryIgnoreCancel=ইনষ্টলেচন বাতিল কৰক
RetryCancelSelectAction=কৰণীয় বাছক
RetryCancelRetry=আকৌ চেষ্টা কৰক
RetryCancelCancel=বাতিল

; *** Installation status messages
StatusClosingApplications=এপ্লিকেচন বন্ধ কৰা হৈ আছে...
StatusCreateDirs=ফোল্ডাৰ সৃষ্টি কৰা হৈ আছে...
StatusExtractFiles=ফাইল উলিওৱা হৈ আছে...
StatusDownloadFiles=ফাইল ডাউনলোড হৈ আছে...
StatusCreateIcons=শৰ্টকাট সৃষ্টি কৰা হৈ আছে...
StatusCreateIniEntries=INI এণ্ট্ৰি সৃষ্টি কৰা হৈ আছে...
StatusCreateRegistryEntries=ৰেজিষ্ট্ৰী এণ্ট্ৰি সৃষ্টি কৰা হৈ আছে...
StatusRegisterFiles=ফাইল পঞ্জীয়ন কৰা হৈ আছে...
StatusSavingUninstall=আনইনষ্টলৰ তথ্য সংৰক্ষণ কৰা হৈ আছে...
StatusRunProgram=ইনষ্টলেচন সম্পূৰ্ণ কৰা হৈ আছে...
StatusRestartingApplications=এপ্লিকেচন আকৌ আৰম্ভ কৰা হৈ আছে...
StatusRollback=পৰিৱৰ্তন উভতাই নিয়া হৈ আছে...

; *** Misc. errors
ErrorInternal2=আভ্যন্তৰীণ ত্ৰুটি: %1
ErrorFunctionFailedNoCode=%1 বিফল হৈছে
ErrorFunctionFailed=%1 বিফল হৈছে; কোড %2
ErrorFunctionFailedWithMessage=%1 বিফল হৈছে; কোড %2।%n%3
ErrorExecutingProgram=ফাইল চলাব নোৱাৰি:%n%1

; *** Registry errors
ErrorRegOpenKey=ৰেজিষ্ট্ৰী কী খুলোঁতে ত্ৰুটি:%n%1\%2
ErrorRegCreateKey=ৰেজিষ্ট্ৰী কী সৃষ্টি কৰোঁতে ত্ৰুটি:%n%1\%2
ErrorRegWriteKey=ৰেজিষ্ট্ৰী কীত লিখোঁতে ত্ৰুটি:%n%1\%2

; *** INI errors
ErrorIniEntry=“%1” ফাইলত INI এণ্ট্ৰি সৃষ্টি কৰোঁতে ত্ৰুটি।

; *** File copying errors
FileAbortRetryIgnoreSkipNotRecommended=এই ফাইলটো এৰি দিয়ক (পৰামৰ্শযোগ্য নহয়)
FileAbortRetryIgnoreIgnoreNotRecommended=ত্ৰুটি উপেক্ষা কৰি আগবাঢ়ক (পৰামৰ্শযোগ্য নহয়)
SourceIsCorrupted=উৎস ফাইলটো নষ্ট হৈছে
SourceDoesntExist=“%1” উৎস ফাইলটো নাই
SourceVerificationFailed=উৎস ফাইলৰ পৰীক্ষা বিফল হৈছে: %1
VerificationSignatureDoesntExist=“%1” স্বাক্ষৰ ফাইলটো নাই
VerificationSignatureInvalid=“%1” স্বাক্ষৰ ফাইলটো অবৈধ
VerificationKeyNotFound=“%1” স্বাক্ষৰ ফাইলটোৱে এটা অজ্ঞাত কী ব্যৱহাৰ কৰিছে
VerificationFileNameIncorrect=ফাইলৰ নাম শুদ্ধ নহয়
VerificationFileTagIncorrect=ফাইলৰ টেগ শুদ্ধ নহয়
VerificationFileSizeIncorrect=ফাইলৰ আকাৰ শুদ্ধ নহয়
VerificationFileHashIncorrect=ফাইলৰ হেচ শুদ্ধ নহয়
ExistingFileReadOnly2=বিদ্যমান ফাইলটো কেৱল-পঢ়িব-পৰা বুলি চিহ্নিত হোৱাৰ বাবে সলনি কৰিব নোৱাৰিলে।
ExistingFileReadOnlyRetry=কেৱল-পঢ়িব-পৰা বৈশিষ্ট্য আঁতৰাই আকৌ চেষ্টা কৰক
ExistingFileReadOnlyKeepExisting=বিদ্যমান ফাইলটো ৰাখক
ErrorReadingExistingDest=বিদ্যমান ফাইলটো পঢ়োঁতে ত্ৰুটি হৈছে:
FileExistsSelectAction=কৰণীয় বাছক
FileExists2=ফাইলটো আগৰে পৰা আছে।
FileExistsOverwriteExisting=বিদ্যমান ফাইলটো প্ৰতিস্থাপন কৰক
FileExistsKeepExisting=বিদ্যমান ফাইলটো ৰাখক
FileExistsOverwriteOrKeepAll=পৰৱৰ্তী সংঘাতবোৰতো এইটো কৰক
ExistingFileNewerSelectAction=কৰণীয় বাছক
ExistingFileNewer2=ছেটআপে ইনষ্টল কৰিব বিচৰা ফাইলতকৈ বিদ্যমান ফাইলটো নতুন।
ExistingFileNewerOverwriteExisting=বিদ্যমান ফাইলটো প্ৰতিস্থাপন কৰক
ExistingFileNewerKeepExisting=বিদ্যমান ফাইলটো ৰাখক (পৰামৰ্শযোগ্য)
ExistingFileNewerOverwriteOrKeepAll=পৰৱৰ্তী সংঘাতবোৰতো এইটো কৰক
ErrorChangingAttr=বিদ্যমান ফাইলৰ বৈশিষ্ট্য সলনি কৰোঁতে ত্ৰুটি হৈছে:
ErrorCreatingTemp=গন্তব্য ফোল্ডাৰত ফাইল সৃষ্টি কৰোঁতে ত্ৰুটি হৈছে:
ErrorReadingSource=উৎস ফাইল পঢ়োঁতে ত্ৰুটি হৈছে:
ErrorCopying=ফাইল কপি কৰোঁতে ত্ৰুটি হৈছে:
ErrorDownloading=ফাইল ডাউনলোড কৰোঁতে ত্ৰুটি হৈছে:
ErrorExtracting=আৰ্কাইভ উলিওৱাৰ সময়ত ত্ৰুটি হৈছে:
ErrorReplacingExistingFile=বিদ্যমান ফাইল প্ৰতিস্থাপন কৰোঁতে ত্ৰুটি হৈছে:
ErrorRestartReplace=RestartReplace বিফল হৈছে:
ErrorRenamingTemp=গন্তব্য ফোল্ডাৰত ফাইলৰ নাম সলনি কৰোঁতে ত্ৰুটি হৈছে:
ErrorRegisterServer=DLL/OCX পঞ্জীয়ন কৰিব নোৱাৰি: %1
ErrorRegSvr32Failed=RegSvr32 এক্সিট কোড %1 লৈ বিফল হৈছে
ErrorRegisterTypeLib=টাইপ লাইব্ৰেৰী পঞ্জীয়ন কৰিব নোৱাৰি: %1

; *** Uninstall display name markings
; used for example as 'My Program (32-bit)'
UninstallDisplayNameMark=%1 (%2)
; used for example as 'My Program (32-bit, All users)'
UninstallDisplayNameMarks=%1 (%2, %3)
UninstallDisplayNameMark32Bit=32-বিট
UninstallDisplayNameMark64Bit=64-বিট
UninstallDisplayNameMarkAllUsers=সকলো ব্যৱহাৰকাৰী
UninstallDisplayNameMarkCurrentUser=বৰ্তমানৰ ব্যৱহাৰকাৰী

; *** Post-installation errors
ErrorOpeningReadme=README ফাইলটো খুলোঁতে ত্ৰুটি হৈছে।
ErrorRestartingComputer=ছেটআপে কম্পিউটাৰ ৰিষ্টাৰ্ট কৰিব পৰা নাই। অনুগ্ৰহ কৰি এইটো নিজে কৰক।

; *** Uninstaller messages
UninstallNotFound=“%1” ফাইলটো নাই। আনইনষ্টল কৰিব নোৱাৰি।
UninstallOpenError=“%1” ফাইলটো খুলিব নোৱাৰিলে। আনইনষ্টল কৰিব নোৱাৰি
UninstallUnsupportedVer=আনইনষ্টল লগ ফাইল “%1” এনে এটা ফৰ্মেটত আছে যাক আনইনষ্টলাৰৰ এই সংস্কৰণে চিনি নাপায়। আনইনষ্টল কৰিব নোৱাৰি
UninstallUnknownEntry=আনইনষ্টল লগত এটা অজ্ঞাত এণ্ট্ৰি (%1) পোৱা গৈছে
ConfirmUninstall=আপুনি নিশ্চিতনে যে %1 আৰু ইয়াৰ সকলো উপাদান সম্পূৰ্ণকৈ আঁতৰাব বিচাৰে?
UninstallOnlyOnWin64=এই ইনষ্টলেচন কেৱল 64-বিট Windows-ত আনইনষ্টল কৰিব পাৰি।
OnlyAdminCanUninstall=এই ইনষ্টলেচন কেৱল প্ৰশাসকৰ অধিকাৰ থকা ব্যৱহাৰকাৰীয়ে আনইনষ্টল কৰিব পাৰে।
UninstallStatusLabel=আপোনাৰ কম্পিউটাৰৰ পৰা %1 আঁতৰোৱা পৰ্যন্ত অনুগ্ৰহ কৰি অপেক্ষা কৰক।
UninstalledAll=%1 সফলভাৱে আপোনাৰ কম্পিউটাৰৰ পৰা আঁতৰোৱা হৈছে।
UninstalledMost=%1 আনইনষ্টল সম্পূৰ্ণ হৈছে।%n%nকিছুমান উপাদান আঁতৰাব পৰা নাই। এইবোৰ নিজে আঁতৰাব পাৰি।
UninstalledAndNeedsRestart=%1 আনইনষ্টল সম্পূৰ্ণ কৰিবলৈ আপোনাৰ কম্পিউটাৰ ৰিষ্টাৰ্ট কৰিব লাগিব।%n%nএতিয়া ৰিষ্টাৰ্ট কৰিব বিচাৰেনে?
UninstallDataCorrupted=“%1” ফাইলটো নষ্ট হৈছে। আনইনষ্টল কৰিব নোৱাৰি

; *** Uninstallation phase messages
ConfirmDeleteSharedFileTitle=শ্বেয়াৰ কৰা ফাইল আঁতৰাবনে?
ConfirmDeleteSharedFile2=চিষ্টেমে জনাইছে যে তলৰ শ্বেয়াৰ কৰা ফাইলটো আৰু কোনো প্ৰগ্ৰামে ব্যৱহাৰ কৰা নাই। আনইনষ্টলে এই শ্বেয়াৰ কৰা ফাইলটো আঁতৰাব বিচাৰেনে?%n%nযদি কোনো প্ৰগ্ৰামে এতিয়াও এই ফাইল ব্যৱহাৰ কৰি থাকে আৰু ইয়াক আঁতৰোৱা হয়, তেন্তে সেই প্ৰগ্ৰামবোৰে ঠিকমতে কাম নকৰিবও পাৰে। নিশ্চিত নহলে “নহয়” বাছক। ফাইলটো চিষ্টেমত থাকি দিলে কোনো ক্ষতি নহয়।
SharedFileNameLabel=ফাইলৰ নাম:
SharedFileLocationLabel=অৱস্থান:
WizardUninstalling=আনইনষ্টলৰ অৱস্থা
StatusUninstalling=%1 আনইনষ্টল হৈ আছে...

; *** Shutdown block reasons
ShutdownBlockReasonInstallingApp=%1 ইনষ্টল হৈ আছে।
ShutdownBlockReasonUninstallingApp=%1 আনইনষ্টল হৈ আছে।

; The custom messages below aren't used by Setup itself, but if you make
; use of them in your scripts, you'll want to translate them.

[CustomMessages]

NameAndVersion=%1 সংস্কৰণ %2
AdditionalIcons=অতিৰিক্ত শৰ্টকাট:
CreateDesktopIcon=ডেস্কটপ শৰ্টকাট সৃষ্টি কৰক
CreateQuickLaunchIcon=Quick Launch শৰ্টকাট সৃষ্টি কৰক
ProgramOnTheWeb=ৱেবত %1
UninstallProgram=%1 আনইনষ্টল কৰক
LaunchProgram=%1 আৰম্ভ কৰক
AssocFileExtension=%1-ক %2 ফাইল এক্সটেনচনৰ সৈতে যুক্ত কৰক
AssocingFileExtension=%1-ক %2 ফাইল এক্সটেনচনৰ সৈতে যুক্ত কৰা হৈ আছে...
AutoStartProgramGroupDescription=ষ্টাৰ্টআপ:
AutoStartProgram=%1 নিজে নিজে আৰম্ভ কৰক
AddonHostProgramNotFound=আপুনি বাছি লোৱা ফোল্ডাৰত %1 পোৱা নাযায়।%n%nতথাপি আগবাঢ়িবনে?

; The messages that are Lang Switcher's own (the sample is English.isl).
LocalCertReady=ছেটআপে এই কম্পিউটাৰত এটা চাৰ্টিফিকেট সৃষ্টি কৰি তাৰে Lang Switcher-ত স্বাক্ষৰ কৰিব, যাতে প্ৰশাসক হিচাপে চলোৱা প্ৰগ্ৰামৰ ৱিণ্ডোতো ই কাম কৰিব পাৰে।
LocalCertFailed=Lang Switcher ইনষ্টল হৈছে, কিন্তু প্ৰশাসক হিচাপে চলোৱা প্ৰগ্ৰামৰ ৱিণ্ডোত ই কাম নকৰিব: Windows-এ এই কম্পিউটাৰত সৃষ্টি কৰা চাৰ্টিফিকেট গ্ৰহণ কৰা নাই। বিৱৰণৰ বাবে ছেটআপ লগ চাওক।
RemoveSettingsQuestion=Lang Switcher-ৰ ছেটিংছ ফাইলো আঁতৰাবনে?
RemoveSettingsKeep=পিছত আকৌ ইনষ্টল কৰিবলৈ ছেটিংছ ৰাখিবলৈ “নহয়” বাছক।
AuthorSlogan=মনেৰে বনোৱা প্ৰগ্ৰাম
