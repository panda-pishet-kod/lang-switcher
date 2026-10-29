; Lang Switcher -- the messages of the installer in Uzbek, Latin script (stage E108, group A).
;
; Inno Setup has no official Uzbek translation (inno_official = false in res\lang\languages.toml),
; so this file carries the whole of it: [LangOptions], every message of the compiler:Default.isl
; (6.5.0+), and the messages of the installer that are Lang Switcher's own. It is read after
; compiler:Default.isl as the second MessagesFile of the "uzbek" entry of [Languages] in
; ..\LangSwitcher.iss, and so replaces the English text name for name.
; English.isl is the sample the own messages translate; the glossary of the language is
; l10n\glossary\uz.tsv of the control contour. The name of the program is not translated.
; The letters o' and g' carry U+02BB, the tutuq belgisi is U+02BC (never the straight apostrophe).
; The names of the buttons inside a message stand in the quotation marks of the language (RULES 16).
; The accelerators (&) are not used, as in Tamil.isl: no two equal letters in one window.
; As in Default.isl: a message that has no full stop there has none here (Setup adds its own).

[LangOptions]
LanguageName=Oʻzbekcha
LanguageID=$0443
LanguageCodePage=0
;DialogFontName=
;DialogFontSize=9
;DialogFontBaseScaleWidth=7
;DialogFontBaseScaleHeight=15
;WelcomeFontName=Segoe UI
;WelcomeFontSize=14

[Messages]

; *** Application titles
SetupAppTitle=Oʻrnatish
SetupWindowTitle=Oʻrnatish - %1
UninstallAppTitle=Oʻchirish
UninstallAppFullTitle=%1 dasturini oʻchirish

; *** Misc. common
InformationTitle=Maʼlumot
ConfirmTitle=Tasdiqlash
ErrorTitle=Xato

; *** SetupLdr messages
SetupLdrStartupMessage=Bu %1 dasturini oʻrnatadi. Davom etasizmi?
LdrCannotCreateTemp=Vaqtinchalik fayl yaratib boʻlmadi. Oʻrnatish bekor qilindi
LdrCannotExecTemp=Vaqtinchalik jilddagi faylni ishga tushirib boʻlmadi. Oʻrnatish bekor qilindi
HelpTextNote=

; *** Startup error messages
LastErrorMessage=%1.%n%nXato %2: %3
SetupFileMissing=Oʻrnatish jildida %1 fayli yoʻq. Muammoni bartaraf qiling yoki dasturning yangi nusxasini oling.
SetupFileCorrupt=Oʻrnatish fayllari buzilgan. Dasturning yangi nusxasini oling.
SetupFileCorruptOrWrongVer=Oʻrnatish fayllari buzilgan yoki Oʻrnatuvchining bu versiyasiga mos kelmaydi. Muammoni bartaraf qiling yoki dasturning yangi nusxasini oling.
InvalidParameter=Buyruq satriga notoʻgʻri parametr berildi:%n%n%1
SetupAlreadyRunning=Oʻrnatuvchi allaqachon ishlamoqda.
WindowsVersionNotSupported=Bu dastur kompyuteringizdagi Windows versiyasini qoʻllab-quvvatlamaydi.
WindowsServicePackRequired=Bu dastur uchun %1 Service Pack %2 yoki undan yangisi kerak.
NotOnThisPlatform=Bu dastur %1 muhitida ishlamaydi.
OnlyOnThisPlatform=Bu dasturni %1 muhitida ishga tushirish kerak.
OnlyOnTheseArchitectures=Bu dasturni faqat quyidagi protsessor arxitekturalari uchun moʻljallangan Windows versiyalariga oʻrnatish mumkin:%n%n%1
WinVersionTooLowError=Bu dastur uchun %1 versiyasi %2 yoki undan yangisi kerak.
WinVersionTooHighError=Bu dasturni %1 versiyasi %2 yoki undan yangisiga oʻrnatib boʻlmaydi.
AdminPrivilegesRequired=Bu dasturni oʻrnatishda administrator sifatida tizimga kirgan boʻlishingiz kerak.
PowerUserPrivilegesRequired=Bu dasturni oʻrnatishda administrator yoki Power Users guruhi aʼzosi sifatida tizimga kirgan boʻlishingiz kerak.
SetupAppRunningError=Oʻrnatuvchi %1 hozir ishlayotganini aniqladi.%n%nUning barcha nusxalarini yoping, soʻng davom etish uchun «OK» tugmasini yoki chiqish uchun «Bekor qilish» tugmasini bosing.
UninstallAppRunningError=Oʻchirish dasturi %1 hozir ishlayotganini aniqladi.%n%nUning barcha nusxalarini yoping, soʻng davom etish uchun «OK» tugmasini yoki chiqish uchun «Bekor qilish» tugmasini bosing.

; *** Startup questions
PrivilegesRequiredOverrideTitle=Oʻrnatish rejimini tanlang
PrivilegesRequiredOverrideInstruction=Oʻrnatish rejimini tanlang
PrivilegesRequiredOverrideText1=%1 dasturini barcha foydalanuvchilar uchun (administrator huquqlari kerak) yoki faqat siz uchun oʻrnatish mumkin.
PrivilegesRequiredOverrideText2=%1 dasturini faqat siz uchun yoki barcha foydalanuvchilar uchun (administrator huquqlari kerak) oʻrnatish mumkin.
PrivilegesRequiredOverrideAllUsers=Barcha foydalanuvchilar uchun oʻrnatish
PrivilegesRequiredOverrideAllUsersRecommended=Barcha foydalanuvchilar uchun oʻrnatish (tavsiya etiladi)
PrivilegesRequiredOverrideCurrentUser=Faqat men uchun oʻrnatish
PrivilegesRequiredOverrideCurrentUserRecommended=Faqat men uchun oʻrnatish (tavsiya etiladi)

; *** Misc. errors
ErrorCreatingDir=Oʻrnatuvchi "%1" jildini yarata olmadi
ErrorTooManyFilesInDir="%1" jildida fayllar juda koʻp boʻlgani uchun unda fayl yaratib boʻlmadi

; *** Setup common messages
ExitSetupTitle=Oʻrnatishdan chiqish
ExitSetupMessage=Oʻrnatish tugallanmadi. Hozir chiqsangiz, dastur oʻrnatilmaydi.%n%nOʻrnatishni tugatish uchun Oʻrnatuvchini keyinroq yana ishga tushirishingiz mumkin.%n%nOʻrnatishdan chiqilsinmi?
AboutSetupMenuItem=Oʻrnatuvchi haqida...
AboutSetupTitle=Oʻrnatuvchi haqida
AboutSetupMessage=%1 versiya %2%n%3%n%n%1 bosh sahifasi:%n%4
AboutSetupNote=
TranslatorNote=

; *** Buttons
ButtonBack=< Orqaga
ButtonNext=Keyingisi >
ButtonInstall=Oʻrnatish
ButtonOK=OK
ButtonCancel=Bekor qilish
ButtonYes=Ha
ButtonYesToAll=Hammasiga ha
ButtonNo=Yoʻq
ButtonNoToAll=Hammasiga yoʻq
ButtonFinish=Yakunlash
ButtonBrowse=Koʻrib chiqish...
ButtonWizardBrowse=Koʻrib chiqish...
ButtonNewFolder=Yangi jild yaratish

; *** "Select Language" dialog messages
SelectLanguageTitle=Oʻrnatish tilini tanlang
SelectLanguageLabel=Oʻrnatish vaqtida foydalaniladigan tilni tanlang.

; *** Common wizard text
ClickNext=Davom etish uchun «Keyingisi» tugmasini, chiqish uchun «Bekor qilish» tugmasini bosing.
BeveledLabel=
BrowseDialogTitle=Jildni tanlash
BrowseDialogLabel=Quyidagi roʻyxatdan jildni tanlang, soʻng «OK» tugmasini bosing.
NewFolderName=Yangi jild

; *** "Welcome" wizard page
WelcomeLabel1=[name] oʻrnatish ustasiga xush kelibsiz
WelcomeLabel2=Bu [name/ver] dasturini kompyuteringizga oʻrnatadi.%n%nDavom etishdan oldin barcha boshqa dasturlarni yopish tavsiya etiladi.

; *** "Password" wizard page
WizardPassword=Parol
PasswordLabel1=Bu oʻrnatish parol bilan himoyalangan.
PasswordLabel3=Parolni kiriting, soʻng davom etish uchun «Keyingisi» tugmasini bosing. Parolda bosh va kichik harflar farqlanadi.
PasswordEditLabel=Parol:
IncorrectPassword=Kiritilgan parol notoʻgʻri. Qaytadan urinib koʻring.

; *** "License Agreement" wizard page
WizardLicense=Litsenziya shartnomasi
LicenseLabel=Davom etishdan oldin quyidagi muhim maʼlumotni oʻqing.
LicenseLabel3=Quyidagi litsenziya shartnomasini oʻqing. Oʻrnatishni davom ettirishdan oldin shartnoma shartlarini qabul qilishingiz kerak.
LicenseAccepted=Shartnomani qabul qilaman
LicenseNotAccepted=Shartnomani qabul qilmayman

; *** "Information" wizard pages
WizardInfoBefore=Maʼlumot
InfoBeforeLabel=Davom etishdan oldin quyidagi muhim maʼlumotni oʻqing.
InfoBeforeClickLabel=Oʻrnatishni davom ettirishga tayyor boʻlganingizda «Keyingisi» tugmasini bosing.
WizardInfoAfter=Maʼlumot
InfoAfterLabel=Davom etishdan oldin quyidagi muhim maʼlumotni oʻqing.
InfoAfterClickLabel=Oʻrnatishni davom ettirishga tayyor boʻlganingizda «Keyingisi» tugmasini bosing.

; *** "User Information" wizard page
WizardUserInfo=Foydalanuvchi maʼlumotlari
UserInfoDesc=Maʼlumotlaringizni kiriting.
UserInfoName=Foydalanuvchi nomi:
UserInfoOrg=Tashkilot:
UserInfoSerial=Seriya raqami:
UserInfoNameRequired=Ism kiritishingiz shart.

; *** "Select Destination Location" wizard page
WizardSelectDir=Oʻrnatish jildini tanlang
SelectDirDesc=[name] qayerga oʻrnatilsin?
SelectDirLabel3=Oʻrnatuvchi [name] dasturini quyidagi jildga oʻrnatadi.
SelectDirBrowseLabel=Davom etish uchun «Keyingisi» tugmasini bosing. Boshqa jildni tanlamoqchi boʻlsangiz, «Koʻrib chiqish» tugmasini bosing.
DiskSpaceGBLabel=Kamida [gb] GB boʻsh disk joyi kerak.
DiskSpaceMBLabel=Kamida [mb] MB boʻsh disk joyi kerak.
CannotInstallToNetworkDrive=Oʻrnatuvchi tarmoq diskiga oʻrnata olmaydi.
CannotInstallToUNCPath=Oʻrnatuvchi UNC yoʻliga oʻrnata olmaydi.
InvalidPath=Disk harfi bilan toʻliq yoʻlni kiriting, masalan:%n%nC:\APP%n%nyoki UNC yoʻlini quyidagi koʻrinishda:%n%n\\server\share
InvalidDrive=Tanlangan disk yoki UNC ulushi mavjud emas yoki unga kirib boʻlmaydi. Boshqasini tanlang.
DiskSpaceWarningTitle=Disk joyi yetarli emas
DiskSpaceWarning=Oʻrnatish uchun kamida %1 KB boʻsh joy kerak, tanlangan diskda esa faqat %2 KB bor.%n%nBaribir davom etasizmi?
DirNameTooLong=Jild nomi yoki yoʻli juda uzun.
InvalidDirName=Jild nomi yaroqsiz.
BadDirName32=Jild nomida quyidagi belgilar boʻlishi mumkin emas:%n%n%1
DirExistsTitle=Jild mavjud
DirExists=Quyidagi jild:%n%n%1%n%nallaqachon mavjud. Baribir shu jildga oʻrnatilsinmi?
DirDoesntExistTitle=Jild mavjud emas
DirDoesntExist=Quyidagi jild:%n%n%1%n%nmavjud emas. Jild yaratilsinmi?

; *** "Select Components" wizard page
WizardSelectComponents=Komponentlarni tanlang
SelectComponentsDesc=Qaysi komponentlar oʻrnatilsin?
SelectComponentsLabel2=Oʻrnatmoqchi boʻlgan komponentlarni belgilang; oʻrnatmoqchi boʻlmaganlarining belgisini olib tashlang. Davom etishga tayyor boʻlganingizda «Keyingisi» tugmasini bosing.
FullInstallation=Toʻliq oʻrnatish
; if possible don't translate 'Compact' as 'Minimal' (I mean 'Minimal' in your language)
CompactInstallation=Ixcham oʻrnatish
CustomInstallation=Moslashtirilgan oʻrnatish
NoUninstallWarningTitle=Komponentlar mavjud
NoUninstallWarning=Oʻrnatuvchi quyidagi komponentlar kompyuteringizda allaqachon oʻrnatilganini aniqladi:%n%n%1%n%nBu komponentlarning belgisini olib tashlash ularni oʻchirmaydi.%n%nBaribir davom etasizmi?
ComponentSize1=%1 KB
ComponentSize2=%1 MB
ComponentsDiskSpaceGBLabel=Tanlangan komponentlar uchun kamida [gb] GB disk joyi kerak.
ComponentsDiskSpaceMBLabel=Tanlangan komponentlar uchun kamida [mb] MB disk joyi kerak.

; *** "Select Additional Tasks" wizard page
WizardSelectTasks=Qoʻshimcha vazifalarni tanlang
SelectTasksDesc=Qaysi qoʻshimcha vazifalar bajarilsin?
SelectTasksLabel2=[name] oʻrnatilayotganda Oʻrnatuvchi bajarishi kerak boʻlgan qoʻshimcha vazifalarni tanlang, soʻng «Keyingisi» tugmasini bosing.

; *** "Select Start Menu Folder" wizard page
WizardSelectProgramGroup=«Boshlash» menyusi jildini tanlang
SelectStartMenuFolderDesc=Oʻrnatuvchi dastur yorliqlarini qayerga joylashtirsin?
SelectStartMenuFolderLabel3=Oʻrnatuvchi dastur yorliqlarini «Boshlash» menyusining quyidagi jildida yaratadi.
SelectStartMenuFolderBrowseLabel=Davom etish uchun «Keyingisi» tugmasini bosing. Boshqa jildni tanlamoqchi boʻlsangiz, «Koʻrib chiqish» tugmasini bosing.
MustEnterGroupName=Jild nomini kiritishingiz shart.
GroupNameTooLong=Jild nomi yoki yoʻli juda uzun.
InvalidGroupName=Jild nomi yaroqsiz.
BadGroupName=Jild nomida quyidagi belgilar boʻlishi mumkin emas:%n%n%1
NoProgramGroupCheck2=«Boshlash» menyusida jild yaratilmasin

; *** "Ready to Install" wizard page
WizardReady=Oʻrnatishga tayyor
ReadyLabel1=Oʻrnatuvchi [name] dasturini kompyuteringizga oʻrnatishni boshlashga tayyor.
ReadyLabel2a=Oʻrnatishni davom ettirish uchun «Oʻrnatish» tugmasini bosing yoki sozlamalarni koʻrib chiqish yoxud oʻzgartirish uchun «Orqaga» tugmasini bosing.
ReadyLabel2b=Oʻrnatishni davom ettirish uchun «Oʻrnatish» tugmasini bosing.
ReadyMemoUserInfo=Foydalanuvchi maʼlumotlari:
ReadyMemoDir=Oʻrnatish jildi:
ReadyMemoType=Oʻrnatish turi:
ReadyMemoComponents=Tanlangan komponentlar:
ReadyMemoGroup=«Boshlash» menyusi jildi:
ReadyMemoTasks=Qoʻshimcha vazifalar:

; *** TDownloadWizardPage wizard page and DownloadTemporaryFile
DownloadingLabel2=Fayllar yuklab olinmoqda...
ButtonStopDownload=Yuklab olishni toʻxtatish
StopDownload=Yuklab olishni toʻxtatmoqchimisiz?
ErrorDownloadAborted=Yuklab olish toʻxtatildi
ErrorDownloadFailed=Yuklab olib boʻlmadi: %1 %2
ErrorDownloadSizeFailed=Hajmni aniqlab boʻlmadi: %1 %2
ErrorProgress=Bajarilish koʻrsatkichi notoʻgʻri: %1 / %2
ErrorFileSize=Fayl hajmi notoʻgʻri: kutilgani %1, topilgani %2

; *** TExtractionWizardPage wizard page and ExtractArchive
ExtractingLabel=Fayllar chiqarilmoqda...
ButtonStopExtraction=Chiqarishni toʻxtatish
StopExtraction=Chiqarishni toʻxtatmoqchimisiz?
ErrorExtractionAborted=Chiqarish toʻxtatildi
ErrorExtractionFailed=Chiqarib boʻlmadi: %1

; *** Archive extraction failure details
ArchiveIncorrectPassword=Parol notoʻgʻri
ArchiveIsCorrupted=Arxiv buzilgan
ArchiveUnsupportedFormat=Arxiv formati qoʻllab-quvvatlanmaydi

; *** "Preparing to Install" wizard page
WizardPreparing=Oʻrnatishga tayyorlanmoqda
PreparingDesc=Oʻrnatuvchi [name] dasturini kompyuteringizga oʻrnatishga tayyorlanmoqda.
PreviousInstallNotCompleted=Oldingi dasturni oʻrnatish yoki oʻchirish tugallanmagan. Oʻrnatishni tugatish uchun kompyuterni qayta ishga tushirishingiz kerak.%n%nKompyuter qayta ishga tushgach, [name] oʻrnatishni tugatish uchun Oʻrnatuvchini yana ishga tushiring.
CannotContinue=Oʻrnatishni davom ettirib boʻlmaydi. Chiqish uchun «Bekor qilish» tugmasini bosing.
ApplicationsFound=Quyidagi dasturlar Oʻrnatuvchi yangilashi kerak boʻlgan fayllardan foydalanmoqda. Oʻrnatuvchiga bu dasturlarni avtomatik yopishga ruxsat berish tavsiya etiladi.
ApplicationsFound2=Quyidagi dasturlar Oʻrnatuvchi yangilashi kerak boʻlgan fayllardan foydalanmoqda. Oʻrnatuvchiga bu dasturlarni avtomatik yopishga ruxsat berish tavsiya etiladi. Oʻrnatish tugagach, Oʻrnatuvchi dasturlarni qayta ishga tushirishga urinadi.
CloseApplications=Dasturlarni avtomatik yopish
DontCloseApplications=Dasturlarni yopmaslik
ErrorCloseApplications=Oʻrnatuvchi barcha dasturlarni avtomatik yopa olmadi. Davom etishdan oldin Oʻrnatuvchi yangilashi kerak boʻlgan fayllardan foydalanayotgan barcha dasturlarni yopish tavsiya etiladi.
PrepareToInstallNeedsRestart=Oʻrnatuvchi kompyuteringizni qayta ishga tushirishi kerak. Kompyuter qayta ishga tushgach, [name] oʻrnatishni tugatish uchun Oʻrnatuvchini yana ishga tushiring.%n%nHozir qayta ishga tushirilsinmi?

; *** "Installing" wizard page
WizardInstalling=Oʻrnatilmoqda
InstallingLabel=Oʻrnatuvchi [name] dasturini kompyuteringizga oʻrnatguncha kuting.

; *** "Setup Completed" wizard page
FinishedHeadingLabel=[name] oʻrnatish ustasi yakunlanmoqda
FinishedLabelNoIcons=Oʻrnatuvchi [name] dasturini kompyuteringizga oʻrnatib boʻldi.
FinishedLabel=Oʻrnatuvchi [name] dasturini kompyuteringizga oʻrnatib boʻldi. Dasturni oʻrnatilgan yorliqlar orqali ishga tushirish mumkin.
ClickFinish=Oʻrnatuvchidan chiqish uchun «Yakunlash» tugmasini bosing.
FinishedRestartLabel=[name] oʻrnatishni tugatish uchun Oʻrnatuvchi kompyuteringizni qayta ishga tushirishi kerak. Hozir qayta ishga tushirilsinmi?
FinishedRestartMessage=[name] oʻrnatishni tugatish uchun Oʻrnatuvchi kompyuteringizni qayta ishga tushirishi kerak.%n%nHozir qayta ishga tushirilsinmi?
ShowReadmeCheck=Ha, README faylini koʻrmoqchiman
YesRadio=Ha, kompyuter hozir qayta ishga tushirilsin
NoRadio=Yoʻq, kompyuterni keyinroq oʻzim qayta ishga tushiraman
; used for example as 'Run MyProg.exe'
RunEntryExec=Ishga tushirish: %1
; used for example as 'View Readme.txt'
RunEntryShellExec=Koʻrish: %1

; *** "Setup Needs the Next Disk" stuff
ChangeDiskTitle=Oʻrnatuvchiga navbatdagi disk kerak
SelectDiskLabel2=%1-diskni joylang va «OK» tugmasini bosing.%n%nAgar bu diskdagi fayllar quyida koʻrsatilgan jilddan boshqa jildda boʻlsa, toʻgʻri yoʻlni kiriting yoki «Koʻrib chiqish» tugmasini bosing.
PathLabel=Yoʻl:
FileNotInDir2="%1" fayli "%2" ichidan topilmadi. Toʻgʻri diskni joylang yoki boshqa jildni tanlang.
SelectDirectoryLabel=Navbatdagi diskning joylashuvini koʻrsating.

; *** Installation phase messages
SetupAborted=Oʻrnatish tugallanmadi.%n%nMuammoni bartaraf qiling va Oʻrnatuvchini qaytadan ishga tushiring.
AbortRetryIgnoreSelectAction=Amalni tanlang
AbortRetryIgnoreRetry=Qaytadan urinish
AbortRetryIgnoreIgnore=Xatoga eʼtibor bermay davom etish
AbortRetryIgnoreCancel=Oʻrnatishni bekor qilish
RetryCancelSelectAction=Amalni tanlang
RetryCancelRetry=Qaytadan urinish
RetryCancelCancel=Bekor qilish

; *** Installation status messages
StatusClosingApplications=Dasturlar yopilmoqda...
StatusCreateDirs=Jildlar yaratilmoqda...
StatusExtractFiles=Fayllar chiqarilmoqda...
StatusDownloadFiles=Fayllar yuklab olinmoqda...
StatusCreateIcons=Yorliqlar yaratilmoqda...
StatusCreateIniEntries=INI yozuvlari yaratilmoqda...
StatusCreateRegistryEntries=Reyestr yozuvlari yaratilmoqda...
StatusRegisterFiles=Fayllar roʻyxatdan oʻtkazilmoqda...
StatusSavingUninstall=Oʻchirish maʼlumotlari saqlanmoqda...
StatusRunProgram=Oʻrnatish yakunlanmoqda...
StatusRestartingApplications=Dasturlar qayta ishga tushirilmoqda...
StatusRollback=Oʻzgarishlar bekor qilinmoqda...

; *** Misc. errors
ErrorInternal2=Ichki xato: %1
ErrorFunctionFailedNoCode=%1 bajarilmadi
ErrorFunctionFailed=%1 bajarilmadi; kod %2
ErrorFunctionFailedWithMessage=%1 bajarilmadi; kod %2.%n%3
ErrorExecutingProgram=Faylni ishga tushirib boʻlmadi:%n%1

; *** Registry errors
ErrorRegOpenKey=Reyestr kalitini ochishda xato:%n%1\%2
ErrorRegCreateKey=Reyestr kalitini yaratishda xato:%n%1\%2
ErrorRegWriteKey=Reyestr kalitiga yozishda xato:%n%1\%2

; *** INI errors
ErrorIniEntry="%1" faylida INI yozuvini yaratishda xato.

; *** File copying errors
FileAbortRetryIgnoreSkipNotRecommended=Bu faylni oʻtkazib yuborish (tavsiya etilmaydi)
FileAbortRetryIgnoreIgnoreNotRecommended=Xatoga eʼtibor bermay davom etish (tavsiya etilmaydi)
SourceIsCorrupted=Manba fayl buzilgan
SourceDoesntExist="%1" manba fayli mavjud emas
SourceVerificationFailed=Manba faylni tekshirish muvaffaqiyatsiz tugadi: %1
VerificationSignatureDoesntExist="%1" imzo fayli mavjud emas
VerificationSignatureInvalid="%1" imzo fayli yaroqsiz
VerificationKeyNotFound="%1" imzo fayli nomaʼlum kalitdan foydalanadi
VerificationFileNameIncorrect=Fayl nomi notoʻgʻri
VerificationFileTagIncorrect=Fayl tegi notoʻgʻri
VerificationFileSizeIncorrect=Fayl hajmi notoʻgʻri
VerificationFileHashIncorrect=Fayl xeshi notoʻgʻri
ExistingFileReadOnly2=Mavjud faylni almashtirib boʻlmadi, chunki u «faqat oʻqish uchun» deb belgilangan.
ExistingFileReadOnlyRetry=«Faqat oʻqish uchun» xususiyatini olib tashlab, qaytadan urinish
ExistingFileReadOnlyKeepExisting=Mavjud faylni saqlab qolish
ErrorReadingExistingDest=Mavjud faylni oʻqishda xato yuz berdi:
FileExistsSelectAction=Amalni tanlang
FileExists2=Fayl allaqachon mavjud.
FileExistsOverwriteExisting=Mavjud faylni almashtirish
FileExistsKeepExisting=Mavjud faylni saqlab qolish
FileExistsOverwriteOrKeepAll=Keyingi toʻqnashuvlarda ham shunday qilish
ExistingFileNewerSelectAction=Amalni tanlang
ExistingFileNewer2=Mavjud fayl Oʻrnatuvchi oʻrnatmoqchi boʻlgan fayldan yangiroq.
ExistingFileNewerOverwriteExisting=Mavjud faylni almashtirish
ExistingFileNewerKeepExisting=Mavjud faylni saqlab qolish (tavsiya etiladi)
ExistingFileNewerOverwriteOrKeepAll=Keyingi toʻqnashuvlarda ham shunday qilish
ErrorChangingAttr=Mavjud fayl xususiyatlarini oʻzgartirishda xato yuz berdi:
ErrorCreatingTemp=Manzil jildida fayl yaratishda xato yuz berdi:
ErrorReadingSource=Manba faylni oʻqishda xato yuz berdi:
ErrorCopying=Faylni nusxalashda xato yuz berdi:
ErrorDownloading=Faylni yuklab olishda xato yuz berdi:
ErrorExtracting=Arxivni chiqarishda xato yuz berdi:
ErrorReplacingExistingFile=Mavjud faylni almashtirishda xato yuz berdi:
ErrorRestartReplace=RestartReplace bajarilmadi:
ErrorRenamingTemp=Manzil jildidagi faylning nomini oʻzgartirishda xato yuz berdi:
ErrorRegisterServer=DLL/OCX ni roʻyxatdan oʻtkazib boʻlmadi: %1
ErrorRegSvr32Failed=RegSvr32 %1 chiqish kodi bilan tugadi
ErrorRegisterTypeLib=Tur kutubxonasini roʻyxatdan oʻtkazib boʻlmadi: %1

; *** Uninstall display name markings
; used for example as 'My Program (32-bit)'
UninstallDisplayNameMark=%1 (%2)
; used for example as 'My Program (32-bit, All users)'
UninstallDisplayNameMarks=%1 (%2, %3)
UninstallDisplayNameMark32Bit=32 bitli
UninstallDisplayNameMark64Bit=64 bitli
UninstallDisplayNameMarkAllUsers=Barcha foydalanuvchilar
UninstallDisplayNameMarkCurrentUser=Joriy foydalanuvchi

; *** Post-installation errors
ErrorOpeningReadme=README faylini ochishda xato yuz berdi.
ErrorRestartingComputer=Oʻrnatuvchi kompyuterni qayta ishga tushira olmadi. Buni qoʻlda bajaring.

; *** Uninstaller messages
UninstallNotFound="%1" fayli mavjud emas. Oʻchirib boʻlmaydi.
UninstallOpenError="%1" faylini ochib boʻlmadi. Oʻchirib boʻlmaydi
UninstallUnsupportedVer="%1" oʻchirish jurnali fayli Oʻchirish dasturining bu versiyasi tanimaydigan formatda. Oʻchirib boʻlmaydi
UninstallUnknownEntry=Oʻchirish jurnalida nomaʼlum yozuv (%1) uchradi
ConfirmUninstall=%1 va uning barcha komponentlarini butunlay oʻchirib tashlamoqchimisiz?
UninstallOnlyOnWin64=Bu oʻrnatishni faqat 64 bitli Windows muhitida oʻchirish mumkin.
OnlyAdminCanUninstall=Bu oʻrnatishni faqat administrator huquqlariga ega foydalanuvchi oʻchira oladi.
UninstallStatusLabel=%1 kompyuteringizdan oʻchirilguncha kuting.
UninstalledAll=%1 kompyuteringizdan muvaffaqiyatli oʻchirildi.
UninstalledMost=%1 oʻchirildi.%n%nBaʼzi elementlarni oʻchirib boʻlmadi. Ularni qoʻlda oʻchirish mumkin.
UninstalledAndNeedsRestart=%1 oʻchirishni tugatish uchun kompyuterni qayta ishga tushirish kerak.%n%nHozir qayta ishga tushirilsinmi?
UninstallDataCorrupted="%1" fayli buzilgan. Oʻchirib boʻlmaydi

; *** Uninstallation phase messages
ConfirmDeleteSharedFileTitle=Umumiy fayl oʻchirilsinmi?
ConfirmDeleteSharedFile2=Tizim quyidagi umumiy fayldan endi hech qanday dastur foydalanmayotganini bildirmoqda. Oʻchirish dasturi bu umumiy faylni oʻchirsinmi?%n%nAgar baʼzi dasturlar bu fayldan hamon foydalanayotgan boʻlsa va u oʻchirilsa, ular toʻgʻri ishlamay qolishi mumkin. Ishonchingiz komil boʻlmasa, «Yoʻq» tugmasini tanlang. Faylni tizimda qoldirish hech qanday zarar keltirmaydi.
SharedFileNameLabel=Fayl nomi:
SharedFileLocationLabel=Joylashuvi:
WizardUninstalling=Oʻchirish holati
StatusUninstalling=%1 oʻchirilmoqda...

; *** Shutdown block reasons
ShutdownBlockReasonInstallingApp=%1 oʻrnatilmoqda.
ShutdownBlockReasonUninstallingApp=%1 oʻchirilmoqda.

; The custom messages below aren't used by Setup itself, but if you make
; use of them in your scripts, you'll want to translate them.

[CustomMessages]

NameAndVersion=%1 versiya %2
AdditionalIcons=Qoʻshimcha yorliqlar:
CreateDesktopIcon=Ish stolida yorliq yaratish
CreateQuickLaunchIcon=Tez ishga tushirish panelida yorliq yaratish
ProgramOnTheWeb=%1 internetda
UninstallProgram=Oʻchirish: %1
LaunchProgram=Ishga tushirish: %1
AssocFileExtension=%1 dasturini %2 fayl kengaytmasi bilan bogʻlash
AssocingFileExtension=%1 dasturi %2 fayl kengaytmasi bilan bogʻlanmoqda...
AutoStartProgramGroupDescription=Avtoyuklash:
AutoStartProgram=Avtomatik ishga tushirish: %1
AddonHostProgramNotFound=%1 siz tanlagan jildda topilmadi.%n%nBaribir davom etasizmi?

; The messages of the installer of Lang Switcher that are its own (the sample is English.isl).
LocalCertReady=Oʻrnatuvchi ushbu kompyuterda sertifikat yaratadi va Lang Switcher dasturini u bilan imzolaydi, shunda dastur administrator nomidan ishga tushirilgan dasturlar oynalarida ham ishlay oladi.
LocalCertFailed=Lang Switcher oʻrnatildi, lekin administrator nomidan ishga tushirilgan dasturlar oynalarida ishlamaydi: Windows ushbu kompyuterda yaratilgan sertifikatni qabul qilmadi. Tafsilotlar — oʻrnatish jurnalida.
RemoveSettingsQuestion=Lang Switcher sozlamalar fayli ham oʻchirilsinmi?
RemoveSettingsKeep=Sozlamalar keyinroq qayta oʻrnatish uchun saqlansin desangiz, «Yoʻq» tugmasini tanlang.
AuthorSlogan=dil bilan yaratilgan dasturlar
