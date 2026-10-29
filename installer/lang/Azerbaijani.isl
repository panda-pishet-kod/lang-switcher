; Lang Switcher -- the messages of the installer in Azerbaijani, Latin script (stage E108, group A).
;
; Inno Setup has no official Azerbaijani translation (inno_official = false in res\lang\languages.toml),
; so this file carries the whole of it: [LangOptions], every message of the compiler:Default.isl
; (6.5.0+), and the messages of the installer that are Lang Switcher's own. It is read after
; compiler:Default.isl as the second MessagesFile of the "azerbaijani" entry of [Languages] in
; ..\LangSwitcher.iss, and so replaces the English text name for name.
; English.isl is the sample the own messages translate; the glossary of the language is
; l10n\glossary\az.tsv of the control contour. The name of the program is not translated.
; As in Default.isl: a message that has no full stop there has none here (Setup adds its own).
; No accelerators (&) in this file, as in Tamil.isl (RULES 16): the names of the buttons inside
; a message stand in the guillemets of the language and match the captions of the buttons.
; The dotted and the dotless i are different letters: İ i / I ı.

[LangOptions]
LanguageName=Azərbaycan dili
LanguageID=$042C
LanguageCodePage=0
;DialogFontName=
;DialogFontSize=9
;DialogFontBaseScaleWidth=7
;DialogFontBaseScaleHeight=15
;WelcomeFontName=Segoe UI
;WelcomeFontSize=14

[Messages]

; *** Application titles
SetupAppTitle=Quraşdırma
SetupWindowTitle=Quraşdırma - %1
UninstallAppTitle=Silmə
UninstallAppFullTitle=%1 proqramının silinməsi

; *** Misc. common
InformationTitle=Məlumat
ConfirmTitle=Təsdiq
ErrorTitle=Xəta

; *** SetupLdr messages
SetupLdrStartupMessage=Bu, %1 proqramını quraşdıracaq. Davam etmək istəyirsiniz?
LdrCannotCreateTemp=Müvəqqəti fayl yaradıla bilmədi. Quraşdırma dayandırıldı
LdrCannotExecTemp=Müvəqqəti qovluqdakı fayl icra edilə bilmədi. Quraşdırma dayandırıldı
HelpTextNote=

; *** Startup error messages
LastErrorMessage=%1.%n%nXəta %2: %3
SetupFileMissing=%1 faylı quraşdırma qovluğunda yoxdur. Problemi həll edin və ya proqramın yeni nüsxəsini əldə edin.
SetupFileCorrupt=Quraşdırma faylları zədələnib. Proqramın yeni nüsxəsini əldə edin.
SetupFileCorruptOrWrongVer=Quraşdırma faylları zədələnib və ya Quraşdırmanın bu versiyası ilə uyğun gəlmir. Problemi həll edin və ya proqramın yeni nüsxəsini əldə edin.
InvalidParameter=Əmr sətrində yanlış parametr verilib:%n%n%1
SetupAlreadyRunning=Quraşdırma artıq işləyir.
WindowsVersionNotSupported=Bu proqram kompüterinizdə işləyən Windows versiyasını dəstəkləmir.
WindowsServicePackRequired=Bu proqram üçün %1 Service Pack %2 və ya daha yeni versiya tələb olunur.
NotOnThisPlatform=Bu proqram %1 üzərində işləməyəcək.
OnlyOnThisPlatform=Bu proqram %1 üzərində işə salınmalıdır.
OnlyOnTheseArchitectures=Bu proqram yalnız aşağıdakı prosessor arxitekturaları üçün nəzərdə tutulmuş Windows versiyalarına quraşdırıla bilər:%n%n%1
WinVersionTooLowError=Bu proqram üçün %1 versiya %2 və ya daha yeni versiya tələb olunur.
WinVersionTooHighError=Bu proqram %1 versiya %2 və ya daha yeni versiyaya quraşdırıla bilməz.
AdminPrivilegesRequired=Bu proqramı quraşdırarkən administrator kimi daxil olmalısınız.
PowerUserPrivilegesRequired=Bu proqramı quraşdırarkən administrator kimi və ya Power Users qrupunun üzvü kimi daxil olmalısınız.
SetupAppRunningError=Quraşdırma %1 proqramının hazırda işlədiyini aşkar etdi.%n%nOnun bütün nüsxələrini indi bağlayın, sonra davam etmək üçün «OK», çıxmaq üçün «Ləğv et» düyməsini basın.
UninstallAppRunningError=Silmə proqramı %1 proqramının hazırda işlədiyini aşkar etdi.%n%nOnun bütün nüsxələrini indi bağlayın, sonra davam etmək üçün «OK», çıxmaq üçün «Ləğv et» düyməsini basın.

; *** Startup questions
PrivilegesRequiredOverrideTitle=Quraşdırma rejimini seçin
PrivilegesRequiredOverrideInstruction=Quraşdırma rejimini seçin
PrivilegesRequiredOverrideText1=%1 bütün istifadəçilər üçün (administrator hüquqları tələb olunur) və ya yalnız sizin üçün quraşdırıla bilər.
PrivilegesRequiredOverrideText2=%1 yalnız sizin üçün və ya bütün istifadəçilər üçün (administrator hüquqları tələb olunur) quraşdırıla bilər.
PrivilegesRequiredOverrideAllUsers=Bütün istifadəçilər üçün quraşdır
PrivilegesRequiredOverrideAllUsersRecommended=Bütün istifadəçilər üçün quraşdır (tövsiyə olunur)
PrivilegesRequiredOverrideCurrentUser=Yalnız mənim üçün quraşdır
PrivilegesRequiredOverrideCurrentUserRecommended=Yalnız mənim üçün quraşdır (tövsiyə olunur)

; *** Misc. errors
ErrorCreatingDir=Quraşdırma "%1" qovluğunu yarada bilmədi
ErrorTooManyFilesInDir="%1" qovluğunda fayl yaratmaq mümkün deyil, çünki orada çox fayl var

; *** Setup common messages
ExitSetupTitle=Quraşdırmadan çıxış
ExitSetupMessage=Quraşdırma tamamlanmayıb. İndi çıxsanız, proqram quraşdırılmayacaq.%n%nQuraşdırmanı tamamlamaq üçün Quraşdırmanı sonra yenidən işə sala bilərsiniz.%n%nQuraşdırmadan çıxılsın?
AboutSetupMenuItem=Quraşdırma haqqında...
AboutSetupTitle=Quraşdırma haqqında
AboutSetupMessage=%1 versiya %2%n%3%n%n%1 ana səhifəsi:%n%4
AboutSetupNote=
TranslatorNote=

; *** Buttons
ButtonBack=< Geri
ButtonNext=Növbəti >
ButtonInstall=Quraşdır
ButtonOK=OK
ButtonCancel=Ləğv et
ButtonYes=Bəli
ButtonYesToAll=Hamısına bəli
ButtonNo=Xeyr
ButtonNoToAll=Hamısına xeyr
ButtonFinish=Bitir
ButtonBrowse=Nəzərdən keçir...
ButtonWizardBrowse=Nəzərdən keçir...
ButtonNewFolder=Yeni qovluq yarat

; *** "Select Language" dialog messages
SelectLanguageTitle=Quraşdırma dilini seçin
SelectLanguageLabel=Quraşdırma zamanı istifadə ediləcək dili seçin.

; *** Common wizard text
ClickNext=Davam etmək üçün «Növbəti», çıxmaq üçün «Ləğv et» düyməsini basın.
BeveledLabel=
BrowseDialogTitle=Qovluğu seçin
BrowseDialogLabel=Aşağıdakı siyahıdan qovluğu seçin, sonra «OK» düyməsini basın.
NewFolderName=Yeni qovluq

; *** "Welcome" wizard page
WelcomeLabel1=[name] quraşdırma sehrbazına xoş gəlmisiniz
WelcomeLabel2=Bu sehrbaz [name/ver] proqramını kompüterinizə quraşdıracaq.%n%nDavam etməzdən əvvəl bütün digər proqramları bağlamaq tövsiyə olunur.

; *** "Password" wizard page
WizardPassword=Parol
PasswordLabel1=Bu quraşdırma parolla qorunur.
PasswordLabel3=Parolu daxil edin və davam etmək üçün «Növbəti» düyməsini basın. Parol böyük və kiçik hərflərə həssasdır.
PasswordEditLabel=Parol:
IncorrectPassword=Daxil etdiyiniz parol düzgün deyil. Yenidən cəhd edin.

; *** "License Agreement" wizard page
WizardLicense=Lisenziya razılaşması
LicenseLabel=Davam etməzdən əvvəl aşağıdakı vacib məlumatı oxuyun.
LicenseLabel3=Aşağıdakı lisenziya razılaşmasını oxuyun. Quraşdırmanı davam etdirmək üçün bu razılaşmanın şərtlərini qəbul etməlisiniz.
LicenseAccepted=Razılaşmanı qəbul edirəm
LicenseNotAccepted=Razılaşmanı qəbul etmirəm

; *** "Information" wizard pages
WizardInfoBefore=Məlumat
InfoBeforeLabel=Davam etməzdən əvvəl aşağıdakı vacib məlumatı oxuyun.
InfoBeforeClickLabel=Quraşdırmanı davam etdirməyə hazır olduqda «Növbəti» düyməsini basın.
WizardInfoAfter=Məlumat
InfoAfterLabel=Davam etməzdən əvvəl aşağıdakı vacib məlumatı oxuyun.
InfoAfterClickLabel=Quraşdırmanı davam etdirməyə hazır olduqda «Növbəti» düyməsini basın.

; *** "User Information" wizard page
WizardUserInfo=İstifadəçi məlumatı
UserInfoDesc=Məlumatlarınızı daxil edin.
UserInfoName=İstifadəçi adı:
UserInfoOrg=Təşkilat:
UserInfoSerial=Seriya nömrəsi:
UserInfoNameRequired=Ad daxil etməlisiniz.

; *** "Select Destination Location" wizard page
WizardSelectDir=Quraşdırma yerini seçin
SelectDirDesc=[name] harada quraşdırılsın?
SelectDirLabel3=Quraşdırma [name] proqramını aşağıdakı qovluğa quraşdıracaq.
SelectDirBrowseLabel=Davam etmək üçün «Növbəti» düyməsini basın. Başqa qovluq seçmək istəyirsinizsə, «Nəzərdən keçir» düyməsini basın.
DiskSpaceGBLabel=Ən azı [gb] GB boş disk sahəsi tələb olunur.
DiskSpaceMBLabel=Ən azı [mb] MB boş disk sahəsi tələb olunur.
CannotInstallToNetworkDrive=Quraşdırma şəbəkə diskinə quraşdıra bilmir.
CannotInstallToUNCPath=Quraşdırma UNC yoluna quraşdıra bilmir.
InvalidPath=Disk adı ilə tam yol daxil etməlisiniz; məsələn:%n%nC:\APP%n%nvə ya aşağıdakı formada UNC yolu:%n%n\\server\share
InvalidDrive=Seçdiyiniz disk və ya UNC şəbəkə qovluğu mövcud deyil və ya əlçatan deyil. Başqasını seçin.
DiskSpaceWarningTitle=Disk sahəsi kifayət deyil
DiskSpaceWarning=Quraşdırma üçün ən azı %1 KB boş yer lazımdır, lakin seçilmiş diskdə yalnız %2 KB var.%n%nYenə də davam etmək istəyirsiniz?
DirNameTooLong=Qovluğun adı və ya yolu çox uzundur.
InvalidDirName=Qovluğun adı yararsızdır.
BadDirName32=Qovluq adlarında aşağıdakı simvollar ola bilməz:%n%n%1
DirExistsTitle=Qovluq mövcuddur
DirExists=Bu qovluq:%n%n%1%n%nartıq mövcuddur. Yenə də həmin qovluğa quraşdırmaq istəyirsiniz?
DirDoesntExistTitle=Qovluq mövcud deyil
DirDoesntExist=Bu qovluq:%n%n%1%n%nmövcud deyil. Qovluğun yaradılmasını istəyirsiniz?

; *** "Select Components" wizard page
WizardSelectComponents=Komponentləri seçin
SelectComponentsDesc=Hansı komponentlər quraşdırılsın?
SelectComponentsLabel2=Quraşdırmaq istədiyiniz komponentləri seçin; quraşdırmaq istəmədiklərinizin seçimini ləğv edin. Davam etməyə hazır olduqda «Növbəti» düyməsini basın.
FullInstallation=Tam quraşdırma
; if possible don't translate 'Compact' as 'Minimal' (I mean 'Minimal' in your language)
CompactInstallation=Yığcam quraşdırma
CustomInstallation=Xüsusi quraşdırma
NoUninstallWarningTitle=Komponentlər mövcuddur
NoUninstallWarning=Quraşdırma aşağıdakı komponentlərin kompüterinizdə artıq quraşdırıldığını aşkar etdi:%n%n%1%n%nBu komponentlərin seçimini ləğv etmək onları silməyəcək.%n%nYenə də davam etmək istəyirsiniz?
ComponentSize1=%1 KB
ComponentSize2=%1 MB
ComponentsDiskSpaceGBLabel=Cari seçim ən azı [gb] GB disk sahəsi tələb edir.
ComponentsDiskSpaceMBLabel=Cari seçim ən azı [mb] MB disk sahəsi tələb edir.

; *** "Select Additional Tasks" wizard page
WizardSelectTasks=Əlavə tapşırıqları seçin
SelectTasksDesc=Hansı əlavə tapşırıqlar yerinə yetirilsin?
SelectTasksLabel2=[name] quraşdırılarkən Quraşdırmanın yerinə yetirməsini istədiyiniz əlavə tapşırıqları seçin, sonra «Növbəti» düyməsini basın.

; *** "Select Start Menu Folder" wizard page
WizardSelectProgramGroup=Start menyusu qovluğunu seçin
SelectStartMenuFolderDesc=Quraşdırma proqramın qısayollarını harada yerləşdirsin?
SelectStartMenuFolderLabel3=Quraşdırma proqramın qısayollarını Start menyusunun aşağıdakı qovluğunda yaradacaq.
SelectStartMenuFolderBrowseLabel=Davam etmək üçün «Növbəti» düyməsini basın. Başqa qovluq seçmək istəyirsinizsə, «Nəzərdən keçir» düyməsini basın.
MustEnterGroupName=Qovluq adı daxil etməlisiniz.
GroupNameTooLong=Qovluğun adı və ya yolu çox uzundur.
InvalidGroupName=Qovluğun adı yararsızdır.
BadGroupName=Qovluq adında aşağıdakı simvollar ola bilməz:%n%n%1
NoProgramGroupCheck2=Start menyusunda qovluq yaradılmasın

; *** "Ready to Install" wizard page
WizardReady=Quraşdırmağa hazırdır
ReadyLabel1=Quraşdırma [name] proqramını kompüterinizə quraşdırmağa hazırdır.
ReadyLabel2a=Quraşdırmanı davam etdirmək üçün «Quraşdır» düyməsini, parametrlərə baxmaq və ya dəyişmək istəyirsinizsə, «Geri» düyməsini basın.
ReadyLabel2b=Quraşdırmanı davam etdirmək üçün «Quraşdır» düyməsini basın.
ReadyMemoUserInfo=İstifadəçi məlumatı:
ReadyMemoDir=Quraşdırma yeri:
ReadyMemoType=Quraşdırma növü:
ReadyMemoComponents=Seçilmiş komponentlər:
ReadyMemoGroup=Start menyusu qovluğu:
ReadyMemoTasks=Əlavə tapşırıqlar:

; *** TDownloadWizardPage wizard page and DownloadTemporaryFile
DownloadingLabel2=Fayllar yüklənir...
ButtonStopDownload=Yükləməni dayandır
StopDownload=Yükləməni dayandırmaq istədiyinizə əminsiniz?
ErrorDownloadAborted=Yükləmə dayandırıldı
ErrorDownloadFailed=Yükləmə alınmadı: %1 %2
ErrorDownloadSizeFailed=Ölçünü almaq alınmadı: %1 %2
ErrorProgress=Yanlış irəliləyiş: %1 / %2
ErrorFileSize=Yanlış fayl ölçüsü: gözlənilən %1, tapılan %2

; *** TExtractionWizardPage wizard page and ExtractArchive
ExtractingLabel=Fayllar çıxarılır...
ButtonStopExtraction=Çıxarmanı dayandır
StopExtraction=Çıxarmanı dayandırmaq istədiyinizə əminsiniz?
ErrorExtractionAborted=Çıxarma dayandırıldı
ErrorExtractionFailed=Çıxarma alınmadı: %1

; *** Archive extraction failure details
ArchiveIncorrectPassword=Parol yanlışdır
ArchiveIsCorrupted=Arxiv zədələnib
ArchiveUnsupportedFormat=Arxiv formatı dəstəklənmir

; *** "Preparing to Install" wizard page
WizardPreparing=Quraşdırmaya hazırlıq
PreparingDesc=Quraşdırma [name] proqramını kompüterinizə quraşdırmağa hazırlaşır.
PreviousInstallNotCompleted=Əvvəlki proqramın quraşdırılması/silinməsi tamamlanmayıb. Həmin quraşdırmanı tamamlamaq üçün kompüteri yenidən başlatmalısınız.%n%nKompüteri yenidən başlatdıqdan sonra [name] proqramının quraşdırılmasını tamamlamaq üçün Quraşdırmanı yenidən işə salın.
CannotContinue=Quraşdırma davam edə bilmir. Çıxmaq üçün «Ləğv et» düyməsini basın.
ApplicationsFound=Aşağıdakı proqramlar Quraşdırma tərəfindən yenilənməli olan fayllardan istifadə edir. Quraşdırmanın bu proqramları avtomatik bağlamasına icazə verməyiniz tövsiyə olunur.
ApplicationsFound2=Aşağıdakı proqramlar Quraşdırma tərəfindən yenilənməli olan fayllardan istifadə edir. Quraşdırmanın bu proqramları avtomatik bağlamasına icazə verməyiniz tövsiyə olunur. Quraşdırma başa çatdıqdan sonra Quraşdırma bu proqramları yenidən başlatmağa çalışacaq.
CloseApplications=Proqramlar avtomatik bağlansın
DontCloseApplications=Proqramlar bağlanmasın
ErrorCloseApplications=Quraşdırma bütün proqramları avtomatik bağlaya bilmədi. Davam etməzdən əvvəl Quraşdırma tərəfindən yenilənməli olan fayllardan istifadə edən bütün proqramları bağlamaq tövsiyə olunur.
PrepareToInstallNeedsRestart=Quraşdırma kompüterinizi yenidən başlatmalıdır. Kompüteri yenidən başlatdıqdan sonra [name] proqramının quraşdırılmasını tamamlamaq üçün Quraşdırmanı yenidən işə salın.%n%nİndi yenidən başladılsın?

; *** "Installing" wizard page
WizardInstalling=Quraşdırılır
InstallingLabel=Quraşdırma [name] proqramını kompüterinizə quraşdırarkən gözləyin.

; *** "Setup Completed" wizard page
FinishedHeadingLabel=[name] quraşdırma sehrbazı tamamlanır
FinishedLabelNoIcons=Quraşdırma [name] proqramının kompüterinizə quraşdırılmasını başa çatdırdı.
FinishedLabel=Quraşdırma [name] proqramının kompüterinizə quraşdırılmasını başa çatdırdı. Proqramı quraşdırılmış qısayollar vasitəsilə işə salmaq olar.
ClickFinish=Quraşdırmadan çıxmaq üçün «Bitir» düyməsini basın.
FinishedRestartLabel=[name] proqramının quraşdırılmasını tamamlamaq üçün Quraşdırma kompüterinizi yenidən başlatmalıdır. İndi yenidən başladılsın?
FinishedRestartMessage=[name] proqramının quraşdırılmasını tamamlamaq üçün Quraşdırma kompüterinizi yenidən başlatmalıdır.%n%nİndi yenidən başladılsın?
ShowReadmeCheck=Bəli, README faylına baxmaq istəyirəm
YesRadio=Bəli, kompüteri indi yenidən başlat
NoRadio=Xeyr, kompüteri sonra yenidən başladacağam
; used for example as 'Run MyProg.exe'
RunEntryExec=%1 proqramını işə sal
; used for example as 'View Readme.txt'
RunEntryShellExec=%1 faylına bax

; *** "Setup Needs the Next Disk" stuff
ChangeDiskTitle=Quraşdırma növbəti diski tələb edir
SelectDiskLabel2=%1 diskini daxil edin və «OK» düyməsini basın.%n%nBu diskdəki fayllar aşağıda göstərilən qovluqdan başqa qovluqda yerləşirsə, düzgün yolu daxil edin və ya «Nəzərdən keçir» düyməsini basın.
PathLabel=Yol:
FileNotInDir2="%1" faylı "%2" içində tapılmadı. Düzgün diski daxil edin və ya başqa qovluq seçin.
SelectDirectoryLabel=Növbəti diskin yerini göstərin.

; *** Installation phase messages
SetupAborted=Quraşdırma tamamlanmadı.%n%nProblemi həll edin və Quraşdırmanı yenidən işə salın.
AbortRetryIgnoreSelectAction=Əməliyyatı seçin
AbortRetryIgnoreRetry=Yenidən cəhd et
AbortRetryIgnoreIgnore=Xətanı nəzərə alma və davam et
AbortRetryIgnoreCancel=Quraşdırmanı ləğv et
RetryCancelSelectAction=Əməliyyatı seçin
RetryCancelRetry=Yenidən cəhd et
RetryCancelCancel=Ləğv et

; *** Installation status messages
StatusClosingApplications=Proqramlar bağlanır...
StatusCreateDirs=Qovluqlar yaradılır...
StatusExtractFiles=Fayllar çıxarılır...
StatusDownloadFiles=Fayllar yüklənir...
StatusCreateIcons=Qısayollar yaradılır...
StatusCreateIniEntries=INI qeydləri yaradılır...
StatusCreateRegistryEntries=Reyestr qeydləri yaradılır...
StatusRegisterFiles=Fayllar qeydiyyatdan keçirilir...
StatusSavingUninstall=Silmə məlumatı saxlanılır...
StatusRunProgram=Quraşdırma tamamlanır...
StatusRestartingApplications=Proqramlar yenidən başladılır...
StatusRollback=Dəyişikliklər geri qaytarılır...

; *** Misc. errors
ErrorInternal2=Daxili xəta: %1
ErrorFunctionFailedNoCode=%1 alınmadı
ErrorFunctionFailed=%1 alınmadı; kod %2
ErrorFunctionFailedWithMessage=%1 alınmadı; kod %2.%n%3
ErrorExecutingProgram=Fayl icra edilə bilmədi:%n%1

; *** Registry errors
ErrorRegOpenKey=Reyestr açarı açılarkən xəta:%n%1\%2
ErrorRegCreateKey=Reyestr açarı yaradılarkən xəta:%n%1\%2
ErrorRegWriteKey=Reyestr açarına yazılarkən xəta:%n%1\%2

; *** INI errors
ErrorIniEntry="%1" faylında INI qeydi yaradılarkən xəta.

; *** File copying errors
FileAbortRetryIgnoreSkipNotRecommended=Bu faylı keç (tövsiyə olunmur)
FileAbortRetryIgnoreIgnoreNotRecommended=Xətanı nəzərə alma və davam et (tövsiyə olunmur)
SourceIsCorrupted=Mənbə faylı zədələnib
SourceDoesntExist="%1" mənbə faylı mövcud deyil
SourceVerificationFailed=Mənbə faylının yoxlanışı alınmadı: %1
VerificationSignatureDoesntExist="%1" imza faylı mövcud deyil
VerificationSignatureInvalid="%1" imza faylı yararsızdır
VerificationKeyNotFound="%1" imza faylı naməlum açardan istifadə edir
VerificationFileNameIncorrect=Faylın adı yanlışdır
VerificationFileTagIncorrect=Faylın etiketi yanlışdır
VerificationFileSizeIncorrect=Faylın ölçüsü yanlışdır
VerificationFileHashIncorrect=Faylın heş-kodu yanlışdır
ExistingFileReadOnly2=Mövcud faylı əvəz etmək mümkün olmadı, çünki o, «yalnız oxumaq üçün» kimi işarələnib.
ExistingFileReadOnlyRetry=«Yalnız oxumaq üçün» atributunu sil və yenidən cəhd et
ExistingFileReadOnlyKeepExisting=Mövcud faylı saxla
ErrorReadingExistingDest=Mövcud fayl oxunarkən xəta baş verdi:
FileExistsSelectAction=Əməliyyatı seçin
FileExists2=Fayl artıq mövcuddur.
FileExistsOverwriteExisting=Mövcud faylın üzərinə yaz
FileExistsKeepExisting=Mövcud faylı saxla
FileExistsOverwriteOrKeepAll=Bunu növbəti ziddiyyətlər üçün də et
ExistingFileNewerSelectAction=Əməliyyatı seçin
ExistingFileNewer2=Mövcud fayl Quraşdırmanın quraşdırmağa çalışdığı fayldan daha yenidir.
ExistingFileNewerOverwriteExisting=Mövcud faylın üzərinə yaz
ExistingFileNewerKeepExisting=Mövcud faylı saxla (tövsiyə olunur)
ExistingFileNewerOverwriteOrKeepAll=Bunu növbəti ziddiyyətlər üçün də et
ErrorChangingAttr=Mövcud faylın atributları dəyişdirilərkən xəta baş verdi:
ErrorCreatingTemp=Təyinat qovluğunda fayl yaradılarkən xəta baş verdi:
ErrorReadingSource=Mənbə faylı oxunarkən xəta baş verdi:
ErrorCopying=Fayl kopyalanarkən xəta baş verdi:
ErrorDownloading=Fayl yüklənərkən xəta baş verdi:
ErrorExtracting=Arxiv çıxarılarkən xəta baş verdi:
ErrorReplacingExistingFile=Mövcud fayl əvəz edilərkən xəta baş verdi:
ErrorRestartReplace=RestartReplace alınmadı:
ErrorRenamingTemp=Təyinat qovluğunda fayl yenidən adlandırılarkən xəta baş verdi:
ErrorRegisterServer=DLL/OCX qeydiyyatdan keçirilə bilmədi: %1
ErrorRegSvr32Failed=RegSvr32 %1 çıxış kodu ilə alınmadı
ErrorRegisterTypeLib=Tip kitabxanası qeydiyyatdan keçirilə bilmədi: %1

; *** Uninstall display name markings
; used for example as 'My Program (32-bit)'
UninstallDisplayNameMark=%1 (%2)
; used for example as 'My Program (32-bit, All users)'
UninstallDisplayNameMarks=%1 (%2, %3)
UninstallDisplayNameMark32Bit=32-bit
UninstallDisplayNameMark64Bit=64-bit
UninstallDisplayNameMarkAllUsers=Bütün istifadəçilər
UninstallDisplayNameMarkCurrentUser=Cari istifadəçi

; *** Post-installation errors
ErrorOpeningReadme=README faylı açılarkən xəta baş verdi.
ErrorRestartingComputer=Quraşdırma kompüteri yenidən başlada bilmədi. Bunu əl ilə edin.

; *** Uninstaller messages
UninstallNotFound="%1" faylı mövcud deyil. Silmək mümkün deyil.
UninstallOpenError="%1" faylı açıla bilmədi. Silmək mümkün deyil
UninstallUnsupportedVer="%1" silmə jurnalı faylı silmə proqramının bu versiyası tərəfindən tanınmayan formatdadır. Silmək mümkün deyil
UninstallUnknownEntry=Silmə jurnalında naməlum qeyd (%1) rast gəlindi
ConfirmUninstall=%1 proqramını və onun bütün komponentlərini tamamilə silmək istədiyinizə əminsiniz?
UninstallOnlyOnWin64=Bu quraşdırma yalnız 64-bit Windows-da silinə bilər.
OnlyAdminCanUninstall=Bu quraşdırmanı yalnız administrator hüquqlarına malik istifadəçi silə bilər.
UninstallStatusLabel=%1 kompüterinizdən silinərkən gözləyin.
UninstalledAll=%1 kompüterinizdən uğurla silindi.
UninstalledMost=%1 proqramının silinməsi tamamlandı.%n%nBəzi elementləri silmək mümkün olmadı. Onları əl ilə silmək olar.
UninstalledAndNeedsRestart=%1 proqramının silinməsini tamamlamaq üçün kompüteri yenidən başlatmaq lazımdır.%n%nİndi yenidən başladılsın?
UninstallDataCorrupted="%1" faylı zədələnib. Silmək mümkün deyil

; *** Uninstallation phase messages
ConfirmDeleteSharedFileTitle=Ümumi fayl silinsin?
ConfirmDeleteSharedFile2=Sistem aşağıdakı ümumi faylın artıq heç bir proqram tərəfindən istifadə edilmədiyini bildirir. Silmə proqramı bu ümumi faylı silsin?%n%nHər hansı proqram bu faylı hələ də istifadə edirsə və fayl silinərsə, həmin proqramlar düzgün işləməyə bilər. Əmin deyilsinizsə, «Xeyr» seçin. Faylın sistemdə qalması zərər vermir.
SharedFileNameLabel=Fayl adı:
SharedFileLocationLabel=Yeri:
WizardUninstalling=Silmənin vəziyyəti
StatusUninstalling=%1 silinir...

; *** Shutdown block reasons
ShutdownBlockReasonInstallingApp=%1 quraşdırılır.
ShutdownBlockReasonUninstallingApp=%1 silinir.

; The custom messages below aren't used by Setup itself, but if you make
; use of them in your scripts, you'll want to translate them.

[CustomMessages]

NameAndVersion=%1 versiya %2
AdditionalIcons=Əlavə qısayollar:
CreateDesktopIcon=İş masasında qısayol yarat
CreateQuickLaunchIcon=Sürətli işə salma qısayolu yarat
ProgramOnTheWeb=%1 internetdə
UninstallProgram=%1 proqramını sil
LaunchProgram=%1 proqramını işə sal
AssocFileExtension=%1 proqramını %2 fayl uzantısı ilə əlaqələndir
AssocingFileExtension=%1 proqramı %2 fayl uzantısı ilə əlaqələndirilir...
AutoStartProgramGroupDescription=Avtomatik başlatma:
AutoStartProgram=%1 proqramını avtomatik başlat
AddonHostProgramNotFound=%1 seçdiyiniz qovluqda tapılmadı.%n%nYenə də davam etmək istəyirsiniz?

; *** The messages of the installer that are Lang Switcher's own (the sample is English.isl)
LocalCertReady=Quraşdırma bu kompüterdə sertifikat yaradacaq və Lang Switcher proqramını onunla imzalayacaq ki, proqram administrator adından işə salınmış proqramların pəncərələrində də işləsin.
LocalCertFailed=Lang Switcher quraşdırıldı, lakin administrator adından işə salınmış proqramların pəncərələrində işləməyəcək: Windows bu kompüterdə yaradılmış sertifikatı qəbul etmədi. Ətraflı məlumat quraşdırma jurnalındadır.
RemoveSettingsQuestion=Lang Switcher parametrlər faylı da silinsin?
RemoveSettingsKeep=Parametrləri sonradan yenidən quraşdırma üçün saxlamaq istəyirsinizsə, «Xeyr» seçin.
AuthorSlogan=ürəklə hazırlanmış proqramlar
