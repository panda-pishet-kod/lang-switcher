; Lang Switcher -- the messages of the installer in Persian (stage E108, group A).
;
; Inno Setup has no official Persian file, so this file is whole: [LangOptions], all the messages of
; compiler:Default.isl (version 6.5.0 sample) translated, then the stock [CustomMessages] of that
; file and the messages of the installer that are its own (the sample is English.isl). Read as the
; only MessagesFile of the "persian" entry of [Languages] in ..\LangSwitcher.iss; Default.isl may
; stand before it: what is missing here falls back to English.
; Right to left: RightToLeft=yes, no direction marks in the strings. No accelerators (&) in a file
; that has no official translation (the rule of the stage, like Tamil.isl): two equal letters in
; one window are avoided. The names of the buttons inside a message stand in the guillemets of the
; language. Messages that end without a full stop in Default.isl end without one here: Setup adds a
; full stop itself. Persian letters only (KEHEH U+06A9, FARSI YEH U+06CC), the half-space ZWNJ
; U+200C where the norm needs it. The name of the program is not translated.
; AuthorSlogan (stage E99, decision 167.1): the slogan of the signature of the author at the
; bottom of every page of the wizard -- after "Panda Koder" and a middle dot; light, without
; pathos. The name of the author and the address of the site stay as they are in every language.

[LangOptions]
LanguageName=فارسی
LanguageID=$0429
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
SetupAppTitle=نصب
SetupWindowTitle=نصب - %1
UninstallAppTitle=حذف نصب
UninstallAppFullTitle=حذف نصب %1

; *** Misc. common
InformationTitle=اطلاعات
ConfirmTitle=تأیید
ErrorTitle=خطا

; *** SetupLdr messages
SetupLdrStartupMessage=این برنامه %1 را نصب خواهد کرد. آیا می‌خواهید ادامه دهید؟
LdrCannotCreateTemp=ایجاد پرونده‌ی موقت ممکن نشد. نصب لغو شد
LdrCannotExecTemp=اجرای پرونده در پوشه‌ی موقت ممکن نشد. نصب لغو شد
HelpTextNote=

; *** Startup error messages
LastErrorMessage=%1.%n%nخطا %2: %3
SetupFileMissing=پرونده‌ی %1 در پوشه‌ی نصب وجود ندارد. لطفاً مشکل را برطرف کنید یا نسخه‌ی تازه‌ای از برنامه را تهیه کنید.
SetupFileCorrupt=پرونده‌های نصب خراب هستند. لطفاً نسخه‌ی تازه‌ای از برنامه را تهیه کنید.
SetupFileCorruptOrWrongVer=پرونده‌های نصب خراب هستند یا با این نسخه‌ی برنامه‌ی نصب سازگار نیستند. لطفاً مشکل را برطرف کنید یا نسخه‌ی تازه‌ای از برنامه را تهیه کنید.
InvalidParameter=پارامتر نامعتبری در خط فرمان داده شد:%n%n%1
SetupAlreadyRunning=برنامه‌ی نصب از قبل در حال اجراست.
WindowsVersionNotSupported=این برنامه از نسخه‌ای از Windows که روی رایانه‌ی شما اجرا می‌شود پشتیبانی نمی‌کند.
WindowsServicePackRequired=این برنامه به %1 Service Pack %2 یا جدیدتر نیاز دارد.
NotOnThisPlatform=این برنامه روی %1 اجرا نخواهد شد.
OnlyOnThisPlatform=این برنامه باید روی %1 اجرا شود.
OnlyOnTheseArchitectures=این برنامه را فقط می‌توان روی نسخه‌هایی از Windows نصب کرد که برای معماری‌های پردازنده‌ی زیر طراحی شده‌اند:%n%n%1
WinVersionTooLowError=این برنامه به %1 نسخه‌ی %2 یا جدیدتر نیاز دارد.
WinVersionTooHighError=این برنامه را نمی‌توان روی %1 نسخه‌ی %2 یا جدیدتر نصب کرد.
AdminPrivilegesRequired=برای نصب این برنامه باید به‌عنوان مدیر وارد سیستم شده باشید.
PowerUserPrivilegesRequired=برای نصب این برنامه باید به‌عنوان مدیر یا عضوی از گروه Power Users وارد سیستم شده باشید.
SetupAppRunningError=برنامه‌ی نصب تشخیص داد که %1 هم‌اکنون در حال اجراست.%n%nلطفاً همه‌ی نمونه‌های آن را ببندید، سپس برای ادامه روی «تأیید» یا برای خروج روی «لغو» کلیک کنید.
UninstallAppRunningError=برنامه‌ی حذف نصب تشخیص داد که %1 هم‌اکنون در حال اجراست.%n%nلطفاً همه‌ی نمونه‌های آن را ببندید، سپس برای ادامه روی «تأیید» یا برای خروج روی «لغو» کلیک کنید.

; *** Startup questions
PrivilegesRequiredOverrideTitle=انتخاب شیوه‌ی نصب
PrivilegesRequiredOverrideInstruction=شیوه‌ی نصب را انتخاب کنید
PrivilegesRequiredOverrideText1=%1 را می‌توان برای همه‌ی کاربران نصب کرد (به دسترسی مدیر نیاز دارد) یا فقط برای شما.
PrivilegesRequiredOverrideText2=%1 را می‌توان فقط برای شما نصب کرد یا برای همه‌ی کاربران (به دسترسی مدیر نیاز دارد).
PrivilegesRequiredOverrideAllUsers=نصب برای همه‌ی کاربران
PrivilegesRequiredOverrideAllUsersRecommended=نصب برای همه‌ی کاربران (توصیه می‌شود)
PrivilegesRequiredOverrideCurrentUser=نصب فقط برای من
PrivilegesRequiredOverrideCurrentUserRecommended=نصب فقط برای من (توصیه می‌شود)

; *** Misc. errors
ErrorCreatingDir=برنامه‌ی نصب نتوانست پوشه‌ی "%1" را ایجاد کند
ErrorTooManyFilesInDir=ایجاد پرونده در پوشه‌ی "%1" ممکن نیست، زیرا تعداد پرونده‌های آن بسیار زیاد است

; *** Setup common messages
ExitSetupTitle=خروج از نصب
ExitSetupMessage=نصب کامل نشده است. اگر اکنون خارج شوید، برنامه نصب نخواهد شد.%n%nمی‌توانید در زمانی دیگر برنامه‌ی نصب را دوباره اجرا کنید تا نصب کامل شود.%n%nاز نصب خارج می‌شوید؟
AboutSetupMenuItem=درباره‌ی نصب...
AboutSetupTitle=درباره‌ی نصب
AboutSetupMessage=%1 نسخه‌ی %2%n%3%n%nصفحه‌ی اصلی %1:%n%4
AboutSetupNote=
TranslatorNote=

; *** Buttons
ButtonBack=< قبلی
ButtonNext=بعدی >
ButtonInstall=نصب
ButtonOK=تأیید
ButtonCancel=لغو
ButtonYes=بله
ButtonYesToAll=بله برای همه
ButtonNo=خیر
ButtonNoToAll=خیر برای همه
ButtonFinish=پایان
ButtonBrowse=مرور...
ButtonWizardBrowse=مرور...
ButtonNewFolder=ایجاد پوشه‌ی جدید

; *** "Select Language" dialog messages
SelectLanguageTitle=انتخاب زبان نصب
SelectLanguageLabel=زبانی را که هنگام نصب استفاده می‌شود انتخاب کنید.

; *** Common wizard text
ClickNext=برای ادامه روی «بعدی» و برای خروج از نصب روی «لغو» کلیک کنید.
BeveledLabel=
BrowseDialogTitle=مرور برای پوشه
BrowseDialogLabel=پوشه‌ای را از فهرست زیر انتخاب کنید، سپس روی «تأیید» کلیک کنید.
NewFolderName=پوشه‌ی جدید

; *** "Welcome" wizard page
WelcomeLabel1=به جادوگر نصب [name] خوش آمدید
WelcomeLabel2=این برنامه [name/ver] را روی رایانه‌ی شما نصب خواهد کرد.%n%nتوصیه می‌شود پیش از ادامه همه‌ی برنامه‌های دیگر را ببندید.

; *** "Password" wizard page
WizardPassword=گذرواژه
PasswordLabel1=این نصب با گذرواژه محافظت شده است.
PasswordLabel3=لطفاً گذرواژه را وارد کنید، سپس برای ادامه روی «بعدی» کلیک کنید. گذرواژه‌ها به بزرگی و کوچکی حروف حساس هستند.
PasswordEditLabel=گذرواژه:
IncorrectPassword=گذرواژه‌ای که وارد کردید درست نیست. لطفاً دوباره تلاش کنید.

; *** "License Agreement" wizard page
WizardLicense=توافق‌نامه‌ی مجوز
LicenseLabel=لطفاً پیش از ادامه اطلاعات مهم زیر را بخوانید.
LicenseLabel3=لطفاً توافق‌نامه‌ی مجوز زیر را بخوانید. پیش از ادامه‌ی نصب باید شرایط این توافق‌نامه را بپذیرید.
LicenseAccepted=توافق‌نامه را می‌پذیرم
LicenseNotAccepted=توافق‌نامه را نمی‌پذیرم

; *** "Information" wizard pages
WizardInfoBefore=اطلاعات
InfoBeforeLabel=لطفاً پیش از ادامه اطلاعات مهم زیر را بخوانید.
InfoBeforeClickLabel=وقتی آماده‌ی ادامه‌ی نصب شدید، روی «بعدی» کلیک کنید.
WizardInfoAfter=اطلاعات
InfoAfterLabel=لطفاً پیش از ادامه اطلاعات مهم زیر را بخوانید.
InfoAfterClickLabel=وقتی آماده‌ی ادامه‌ی نصب شدید، روی «بعدی» کلیک کنید.

; *** "User Information" wizard page
WizardUserInfo=اطلاعات کاربر
UserInfoDesc=لطفاً اطلاعات خود را وارد کنید.
UserInfoName=نام کاربر:
UserInfoOrg=سازمان:
UserInfoSerial=شماره‌ی سریال:
UserInfoNameRequired=باید یک نام وارد کنید.

; *** "Select Destination Location" wizard page
WizardSelectDir=انتخاب محل مقصد
SelectDirDesc=[name] کجا نصب شود؟
SelectDirLabel3=برنامه‌ی نصب، [name] را در پوشه‌ی زیر نصب خواهد کرد.
SelectDirBrowseLabel=برای ادامه روی «بعدی» کلیک کنید. اگر می‌خواهید پوشه‌ی دیگری انتخاب کنید، روی «مرور» کلیک کنید.
DiskSpaceGBLabel=دست‌کم [gb] GB فضای خالی دیسک لازم است.
DiskSpaceMBLabel=دست‌کم [mb] MB فضای خالی دیسک لازم است.
CannotInstallToNetworkDrive=برنامه‌ی نصب نمی‌تواند روی درایو شبکه نصب کند.
CannotInstallToUNCPath=برنامه‌ی نصب نمی‌تواند روی مسیر UNC نصب کند.
InvalidPath=باید یک مسیر کامل همراه با حرف درایو وارد کنید؛ برای مثال:%n%nC:\APP%n%nیا یک مسیر UNC به این شکل:%n%n\\server\share
InvalidDrive=درایو یا اشتراک UNC که انتخاب کردید وجود ندارد یا در دسترس نیست. لطفاً مورد دیگری انتخاب کنید.
DiskSpaceWarningTitle=فضای دیسک کافی نیست
DiskSpaceWarning=نصب دست‌کم به %1 KB فضای خالی نیاز دارد، اما درایو انتخاب‌شده فقط %2 KB فضای در دسترس دارد.%n%nمی‌خواهید به هر حال ادامه دهید؟
DirNameTooLong=نام یا مسیر پوشه بسیار طولانی است.
InvalidDirName=نام پوشه معتبر نیست.
BadDirName32=نام پوشه‌ها نمی‌تواند شامل هیچ‌یک از نویسه‌های زیر باشد:%n%n%1
DirExistsTitle=پوشه وجود دارد
DirExists=پوشه‌ی:%n%n%1%n%nاز قبل وجود دارد. می‌خواهید به هر حال در آن پوشه نصب کنید؟
DirDoesntExistTitle=پوشه وجود ندارد
DirDoesntExist=پوشه‌ی:%n%n%1%n%nوجود ندارد. می‌خواهید این پوشه ایجاد شود؟

; *** "Select Components" wizard page
WizardSelectComponents=انتخاب مؤلفه‌ها
SelectComponentsDesc=کدام مؤلفه‌ها نصب شوند؟
SelectComponentsLabel2=مؤلفه‌هایی را که می‌خواهید نصب شوند انتخاب کنید؛ علامت مؤلفه‌هایی را که نمی‌خواهید بردارید. وقتی آماده بودید روی «بعدی» کلیک کنید.
FullInstallation=نصب کامل
; if possible don't translate 'Compact' as 'Minimal' (I mean 'Minimal' in your language)
CompactInstallation=نصب فشرده
CustomInstallation=نصب سفارشی
NoUninstallWarningTitle=مؤلفه‌ها وجود دارند
NoUninstallWarning=برنامه‌ی نصب تشخیص داد که مؤلفه‌های زیر از قبل روی رایانه‌ی شما نصب شده‌اند:%n%n%1%n%nبرداشتن علامت این مؤلفه‌ها آن‌ها را حذف نصب نمی‌کند.%n%nمی‌خواهید به هر حال ادامه دهید؟
ComponentSize1=%1 KB
ComponentSize2=%1 MB
ComponentsDiskSpaceGBLabel=انتخاب فعلی دست‌کم [gb] GB فضای دیسک لازم دارد.
ComponentsDiskSpaceMBLabel=انتخاب فعلی دست‌کم [mb] MB فضای دیسک لازم دارد.

; *** "Select Additional Tasks" wizard page
WizardSelectTasks=انتخاب کارهای اضافی
SelectTasksDesc=کدام کارهای اضافی انجام شوند؟
SelectTasksLabel2=کارهای اضافی را که می‌خواهید برنامه‌ی نصب هنگام نصب [name] انجام دهد انتخاب کنید، سپس روی «بعدی» کلیک کنید.

; *** "Select Start Menu Folder" wizard page
WizardSelectProgramGroup=انتخاب پوشه‌ی منوی Start
SelectStartMenuFolderDesc=برنامه‌ی نصب میان‌برهای برنامه را کجا قرار دهد؟
SelectStartMenuFolderLabel3=برنامه‌ی نصب میان‌برهای برنامه را در پوشه‌ی زیر از منوی Start ایجاد خواهد کرد.
SelectStartMenuFolderBrowseLabel=برای ادامه روی «بعدی» کلیک کنید. اگر می‌خواهید پوشه‌ی دیگری انتخاب کنید، روی «مرور» کلیک کنید.
MustEnterGroupName=باید نام یک پوشه را وارد کنید.
GroupNameTooLong=نام یا مسیر پوشه بسیار طولانی است.
InvalidGroupName=نام پوشه معتبر نیست.
BadGroupName=نام پوشه نمی‌تواند شامل هیچ‌یک از نویسه‌های زیر باشد:%n%n%1
NoProgramGroupCheck2=پوشه‌ی منوی Start ایجاد نشود

; *** "Ready to Install" wizard page
WizardReady=آماده‌ی نصب
ReadyLabel1=برنامه‌ی نصب اکنون آماده است [name] را روی رایانه‌ی شما نصب کند.
ReadyLabel2a=برای ادامه‌ی نصب روی «نصب» کلیک کنید، یا اگر می‌خواهید تنظیمی را مرور یا تغییر دهید روی «قبلی» کلیک کنید.
ReadyLabel2b=برای ادامه‌ی نصب روی «نصب» کلیک کنید.
ReadyMemoUserInfo=اطلاعات کاربر:
ReadyMemoDir=محل مقصد:
ReadyMemoType=نوع نصب:
ReadyMemoComponents=مؤلفه‌های انتخاب‌شده:
ReadyMemoGroup=پوشه‌ی منوی Start:
ReadyMemoTasks=کارهای اضافی:

; *** TDownloadWizardPage wizard page and DownloadTemporaryFile
DownloadingLabel2=در حال بارگیری پرونده‌ها...
ButtonStopDownload=توقف بارگیری
StopDownload=آیا مطمئن هستید که می‌خواهید بارگیری را متوقف کنید؟
ErrorDownloadAborted=بارگیری لغو شد
ErrorDownloadFailed=بارگیری ناموفق بود: %1 %2
ErrorDownloadSizeFailed=دریافت اندازه ناموفق بود: %1 %2
ErrorProgress=پیشرفت نامعتبر: %1 از %2
ErrorFileSize=اندازه‌ی پرونده نامعتبر: انتظار %1، یافت‌شده %2

; *** TExtractionWizardPage wizard page and ExtractArchive
ExtractingLabel=در حال استخراج پرونده‌ها...
ButtonStopExtraction=توقف استخراج
StopExtraction=آیا مطمئن هستید که می‌خواهید استخراج را متوقف کنید؟
ErrorExtractionAborted=استخراج لغو شد
ErrorExtractionFailed=استخراج ناموفق بود: %1

; *** Archive extraction failure details
ArchiveIncorrectPassword=گذرواژه نادرست است
ArchiveIsCorrupted=بایگانی خراب است
ArchiveUnsupportedFormat=قالب بایگانی پشتیبانی نمی‌شود

; *** "Preparing to Install" wizard page
WizardPreparing=آماده‌سازی برای نصب
PreparingDesc=برنامه‌ی نصب در حال آماده شدن برای نصب [name] روی رایانه‌ی شما است.
PreviousInstallNotCompleted=نصب یا حذف برنامه‌ای قبلی کامل نشده است. برای تکمیل آن نصب باید رایانه‌ی خود را دوباره راه‌اندازی کنید.%n%nپس از راه‌اندازی مجدد رایانه، برنامه‌ی نصب را دوباره اجرا کنید تا نصب [name] کامل شود.
CannotContinue=نصب نمی‌تواند ادامه یابد. لطفاً برای خروج روی «لغو» کلیک کنید.
ApplicationsFound=برنامه‌های زیر از پرونده‌هایی استفاده می‌کنند که باید توسط برنامه‌ی نصب به‌روزرسانی شوند. توصیه می‌شود به برنامه‌ی نصب اجازه دهید این برنامه‌ها را خودکار ببندد.
ApplicationsFound2=برنامه‌های زیر از پرونده‌هایی استفاده می‌کنند که باید توسط برنامه‌ی نصب به‌روزرسانی شوند. توصیه می‌شود به برنامه‌ی نصب اجازه دهید این برنامه‌ها را خودکار ببندد. پس از پایان نصب، برنامه‌ی نصب تلاش خواهد کرد این برنامه‌ها را دوباره اجرا کند.
CloseApplications=برنامه‌ها خودکار بسته شوند
DontCloseApplications=برنامه‌ها بسته نشوند
ErrorCloseApplications=برنامه‌ی نصب نتوانست همه‌ی برنامه‌ها را خودکار ببندد. توصیه می‌شود پیش از ادامه، همه‌ی برنامه‌هایی را که از پرونده‌های نیازمند به‌روزرسانی استفاده می‌کنند ببندید.
PrepareToInstallNeedsRestart=برنامه‌ی نصب باید رایانه‌ی شما را دوباره راه‌اندازی کند. پس از راه‌اندازی مجدد، برنامه‌ی نصب را دوباره اجرا کنید تا نصب [name] کامل شود.%n%nمی‌خواهید اکنون دوباره راه‌اندازی کنید؟

; *** "Installing" wizard page
WizardInstalling=در حال نصب
InstallingLabel=لطفاً صبر کنید تا برنامه‌ی نصب، [name] را روی رایانه‌ی شما نصب کند.

; *** "Setup Completed" wizard page
FinishedHeadingLabel=تکمیل جادوگر نصب [name]
FinishedLabelNoIcons=برنامه‌ی نصب، نصب [name] را روی رایانه‌ی شما به پایان رساند.
FinishedLabel=برنامه‌ی نصب، نصب [name] را روی رایانه‌ی شما به پایان رساند. برنامه را می‌توان با انتخاب میان‌برهای نصب‌شده اجرا کرد.
ClickFinish=برای خروج از نصب روی «پایان» کلیک کنید.
FinishedRestartLabel=برای تکمیل نصب [name]، برنامه‌ی نصب باید رایانه‌ی شما را دوباره راه‌اندازی کند. می‌خواهید اکنون دوباره راه‌اندازی کنید؟
FinishedRestartMessage=برای تکمیل نصب [name]، برنامه‌ی نصب باید رایانه‌ی شما را دوباره راه‌اندازی کند.%n%nمی‌خواهید اکنون دوباره راه‌اندازی کنید؟
ShowReadmeCheck=بله، می‌خواهم پرونده‌ی README را ببینم
YesRadio=بله، رایانه اکنون دوباره راه‌اندازی شود
NoRadio=خیر، بعداً خودم رایانه را دوباره راه‌اندازی می‌کنم
; used for example as 'Run MyProg.exe'
RunEntryExec=اجرای %1
; used for example as 'View Readme.txt'
RunEntryShellExec=مشاهده‌ی %1

; *** "Setup Needs the Next Disk" stuff
ChangeDiskTitle=برنامه‌ی نصب به دیسک بعدی نیاز دارد
SelectDiskLabel2=لطفاً دیسک %1 را وارد کنید و روی «تأیید» کلیک کنید.%n%nاگر پرونده‌های این دیسک در پوشه‌ای غیر از پوشه‌ی نمایش‌داده‌شده در زیر قرار دارند، مسیر درست را وارد کنید یا روی «مرور» کلیک کنید.
PathLabel=مسیر:
FileNotInDir2=پرونده‌ی "%1" در "%2" پیدا نشد. لطفاً دیسک درست را وارد کنید یا پوشه‌ی دیگری انتخاب کنید.
SelectDirectoryLabel=لطفاً محل دیسک بعدی را مشخص کنید.

; *** Installation phase messages
SetupAborted=نصب کامل نشد.%n%nلطفاً مشکل را برطرف کنید و برنامه‌ی نصب را دوباره اجرا کنید.
AbortRetryIgnoreSelectAction=انتخاب عمل
AbortRetryIgnoreRetry=تلاش دوباره
AbortRetryIgnoreIgnore=خطا نادیده گرفته شود و ادامه یابد
AbortRetryIgnoreCancel=لغو نصب
RetryCancelSelectAction=انتخاب عمل
RetryCancelRetry=تلاش دوباره
RetryCancelCancel=لغو

; *** Installation status messages
StatusClosingApplications=در حال بستن برنامه‌ها...
StatusCreateDirs=در حال ایجاد پوشه‌ها...
StatusExtractFiles=در حال استخراج پرونده‌ها...
StatusDownloadFiles=در حال بارگیری پرونده‌ها...
StatusCreateIcons=در حال ایجاد میان‌برها...
StatusCreateIniEntries=در حال ایجاد مدخل‌های INI...
StatusCreateRegistryEntries=در حال ایجاد مدخل‌های رجیستری...
StatusRegisterFiles=در حال ثبت پرونده‌ها...
StatusSavingUninstall=در حال ذخیره‌ی اطلاعات حذف نصب...
StatusRunProgram=در حال پایان دادن به نصب...
StatusRestartingApplications=در حال راه‌اندازی مجدد برنامه‌ها...
StatusRollback=در حال بازگرداندن تغییرات...

; *** Misc. errors
ErrorInternal2=خطای داخلی: %1
ErrorFunctionFailedNoCode=%1 ناموفق بود
ErrorFunctionFailed=%1 ناموفق بود؛ کد %2
ErrorFunctionFailedWithMessage=%1 ناموفق بود؛ کد %2.%n%3
ErrorExecutingProgram=اجرای پرونده ممکن نشد:%n%1

; *** Registry errors
ErrorRegOpenKey=خطا در باز کردن کلید رجیستری:%n%1\%2
ErrorRegCreateKey=خطا در ایجاد کلید رجیستری:%n%1\%2
ErrorRegWriteKey=خطا در نوشتن در کلید رجیستری:%n%1\%2

; *** INI errors
ErrorIniEntry=خطا در ایجاد مدخل INI در پرونده‌ی "%1".

; *** File copying errors
FileAbortRetryIgnoreSkipNotRecommended=این پرونده رد شود (توصیه نمی‌شود)
FileAbortRetryIgnoreIgnoreNotRecommended=خطا نادیده گرفته شود و ادامه یابد (توصیه نمی‌شود)
SourceIsCorrupted=پرونده‌ی منبع خراب است
SourceDoesntExist=پرونده‌ی منبع "%1" وجود ندارد
SourceVerificationFailed=تأیید پرونده‌ی منبع ناموفق بود: %1
VerificationSignatureDoesntExist=پرونده‌ی امضای "%1" وجود ندارد
VerificationSignatureInvalid=پرونده‌ی امضای "%1" نامعتبر است
VerificationKeyNotFound=پرونده‌ی امضای "%1" از کلیدی ناشناخته استفاده می‌کند
VerificationFileNameIncorrect=نام پرونده نادرست است
VerificationFileTagIncorrect=برچسب پرونده نادرست است
VerificationFileSizeIncorrect=اندازه‌ی پرونده نادرست است
VerificationFileHashIncorrect=هش پرونده نادرست است
ExistingFileReadOnly2=پرونده‌ی موجود را نمی‌توان جایگزین کرد، زیرا به‌صورت فقط‌خواندنی علامت‌گذاری شده است.
ExistingFileReadOnlyRetry=ویژگی فقط‌خواندنی حذف شود و دوباره تلاش شود
ExistingFileReadOnlyKeepExisting=پرونده‌ی موجود نگه داشته شود
ErrorReadingExistingDest=هنگام تلاش برای خواندن پرونده‌ی موجود خطایی رخ داد:
FileExistsSelectAction=انتخاب عمل
FileExists2=پرونده از قبل وجود دارد.
FileExistsOverwriteExisting=پرونده‌ی موجود بازنویسی شود
FileExistsKeepExisting=پرونده‌ی موجود نگه داشته شود
FileExistsOverwriteOrKeepAll=همین کار برای تعارض‌های بعدی انجام شود
ExistingFileNewerSelectAction=انتخاب عمل
ExistingFileNewer2=پرونده‌ی موجود از پرونده‌ای که برنامه‌ی نصب می‌خواهد نصب کند جدیدتر است.
ExistingFileNewerOverwriteExisting=پرونده‌ی موجود بازنویسی شود
ExistingFileNewerKeepExisting=پرونده‌ی موجود نگه داشته شود (توصیه می‌شود)
ExistingFileNewerOverwriteOrKeepAll=همین کار برای تعارض‌های بعدی انجام شود
ErrorChangingAttr=هنگام تلاش برای تغییر ویژگی‌های پرونده‌ی موجود خطایی رخ داد:
ErrorCreatingTemp=هنگام تلاش برای ایجاد پرونده در پوشه‌ی مقصد خطایی رخ داد:
ErrorReadingSource=هنگام تلاش برای خواندن پرونده‌ی منبع خطایی رخ داد:
ErrorCopying=هنگام تلاش برای کپی کردن پرونده خطایی رخ داد:
ErrorDownloading=هنگام تلاش برای بارگیری پرونده خطایی رخ داد:
ErrorExtracting=هنگام تلاش برای استخراج بایگانی خطایی رخ داد:
ErrorReplacingExistingFile=هنگام تلاش برای جایگزین کردن پرونده‌ی موجود خطایی رخ داد:
ErrorRestartReplace=RestartReplace ناموفق بود:
ErrorRenamingTemp=هنگام تلاش برای تغییر نام پرونده در پوشه‌ی مقصد خطایی رخ داد:
ErrorRegisterServer=ثبت DLL/OCX ممکن نشد: %1
ErrorRegSvr32Failed=RegSvr32 با کد خروج %1 ناموفق بود
ErrorRegisterTypeLib=ثبت کتابخانه‌ی نوع ممکن نشد: %1

; *** Uninstall display name markings
; used for example as 'My Program (32-bit)'
UninstallDisplayNameMark=%1 (%2)
; used for example as 'My Program (32-bit, All users)'
UninstallDisplayNameMarks=%1 (%2، %3)
UninstallDisplayNameMark32Bit=32 بیتی
UninstallDisplayNameMark64Bit=64 بیتی
UninstallDisplayNameMarkAllUsers=همه‌ی کاربران
UninstallDisplayNameMarkCurrentUser=کاربر فعلی

; *** Post-installation errors
ErrorOpeningReadme=هنگام تلاش برای باز کردن پرونده‌ی README خطایی رخ داد.
ErrorRestartingComputer=برنامه‌ی نصب نتوانست رایانه را دوباره راه‌اندازی کند. لطفاً این کار را دستی انجام دهید.

; *** Uninstaller messages
UninstallNotFound=پرونده‌ی "%1" وجود ندارد. حذف نصب ممکن نیست.
UninstallOpenError=پرونده‌ی "%1" باز نشد. حذف نصب ممکن نیست
UninstallUnsupportedVer=پرونده‌ی گزارش حذف نصب "%1" در قالبی است که این نسخه‌ی برنامه‌ی حذف نصب آن را نمی‌شناسد. حذف نصب ممکن نیست
UninstallUnknownEntry=در گزارش حذف نصب به مدخلی ناشناخته (%1) برخورد شد
ConfirmUninstall=آیا مطمئن هستید که می‌خواهید %1 و همه‌ی مؤلفه‌های آن را کاملاً حذف کنید؟
UninstallOnlyOnWin64=این نصب را فقط روی Windows 64 بیتی می‌توان حذف کرد.
OnlyAdminCanUninstall=این نصب را فقط کاربری با دسترسی مدیر می‌تواند حذف کند.
UninstallStatusLabel=لطفاً صبر کنید تا %1 از رایانه‌ی شما حذف شود.
UninstalledAll=%1 با موفقیت از رایانه‌ی شما حذف شد.
UninstalledMost=حذف نصب %1 کامل شد.%n%nبرخی عناصر حذف نشدند. آن‌ها را می‌توان دستی حذف کرد.
UninstalledAndNeedsRestart=برای تکمیل حذف نصب %1، باید رایانه‌ی شما دوباره راه‌اندازی شود.%n%nمی‌خواهید اکنون دوباره راه‌اندازی کنید؟
UninstallDataCorrupted=پرونده‌ی "%1" خراب است. حذف نصب ممکن نیست

; *** Uninstallation phase messages
ConfirmDeleteSharedFileTitle=پرونده‌ی مشترک حذف شود؟
ConfirmDeleteSharedFile2=سیستم نشان می‌دهد که پرونده‌ی مشترک زیر دیگر توسط هیچ برنامه‌ای استفاده نمی‌شود. می‌خواهید برنامه‌ی حذف نصب این پرونده‌ی مشترک را حذف کند؟%n%nاگر برنامه‌ای هنوز از این پرونده استفاده کند و حذف شود، ممکن است آن برنامه درست کار نکند. اگر مطمئن نیستید «خیر» را انتخاب کنید. ماندن پرونده روی سیستم شما هیچ آسیبی نمی‌زند.
SharedFileNameLabel=نام پرونده:
SharedFileLocationLabel=محل:
WizardUninstalling=وضعیت حذف نصب
StatusUninstalling=در حال حذف نصب %1...

; *** Shutdown block reasons
ShutdownBlockReasonInstallingApp=در حال نصب %1.
ShutdownBlockReasonUninstallingApp=در حال حذف نصب %1.

; The custom messages below aren't used by Setup itself, but if you make
; use of them in your scripts, you'll want to translate them.

[CustomMessages]

NameAndVersion=%1 نسخه‌ی %2
AdditionalIcons=میان‌برهای اضافی:
CreateDesktopIcon=ایجاد میان‌بر روی میزکار
CreateQuickLaunchIcon=ایجاد میان‌بر Quick Launch
ProgramOnTheWeb=%1 در وب
UninstallProgram=حذف نصب %1
LaunchProgram=اجرای %1
AssocFileExtension=مرتبط کردن %1 با پسوند پرونده‌ی %2
AssocingFileExtension=در حال مرتبط کردن %1 با پسوند پرونده‌ی %2...
AutoStartProgramGroupDescription=شروع خودکار:
AutoStartProgram=اجرای خودکار %1
AddonHostProgramNotFound=%1 در پوشه‌ای که انتخاب کردید پیدا نشد.%n%nمی‌خواهید به هر حال ادامه دهید؟

; The messages of the installer of Lang Switcher that are its own (the sample is English.isl).
LocalCertReady=برنامه‌ی نصب روی این رایانه یک گواهی ایجاد می‌کند و Lang Switcher را با آن امضا می‌کند، تا بتواند در پنجره‌های برنامه‌هایی که به‌عنوان مدیر اجرا می‌شوند هم کار کند.
LocalCertFailed=Lang Switcher نصب شد، اما در پنجره‌های برنامه‌هایی که به‌عنوان مدیر اجرا می‌شوند کار نخواهد کرد: Windows گواهی ایجادشده روی این رایانه را نپذیرفت. برای جزئیات، گزارش نصب را ببینید.
RemoveSettingsQuestion=پرونده‌ی تنظیمات Lang Switcher هم حذف شود؟
RemoveSettingsKeep=برای نگه داشتن تنظیمات تا نصب دوباره‌ی بعدی، «خیر» را انتخاب کنید.
AuthorSlogan=برنامه‌هایی ساخته‌شده با دل و جان
