; Lang Switcher -- the messages of the installer in Urdu (stage E108, group A).
;
; Inno Setup has no official Urdu file, so this file is whole: [LangOptions], all the messages of
; compiler:Default.isl (version 6.5.0 sample) translated, then the stock [CustomMessages] of that
; file and the messages of the installer that are its own (the sample is English.isl). Read as the
; only MessagesFile of the "urdu" entry of [Languages] in ..\LangSwitcher.iss; Default.isl may
; stand before it: what is missing here falls back to English.
; Right to left: RightToLeft=yes, no direction marks in the strings. The accelerators (&) are Urdu
; letters. Messages that end without a full stop in Default.isl end without one here: Setup adds a
; Latin full stop itself. The name of the program is not translated.
; AuthorSlogan (stage E99, decision 167.1): the slogan of the signature of the author at the
; bottom of every page of the wizard -- after "Panda Koder" and a middle dot; light, without
; pathos. The name of the author and the address of the site stay as they are in every language.

[LangOptions]
LanguageName=اردو
LanguageID=$0420
LanguageCodePage=0
RightToLeft=yes
;DialogFontName=
;DialogFontSize=9
;DialogFontBaseScaleWidth=7
;DialogFontBaseScaleHeight=15
;WelcomeFontName=Segoe UI
;WelcomeFontSize=14

[Messages]

; *** Application titles
SetupAppTitle=سیٹ اپ
SetupWindowTitle=سیٹ اپ - %1
UninstallAppTitle=ان انسٹال
UninstallAppFullTitle=%1 ان انسٹال

; *** Misc. common
InformationTitle=معلومات
ConfirmTitle=تصدیق
ErrorTitle=خرابی

; *** SetupLdr messages
SetupLdrStartupMessage=یہ %1 کو انسٹال کرے گا۔ کیا آپ جاری رکھنا چاہتے ہیں؟
LdrCannotCreateTemp=عارضی فائل نہیں بنائی جا سکی۔ سیٹ اپ منسوخ کر دیا گیا
LdrCannotExecTemp=عارضی ڈائریکٹری میں فائل نہیں چلائی جا سکی۔ سیٹ اپ منسوخ کر دیا گیا
HelpTextNote=

; *** Startup error messages
LastErrorMessage=%1.%n%nخرابی %2: %3
SetupFileMissing=فائل %1 انسٹالیشن ڈائریکٹری میں موجود نہیں۔ براہِ کرم مسئلہ حل کریں یا پروگرام کی نئی کاپی حاصل کریں۔
SetupFileCorrupt=سیٹ اپ کی فائلیں خراب ہیں۔ براہِ کرم پروگرام کی نئی کاپی حاصل کریں۔
SetupFileCorruptOrWrongVer=سیٹ اپ کی فائلیں خراب ہیں، یا سیٹ اپ کے اس ورژن سے مطابقت نہیں رکھتیں۔ براہِ کرم مسئلہ حل کریں یا پروگرام کی نئی کاپی حاصل کریں۔
InvalidParameter=کمانڈ لائن پر غلط پیرامیٹر دیا گیا:%n%n%1
SetupAlreadyRunning=سیٹ اپ پہلے سے چل رہا ہے۔
WindowsVersionNotSupported=یہ پروگرام آپ کے کمپیوٹر پر چلنے والے Windows کے ورژن کو سپورٹ نہیں کرتا۔
WindowsServicePackRequired=اس پروگرام کے لیے %1 سروس پیک %2 یا اس کے بعد کا درکار ہے۔
NotOnThisPlatform=یہ پروگرام %1 پر نہیں چلے گا۔
OnlyOnThisPlatform=اس پروگرام کو %1 پر چلانا ضروری ہے۔
OnlyOnTheseArchitectures=یہ پروگرام صرف Windows کے ان ورژنز پر انسٹال ہو سکتا ہے جو درج ذیل پروسیسر آرکیٹیکچرز کے لیے بنائے گئے ہیں:%n%n%1
WinVersionTooLowError=اس پروگرام کے لیے %1 ورژن %2 یا اس کے بعد کا درکار ہے۔
WinVersionTooHighError=یہ پروگرام %1 ورژن %2 یا اس کے بعد کے ورژن پر انسٹال نہیں ہو سکتا۔
AdminPrivilegesRequired=اس پروگرام کو انسٹال کرتے وقت ایڈمنسٹریٹر کے طور پر سائن ان ہونا ضروری ہے۔
PowerUserPrivilegesRequired=اس پروگرام کو انسٹال کرتے وقت ایڈمنسٹریٹر کے طور پر یا Power Users گروپ کے رکن کے طور پر سائن ان ہونا ضروری ہے۔
SetupAppRunningError=سیٹ اپ نے دیکھا ہے کہ %1 اس وقت چل رہا ہے۔%n%nبراہِ کرم اس کی تمام مثالیں ابھی بند کریں، پھر جاری رکھنے کے لیے ٹھیک ہے یا باہر نکلنے کے لیے منسوخ کریں پر کلک کریں۔
UninstallAppRunningError=ان انسٹال نے دیکھا ہے کہ %1 اس وقت چل رہا ہے۔%n%nبراہِ کرم اس کی تمام مثالیں ابھی بند کریں، پھر جاری رکھنے کے لیے ٹھیک ہے یا باہر نکلنے کے لیے منسوخ کریں پر کلک کریں۔

; *** Startup questions
PrivilegesRequiredOverrideTitle=سیٹ اپ انسٹالیشن موڈ منتخب کریں
PrivilegesRequiredOverrideInstruction=انسٹالیشن موڈ منتخب کریں
PrivilegesRequiredOverrideText1=%1 تمام صارفین کے لیے انسٹال کیا جا سکتا ہے (ایڈمنسٹریٹر کی اجازت درکار ہے)، یا صرف آپ کے لیے۔
PrivilegesRequiredOverrideText2=%1 صرف آپ کے لیے انسٹال کیا جا سکتا ہے، یا تمام صارفین کے لیے (ایڈمنسٹریٹر کی اجازت درکار ہے)۔
PrivilegesRequiredOverrideAllUsers=&تمام صارفین کے لیے انسٹال کریں
PrivilegesRequiredOverrideAllUsersRecommended=&تمام صارفین کے لیے انسٹال کریں (تجویز کردہ)
PrivilegesRequiredOverrideCurrentUser=صرف &میرے لیے انسٹال کریں
PrivilegesRequiredOverrideCurrentUserRecommended=صرف &میرے لیے انسٹال کریں (تجویز کردہ)

; *** Misc. errors
ErrorCreatingDir=سیٹ اپ ڈائریکٹری "%1" نہیں بنا سکا
ErrorTooManyFilesInDir=ڈائریکٹری "%1" میں فائل نہیں بنائی جا سکی کیونکہ اس میں بہت زیادہ فائلیں ہیں

; *** Setup common messages
ExitSetupTitle=سیٹ اپ سے باہر نکلیں
ExitSetupMessage=سیٹ اپ مکمل نہیں ہوا۔ اگر آپ ابھی باہر نکلتے ہیں تو پروگرام انسٹال نہیں ہوگا۔%n%nانسٹالیشن مکمل کرنے کے لیے آپ سیٹ اپ کو کسی اور وقت دوبارہ چلا سکتے ہیں۔%n%nسیٹ اپ سے باہر نکلیں؟
AboutSetupMenuItem=سیٹ اپ کے &بارے میں...
AboutSetupTitle=سیٹ اپ کے بارے میں
AboutSetupMessage=%1 ورژن %2%n%3%n%n%1 کا ہوم پیج:%n%4
AboutSetupNote=
TranslatorNote=

; *** Buttons
ButtonBack=< &پچھلا
ButtonNext=&اگلا >
ButtonInstall=&انسٹال کریں
ButtonOK=ٹھیک ہے
ButtonCancel=منسوخ کریں
ButtonYes=&ہاں
ButtonYesToAll=&سب کے لیے ہاں
ButtonNo=&نہیں
ButtonNoToAll=سب کے &لیے نہیں
ButtonFinish=&مکمل کریں
ButtonBrowse=&براؤز کریں...
ButtonWizardBrowse=ب&راؤز کریں...
ButtonNewFolder=&نیا فولڈر بنائیں

; *** "Select Language" dialog messages
SelectLanguageTitle=سیٹ اپ کی زبان منتخب کریں
SelectLanguageLabel=انسٹالیشن کے دوران استعمال ہونے والی زبان منتخب کریں۔

; *** Common wizard text
ClickNext=جاری رکھنے کے لیے اگلا پر کلک کریں، یا سیٹ اپ سے باہر نکلنے کے لیے منسوخ کریں۔
BeveledLabel=
BrowseDialogTitle=فولڈر تلاش کریں
BrowseDialogLabel=نیچے دی گئی فہرست میں سے کوئی فولڈر منتخب کریں، پھر ٹھیک ہے پر کلک کریں۔
NewFolderName=نیا فولڈر

; *** "Welcome" wizard page
WelcomeLabel1=[name] سیٹ اپ وزرڈ میں خوش آمدید
WelcomeLabel2=یہ آپ کے کمپیوٹر پر [name/ver] انسٹال کرے گا۔%n%nجاری رکھنے سے پہلے تمام دیگر ایپلیکیشنز بند کرنے کی سفارش کی جاتی ہے۔

; *** "Password" wizard page
WizardPassword=پاس ورڈ
PasswordLabel1=یہ انسٹالیشن پاس ورڈ سے محفوظ ہے۔
PasswordLabel3=براہِ کرم پاس ورڈ درج کریں، پھر جاری رکھنے کے لیے اگلا پر کلک کریں۔ پاس ورڈ میں بڑے اور چھوٹے حروف کا فرق ہوتا ہے۔
PasswordEditLabel=&پاس ورڈ:
IncorrectPassword=آپ کا درج کردہ پاس ورڈ درست نہیں ہے۔ براہِ کرم دوبارہ کوشش کریں۔

; *** "License Agreement" wizard page
WizardLicense=لائسنس معاہدہ
LicenseLabel=جاری رکھنے سے پہلے براہِ کرم درج ذیل اہم معلومات پڑھیں۔
LicenseLabel3=براہِ کرم درج ذیل لائسنس معاہدہ پڑھیں۔ انسٹالیشن جاری رکھنے سے پہلے آپ کو اس معاہدے کی شرائط قبول کرنا ہوں گی۔
LicenseAccepted=مجھے معاہدہ &قبول ہے
LicenseNotAccepted=مجھے معاہدہ قبول &نہیں ہے

; *** "Information" wizard pages
WizardInfoBefore=معلومات
InfoBeforeLabel=جاری رکھنے سے پہلے براہِ کرم درج ذیل اہم معلومات پڑھیں۔
InfoBeforeClickLabel=جب آپ سیٹ اپ جاری رکھنے کے لیے تیار ہوں تو اگلا پر کلک کریں۔
WizardInfoAfter=معلومات
InfoAfterLabel=جاری رکھنے سے پہلے براہِ کرم درج ذیل اہم معلومات پڑھیں۔
InfoAfterClickLabel=جب آپ سیٹ اپ جاری رکھنے کے لیے تیار ہوں تو اگلا پر کلک کریں۔

; *** "User Information" wizard page
WizardUserInfo=صارف کی معلومات
UserInfoDesc=براہِ کرم اپنی معلومات درج کریں۔
UserInfoName=&صارف کا نام:
UserInfoOrg=&ادارہ:
UserInfoSerial=&سیریل نمبر:
UserInfoNameRequired=آپ کو نام درج کرنا ضروری ہے۔

; *** "Select Destination Location" wizard page
WizardSelectDir=منزل کا مقام منتخب کریں
SelectDirDesc=[name] کہاں انسٹال کیا جائے؟
SelectDirLabel3=سیٹ اپ [name] کو درج ذیل فولڈر میں انسٹال کرے گا۔
SelectDirBrowseLabel=جاری رکھنے کے لیے اگلا پر کلک کریں۔ اگر آپ کوئی دوسرا فولڈر منتخب کرنا چاہتے ہیں تو براؤز کریں پر کلک کریں۔
DiskSpaceGBLabel=کم از کم [gb] GB خالی ڈسک اسپیس درکار ہے۔
DiskSpaceMBLabel=کم از کم [mb] MB خالی ڈسک اسپیس درکار ہے۔
CannotInstallToNetworkDrive=سیٹ اپ نیٹ ورک ڈرائیو پر انسٹال نہیں کر سکتا۔
CannotInstallToUNCPath=سیٹ اپ UNC پاتھ پر انسٹال نہیں کر سکتا۔
InvalidPath=آپ کو ڈرائیو لیٹر کے ساتھ مکمل پاتھ درج کرنا ہوگا؛ مثال کے طور پر:%n%nC:\APP%n%nیا اس شکل میں UNC پاتھ:%n%n\\server\share
InvalidDrive=آپ کی منتخب کردہ ڈرائیو یا UNC شیئر موجود نہیں یا قابلِ رسائی نہیں ہے۔ براہِ کرم کوئی دوسری منتخب کریں۔
DiskSpaceWarningTitle=ڈسک اسپیس ناکافی ہے
DiskSpaceWarning=انسٹالیشن کے لیے کم از کم %1 KB خالی جگہ درکار ہے، مگر منتخب ڈرائیو پر صرف %2 KB دستیاب ہے۔%n%nکیا آپ پھر بھی جاری رکھنا چاہتے ہیں؟
DirNameTooLong=فولڈر کا نام یا پاتھ بہت لمبا ہے۔
InvalidDirName=فولڈر کا نام درست نہیں ہے۔
BadDirName32=فولڈر کے ناموں میں درج ذیل حروف شامل نہیں ہو سکتے:%n%n%1
DirExistsTitle=فولڈر موجود ہے
DirExists=فولڈر:%n%n%1%n%nپہلے سے موجود ہے۔ کیا آپ پھر بھی اسی فولڈر میں انسٹال کرنا چاہتے ہیں؟
DirDoesntExistTitle=فولڈر موجود نہیں
DirDoesntExist=فولڈر:%n%n%1%n%nموجود نہیں ہے۔ کیا آپ چاہتے ہیں کہ فولڈر بنا دیا جائے؟

; *** "Select Components" wizard page
WizardSelectComponents=اجزاء منتخب کریں
SelectComponentsDesc=کون سے اجزاء انسٹال کیے جائیں؟
SelectComponentsLabel2=وہ اجزاء منتخب کریں جو آپ انسٹال کرنا چاہتے ہیں؛ جو نہیں چاہتے ان کا نشان ہٹا دیں۔ جاری رکھنے کے لیے تیار ہوں تو اگلا پر کلک کریں۔
FullInstallation=مکمل انسٹالیشن
; if possible don't translate 'Compact' as 'Minimal' (I mean 'Minimal' in your language)
CompactInstallation=مختصر انسٹالیشن
CustomInstallation=حسبِ ضرورت انسٹالیشن
NoUninstallWarningTitle=اجزاء موجود ہیں
NoUninstallWarning=سیٹ اپ نے دیکھا ہے کہ درج ذیل اجزاء آپ کے کمپیوٹر پر پہلے سے انسٹال ہیں:%n%n%1%n%nان اجزاء کا انتخاب ختم کرنے سے وہ ان انسٹال نہیں ہوں گے۔%n%nکیا آپ پھر بھی جاری رکھنا چاہتے ہیں؟
ComponentSize1=%1 KB
ComponentSize2=%1 MB
ComponentsDiskSpaceGBLabel=موجودہ انتخاب کے لیے کم از کم [gb] GB ڈسک اسپیس درکار ہے۔
ComponentsDiskSpaceMBLabel=موجودہ انتخاب کے لیے کم از کم [mb] MB ڈسک اسپیس درکار ہے۔

; *** "Select Additional Tasks" wizard page
WizardSelectTasks=اضافی کام منتخب کریں
SelectTasksDesc=کون سے اضافی کام انجام دیے جائیں؟
SelectTasksLabel2=وہ اضافی کام منتخب کریں جو آپ چاہتے ہیں کہ سیٹ اپ [name] انسٹال کرتے وقت انجام دے، پھر اگلا پر کلک کریں۔

; *** "Select Start Menu Folder" wizard page
WizardSelectProgramGroup=اسٹارٹ مینو فولڈر منتخب کریں
SelectStartMenuFolderDesc=سیٹ اپ پروگرام کے شارٹ کٹ کہاں رکھے؟
SelectStartMenuFolderLabel3=سیٹ اپ پروگرام کے شارٹ کٹ درج ذیل اسٹارٹ مینو فولڈر میں بنائے گا۔
SelectStartMenuFolderBrowseLabel=جاری رکھنے کے لیے اگلا پر کلک کریں۔ اگر آپ کوئی دوسرا فولڈر منتخب کرنا چاہتے ہیں تو براؤز کریں پر کلک کریں۔
MustEnterGroupName=آپ کو فولڈر کا نام درج کرنا ضروری ہے۔
GroupNameTooLong=فولڈر کا نام یا پاتھ بہت لمبا ہے۔
InvalidGroupName=فولڈر کا نام درست نہیں ہے۔
BadGroupName=فولڈر کے نام میں درج ذیل حروف شامل نہیں ہو سکتے:%n%n%1
NoProgramGroupCheck2=اسٹارٹ مینو فولڈر &نہ بنائیں

; *** "Ready to Install" wizard page
WizardReady=انسٹالیشن کے لیے تیار
ReadyLabel1=سیٹ اپ اب آپ کے کمپیوٹر پر [name] انسٹال کرنے کے لیے تیار ہے۔
ReadyLabel2a=انسٹالیشن جاری رکھنے کے لیے انسٹال کریں پر کلک کریں، یا اگر کوئی ترتیب دیکھنا یا بدلنا چاہتے ہیں تو پچھلا پر کلک کریں۔
ReadyLabel2b=انسٹالیشن جاری رکھنے کے لیے انسٹال کریں پر کلک کریں۔
ReadyMemoUserInfo=صارف کی معلومات:
ReadyMemoDir=منزل کا مقام:
ReadyMemoType=سیٹ اپ کی قسم:
ReadyMemoComponents=منتخب اجزاء:
ReadyMemoGroup=اسٹارٹ مینو فولڈر:
ReadyMemoTasks=اضافی کام:

; *** TDownloadWizardPage wizard page and DownloadTemporaryFile
DownloadingLabel2=فائلیں ڈاؤن لوڈ ہو رہی ہیں...
ButtonStopDownload=ڈاؤن لوڈ &روکیں
StopDownload=کیا آپ واقعی ڈاؤن لوڈ روکنا چاہتے ہیں؟
ErrorDownloadAborted=ڈاؤن لوڈ منسوخ کر دیا گیا
ErrorDownloadFailed=ڈاؤن لوڈ ناکام ہو گیا: %1 %2
ErrorDownloadSizeFailed=سائز معلوم کرنا ناکام ہو گیا: %1 %2
ErrorProgress=غلط پیش رفت: %1 از %2
ErrorFileSize=غلط فائل سائز: متوقع %1، ملا %2

; *** TExtractionWizardPage wizard page and ExtractArchive
ExtractingLabel=فائلیں نکالی جا رہی ہیں...
ButtonStopExtraction=ایکسٹریکشن &روکیں
StopExtraction=کیا آپ واقعی ایکسٹریکشن روکنا چاہتے ہیں؟
ErrorExtractionAborted=ایکسٹریکشن منسوخ کر دی گئی
ErrorExtractionFailed=ایکسٹریکشن ناکام ہو گئی: %1

; *** Archive extraction failure details
ArchiveIncorrectPassword=پاس ورڈ غلط ہے
ArchiveIsCorrupted=آرکائیو خراب ہے
ArchiveUnsupportedFormat=آرکائیو کا فارمیٹ سپورٹ نہیں کیا جاتا

; *** "Preparing to Install" wizard page
WizardPreparing=انسٹالیشن کی تیاری
PreparingDesc=سیٹ اپ آپ کے کمپیوٹر پر [name] انسٹال کرنے کی تیاری کر رہا ہے۔
PreviousInstallNotCompleted=کسی پچھلے پروگرام کی انسٹالیشن یا ہٹانے کا عمل مکمل نہیں ہوا۔ اس انسٹالیشن کو مکمل کرنے کے لیے آپ کو اپنا کمپیوٹر دوبارہ شروع کرنا ہوگا۔%n%nکمپیوٹر دوبارہ شروع کرنے کے بعد [name] کی انسٹالیشن مکمل کرنے کے لیے سیٹ اپ دوبارہ چلائیں۔
CannotContinue=سیٹ اپ جاری نہیں رہ سکتا۔ باہر نکلنے کے لیے براہِ کرم منسوخ کریں پر کلک کریں۔
ApplicationsFound=درج ذیل ایپلیکیشنز ایسی فائلیں استعمال کر رہی ہیں جنہیں سیٹ اپ کو اپ ڈیٹ کرنا ہے۔ سفارش کی جاتی ہے کہ آپ سیٹ اپ کو یہ ایپلیکیشنز خودکار طور پر بند کرنے دیں۔
ApplicationsFound2=درج ذیل ایپلیکیشنز ایسی فائلیں استعمال کر رہی ہیں جنہیں سیٹ اپ کو اپ ڈیٹ کرنا ہے۔ سفارش کی جاتی ہے کہ آپ سیٹ اپ کو یہ ایپلیکیشنز خودکار طور پر بند کرنے دیں۔ انسٹالیشن مکمل ہونے کے بعد سیٹ اپ ایپلیکیشنز کو دوبارہ شروع کرنے کی کوشش کرے گا۔
CloseApplications=ایپلیکیشنز &خودکار طور پر بند کریں
DontCloseApplications=ایپلیکیشنز &بند نہ کریں
ErrorCloseApplications=سیٹ اپ تمام ایپلیکیشنز خودکار طور پر بند نہیں کر سکا۔ سفارش کی جاتی ہے کہ جاری رکھنے سے پہلے آپ ان تمام ایپلیکیشنز کو بند کر دیں جو ایسی فائلیں استعمال کر رہی ہیں جنہیں سیٹ اپ کو اپ ڈیٹ کرنا ہے۔
PrepareToInstallNeedsRestart=سیٹ اپ کو آپ کا کمپیوٹر دوبارہ شروع کرنا ہوگا۔ کمپیوٹر دوبارہ شروع ہونے کے بعد [name] کی انسٹالیشن مکمل کرنے کے لیے سیٹ اپ دوبارہ چلائیں۔%n%nکیا آپ ابھی دوبارہ شروع کرنا چاہتے ہیں؟

; *** "Installing" wizard page
WizardInstalling=انسٹال ہو رہا ہے
InstallingLabel=براہِ کرم انتظار کریں جب تک سیٹ اپ آپ کے کمپیوٹر پر [name] انسٹال کر رہا ہے۔

; *** "Setup Completed" wizard page
FinishedHeadingLabel=[name] سیٹ اپ وزرڈ مکمل ہو رہا ہے
FinishedLabelNoIcons=سیٹ اپ نے آپ کے کمپیوٹر پر [name] انسٹال کرنا مکمل کر لیا ہے۔
FinishedLabel=سیٹ اپ نے آپ کے کمپیوٹر پر [name] انسٹال کرنا مکمل کر لیا ہے۔ انسٹال شدہ شارٹ کٹ منتخب کر کے ایپلیکیشن چلائی جا سکتی ہے۔
ClickFinish=سیٹ اپ سے باہر نکلنے کے لیے مکمل کریں پر کلک کریں۔
FinishedRestartLabel=[name] کی انسٹالیشن مکمل کرنے کے لیے سیٹ اپ کو آپ کا کمپیوٹر دوبارہ شروع کرنا ہوگا۔ کیا آپ ابھی دوبارہ شروع کرنا چاہتے ہیں؟
FinishedRestartMessage=[name] کی انسٹالیشن مکمل کرنے کے لیے سیٹ اپ کو آپ کا کمپیوٹر دوبارہ شروع کرنا ہوگا۔%n%nکیا آپ ابھی دوبارہ شروع کرنا چاہتے ہیں؟
ShowReadmeCheck=ہاں، README فائل دیکھنی ہے
YesRadio=&ہاں، کمپیوٹر ابھی دوبارہ شروع کریں
NoRadio=&نہیں، کمپیوٹر بعد میں دوبارہ شروع کرنا ہے
; used for example as 'Run MyProg.exe'
RunEntryExec=%1 چلائیں
; used for example as 'View Readme.txt'
RunEntryShellExec=%1 دیکھیں

; *** "Setup Needs the Next Disk" stuff
ChangeDiskTitle=سیٹ اپ کو اگلی ڈسک درکار ہے
SelectDiskLabel2=براہِ کرم ڈسک %1 داخل کریں اور ٹھیک ہے پر کلک کریں۔%n%nاگر اس ڈسک کی فائلیں نیچے دکھائے گئے فولڈر کے علاوہ کسی اور فولڈر میں ہوں تو درست پاتھ درج کریں یا براؤز کریں پر کلک کریں۔
PathLabel=&پاتھ:
FileNotInDir2=فائل "%1" کو "%2" میں تلاش نہیں کیا جا سکا۔ براہِ کرم درست ڈسک داخل کریں یا کوئی دوسرا فولڈر منتخب کریں۔
SelectDirectoryLabel=براہِ کرم اگلی ڈسک کا مقام بتائیں۔

; *** Installation phase messages
SetupAborted=سیٹ اپ مکمل نہیں ہوا۔%n%nبراہِ کرم مسئلہ حل کریں اور سیٹ اپ دوبارہ چلائیں۔
AbortRetryIgnoreSelectAction=عمل منتخب کریں
AbortRetryIgnoreRetry=&دوبارہ کوشش کریں
AbortRetryIgnoreIgnore=خرابی کو &نظر انداز کریں اور جاری رکھیں
AbortRetryIgnoreCancel=انسٹالیشن منسوخ کریں
RetryCancelSelectAction=عمل منتخب کریں
RetryCancelRetry=&دوبارہ کوشش کریں
RetryCancelCancel=منسوخ کریں

; *** Installation status messages
StatusClosingApplications=ایپلیکیشنز بند کی جا رہی ہیں...
StatusCreateDirs=ڈائریکٹریاں بنائی جا رہی ہیں...
StatusExtractFiles=فائلیں نکالی جا رہی ہیں...
StatusDownloadFiles=فائلیں ڈاؤن لوڈ ہو رہی ہیں...
StatusCreateIcons=شارٹ کٹ بنائے جا رہے ہیں...
StatusCreateIniEntries=INI اندراجات بنائے جا رہے ہیں...
StatusCreateRegistryEntries=رجسٹری اندراجات بنائے جا رہے ہیں...
StatusRegisterFiles=فائلیں رجسٹر کی جا رہی ہیں...
StatusSavingUninstall=ان انسٹال کی معلومات محفوظ کی جا رہی ہیں...
StatusRunProgram=انسٹالیشن مکمل کی جا رہی ہے...
StatusRestartingApplications=ایپلیکیشنز دوبارہ شروع کی جا رہی ہیں...
StatusRollback=تبدیلیاں واپس کی جا رہی ہیں...

; *** Misc. errors
ErrorInternal2=اندرونی خرابی: %1
ErrorFunctionFailedNoCode=%1 ناکام ہو گیا
ErrorFunctionFailed=%1 ناکام ہو گیا؛ کوڈ %2
ErrorFunctionFailedWithMessage=%1 ناکام ہو گیا؛ کوڈ %2۔%n%3
ErrorExecutingProgram=فائل نہیں چلائی جا سکی:%n%1

; *** Registry errors
ErrorRegOpenKey=رجسٹری کلید کھولنے میں خرابی:%n%1\%2
ErrorRegCreateKey=رجسٹری کلید بنانے میں خرابی:%n%1\%2
ErrorRegWriteKey=رجسٹری کلید میں لکھنے میں خرابی:%n%1\%2

; *** INI errors
ErrorIniEntry=فائل "%1" میں INI اندراج بنانے میں خرابی۔

; *** File copying errors
FileAbortRetryIgnoreSkipNotRecommended=یہ فائل &چھوڑ دیں (تجویز نہیں کی جاتی)
FileAbortRetryIgnoreIgnoreNotRecommended=خرابی کو &نظر انداز کریں اور جاری رکھیں (تجویز نہیں کی جاتی)
SourceIsCorrupted=ماخذ فائل خراب ہے
SourceDoesntExist=ماخذ فائل "%1" موجود نہیں ہے
SourceVerificationFailed=ماخذ فائل کی تصدیق ناکام ہو گئی: %1
VerificationSignatureDoesntExist=دستخط کی فائل "%1" موجود نہیں ہے
VerificationSignatureInvalid=دستخط کی فائل "%1" غلط ہے
VerificationKeyNotFound=دستخط کی فائل "%1" ایک نامعلوم کلید استعمال کرتی ہے
VerificationFileNameIncorrect=فائل کا نام غلط ہے
VerificationFileTagIncorrect=فائل کا ٹیگ غلط ہے
VerificationFileSizeIncorrect=فائل کا سائز غلط ہے
VerificationFileHashIncorrect=فائل کا ہیش غلط ہے
ExistingFileReadOnly2=موجودہ فائل تبدیل نہیں کی جا سکی کیونکہ وہ صرف پڑھنے کے لیے نشان زد ہے۔
ExistingFileReadOnlyRetry=صرف پڑھنے کی صفت &ہٹائیں اور دوبارہ کوشش کریں
ExistingFileReadOnlyKeepExisting=موجودہ فائل &برقرار رکھیں
ErrorReadingExistingDest=موجودہ فائل پڑھنے کی کوشش میں خرابی ہوئی:
FileExistsSelectAction=عمل منتخب کریں
FileExists2=فائل پہلے سے موجود ہے۔
FileExistsOverwriteExisting=موجودہ فائل &اوور رائٹ کریں
FileExistsKeepExisting=موجودہ فائل &برقرار رکھیں
FileExistsOverwriteOrKeepAll=اگلے تنازعات کے لیے &یہی کریں
ExistingFileNewerSelectAction=عمل منتخب کریں
ExistingFileNewer2=موجودہ فائل اس فائل سے نئی ہے جو سیٹ اپ انسٹال کرنے کی کوشش کر رہا ہے۔
ExistingFileNewerOverwriteExisting=موجودہ فائل &اوور رائٹ کریں
ExistingFileNewerKeepExisting=موجودہ فائل &برقرار رکھیں (تجویز کردہ)
ExistingFileNewerOverwriteOrKeepAll=اگلے تنازعات کے لیے &یہی کریں
ErrorChangingAttr=موجودہ فائل کی صفات بدلنے کی کوشش میں خرابی ہوئی:
ErrorCreatingTemp=منزل کی ڈائریکٹری میں فائل بنانے کی کوشش میں خرابی ہوئی:
ErrorReadingSource=ماخذ فائل پڑھنے کی کوشش میں خرابی ہوئی:
ErrorCopying=فائل کاپی کرنے کی کوشش میں خرابی ہوئی:
ErrorDownloading=فائل ڈاؤن لوڈ کرنے کی کوشش میں خرابی ہوئی:
ErrorExtracting=آرکائیو نکالنے کی کوشش میں خرابی ہوئی:
ErrorReplacingExistingFile=موجودہ فائل تبدیل کرنے کی کوشش میں خرابی ہوئی:
ErrorRestartReplace=RestartReplace ناکام ہو گیا:
ErrorRenamingTemp=منزل کی ڈائریکٹری میں فائل کا نام بدلنے کی کوشش میں خرابی ہوئی:
ErrorRegisterServer=DLL/OCX رجسٹر نہیں کی جا سکی: %1
ErrorRegSvr32Failed=RegSvr32 ایگزٹ کوڈ %1 کے ساتھ ناکام ہو گیا
ErrorRegisterTypeLib=ٹائپ لائبریری رجسٹر نہیں کی جا سکی: %1

; *** Uninstall display name markings
; used for example as 'My Program (32-bit)'
UninstallDisplayNameMark=%1 (%2)
; used for example as 'My Program (32-bit, All users)'
UninstallDisplayNameMarks=%1 (%2، %3)
UninstallDisplayNameMark32Bit=32-بٹ
UninstallDisplayNameMark64Bit=64-بٹ
UninstallDisplayNameMarkAllUsers=تمام صارفین
UninstallDisplayNameMarkCurrentUser=موجودہ صارف

; *** Post-installation errors
ErrorOpeningReadme=README فائل کھولنے کی کوشش میں خرابی ہوئی۔
ErrorRestartingComputer=سیٹ اپ کمپیوٹر کو دوبارہ شروع نہیں کر سکا۔ براہِ کرم یہ دستی طور پر کریں۔

; *** Uninstaller messages
UninstallNotFound=فائل "%1" موجود نہیں ہے۔ ان انسٹال نہیں کیا جا سکتا۔
UninstallOpenError=فائل "%1" کھولی نہیں جا سکی۔ ان انسٹال نہیں کیا جا سکتا
UninstallUnsupportedVer=ان انسٹال لاگ فائل "%1" ایسے فارمیٹ میں ہے جسے ان انسٹالر کا یہ ورژن نہیں پہچانتا۔ ان انسٹال نہیں کیا جا سکتا
UninstallUnknownEntry=ان انسٹال لاگ میں ایک نامعلوم اندراج (%1) ملا
ConfirmUninstall=کیا آپ واقعی %1 اور اس کے تمام اجزاء کو مکمل طور پر ہٹانا چاہتے ہیں؟
UninstallOnlyOnWin64=یہ انسٹالیشن صرف 64-بٹ Windows پر ان انسٹال کی جا سکتی ہے۔
OnlyAdminCanUninstall=یہ انسٹالیشن صرف ایڈمنسٹریٹر کے اختیارات رکھنے والا صارف ان انسٹال کر سکتا ہے۔
UninstallStatusLabel=براہِ کرم انتظار کریں جب تک %1 آپ کے کمپیوٹر سے ہٹایا جا رہا ہے۔
UninstalledAll=%1 آپ کے کمپیوٹر سے کامیابی کے ساتھ ہٹا دیا گیا۔
UninstalledMost=%1 کا ان انسٹال مکمل ہوا۔%n%nکچھ عناصر نہیں ہٹائے جا سکے۔ انہیں دستی طور پر ہٹایا جا سکتا ہے۔
UninstalledAndNeedsRestart=%1 کا ان انسٹال مکمل کرنے کے لیے آپ کا کمپیوٹر دوبارہ شروع کرنا ضروری ہے۔%n%nکیا آپ ابھی دوبارہ شروع کرنا چاہتے ہیں؟
UninstallDataCorrupted="%1" فائل خراب ہے۔ ان انسٹال نہیں کیا جا سکتا

; *** Uninstallation phase messages
ConfirmDeleteSharedFileTitle=مشترکہ فائل ہٹائیں؟
ConfirmDeleteSharedFile2=سسٹم کے مطابق درج ذیل مشترکہ فائل اب کسی بھی پروگرام کے زیرِ استعمال نہیں ہے۔ کیا آپ چاہتے ہیں کہ ان انسٹال یہ مشترکہ فائل ہٹا دے؟%n%nاگر کوئی پروگرام اب بھی اس فائل کو استعمال کر رہا ہو اور وہ ہٹا دی جائے تو ممکن ہے وہ پروگرام ٹھیک سے کام نہ کریں۔ اگر آپ کو یقین نہیں تو «نہیں» منتخب کریں۔ فائل کو سسٹم پر رہنے دینے سے کوئی نقصان نہیں ہوگا۔
SharedFileNameLabel=فائل کا نام:
SharedFileLocationLabel=مقام:
WizardUninstalling=ان انسٹال کی حالت
StatusUninstalling=%1 ان انسٹال ہو رہا ہے...

; *** Shutdown block reasons
ShutdownBlockReasonInstallingApp=%1 انسٹال ہو رہا ہے۔
ShutdownBlockReasonUninstallingApp=%1 ان انسٹال ہو رہا ہے۔

; The custom messages below aren't used by Setup itself, but if you make
; use of them in your scripts, you'll want to translate them.

[CustomMessages]

NameAndVersion=%1 ورژن %2
AdditionalIcons=اضافی شارٹ کٹ:
CreateDesktopIcon=&ڈیسک ٹاپ شارٹ کٹ بنائیں
CreateQuickLaunchIcon=&Quick Launch شارٹ کٹ بنائیں
ProgramOnTheWeb=ویب پر %1
UninstallProgram=%1 ان انسٹال کریں
LaunchProgram=%1 چلائیں
AssocFileExtension=%1 کو %2 فائل ایکسٹینشن کے ساتھ &منسلک کریں
AssocingFileExtension=%1 کو %2 فائل ایکسٹینشن کے ساتھ منسلک کیا جا رہا ہے...
AutoStartProgramGroupDescription=اسٹارٹ اپ:
AutoStartProgram=%1 خودکار طور پر شروع کریں
AddonHostProgramNotFound=%1 آپ کے منتخب کردہ فولڈر میں نہیں ملا۔%n%nکیا آپ پھر بھی جاری رکھنا چاہتے ہیں؟

; The messages of the installer of Lang Switcher that are its own (the sample is English.isl).
LocalCertReady=سیٹ اپ اس کمپیوٹر پر ایک سرٹیفکیٹ بنائے گا اور اس سے Lang Switcher پر دستخط کرے گا، تاکہ یہ ایڈمنسٹریٹر کے طور پر چلنے والے پروگراموں کی ونڈوز میں بھی کام کر سکے۔
LocalCertFailed=Lang Switcher انسٹال ہو گیا، لیکن یہ ایڈمنسٹریٹر کے طور پر چلنے والے پروگراموں کی ونڈوز میں کام نہیں کرے گا: Windows نے اس کمپیوٹر پر بنایا گیا سرٹیفکیٹ قبول نہیں کیا۔ تفصیلات کے لیے سیٹ اپ لاگ دیکھیں۔
RemoveSettingsQuestion=کیا Lang Switcher کی ترتیبات کی فائل بھی ہٹا دی جائے؟
RemoveSettingsKeep=دوبارہ انسٹال کرنے کے لیے ترتیبات محفوظ رکھنی ہوں تو «نہیں» منتخب کریں۔
AuthorSlogan=دل سے بنے پروگرام
