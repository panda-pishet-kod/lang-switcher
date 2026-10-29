; Lang Switcher -- the messages of the installer in Bengali (stage E108, group A).
;
; Inno Setup has no official Bengali translation (inno_official = false in res\lang\languages.toml),
; so this file carries the whole of it: [LangOptions], every message of the compiler:Default.isl
; (6.5.0+), and the messages of the installer that are Lang Switcher's own. It is read after
; compiler:Default.isl as the second MessagesFile of the "bengali" entry of [Languages] in
; ..\LangSwitcher.iss, and so replaces the English text name for name.
; English.isl is the sample the own messages translate; the glossary of the language is
; l10n\glossary\bn.tsv of the control contour. The name of the program is not translated.
; As in Default.isl: a message that has no full stop there has no danda here (Setup adds its own).
; No accelerators (&): the file has no official Inno translation, so no letter is claimed twice
; in one window (RULES 16). Names of buttons inside a message stand in the quotes of the language.

[LangOptions]
LanguageName=বাংলা
LanguageID=$0445
LanguageCodePage=0

[Messages]

; *** Application titles
SetupAppTitle=সেটআপ
SetupWindowTitle=সেটআপ - %1
UninstallAppTitle=আনইনস্টল
UninstallAppFullTitle=%1 আনইনস্টল

; *** Misc. common
InformationTitle=তথ্য
ConfirmTitle=নিশ্চিত করুন
ErrorTitle=ত্রুটি

; *** SetupLdr messages
SetupLdrStartupMessage=এটি %1 ইনস্টল করবে। আপনি কি চালিয়ে যেতে চান?
LdrCannotCreateTemp=অস্থায়ী ফাইল তৈরি করা যায়নি। সেটআপ বাতিল করা হয়েছে
LdrCannotExecTemp=অস্থায়ী ফোল্ডারের ফাইল চালানো যায়নি। সেটআপ বাতিল করা হয়েছে
HelpTextNote=

; *** Startup error messages
LastErrorMessage=%1.%n%nত্রুটি %2: %3
SetupFileMissing=ইনস্টলেশন ফোল্ডারে %1 ফাইলটি নেই। অনুগ্রহ করে সমস্যাটি ঠিক করুন অথবা প্রোগ্রামের একটি নতুন কপি সংগ্রহ করুন।
SetupFileCorrupt=সেটআপ ফাইলগুলো নষ্ট হয়ে গেছে। অনুগ্রহ করে প্রোগ্রামের একটি নতুন কপি সংগ্রহ করুন।
SetupFileCorruptOrWrongVer=সেটআপ ফাইলগুলো নষ্ট হয়ে গেছে, অথবা সেটআপের এই সংস্করণের সঙ্গে মেলে না। অনুগ্রহ করে সমস্যাটি ঠিক করুন অথবা প্রোগ্রামের একটি নতুন কপি সংগ্রহ করুন।
InvalidParameter=কমান্ড লাইনে একটি অবৈধ প্যারামিটার দেওয়া হয়েছে:%n%n%1
SetupAlreadyRunning=সেটআপ ইতিমধ্যে চলছে।
WindowsVersionNotSupported=আপনার কম্পিউটারে চলা Windows-এর সংস্করণ এই প্রোগ্রাম সমর্থন করে না।
WindowsServicePackRequired=এই প্রোগ্রামের জন্য %1 সার্ভিস প্যাক %2 বা তার পরের সংস্করণ দরকার।
NotOnThisPlatform=এই প্রোগ্রাম %1-এ চলবে না।
OnlyOnThisPlatform=এই প্রোগ্রাম %1-এ চালাতে হবে।
OnlyOnTheseArchitectures=এই প্রোগ্রাম কেবল নিচের প্রসেসর আর্কিটেকচারের জন্য তৈরি Windows-এ ইনস্টল করা যায়:%n%n%1
WinVersionTooLowError=এই প্রোগ্রামের জন্য %1 সংস্করণ %2 বা তার পরের সংস্করণ দরকার।
WinVersionTooHighError=এই প্রোগ্রাম %1 সংস্করণ %2 বা তার পরের সংস্করণে ইনস্টল করা যায় না।
AdminPrivilegesRequired=এই প্রোগ্রাম ইনস্টল করার সময় আপনাকে অ্যাডমিনিস্ট্রেটর হিসেবে সাইন ইন করতে হবে।
PowerUserPrivilegesRequired=এই প্রোগ্রাম ইনস্টল করার সময় আপনাকে অ্যাডমিনিস্ট্রেটর অথবা Power Users গোষ্ঠীর সদস্য হিসেবে সাইন ইন করতে হবে।
SetupAppRunningError=সেটআপ দেখেছে যে %1 এখন চলছে।%n%nঅনুগ্রহ করে এর সব উইন্ডো এখনই বন্ধ করুন, তারপর চালিয়ে যেতে “ঠিক আছে” বা বের হতে “বাতিল” ক্লিক করুন।
UninstallAppRunningError=আনইনস্টল দেখেছে যে %1 এখন চলছে।%n%nঅনুগ্রহ করে এর সব উইন্ডো এখনই বন্ধ করুন, তারপর চালিয়ে যেতে “ঠিক আছে” বা বের হতে “বাতিল” ক্লিক করুন।

; *** Startup questions
PrivilegesRequiredOverrideTitle=সেটআপের ইনস্টল মোড বেছে নিন
PrivilegesRequiredOverrideInstruction=ইনস্টল মোড বেছে নিন
PrivilegesRequiredOverrideText1=%1 সব ব্যবহারকারীর জন্য (অ্যাডমিনিস্ট্রেটর অধিকার লাগবে) অথবা শুধু আপনার জন্য ইনস্টল করা যায়।
PrivilegesRequiredOverrideText2=%1 শুধু আপনার জন্য অথবা সব ব্যবহারকারীর জন্য (অ্যাডমিনিস্ট্রেটর অধিকার লাগবে) ইনস্টল করা যায়।
PrivilegesRequiredOverrideAllUsers=সব ব্যবহারকারীর জন্য ইনস্টল করুন
PrivilegesRequiredOverrideAllUsersRecommended=সব ব্যবহারকারীর জন্য ইনস্টল করুন (প্রস্তাবিত)
PrivilegesRequiredOverrideCurrentUser=শুধু আমার জন্য ইনস্টল করুন
PrivilegesRequiredOverrideCurrentUserRecommended=শুধু আমার জন্য ইনস্টল করুন (প্রস্তাবিত)

; *** Misc. errors
ErrorCreatingDir=সেটআপ “%1” ফোল্ডারটি তৈরি করতে পারেনি
ErrorTooManyFilesInDir=“%1” ফোল্ডারে ফাইল তৈরি করা যায়নি, কারণ তাতে অনেক বেশি ফাইল আছে

; *** Setup common messages
ExitSetupTitle=সেটআপ থেকে বের হন
ExitSetupMessage=সেটআপ সম্পূর্ণ হয়নি। এখন বের হলে প্রোগ্রামটি ইনস্টল হবে না।%n%nইনস্টলেশন শেষ করতে আপনি পরে আবার সেটআপ চালাতে পারেন।%n%nসেটআপ থেকে বের হবেন?
AboutSetupMenuItem=সেটআপ সম্পর্কে...
AboutSetupTitle=সেটআপ সম্পর্কে
AboutSetupMessage=%1 সংস্করণ %2%n%3%n%n%1 হোম পেজ:%n%4
AboutSetupNote=
TranslatorNote=

; *** Buttons
ButtonBack=< পিছনে
ButtonNext=পরবর্তী >
ButtonInstall=ইনস্টল করুন
ButtonOK=ঠিক আছে
ButtonCancel=বাতিল
ButtonYes=হ্যাঁ
ButtonYesToAll=সবগুলোর জন্য হ্যাঁ
ButtonNo=না
ButtonNoToAll=সবগুলোর জন্য না
ButtonFinish=সমাপ্ত
ButtonBrowse=ব্রাউজ...
ButtonWizardBrowse=ব্রাউজ...
ButtonNewFolder=নতুন ফোল্ডার তৈরি করুন

; *** "Select Language" dialog messages
SelectLanguageTitle=সেটআপের ভাষা বেছে নিন
SelectLanguageLabel=ইনস্টলেশনের সময় যে ভাষা ব্যবহার করবেন, তা বেছে নিন।

; *** Common wizard text
ClickNext=চালিয়ে যেতে “পরবর্তী” বা সেটআপ থেকে বের হতে “বাতিল” ক্লিক করুন।
BeveledLabel=
BrowseDialogTitle=ফোল্ডার বেছে নিন
BrowseDialogLabel=নিচের তালিকা থেকে একটি ফোল্ডার বেছে নিন, তারপর “ঠিক আছে” ক্লিক করুন।
NewFolderName=নতুন ফোল্ডার

; *** "Welcome" wizard page
WelcomeLabel1=[name] সেটআপ উইজার্ডে স্বাগতম
WelcomeLabel2=এটি আপনার কম্পিউটারে [name/ver] ইনস্টল করবে।%n%nচালিয়ে যাওয়ার আগে অন্য সব অ্যাপ্লিকেশন বন্ধ করে নেওয়ার পরামর্শ দেওয়া হচ্ছে।

; *** "Password" wizard page
WizardPassword=পাসওয়ার্ড
PasswordLabel1=এই ইনস্টলেশন পাসওয়ার্ড দিয়ে সুরক্ষিত।
PasswordLabel3=অনুগ্রহ করে পাসওয়ার্ড দিন, তারপর চালিয়ে যেতে “পরবর্তী” ক্লিক করুন। পাসওয়ার্ডে বড় হাতের ও ছোট হাতের অক্ষর আলাদা ধরা হয়।
PasswordEditLabel=পাসওয়ার্ড:
IncorrectPassword=আপনার দেওয়া পাসওয়ার্ড সঠিক নয়। অনুগ্রহ করে আবার চেষ্টা করুন।

; *** "License Agreement" wizard page
WizardLicense=লাইসেন্স চুক্তি
LicenseLabel=চালিয়ে যাওয়ার আগে অনুগ্রহ করে নিচের গুরুত্বপূর্ণ তথ্য পড়ুন।
LicenseLabel3=অনুগ্রহ করে নিচের লাইসেন্স চুক্তি পড়ুন। ইনস্টলেশন চালিয়ে যাওয়ার আগে আপনাকে এই চুক্তির শর্ত মেনে নিতে হবে।
LicenseAccepted=আমি চুক্তিটি মেনে নিচ্ছি
LicenseNotAccepted=আমি চুক্তিটি মেনে নিচ্ছি না

; *** "Information" wizard pages
WizardInfoBefore=তথ্য
InfoBeforeLabel=চালিয়ে যাওয়ার আগে অনুগ্রহ করে নিচের গুরুত্বপূর্ণ তথ্য পড়ুন।
InfoBeforeClickLabel=সেটআপ চালিয়ে যেতে প্রস্তুত হলে “পরবর্তী” ক্লিক করুন।
WizardInfoAfter=তথ্য
InfoAfterLabel=চালিয়ে যাওয়ার আগে অনুগ্রহ করে নিচের গুরুত্বপূর্ণ তথ্য পড়ুন।
InfoAfterClickLabel=সেটআপ চালিয়ে যেতে প্রস্তুত হলে “পরবর্তী” ক্লিক করুন।

; *** "User Information" wizard page
WizardUserInfo=ব্যবহারকারীর তথ্য
UserInfoDesc=অনুগ্রহ করে আপনার তথ্য দিন।
UserInfoName=ব্যবহারকারীর নাম:
UserInfoOrg=প্রতিষ্ঠান:
UserInfoSerial=সিরিয়াল নম্বর:
UserInfoNameRequired=আপনাকে একটি নাম দিতে হবে।

; *** "Select Destination Location" wizard page
WizardSelectDir=গন্তব্য অবস্থান বেছে নিন
SelectDirDesc=[name] কোথায় ইনস্টল করা হবে?
SelectDirLabel3=সেটআপ [name] নিচের ফোল্ডারে ইনস্টল করবে।
SelectDirBrowseLabel=চালিয়ে যেতে “পরবর্তী” ক্লিক করুন। অন্য ফোল্ডার বেছে নিতে চাইলে “ব্রাউজ” ক্লিক করুন।
DiskSpaceGBLabel=কমপক্ষে [gb] GB খালি ডিস্ক জায়গা দরকার।
DiskSpaceMBLabel=কমপক্ষে [mb] MB খালি ডিস্ক জায়গা দরকার।
CannotInstallToNetworkDrive=সেটআপ নেটওয়ার্ক ড্রাইভে ইনস্টল করতে পারে না।
CannotInstallToUNCPath=সেটআপ UNC পাথে ইনস্টল করতে পারে না।
InvalidPath=ড্রাইভ অক্ষরসহ একটি সম্পূর্ণ পাথ দিতে হবে; যেমন:%n%nC:\APP%n%nঅথবা এই আকারের একটি UNC পাথ:%n%n\\server\share
InvalidDrive=আপনার বাছা ড্রাইভ বা UNC শেয়ার নেই অথবা পাওয়া যাচ্ছে না। অনুগ্রহ করে অন্যটি বেছে নিন।
DiskSpaceWarningTitle=ডিস্কে যথেষ্ট জায়গা নেই
DiskSpaceWarning=ইনস্টল করতে সেটআপের কমপক্ষে %1 KB খালি জায়গা লাগবে, কিন্তু বাছা ড্রাইভে আছে মাত্র %2 KB।%n%nতবুও কি চালিয়ে যেতে চান?
DirNameTooLong=ফোল্ডারের নাম বা পাথ খুব লম্বা।
InvalidDirName=ফোল্ডারের নাম সঠিক নয়।
BadDirName32=ফোল্ডারের নামে নিচের কোনো অক্ষর থাকতে পারবে না:%n%n%1
DirExistsTitle=ফোল্ডার আছে
DirExists=এই ফোল্ডারটি:%n%n%1%n%nআগে থেকেই আছে। তবুও কি ওই ফোল্ডারে ইনস্টল করতে চান?
DirDoesntExistTitle=ফোল্ডার নেই
DirDoesntExist=এই ফোল্ডারটি:%n%n%1%n%nনেই। ফোল্ডারটি কি তৈরি করা হবে?

; *** "Select Components" wizard page
WizardSelectComponents=কম্পোনেন্ট বেছে নিন
SelectComponentsDesc=কোন কম্পোনেন্টগুলো ইনস্টল করা হবে?
SelectComponentsLabel2=যে কম্পোনেন্টগুলো ইনস্টল করতে চান সেগুলো বেছে নিন, আর যেগুলো চান না সেগুলোর টিক তুলে দিন। প্রস্তুত হলে “পরবর্তী” ক্লিক করুন।
FullInstallation=পূর্ণ ইনস্টলেশন
; if possible don't translate 'Compact' as 'Minimal' (I mean 'Minimal' in your language)
CompactInstallation=সংক্ষিপ্ত ইনস্টলেশন
CustomInstallation=কাস্টম ইনস্টলেশন
NoUninstallWarningTitle=কম্পোনেন্ট আগে থেকেই আছে
NoUninstallWarning=সেটআপ দেখেছে যে নিচের কম্পোনেন্টগুলো আপনার কম্পিউটারে আগে থেকেই ইনস্টল করা আছে:%n%n%1%n%nএগুলো বাদ দিলে এগুলো আনইনস্টল হবে না।%n%nতবুও কি চালিয়ে যেতে চান?
ComponentSize1=%1 KB
ComponentSize2=%1 MB
ComponentsDiskSpaceGBLabel=বর্তমান নির্বাচনের জন্য কমপক্ষে [gb] GB ডিস্ক জায়গা লাগবে।
ComponentsDiskSpaceMBLabel=বর্তমান নির্বাচনের জন্য কমপক্ষে [mb] MB ডিস্ক জায়গা লাগবে।

; *** "Select Additional Tasks" wizard page
WizardSelectTasks=অতিরিক্ত কাজ বেছে নিন
SelectTasksDesc=কোন অতিরিক্ত কাজগুলো করা হবে?
SelectTasksLabel2=[name] ইনস্টল করার সময় সেটআপ যেসব অতিরিক্ত কাজ করবে সেগুলো বেছে নিন, তারপর “পরবর্তী” ক্লিক করুন।

; *** "Select Start Menu Folder" wizard page
WizardSelectProgramGroup=স্টার্ট মেনু ফোল্ডার বেছে নিন
SelectStartMenuFolderDesc=সেটআপ প্রোগ্রামের শর্টকাটগুলো কোথায় রাখবে?
SelectStartMenuFolderLabel3=সেটআপ প্রোগ্রামের শর্টকাটগুলো নিচের স্টার্ট মেনু ফোল্ডারে তৈরি করবে।
SelectStartMenuFolderBrowseLabel=চালিয়ে যেতে “পরবর্তী” ক্লিক করুন। অন্য ফোল্ডার বেছে নিতে চাইলে “ব্রাউজ” ক্লিক করুন।
MustEnterGroupName=আপনাকে একটি ফোল্ডারের নাম দিতে হবে।
GroupNameTooLong=ফোল্ডারের নাম বা পাথ খুব লম্বা।
InvalidGroupName=ফোল্ডারের নাম সঠিক নয়।
BadGroupName=ফোল্ডারের নামে নিচের কোনো অক্ষর থাকতে পারবে না:%n%n%1
NoProgramGroupCheck2=স্টার্ট মেনু ফোল্ডার তৈরি করবেন না

; *** "Ready to Install" wizard page
WizardReady=ইনস্টলের জন্য প্রস্তুত
ReadyLabel1=সেটআপ এখন আপনার কম্পিউটারে [name] ইনস্টল করা শুরু করতে প্রস্তুত।
ReadyLabel2a=ইনস্টলেশন চালিয়ে যেতে “ইনস্টল করুন” ক্লিক করুন, অথবা কোনো সেটিং দেখতে বা বদলাতে চাইলে “পিছনে” ক্লিক করুন।
ReadyLabel2b=ইনস্টলেশন চালিয়ে যেতে “ইনস্টল করুন” ক্লিক করুন।
ReadyMemoUserInfo=ব্যবহারকারীর তথ্য:
ReadyMemoDir=গন্তব্য অবস্থান:
ReadyMemoType=সেটআপের ধরন:
ReadyMemoComponents=বাছা কম্পোনেন্ট:
ReadyMemoGroup=স্টার্ট মেনু ফোল্ডার:
ReadyMemoTasks=অতিরিক্ত কাজ:

; *** TDownloadWizardPage wizard page and DownloadTemporaryFile
DownloadingLabel2=ফাইল ডাউনলোড হচ্ছে...
ButtonStopDownload=ডাউনলোড বন্ধ করুন
StopDownload=আপনি কি নিশ্চিত যে ডাউনলোড বন্ধ করতে চান?
ErrorDownloadAborted=ডাউনলোড বাতিল করা হয়েছে
ErrorDownloadFailed=ডাউনলোড ব্যর্থ: %1 %2
ErrorDownloadSizeFailed=আকার জানা যায়নি: %1 %2
ErrorProgress=অবৈধ অগ্রগতি: %2-এর মধ্যে %1
ErrorFileSize=ফাইলের আকার অবৈধ: প্রত্যাশিত %1, পাওয়া গেছে %2

; *** TExtractionWizardPage wizard page and ExtractArchive
ExtractingLabel=ফাইল বের করা হচ্ছে...
ButtonStopExtraction=বের করা বন্ধ করুন
StopExtraction=আপনি কি নিশ্চিত যে বের করা বন্ধ করতে চান?
ErrorExtractionAborted=বের করা বাতিল করা হয়েছে
ErrorExtractionFailed=বের করা ব্যর্থ: %1

; *** Archive extraction failure details
ArchiveIncorrectPassword=পাসওয়ার্ড সঠিক নয়
ArchiveIsCorrupted=আর্কাইভ নষ্ট হয়ে গেছে
ArchiveUnsupportedFormat=আর্কাইভের ফরম্যাট সমর্থিত নয়

; *** "Preparing to Install" wizard page
WizardPreparing=ইনস্টলের প্রস্তুতি চলছে
PreparingDesc=সেটআপ আপনার কম্পিউটারে [name] ইনস্টল করার প্রস্তুতি নিচ্ছে।
PreviousInstallNotCompleted=আগের কোনো প্রোগ্রামের ইনস্টলেশন বা অপসারণ সম্পূর্ণ হয়নি। সেই ইনস্টলেশন শেষ করতে আপনাকে কম্পিউটার রিস্টার্ট করতে হবে।%n%nকম্পিউটার রিস্টার্ট করার পর [name] ইনস্টলেশন শেষ করতে আবার সেটআপ চালান।
CannotContinue=সেটআপ চালিয়ে যেতে পারছে না। বের হতে অনুগ্রহ করে “বাতিল” ক্লিক করুন।
ApplicationsFound=সেটআপ যেসব ফাইল আপডেট করবে, নিচের অ্যাপ্লিকেশনগুলো সেগুলো ব্যবহার করছে। সেটআপকে এই অ্যাপ্লিকেশনগুলো নিজে থেকে বন্ধ করতে দেওয়ার পরামর্শ দেওয়া হচ্ছে।
ApplicationsFound2=সেটআপ যেসব ফাইল আপডেট করবে, নিচের অ্যাপ্লিকেশনগুলো সেগুলো ব্যবহার করছে। সেটআপকে এই অ্যাপ্লিকেশনগুলো নিজে থেকে বন্ধ করতে দেওয়ার পরামর্শ দেওয়া হচ্ছে। ইনস্টলেশন শেষ হলে সেটআপ অ্যাপ্লিকেশনগুলো আবার চালু করার চেষ্টা করবে।
CloseApplications=অ্যাপ্লিকেশনগুলো নিজে থেকে বন্ধ করুন
DontCloseApplications=অ্যাপ্লিকেশনগুলো বন্ধ করবেন না
ErrorCloseApplications=সেটআপ সব অ্যাপ্লিকেশন নিজে থেকে বন্ধ করতে পারেনি। সেটআপ যেসব ফাইল আপডেট করবে, চালিয়ে যাওয়ার আগে সেগুলো ব্যবহার করছে এমন সব অ্যাপ্লিকেশন বন্ধ করে নেওয়ার পরামর্শ দেওয়া হচ্ছে।
PrepareToInstallNeedsRestart=সেটআপকে আপনার কম্পিউটার রিস্টার্ট করতে হবে। কম্পিউটার রিস্টার্ট করার পর [name] ইনস্টলেশন শেষ করতে আবার সেটআপ চালান।%n%nএখন কি রিস্টার্ট করতে চান?

; *** "Installing" wizard page
WizardInstalling=ইনস্টল হচ্ছে
InstallingLabel=সেটআপ আপনার কম্পিউটারে [name] ইনস্টল করা পর্যন্ত অনুগ্রহ করে অপেক্ষা করুন।

; *** "Setup Completed" wizard page
FinishedHeadingLabel=[name] সেটআপ উইজার্ড সম্পূর্ণ হচ্ছে
FinishedLabelNoIcons=সেটআপ আপনার কম্পিউটারে [name] ইনস্টল করা শেষ করেছে।
FinishedLabel=সেটআপ আপনার কম্পিউটারে [name] ইনস্টল করা শেষ করেছে। ইনস্টল হওয়া শর্টকাট বেছে নিয়ে অ্যাপ্লিকেশনটি চালু করা যাবে।
ClickFinish=সেটআপ থেকে বের হতে “সমাপ্ত” ক্লিক করুন।
FinishedRestartLabel=[name] ইনস্টলেশন সম্পূর্ণ করতে সেটআপকে আপনার কম্পিউটার রিস্টার্ট করতে হবে। এখন কি রিস্টার্ট করতে চান?
FinishedRestartMessage=[name] ইনস্টলেশন সম্পূর্ণ করতে সেটআপকে আপনার কম্পিউটার রিস্টার্ট করতে হবে।%n%nএখন কি রিস্টার্ট করতে চান?
ShowReadmeCheck=হ্যাঁ, আমি README ফাইলটি দেখতে চাই
YesRadio=হ্যাঁ, এখনই কম্পিউটার রিস্টার্ট করুন
NoRadio=না, আমি পরে কম্পিউটার রিস্টার্ট করব
; used for example as 'Run MyProg.exe'
RunEntryExec=%1 চালান
; used for example as 'View Readme.txt'
RunEntryShellExec=%1 দেখুন

; *** "Setup Needs the Next Disk" stuff
ChangeDiskTitle=সেটআপের পরের ডিস্ক দরকার
SelectDiskLabel2=অনুগ্রহ করে ডিস্ক %1 ঢোকান এবং “ঠিক আছে” ক্লিক করুন।%n%nএই ডিস্কের ফাইলগুলো নিচে দেখানো ফোল্ডার ছাড়া অন্য কোথাও থাকলে সঠিক পাথ লিখুন অথবা “ব্রাউজ” ক্লিক করুন।
PathLabel=পাথ:
FileNotInDir2=“%2”-এ “%1” ফাইলটি পাওয়া যায়নি। অনুগ্রহ করে সঠিক ডিস্ক ঢোকান অথবা অন্য ফোল্ডার বেছে নিন।
SelectDirectoryLabel=অনুগ্রহ করে পরের ডিস্কের অবস্থান দিন।

; *** Installation phase messages
SetupAborted=সেটআপ সম্পূর্ণ হয়নি।%n%nঅনুগ্রহ করে সমস্যাটি ঠিক করে আবার সেটআপ চালান।
AbortRetryIgnoreSelectAction=করণীয় বেছে নিন
AbortRetryIgnoreRetry=আবার চেষ্টা করুন
AbortRetryIgnoreIgnore=ত্রুটি উপেক্ষা করে চালিয়ে যান
AbortRetryIgnoreCancel=ইনস্টলেশন বাতিল করুন
RetryCancelSelectAction=করণীয় বেছে নিন
RetryCancelRetry=আবার চেষ্টা করুন
RetryCancelCancel=বাতিল

; *** Installation status messages
StatusClosingApplications=অ্যাপ্লিকেশন বন্ধ করা হচ্ছে...
StatusCreateDirs=ফোল্ডার তৈরি হচ্ছে...
StatusExtractFiles=ফাইল বের করা হচ্ছে...
StatusDownloadFiles=ফাইল ডাউনলোড হচ্ছে...
StatusCreateIcons=শর্টকাট তৈরি হচ্ছে...
StatusCreateIniEntries=INI এন্ট্রি তৈরি হচ্ছে...
StatusCreateRegistryEntries=রেজিস্ট্রি এন্ট্রি তৈরি হচ্ছে...
StatusRegisterFiles=ফাইল নিবন্ধন করা হচ্ছে...
StatusSavingUninstall=আনইনস্টলের তথ্য সংরক্ষণ করা হচ্ছে...
StatusRunProgram=ইনস্টলেশন শেষ করা হচ্ছে...
StatusRestartingApplications=অ্যাপ্লিকেশন আবার চালু করা হচ্ছে...
StatusRollback=পরিবর্তন ফিরিয়ে নেওয়া হচ্ছে...

; *** Misc. errors
ErrorInternal2=অভ্যন্তরীণ ত্রুটি: %1
ErrorFunctionFailedNoCode=%1 ব্যর্থ হয়েছে
ErrorFunctionFailed=%1 ব্যর্থ হয়েছে; কোড %2
ErrorFunctionFailedWithMessage=%1 ব্যর্থ হয়েছে; কোড %2।%n%3
ErrorExecutingProgram=ফাইল চালানো যায়নি:%n%1

; *** Registry errors
ErrorRegOpenKey=রেজিস্ট্রি কী খুলতে ত্রুটি:%n%1\%2
ErrorRegCreateKey=রেজিস্ট্রি কী তৈরি করতে ত্রুটি:%n%1\%2
ErrorRegWriteKey=রেজিস্ট্রি কী-তে লিখতে ত্রুটি:%n%1\%2

; *** INI errors
ErrorIniEntry=“%1” ফাইলে INI এন্ট্রি তৈরি করতে ত্রুটি।

; *** File copying errors
FileAbortRetryIgnoreSkipNotRecommended=এই ফাইলটি বাদ দিন (প্রস্তাবিত নয়)
FileAbortRetryIgnoreIgnoreNotRecommended=ত্রুটি উপেক্ষা করে চালিয়ে যান (প্রস্তাবিত নয়)
SourceIsCorrupted=উৎস ফাইলটি নষ্ট হয়ে গেছে
SourceDoesntExist=“%1” উৎস ফাইলটি নেই
SourceVerificationFailed=উৎস ফাইলের যাচাই ব্যর্থ হয়েছে: %1
VerificationSignatureDoesntExist=“%1” স্বাক্ষর ফাইলটি নেই
VerificationSignatureInvalid=“%1” স্বাক্ষর ফাইলটি অবৈধ
VerificationKeyNotFound=“%1” স্বাক্ষর ফাইলটি একটি অজানা কী ব্যবহার করেছে
VerificationFileNameIncorrect=ফাইলের নাম সঠিক নয়
VerificationFileTagIncorrect=ফাইলের ট্যাগ সঠিক নয়
VerificationFileSizeIncorrect=ফাইলের আকার সঠিক নয়
VerificationFileHashIncorrect=ফাইলের হ্যাশ সঠিক নয়
ExistingFileReadOnly2=বিদ্যমান ফাইলটি শুধু-পড়ার চিহ্নিত থাকায় বদলানো যায়নি।
ExistingFileReadOnlyRetry=শুধু-পড়ার বৈশিষ্ট্য সরিয়ে আবার চেষ্টা করুন
ExistingFileReadOnlyKeepExisting=বিদ্যমান ফাইলটি রাখুন
ErrorReadingExistingDest=বিদ্যমান ফাইলটি পড়ার সময় ত্রুটি হয়েছে:
FileExistsSelectAction=করণীয় বেছে নিন
FileExists2=ফাইলটি আগে থেকেই আছে।
FileExistsOverwriteExisting=বিদ্যমান ফাইলটি প্রতিস্থাপন করুন
FileExistsKeepExisting=বিদ্যমান ফাইলটি রাখুন
FileExistsOverwriteOrKeepAll=পরবর্তী সংঘাতগুলোতেও এটি করুন
ExistingFileNewerSelectAction=করণীয় বেছে নিন
ExistingFileNewer2=সেটআপ যে ফাইলটি ইনস্টল করতে চাইছে, বিদ্যমান ফাইলটি তার চেয়ে নতুন।
ExistingFileNewerOverwriteExisting=বিদ্যমান ফাইলটি প্রতিস্থাপন করুন
ExistingFileNewerKeepExisting=বিদ্যমান ফাইলটি রাখুন (প্রস্তাবিত)
ExistingFileNewerOverwriteOrKeepAll=পরবর্তী সংঘাতগুলোতেও এটি করুন
ErrorChangingAttr=বিদ্যমান ফাইলের বৈশিষ্ট্য বদলানোর সময় ত্রুটি হয়েছে:
ErrorCreatingTemp=গন্তব্য ফোল্ডারে ফাইল তৈরির সময় ত্রুটি হয়েছে:
ErrorReadingSource=উৎস ফাইল পড়ার সময় ত্রুটি হয়েছে:
ErrorCopying=ফাইল কপি করার সময় ত্রুটি হয়েছে:
ErrorDownloading=ফাইল ডাউনলোড করার সময় ত্রুটি হয়েছে:
ErrorExtracting=আর্কাইভ বের করার সময় ত্রুটি হয়েছে:
ErrorReplacingExistingFile=বিদ্যমান ফাইল প্রতিস্থাপনের সময় ত্রুটি হয়েছে:
ErrorRestartReplace=RestartReplace ব্যর্থ হয়েছে:
ErrorRenamingTemp=গন্তব্য ফোল্ডারে ফাইলের নাম বদলানোর সময় ত্রুটি হয়েছে:
ErrorRegisterServer=DLL/OCX নিবন্ধন করা যায়নি: %1
ErrorRegSvr32Failed=RegSvr32 এক্সিট কোড %1 দিয়ে ব্যর্থ হয়েছে
ErrorRegisterTypeLib=টাইপ লাইব্রেরি নিবন্ধন করা যায়নি: %1

; *** Uninstall display name markings
; used for example as 'My Program (32-bit)'
UninstallDisplayNameMark=%1 (%2)
; used for example as 'My Program (32-bit, All users)'
UninstallDisplayNameMarks=%1 (%2, %3)
UninstallDisplayNameMark32Bit=32-বিট
UninstallDisplayNameMark64Bit=64-বিট
UninstallDisplayNameMarkAllUsers=সব ব্যবহারকারী
UninstallDisplayNameMarkCurrentUser=বর্তমান ব্যবহারকারী

; *** Post-installation errors
ErrorOpeningReadme=README ফাইলটি খুলতে ত্রুটি হয়েছে।
ErrorRestartingComputer=সেটআপ কম্পিউটার রিস্টার্ট করতে পারেনি। অনুগ্রহ করে এটি নিজে করুন।

; *** Uninstaller messages
UninstallNotFound=“%1” ফাইলটি নেই। আনইনস্টল করা যাবে না।
UninstallOpenError=“%1” ফাইলটি খোলা যায়নি। আনইনস্টল করা যাবে না
UninstallUnsupportedVer=আনইনস্টল লগ ফাইল “%1” এমন একটি ফরম্যাটে আছে যা আনইনস্টলারের এই সংস্করণ চেনে না। আনইনস্টল করা যাবে না
UninstallUnknownEntry=আনইনস্টল লগে একটি অজানা এন্ট্রি (%1) পাওয়া গেছে
ConfirmUninstall=আপনি কি নিশ্চিত যে %1 এবং এর সব কম্পোনেন্ট সম্পূর্ণভাবে সরাতে চান?
UninstallOnlyOnWin64=এই ইনস্টলেশন শুধু 64-বিট Windows-এ আনইনস্টল করা যায়।
OnlyAdminCanUninstall=এই ইনস্টলেশন শুধু অ্যাডমিনিস্ট্রেটর অধিকারসম্পন্ন ব্যবহারকারী আনইনস্টল করতে পারেন।
UninstallStatusLabel=আপনার কম্পিউটার থেকে %1 সরানো পর্যন্ত অনুগ্রহ করে অপেক্ষা করুন।
UninstalledAll=%1 সফলভাবে আপনার কম্পিউটার থেকে সরানো হয়েছে।
UninstalledMost=%1 আনইনস্টল সম্পূর্ণ হয়েছে।%n%nকিছু উপাদান সরানো যায়নি। এগুলো নিজে সরিয়ে নেওয়া যাবে।
UninstalledAndNeedsRestart=%1 আনইনস্টল সম্পূর্ণ করতে আপনার কম্পিউটার রিস্টার্ট করতে হবে।%n%nএখন কি রিস্টার্ট করতে চান?
UninstallDataCorrupted=“%1” ফাইলটি নষ্ট হয়ে গেছে। আনইনস্টল করা যাবে না

; *** Uninstallation phase messages
ConfirmDeleteSharedFileTitle=শেয়ার করা ফাইল সরাবেন?
ConfirmDeleteSharedFile2=সিস্টেম জানাচ্ছে যে নিচের শেয়ার করা ফাইলটি আর কোনো প্রোগ্রাম ব্যবহার করছে না। আনইনস্টল কি এই শেয়ার করা ফাইলটি সরাবে?%n%nকোনো প্রোগ্রাম এখনও এই ফাইলটি ব্যবহার করলে এবং এটি সরানো হলে সেই প্রোগ্রামগুলো ঠিকমতো কাজ না-ও করতে পারে। নিশ্চিত না হলে “না” বেছে নিন। ফাইলটি সিস্টেমে রেখে দিলে কোনো ক্ষতি হবে না।
SharedFileNameLabel=ফাইলের নাম:
SharedFileLocationLabel=অবস্থান:
WizardUninstalling=আনইনস্টলের অবস্থা
StatusUninstalling=%1 আনইনস্টল হচ্ছে...

; *** Shutdown block reasons
ShutdownBlockReasonInstallingApp=%1 ইনস্টল হচ্ছে।
ShutdownBlockReasonUninstallingApp=%1 আনইনস্টল হচ্ছে।

; The custom messages below aren't used by Setup itself, but if you make
; use of them in your scripts, you'll want to translate them.

[CustomMessages]

NameAndVersion=%1 সংস্করণ %2
AdditionalIcons=অতিরিক্ত শর্টকাট:
CreateDesktopIcon=ডেস্কটপ শর্টকাট তৈরি করুন
CreateQuickLaunchIcon=Quick Launch শর্টকাট তৈরি করুন
ProgramOnTheWeb=ওয়েবে %1
UninstallProgram=%1 আনইনস্টল করুন
LaunchProgram=%1 চালু করুন
AssocFileExtension=%1-কে %2 ফাইল এক্সটেনশনের সঙ্গে যুক্ত করুন
AssocingFileExtension=%1-কে %2 ফাইল এক্সটেনশনের সঙ্গে যুক্ত করা হচ্ছে...
AutoStartProgramGroupDescription=স্টার্টআপ:
AutoStartProgram=%1 নিজে থেকে চালু করুন
AddonHostProgramNotFound=আপনার বাছা ফোল্ডারে %1 পাওয়া যায়নি।%n%nতবুও কি চালিয়ে যেতে চান?

; The messages that are Lang Switcher's own (the sample is English.isl).
LocalCertReady=সেটআপ এই কম্পিউটারে একটি সার্টিফিকেট তৈরি করে তা দিয়ে Lang Switcher-এ স্বাক্ষর করবে, যাতে অ্যাডমিনিস্ট্রেটর হিসেবে চালানো প্রোগ্রামের উইন্ডোতেও এটি কাজ করতে পারে।
LocalCertFailed=Lang Switcher ইনস্টল হয়েছে, কিন্তু অ্যাডমিনিস্ট্রেটর হিসেবে চালানো প্রোগ্রামের উইন্ডোতে এটি কাজ করবে না: Windows এই কম্পিউটারে তৈরি সার্টিফিকেট গ্রহণ করেনি। বিস্তারিত জানতে সেটআপ লগ দেখুন।
RemoveSettingsQuestion=Lang Switcher-এর সেটিংস ফাইলও কি সরাবেন?
RemoveSettingsKeep=পরে আবার ইনস্টল করার জন্য সেটিংস রাখতে “না” বেছে নিন।
AuthorSlogan=মন দিয়ে তৈরি প্রোগ্রাম
