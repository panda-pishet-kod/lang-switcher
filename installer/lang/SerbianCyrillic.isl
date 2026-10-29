; Lang Switcher -- the messages of the installer in Serbian, Cyrillic (stage E108, group A).
;
; Inno Setup has no official Serbian translation (inno_official = false in res\lang\languages.toml),
; so this file carries the whole of it: [LangOptions], every message of the compiler:Default.isl
; (6.5.0+), and the messages of the installer that are Lang Switcher's own. It is read after
; compiler:Default.isl as the second MessagesFile of the "serbiancyrillic" entry of [Languages] in
; ..\LangSwitcher.iss, and so replaces the English text name for name.
; English.isl is the sample the own messages translate; the glossary of the language is
; l10n\glossary\sr.tsv of the control contour. The name of the program is not translated.
; The same text in Latin letters is SerbianLatin.isl (one dictionary for both, letter for letter).
; As in Default.isl: a message that has no full stop there has none here (Setup adds its own).
; No accelerators (&): in a file without an official Inno translation they are left out, so that
; one window never shows two equal letters (RULES 16). The names of the buttons inside a message
; are in the quotation marks of the language, „ “ (RULES 16).

[LangOptions]
LanguageName=Српски
LanguageID=$281A
LanguageCodePage=0

[Messages]

; *** Application titles
SetupAppTitle=Инсталација
SetupWindowTitle=Инсталација - %1
UninstallAppTitle=Деинсталација
UninstallAppFullTitle=%1 — деинсталација

; *** Misc. common
InformationTitle=Информација
ConfirmTitle=Потврда
ErrorTitle=Грешка

; *** SetupLdr messages
SetupLdrStartupMessage=Ово ће инсталирати %1. Желите ли да наставите?
LdrCannotCreateTemp=Није могуће направити привремену датотеку. Инсталација је прекинута
LdrCannotExecTemp=Није могуће покренути датотеку у привременој фасцикли. Инсталација је прекинута
HelpTextNote=

; *** Startup error messages
LastErrorMessage=%1.%n%nГрешка %2: %3
SetupFileMissing=Датотека %1 недостаје у фасцикли инсталације. Отклоните проблем или набавите нову копију програма.
SetupFileCorrupt=Датотеке инсталације су оштећене. Набавите нову копију програма.
SetupFileCorruptOrWrongVer=Датотеке инсталације су оштећене или нису у складу са овом верзијом инсталације. Отклоните проблем или набавите нову копију програма.
InvalidParameter=У командној линији је прослеђен неисправан параметар:%n%n%1
SetupAlreadyRunning=Инсталација је већ покренута.
WindowsVersionNotSupported=Овај програм не подржава верзију система Windows која је покренута на вашем рачунару.
WindowsServicePackRequired=Овај програм захтева %1 Service Pack %2 или новији.
NotOnThisPlatform=Овај програм се неће покренути на систему %1.
OnlyOnThisPlatform=Овај програм мора да се покрене на систему %1.
OnlyOnTheseArchitectures=Овај програм се може инсталирати само на верзијама система Windows намењеним следећим архитектурама процесора:%n%n%1
WinVersionTooLowError=Овај програм захтева %1 верзије %2 или новије.
WinVersionTooHighError=Овај програм се не може инсталирати на %1 верзије %2 или новије.
AdminPrivilegesRequired=Приликом инсталације овог програма морате бити пријављени као администратор.
PowerUserPrivilegesRequired=Приликом инсталације овог програма морате бити пријављени као администратор или као члан групе Power Users.
SetupAppRunningError=Инсталација је открила да је %1 тренутно покренут.%n%nЗатворите све његове покренуте примерке, па кликните на „У реду“ да наставите или на „Откажи“ да изађете.
UninstallAppRunningError=Деинсталација је открила да је %1 тренутно покренут.%n%nЗатворите све његове покренуте примерке, па кликните на „У реду“ да наставите или на „Откажи“ да изађете.

; *** Startup questions
PrivilegesRequiredOverrideTitle=Избор режима инсталације
PrivilegesRequiredOverrideInstruction=Изаберите режим инсталације
PrivilegesRequiredOverrideText1=%1 се може инсталирати за све кориснике (потребна су администраторска права) или само за вас.
PrivilegesRequiredOverrideText2=%1 се може инсталирати само за вас или за све кориснике (потребна су администраторска права).
PrivilegesRequiredOverrideAllUsers=Инсталирај за све кориснике
PrivilegesRequiredOverrideAllUsersRecommended=Инсталирај за све кориснике (препоручује се)
PrivilegesRequiredOverrideCurrentUser=Инсталирај само за мене
PrivilegesRequiredOverrideCurrentUserRecommended=Инсталирај само за мене (препоручује се)

; *** Misc. errors
ErrorCreatingDir=Инсталација није могла да направи фасциклу „%1“
ErrorTooManyFilesInDir=Није могуће направити датотеку у фасцикли „%1“ јер садржи превише датотека

; *** Setup common messages
ExitSetupTitle=Излаз из инсталације
ExitSetupMessage=Инсталација није завршена. Ако сада изађете, програм неће бити инсталиран.%n%nИнсталацију можете поново покренути касније да бисте је завршили.%n%nИзаћи из инсталације?
AboutSetupMenuItem=О инсталацији...
AboutSetupTitle=О инсталацији
AboutSetupMessage=%1 верзија %2%n%3%n%n%1 — почетна страница:%n%4
AboutSetupNote=
TranslatorNote=

; *** Buttons
ButtonBack=< Назад
ButtonNext=Даље >
ButtonInstall=Инсталирај
ButtonOK=У реду
ButtonCancel=Откажи
ButtonYes=Да
ButtonYesToAll=Да за све
ButtonNo=Не
ButtonNoToAll=Не за све
ButtonFinish=Заврши
ButtonBrowse=Прегледај...
ButtonWizardBrowse=Прегледај...
ButtonNewFolder=Нова фасцикла

; *** "Select Language" dialog messages
SelectLanguageTitle=Избор језика инсталације
SelectLanguageLabel=Изаберите језик који ће се користити током инсталације.

; *** Common wizard text
ClickNext=Кликните на „Даље“ да наставите или на „Откажи“ да изађете из инсталације.
BeveledLabel=
BrowseDialogTitle=Избор фасцикле
BrowseDialogLabel=Изаберите фасциклу са списка испод, а затим кликните на „У реду“.
NewFolderName=Нова фасцикла

; *** "Welcome" wizard page
WelcomeLabel1=Добро дошли у чаробњак за инсталацију програма [name]
WelcomeLabel2=Ово ће инсталирати [name/ver] на ваш рачунар.%n%nПрепоручује се да затворите све остале програме пре него што наставите.

; *** "Password" wizard page
WizardPassword=Лозинка
PasswordLabel1=Ова инсталација је заштићена лозинком.
PasswordLabel3=Унесите лозинку, па кликните на „Даље“ да наставите. Лозинке разликују велика и мала слова.
PasswordEditLabel=Лозинка:
IncorrectPassword=Унета лозинка није тачна. Покушајте поново.

; *** "License Agreement" wizard page
WizardLicense=Уговор о лиценци
LicenseLabel=Пажљиво прочитајте следеће важне информације пре него што наставите.
LicenseLabel3=Прочитајте следећи уговор о лиценци. Морате прихватити услове овог уговора да бисте наставили инсталацију.
LicenseAccepted=Прихватам уговор
LicenseNotAccepted=Не прихватам уговор

; *** "Information" wizard pages
WizardInfoBefore=Информације
InfoBeforeLabel=Пажљиво прочитајте следеће важне информације пре него што наставите.
InfoBeforeClickLabel=Када будете спремни да наставите инсталацију, кликните на „Даље“.
WizardInfoAfter=Информације
InfoAfterLabel=Пажљиво прочитајте следеће важне информације пре него што наставите.
InfoAfterClickLabel=Када будете спремни да наставите инсталацију, кликните на „Даље“.

; *** "User Information" wizard page
WizardUserInfo=Подаци о кориснику
UserInfoDesc=Унесите своје податке.
UserInfoName=Корисничко име:
UserInfoOrg=Организација:
UserInfoSerial=Серијски број:
UserInfoNameRequired=Морате унети име.

; *** "Select Destination Location" wizard page
WizardSelectDir=Избор одредишне локације
SelectDirDesc=Где треба инсталирати програм [name]?
SelectDirLabel3=Инсталација ће инсталирати програм [name] у следећу фасциклу.
SelectDirBrowseLabel=Кликните на „Даље“ да наставите. Ако желите да изаберете другу фасциклу, кликните на „Прегледај“.
DiskSpaceGBLabel=Потребно је најмање [gb] GB слободног простора на диску.
DiskSpaceMBLabel=Потребно је најмање [mb] MB слободног простора на диску.
CannotInstallToNetworkDrive=Инсталација не може да инсталира на мрежни диск.
CannotInstallToUNCPath=Инсталација не може да инсталира на UNC путању.
InvalidPath=Морате унети пуну путању са ознаком диска, на пример:%n%nC:\APP%n%nили UNC путању у облику:%n%n\\server\share
InvalidDrive=Изабрани диск или UNC дељена фасцикла не постоји или није доступна. Изаберите други.
DiskSpaceWarningTitle=Нема довољно простора на диску
DiskSpaceWarning=Инсталација захтева најмање %1 KB слободног простора, а изабрани диск има само %2 KB.%n%nЖелите ли ипак да наставите?
DirNameTooLong=Назив фасцикле или путања је предугачка.
InvalidDirName=Назив фасцикле није исправан.
BadDirName32=Називи фасцикли не могу садржати ниједан од следећих знакова:%n%n%1
DirExistsTitle=Фасцикла већ постоји
DirExists=Фасцикла:%n%n%1%n%nвећ постоји. Желите ли ипак да инсталирате у ту фасциклу?
DirDoesntExistTitle=Фасцикла не постоји
DirDoesntExist=Фасцикла:%n%n%1%n%nне постоји. Желите ли да се направи?

; *** "Select Components" wizard page
WizardSelectComponents=Избор компоненти
SelectComponentsDesc=Које компоненте треба инсталирати?
SelectComponentsLabel2=Означите компоненте које желите да инсталирате, а уклоните ознаку са оних које не желите. Кликните на „Даље“ када будете спремни да наставите.
FullInstallation=Потпуна инсталација
; if possible don't translate 'Compact' as 'Minimal' (I mean 'Minimal' in your language)
CompactInstallation=Компактна инсталација
CustomInstallation=Прилагођена инсталација
NoUninstallWarningTitle=Компоненте већ постоје
NoUninstallWarning=Инсталација је открила да су следеће компоненте већ инсталиране на вашем рачунару:%n%n%1%n%nАко уклоните ознаку са њих, оне неће бити деинсталиране.%n%nЖелите ли ипак да наставите?
ComponentSize1=%1 KB
ComponentSize2=%1 MB
ComponentsDiskSpaceGBLabel=Тренутни избор захтева најмање [gb] GB простора на диску.
ComponentsDiskSpaceMBLabel=Тренутни избор захтева најмање [mb] MB простора на диску.

; *** "Select Additional Tasks" wizard page
WizardSelectTasks=Избор додатних задатака
SelectTasksDesc=Које додатне задатке треба извршити?
SelectTasksLabel2=Изаберите додатне задатке које инсталација треба да изврши током инсталирања програма [name], па кликните на „Даље“.

; *** "Select Start Menu Folder" wizard page
WizardSelectProgramGroup=Избор фасцикле у менију „Старт“
SelectStartMenuFolderDesc=Где инсталација треба да постави пречице програма?
SelectStartMenuFolderLabel3=Инсталација ће направити пречице програма у следећој фасцикли менија „Старт“.
SelectStartMenuFolderBrowseLabel=Кликните на „Даље“ да наставите. Ако желите да изаберете другу фасциклу, кликните на „Прегледај“.
MustEnterGroupName=Морате унети назив фасцикле.
GroupNameTooLong=Назив фасцикле или путања је предугачка.
InvalidGroupName=Назив фасцикле није исправан.
BadGroupName=Назив фасцикле не може садржати ниједан од следећих знакова:%n%n%1
NoProgramGroupCheck2=Не прави фасциклу у менију „Старт“

; *** "Ready to Install" wizard page
WizardReady=Спремно за инсталацију
ReadyLabel1=Инсталација је спремна да започне инсталирање програма [name] на ваш рачунар.
ReadyLabel2a=Кликните на „Инсталирај“ да наставите инсталацију или на „Назад“ ако желите да прегледате или промените подешавања.
ReadyLabel2b=Кликните на „Инсталирај“ да наставите инсталацију.
ReadyMemoUserInfo=Подаци о кориснику:
ReadyMemoDir=Одредишна локација:
ReadyMemoType=Врста инсталације:
ReadyMemoComponents=Изабране компоненте:
ReadyMemoGroup=Фасцикла у менију „Старт“:
ReadyMemoTasks=Додатни задаци:

; *** TDownloadWizardPage wizard page and DownloadTemporaryFile
DownloadingLabel2=Преузимање датотека...
ButtonStopDownload=Заустави преузимање
StopDownload=Да ли сте сигурни да желите да зауставите преузимање?
ErrorDownloadAborted=Преузимање је прекинуто
ErrorDownloadFailed=Преузимање није успело: %1 %2
ErrorDownloadSizeFailed=Добављање величине није успело: %1 %2
ErrorProgress=Неисправан напредак: %1 од %2
ErrorFileSize=Неисправна величина датотеке: очекивано %1, пронађено %2

; *** TExtractionWizardPage wizard page and ExtractArchive
ExtractingLabel=Распакивање датотека...
ButtonStopExtraction=Заустави распакивање
StopExtraction=Да ли сте сигурни да желите да зауставите распакивање?
ErrorExtractionAborted=Распакивање је прекинуто
ErrorExtractionFailed=Распакивање није успело: %1

; *** Archive extraction failure details
ArchiveIncorrectPassword=Лозинка није тачна
ArchiveIsCorrupted=Архива је оштећена
ArchiveUnsupportedFormat=Формат архиве није подржан

; *** "Preparing to Install" wizard page
WizardPreparing=Припрема за инсталацију
PreparingDesc=Инсталација се припрема да инсталира програм [name] на ваш рачунар.
PreviousInstallNotCompleted=Инсталација или уклањање претходног програма није завршено. Морате поново покренути рачунар да бисте завршили ту инсталацију.%n%nНакон поновног покретања рачунара поново покрените инсталацију да бисте завршили инсталирање програма [name].
CannotContinue=Инсталација не може да настави. Кликните на „Откажи“ да изађете.
ApplicationsFound=Следећи програми користе датотеке које инсталација треба да ажурира. Препоручује се да дозволите инсталацији да аутоматски затвори те програме.
ApplicationsFound2=Следећи програми користе датотеке које инсталација треба да ажурира. Препоручује се да дозволите инсталацији да аутоматски затвори те програме. Када се инсталирање заврши, инсталација ће покушати да поново покрене те програме.
CloseApplications=Аутоматски затвори програме
DontCloseApplications=Не затварај програме
ErrorCloseApplications=Инсталација није могла аутоматски да затвори све програме. Препоручује се да затворите све програме који користе датотеке које инсталација треба да ажурира, пре него што наставите.
PrepareToInstallNeedsRestart=Инсталација мора поново да покрене рачунар. Након поновног покретања рачунара поново покрените инсталацију да бисте завршили инсталирање програма [name].%n%nЖелите ли да поново покренете рачунар сада?

; *** "Installing" wizard page
WizardInstalling=Инсталирање
InstallingLabel=Сачекајте док инсталација инсталира програм [name] на ваш рачунар.

; *** "Setup Completed" wizard page
FinishedHeadingLabel=Завршетак чаробњака за инсталацију програма [name]
FinishedLabelNoIcons=Инсталација је завршила инсталирање програма [name] на ваш рачунар.
FinishedLabel=Инсталација је завршила инсталирање програма [name] на ваш рачунар. Програм можете покренути помоћу инсталираних пречица.
ClickFinish=Кликните на „Заврши“ да изађете из инсталације.
FinishedRestartLabel=Да би инсталирање програма [name] било завршено, инсталација мора поново да покрене рачунар. Желите ли да то урадите сада?
FinishedRestartMessage=Да би инсталирање програма [name] било завршено, инсталација мора поново да покрене рачунар.%n%nЖелите ли да то урадите сада?
ShowReadmeCheck=Да, желим да прегледам датотеку README
YesRadio=Да, поново покрени рачунар сада
NoRadio=Не, рачунар ћу поново покренути касније
; used for example as 'Run MyProg.exe'
RunEntryExec=Покрени %1
; used for example as 'View Readme.txt'
RunEntryShellExec=Прегледај %1

; *** "Setup Needs the Next Disk" stuff
ChangeDiskTitle=Инсталација тражи следећи диск
SelectDiskLabel2=Убаците диск %1 и кликните на „У реду“.%n%nАко се датотеке са овог диска налазе у другој фасцикли од приказане испод, унесите исправну путању или кликните на „Прегледај“.
PathLabel=Путања:
FileNotInDir2=Датотека „%1“ није пронађена у „%2“. Убаците исправан диск или изаберите другу фасциклу.
SelectDirectoryLabel=Наведите локацију следећег диска.

; *** Installation phase messages
SetupAborted=Инсталација није завршена.%n%nОтклоните проблем и поново покрените инсталацију.
AbortRetryIgnoreSelectAction=Изаберите радњу
AbortRetryIgnoreRetry=Покушај поново
AbortRetryIgnoreIgnore=Занемари грешку и настави
AbortRetryIgnoreCancel=Откажи инсталацију
RetryCancelSelectAction=Изаберите радњу
RetryCancelRetry=Покушај поново
RetryCancelCancel=Откажи

; *** Installation status messages
StatusClosingApplications=Затварање програма...
StatusCreateDirs=Прављење фасцикли...
StatusExtractFiles=Распакивање датотека...
StatusDownloadFiles=Преузимање датотека...
StatusCreateIcons=Прављење пречица...
StatusCreateIniEntries=Прављење INI уноса...
StatusCreateRegistryEntries=Прављење уноса у регистру...
StatusRegisterFiles=Регистровање датотека...
StatusSavingUninstall=Чување података за деинсталацију...
StatusRunProgram=Завршавање инсталације...
StatusRestartingApplications=Поновно покретање програма...
StatusRollback=Поништавање промена...

; *** Misc. errors
ErrorInternal2=Интерна грешка: %1
ErrorFunctionFailedNoCode=%1 није успело
ErrorFunctionFailed=%1 није успело; код %2
ErrorFunctionFailedWithMessage=%1 није успело; код %2.%n%3
ErrorExecutingProgram=Није могуће покренути датотеку:%n%1

; *** Registry errors
ErrorRegOpenKey=Грешка при отварању кључа регистра:%n%1\%2
ErrorRegCreateKey=Грешка при прављењу кључа регистра:%n%1\%2
ErrorRegWriteKey=Грешка при уписивању у кључ регистра:%n%1\%2

; *** INI errors
ErrorIniEntry=Грешка при прављењу INI уноса у датотеци „%1“.

; *** File copying errors
FileAbortRetryIgnoreSkipNotRecommended=Прескочи ову датотеку (не препоручује се)
FileAbortRetryIgnoreIgnoreNotRecommended=Занемари грешку и настави (не препоручује се)
SourceIsCorrupted=Изворна датотека је оштећена
SourceDoesntExist=Изворна датотека „%1“ не постоји
SourceVerificationFailed=Провера изворне датотеке није успела: %1
VerificationSignatureDoesntExist=Датотека потписа „%1“ не постоји
VerificationSignatureInvalid=Датотека потписа „%1“ је неисправна
VerificationKeyNotFound=Датотека потписа „%1“ користи непознат кључ
VerificationFileNameIncorrect=Назив датотеке није исправан
VerificationFileTagIncorrect=Ознака датотеке није исправна
VerificationFileSizeIncorrect=Величина датотеке није исправна
VerificationFileHashIncorrect=Хеш датотеке није исправан
ExistingFileReadOnly2=Постојећа датотека није могла бити замењена јер је означена као „само за читање“.
ExistingFileReadOnlyRetry=Уклони ознаку „само за читање“ и покушај поново
ExistingFileReadOnlyKeepExisting=Задржи постојећу датотеку
ErrorReadingExistingDest=Дошло је до грешке при покушају читања постојеће датотеке:
FileExistsSelectAction=Изаберите радњу
FileExists2=Датотека већ постоји.
FileExistsOverwriteExisting=Замени постојећу датотеку
FileExistsKeepExisting=Задржи постојећу датотеку
FileExistsOverwriteOrKeepAll=Уради то и за следеће сукобе
ExistingFileNewerSelectAction=Изаберите радњу
ExistingFileNewer2=Постојећа датотека је новија од оне коју инсталација покушава да инсталира.
ExistingFileNewerOverwriteExisting=Замени постојећу датотеку
ExistingFileNewerKeepExisting=Задржи постојећу датотеку (препоручује се)
ExistingFileNewerOverwriteOrKeepAll=Уради то и за следеће сукобе
ErrorChangingAttr=Дошло је до грешке при покушају промене атрибута постојеће датотеке:
ErrorCreatingTemp=Дошло је до грешке при покушају прављења датотеке у одредишној фасцикли:
ErrorReadingSource=Дошло је до грешке при покушају читања изворне датотеке:
ErrorCopying=Дошло је до грешке при покушају копирања датотеке:
ErrorDownloading=Дошло је до грешке при покушају преузимања датотеке:
ErrorExtracting=Дошло је до грешке при покушају распакивања архиве:
ErrorReplacingExistingFile=Дошло је до грешке при покушају замене постојеће датотеке:
ErrorRestartReplace=RestartReplace није успео:
ErrorRenamingTemp=Дошло је до грешке при покушају промене назива датотеке у одредишној фасцикли:
ErrorRegisterServer=Није могуће регистровати DLL/OCX: %1
ErrorRegSvr32Failed=RegSvr32 није успео, излазни код %1
ErrorRegisterTypeLib=Није могуће регистровати библиотеку типова: %1

; *** Uninstall display name markings
; used for example as 'My Program (32-bit)'
UninstallDisplayNameMark=%1 (%2)
; used for example as 'My Program (32-bit, All users)'
UninstallDisplayNameMarks=%1 (%2, %3)
UninstallDisplayNameMark32Bit=32-битна
UninstallDisplayNameMark64Bit=64-битна
UninstallDisplayNameMarkAllUsers=Сви корисници
UninstallDisplayNameMarkCurrentUser=Тренутни корисник

; *** Post-installation errors
ErrorOpeningReadme=Дошло је до грешке при покушају отварања датотеке README.
ErrorRestartingComputer=Инсталација није могла да поново покрене рачунар. Урадите то ручно.

; *** Uninstaller messages
UninstallNotFound=Датотека „%1“ не постоји. Деинсталација није могућа.
UninstallOpenError=Датотека „%1“ није могла да се отвори. Деинсталација није могућа
UninstallUnsupportedVer=Датотека дневника деинсталације „%1“ је у формату који ова верзија програма за деинсталацију не препознаје. Деинсталација није могућа
UninstallUnknownEntry=У дневнику деинсталације пронађен је непознат унос (%1)
ConfirmUninstall=Да ли сте сигурни да желите потпуно да уклоните %1 и све његове компоненте?
UninstallOnlyOnWin64=Ова инсталација се може деинсталирати само на 64-битном систему Windows.
OnlyAdminCanUninstall=Ову инсталацију може деинсталирати само корисник са администраторским правима.
UninstallStatusLabel=Сачекајте док се %1 уклања са вашег рачунара.
UninstalledAll=%1 је успешно уклоњен са вашег рачунара.
UninstalledMost=Деинсталација програма %1 је завршена.%n%nНеке ставке нису могле бити уклоњене. Можете их уклонити ручно.
UninstalledAndNeedsRestart=Да би деинсталација програма %1 била завршена, рачунар мора поново да се покрене.%n%nЖелите ли да то урадите сада?
UninstallDataCorrupted=Датотека „%1“ је оштећена. Деинсталација није могућа

; *** Uninstallation phase messages
ConfirmDeleteSharedFileTitle=Уклонити дељену датотеку?
ConfirmDeleteSharedFile2=Систем показује да ниједан програм више не користи следећу дељену датотеку. Желите ли да деинсталација уклони ту дељену датотеку?%n%nАко неки програми још увек користе ту датотеку, а она буде уклоњена, ти програми можда неће радити исправно. Ако нисте сигурни, изаберите „Не“. Остављање датотеке на систему не може да нашкоди.
SharedFileNameLabel=Назив датотеке:
SharedFileLocationLabel=Локација:
WizardUninstalling=Стање деинсталације
StatusUninstalling=Деинсталирање програма %1...

; *** Shutdown block reasons
ShutdownBlockReasonInstallingApp=Инсталирање програма %1.
ShutdownBlockReasonUninstallingApp=Деинсталирање програма %1.

; The custom messages below aren't used by Setup itself, but if you make
; use of them in your scripts, you'll want to translate them.

[CustomMessages]

NameAndVersion=%1 верзија %2
AdditionalIcons=Додатне пречице:
CreateDesktopIcon=Направи пречицу на радној површини
CreateQuickLaunchIcon=Направи пречицу за брзо покретање
ProgramOnTheWeb=%1 на вебу
UninstallProgram=Деинсталирај %1
LaunchProgram=Покрени %1
AssocFileExtension=Повежи %1 са екстензијом датотеке %2
AssocingFileExtension=Повезивање програма %1 са екстензијом датотеке %2...
AutoStartProgramGroupDescription=Покретање са системом:
AutoStartProgram=Аутоматски покрени %1
AddonHostProgramNotFound=%1 није пронађен у изабраној фасцикли.%n%nЖелите ли ипак да наставите?

; The messages that are Lang Switcher's own (the sample is English.isl).
LocalCertReady=Инсталација ће на овом рачунару направити сертификат и њиме потписати Lang Switcher, како би програм радио и у прозорима програма покренутих као администратор.
LocalCertFailed=Lang Switcher је инсталиран, али неће радити у прозорима програма покренутих као администратор: Windows није прихватио сертификат направљен на овом рачунару. Детаље потражите у дневнику инсталације.
RemoveSettingsQuestion=Желите ли да уклоните и датотеку подешавања програма Lang Switcher?
RemoveSettingsKeep=Изаберите „Не“ да задржите подешавања за поновну инсталацију.
AuthorSlogan=програми направљени од срца
