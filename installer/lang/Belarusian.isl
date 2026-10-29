; Lang Switcher -- the messages of the installer in Belarusian (stage E108, group A).
;
; Inno Setup has no official Belarusian translation (inno_official = false in res\lang\languages.toml),
; so this file carries the whole of it: [LangOptions], every message of the compiler:Default.isl
; (6.5.0+), and the messages of the installer that are Lang Switcher's own. It is read after
; compiler:Default.isl as the second MessagesFile of the "belarusian" entry of [Languages] in
; ..\LangSwitcher.iss, and so replaces the English text name for name.
; English.isl is the sample the own messages translate; the glossary of the language is
; l10n\glossary\be.tsv of the control contour. The name of the program is not translated.
; As in Default.isl: a message that has no full stop there has none here (Setup adds its own).
; No accelerators (&) in this file, as in Tamil.isl (RULES 16): the names of the buttons inside
; a message stand in the guillemets of the language and match the captions of the buttons.
; Belarusian letters: ў і ' (apostrophe); no и, щ, ъ. The word "у" turns into "ў" after a vowel.

[LangOptions]
LanguageName=Беларуская
LanguageID=$0423
LanguageCodePage=0
;DialogFontName=
;DialogFontSize=9
;DialogFontBaseScaleWidth=7
;DialogFontBaseScaleHeight=15
;WelcomeFontName=Segoe UI
;WelcomeFontSize=14

[Messages]

; *** Application titles
SetupAppTitle=Усталёўка
SetupWindowTitle=Усталёўка - %1
UninstallAppTitle=Выдаленне
UninstallAppFullTitle=Выдаленне %1

; *** Misc. common
InformationTitle=Інфармацыя
ConfirmTitle=Пацвярджэнне
ErrorTitle=Памылка

; *** SetupLdr messages
SetupLdrStartupMessage=Будзе ўсталявана %1. Працягнуць?
LdrCannotCreateTemp=Не ўдалося стварыць часовы файл. Усталёўка перапынена
LdrCannotExecTemp=Не ўдалося запусціць файл з часовай папкі. Усталёўка перапынена
HelpTextNote=

; *** Startup error messages
LastErrorMessage=%1.%n%nПамылка %2: %3
SetupFileMissing=Файл %1 адсутнічае ў папцы ўсталёўкі. Выправіце праблему або атрымайце новую копію праграмы.
SetupFileCorrupt=Файлы ўсталёўкі пашкоджаны. Атрымайце новую копію праграмы.
SetupFileCorruptOrWrongVer=Файлы ўсталёўкі пашкоджаны або несумяшчальныя з гэтай версіяй праграмы ўсталёўкі. Выправіце праблему або атрымайце новую копію праграмы.
InvalidParameter=У камандным радку перададзены недапушчальны параметр:%n%n%1
SetupAlreadyRunning=Усталёўка ўжо запушчана.
WindowsVersionNotSupported=Гэта праграма не падтрымлівае версію Windows, пад кіраваннем якой працуе ваш камп'ютар.
WindowsServicePackRequired=Для гэтай праграмы патрабуецца %1 з пакетам абнаўлення %2 або навейшым.
NotOnThisPlatform=Гэта праграма не будзе працаваць у %1.
OnlyOnThisPlatform=Гэтую праграму трэба запускаць у %1.
OnlyOnTheseArchitectures=Гэтую праграму можна ўсталяваць толькі ў тых версіях Windows, якія прызначаны для наступных архітэктур працэсараў:%n%n%1
WinVersionTooLowError=Для гэтай праграмы патрабуецца %1 версіі %2 або навейшай.
WinVersionTooHighError=Гэтую праграму нельга ўсталяваць у %1 версіі %2 або навейшай.
AdminPrivilegesRequired=Для ўсталёўкі гэтай праграмы трэба ўвайсці ў сістэму як адміністратар.
PowerUserPrivilegesRequired=Для ўсталёўкі гэтай праграмы трэба ўвайсці ў сістэму як адміністратар або як член групы Power Users.
SetupAppRunningError=Усталёўка выявіла, што %1 зараз запушчана.%n%nЗакрыйце ўсе яе экзэмпляры, а потым націсніце «ОК», каб працягнуць, або «Скасаваць», каб выйсці.
UninstallAppRunningError=Выдаленне выявіла, што %1 зараз запушчана.%n%nЗакрыйце ўсе яе экзэмпляры, а потым націсніце «ОК», каб працягнуць, або «Скасаваць», каб выйсці.

; *** Startup questions
PrivilegesRequiredOverrideTitle=Выберыце рэжым усталёўкі
PrivilegesRequiredOverrideInstruction=Выберыце рэжым усталёўкі
PrivilegesRequiredOverrideText1=%1 можна ўсталяваць для ўсіх карыстальнікаў (патрэбныя правы адміністратара) або толькі для вас.
PrivilegesRequiredOverrideText2=%1 можна ўсталяваць толькі для вас або для ўсіх карыстальнікаў (патрэбныя правы адміністратара).
PrivilegesRequiredOverrideAllUsers=Усталяваць для ўсіх карыстальнікаў
PrivilegesRequiredOverrideAllUsersRecommended=Усталяваць для ўсіх карыстальнікаў (рэкамендуецца)
PrivilegesRequiredOverrideCurrentUser=Усталяваць толькі для мяне
PrivilegesRequiredOverrideCurrentUserRecommended=Усталяваць толькі для мяне (рэкамендуецца)

; *** Misc. errors
ErrorCreatingDir=Не ўдалося стварыць папку "%1"
ErrorTooManyFilesInDir=Немагчыма стварыць файл у папцы "%1", бо ў ёй занадта шмат файлаў

; *** Setup common messages
ExitSetupTitle=Выхад з усталёўкі
ExitSetupMessage=Усталёўка не завершана. Калі вы выйдзеце цяпер, праграма не будзе ўсталявана.%n%nВы можаце запусціць усталёўку зноў пазней, каб завяршыць яе.%n%nВыйсці з усталёўкі?
AboutSetupMenuItem=Аб усталёўцы...
AboutSetupTitle=Аб усталёўцы
AboutSetupMessage=%1, версія %2%n%3%n%nДамашняя старонка %1:%n%4
AboutSetupNote=
TranslatorNote=

; *** Buttons
ButtonBack=< Назад
ButtonNext=Далей >
ButtonInstall=Усталяваць
ButtonOK=ОК
ButtonCancel=Скасаваць
ButtonYes=Так
ButtonYesToAll=Так для ўсіх
ButtonNo=Не
ButtonNoToAll=Не для ўсіх
ButtonFinish=Завяршыць
ButtonBrowse=Агляд...
ButtonWizardBrowse=Агляд...
ButtonNewFolder=Стварыць папку

; *** "Select Language" dialog messages
SelectLanguageTitle=Выбар мовы ўсталёўкі
SelectLanguageLabel=Выберыце мову, якая будзе выкарыстоўвацца падчас усталёўкі.

; *** Common wizard text
ClickNext=Націсніце «Далей», каб працягнуць, або «Скасаваць», каб выйсці з усталёўкі.
BeveledLabel=
BrowseDialogTitle=Агляд папак
BrowseDialogLabel=Выберыце папку са спісу ніжэй, а потым націсніце «ОК».
NewFolderName=Новая папка

; *** "Welcome" wizard page
WelcomeLabel1=Вас вітае майстар усталёўкі [name]
WelcomeLabel2=Майстар усталюе [name/ver] на ваш камп'ютар.%n%nПерад тым як працягнуць, рэкамендуецца закрыць усе астатнія праграмы.

; *** "Password" wizard page
WizardPassword=Пароль
PasswordLabel1=Гэта ўсталёўка абаронена паролем.
PasswordLabel3=Увядзіце пароль і націсніце «Далей», каб працягнуць. Пароль адрознівае вялікія і малыя літары.
PasswordEditLabel=Пароль:
IncorrectPassword=Уведзены пароль няправільны. Паспрабуйце яшчэ раз.

; *** "License Agreement" wizard page
WizardLicense=Ліцэнзійнае пагадненне
LicenseLabel=Перад тым як працягнуць, прачытайце наступную важную інфармацыю.
LicenseLabel3=Прачытайце наступнае ліцэнзійнае пагадненне. Каб працягнуць усталёўку, вы павінны прыняць умовы гэтага пагаднення.
LicenseAccepted=Я прымаю ўмовы пагаднення
LicenseNotAccepted=Я не прымаю ўмоў пагаднення

; *** "Information" wizard pages
WizardInfoBefore=Інфармацыя
InfoBeforeLabel=Перад тым як працягнуць, прачытайце наступную важную інфармацыю.
InfoBeforeClickLabel=Калі будзеце гатовыя працягнуць усталёўку, націсніце «Далей».
WizardInfoAfter=Інфармацыя
InfoAfterLabel=Перад тым як працягнуць, прачытайце наступную важную інфармацыю.
InfoAfterClickLabel=Калі будзеце гатовыя працягнуць усталёўку, націсніце «Далей».

; *** "User Information" wizard page
WizardUserInfo=Звесткі пра карыстальніка
UserInfoDesc=Увядзіце вашы звесткі.
UserInfoName=Імя карыстальніка:
UserInfoOrg=Арганізацыя:
UserInfoSerial=Серыйны нумар:
UserInfoNameRequired=Вы павінны ўвесці імя.

; *** "Select Destination Location" wizard page
WizardSelectDir=Выбар папкі прызначэння
SelectDirDesc=Куды трэба ўсталяваць [name]?
SelectDirLabel3=Майстар усталюе [name] у наступную папку.
SelectDirBrowseLabel=Каб працягнуць, націсніце «Далей». Каб выбраць іншую папку, націсніце «Агляд».
DiskSpaceGBLabel=Патрабуецца не менш за [gb] ГБ вольнага месца на дыску.
DiskSpaceMBLabel=Патрабуецца не менш за [mb] МБ вольнага месца на дыску.
CannotInstallToNetworkDrive=Усталёўку на сеткавы дыск выканаць нельга.
CannotInstallToUNCPath=Усталёўку ў шлях UNC выканаць нельга.
InvalidPath=Трэба ўвесці поўны шлях з літарай дыска, напрыклад:%n%nC:\APP%n%nабо шлях UNC у выглядзе:%n%n\\server\share
InvalidDrive=Выбраны дыск або сеткавы рэсурс UNC не існуе або недаступны. Выберыце іншы.
DiskSpaceWarningTitle=Не хапае месца на дыску
DiskSpaceWarning=Для ўсталёўкі патрабуецца не менш за %1 КБ вольнага месца, але на выбраным дыску даступна толькі %2 КБ.%n%nПрацягнуць усё роўна?
DirNameTooLong=Назва папкі або шлях занадта доўгія.
InvalidDirName=Недапушчальная назва папкі.
BadDirName32=Назвы папак не могуць змяшчаць наступныя сімвалы:%n%n%1
DirExistsTitle=Папка існуе
DirExists=Папка:%n%n%1%n%nужо існуе. Усталяваць у гэту папку ўсё роўна?
DirDoesntExistTitle=Папкі няма
DirDoesntExist=Папкі:%n%n%1%n%nне існуе. Стварыць яе?

; *** "Select Components" wizard page
WizardSelectComponents=Выбар кампанентаў
SelectComponentsDesc=Якія кампаненты трэба ўсталяваць?
SelectComponentsLabel2=Адзначце кампаненты, якія хочаце ўсталяваць, і зніміце адзнакі з тых, якія ўсталёўваць не трэба. Калі будзеце гатовыя, націсніце «Далей».
FullInstallation=Поўная ўсталёўка
; if possible don't translate 'Compact' as 'Minimal' (I mean 'Minimal' in your language)
CompactInstallation=Кампактная ўсталёўка
CustomInstallation=Выбарачная ўсталёўка
NoUninstallWarningTitle=Кампаненты ўжо існуюць
NoUninstallWarning=Усталёўка выявіла, што на вашым камп'ютары ўжо ўсталяваны наступныя кампаненты:%n%n%1%n%nЗняцце адзнакі з гэтых кампанентаў іх не выдаліць.%n%nПрацягнуць усё роўна?
ComponentSize1=%1 КБ
ComponentSize2=%1 МБ
ComponentsDiskSpaceGBLabel=Для бягучага выбару патрабуецца не менш за [gb] ГБ месца на дыску.
ComponentsDiskSpaceMBLabel=Для бягучага выбару патрабуецца не менш за [mb] МБ месца на дыску.

; *** "Select Additional Tasks" wizard page
WizardSelectTasks=Выбар дадатковых заданняў
SelectTasksDesc=Якія дадатковыя заданні трэба выканаць?
SelectTasksLabel2=Адзначце дадатковыя заданні, якія майстар павінен выканаць падчас усталёўкі [name], а потым націсніце «Далей».

; *** "Select Start Menu Folder" wizard page
WizardSelectProgramGroup=Выбар папкі ў меню «Пуск»
SelectStartMenuFolderDesc=Куды майстар павінен змясціць ярлыкі праграмы?
SelectStartMenuFolderLabel3=Майстар створыць ярлыкі праграмы ў наступнай папцы меню «Пуск».
SelectStartMenuFolderBrowseLabel=Каб працягнуць, націсніце «Далей». Каб выбраць іншую папку, націсніце «Агляд».
MustEnterGroupName=Трэба ўвесці назву папкі.
GroupNameTooLong=Назва папкі або шлях занадта доўгія.
InvalidGroupName=Недапушчальная назва папкі.
BadGroupName=Назва папкі не можа змяшчаць наступныя сімвалы:%n%n%1
NoProgramGroupCheck2=Не ствараць папку ў меню «Пуск»

; *** "Ready to Install" wizard page
WizardReady=Усё гатова да ўсталёўкі
ReadyLabel1=Майстар гатовы пачаць усталёўку [name] на ваш камп'ютар.
ReadyLabel2a=Націсніце «Усталяваць», каб працягнуць, або «Назад», калі хочаце праглядзець ці змяніць налады.
ReadyLabel2b=Націсніце «Усталяваць», каб працягнуць усталёўку.
ReadyMemoUserInfo=Звесткі пра карыстальніка:
ReadyMemoDir=Папка прызначэння:
ReadyMemoType=Тып усталёўкі:
ReadyMemoComponents=Выбраныя кампаненты:
ReadyMemoGroup=Папка ў меню «Пуск»:
ReadyMemoTasks=Дадатковыя заданні:

; *** TDownloadWizardPage wizard page and DownloadTemporaryFile
DownloadingLabel2=Загрузка файлаў...
ButtonStopDownload=Спыніць загрузку
StopDownload=Вы ўпэўнены, што хочаце спыніць загрузку?
ErrorDownloadAborted=Загрузка перапынена
ErrorDownloadFailed=Не ўдалося выканаць загрузку: %1 %2
ErrorDownloadSizeFailed=Не ўдалося атрымаць памер: %1 %2
ErrorProgress=Недапушчальны ход выканання: %1 з %2
ErrorFileSize=Недапушчальны памер файла: чакалася %1, атрымана %2

; *** TExtractionWizardPage wizard page and ExtractArchive
ExtractingLabel=Распакоўка файлаў...
ButtonStopExtraction=Спыніць распакоўку
StopExtraction=Вы ўпэўнены, што хочаце спыніць распакоўку?
ErrorExtractionAborted=Распакоўка перапынена
ErrorExtractionFailed=Не ўдалося выканаць распакоўку: %1

; *** Archive extraction failure details
ArchiveIncorrectPassword=Пароль няправільны
ArchiveIsCorrupted=Архіў пашкоджаны
ArchiveUnsupportedFormat=Фармат архіва не падтрымліваецца

; *** "Preparing to Install" wizard page
WizardPreparing=Падрыхтоўка да ўсталёўкі
PreparingDesc=Майстар рыхтуецца ўсталяваць [name] на ваш камп'ютар.
PreviousInstallNotCompleted=Усталёўка або выдаленне папярэдняй праграмы не завершана. Каб завяршыць яе, трэба перазапусціць камп'ютар.%n%nПасля перазапуску камп'ютара зноў запусціце ўсталёўку, каб завяршыць усталёўку [name].
CannotContinue=Працягнуць усталёўку немагчыма. Націсніце «Скасаваць», каб выйсці.
ApplicationsFound=Наступныя праграмы выкарыстоўваюць файлы, якія майстар павінен абнавіць. Рэкамендуецца дазволіць майстру закрыць гэтыя праграмы аўтаматычна.
ApplicationsFound2=Наступныя праграмы выкарыстоўваюць файлы, якія майстар павінен абнавіць. Рэкамендуецца дазволіць майстру закрыць гэтыя праграмы аўтаматычна. Пасля завяршэння ўсталёўкі майстар паспрабуе запусціць гэтыя праграмы зноў.
CloseApplications=Закрыць праграмы аўтаматычна
DontCloseApplications=Не закрываць праграмы
ErrorCloseApplications=Майстру не ўдалося аўтаматычна закрыць усе праграмы. Перад працягам рэкамендуецца закрыць усе праграмы, якія выкарыстоўваюць файлы, што павінен абнавіць майстар.
PrepareToInstallNeedsRestart=Майстру неабходна перазапусціць камп'ютар. Пасля перазапуску зноў запусціце ўсталёўку, каб завяршыць усталёўку [name].%n%nПеразапусціць камп'ютар цяпер?

; *** "Installing" wizard page
WizardInstalling=Усталёўка
InstallingLabel=Пачакайце, пакуль майстар усталюе [name] на ваш камп'ютар.

; *** "Setup Completed" wizard page
FinishedHeadingLabel=Завяршэнне работы майстра ўсталёўкі [name]
FinishedLabelNoIcons=Майстар завяршыў усталёўку [name] на ваш камп'ютар.
FinishedLabel=Майстар завяршыў усталёўку [name] на ваш камп'ютар. Праграму можна запусціць праз усталяваныя ярлыкі.
ClickFinish=Націсніце «Завяршыць», каб выйсці з усталёўкі.
FinishedRestartLabel=Каб завяршыць усталёўку [name], майстар павінен перазапусціць ваш камп'ютар. Перазапусціць цяпер?
FinishedRestartMessage=Каб завяршыць усталёўку [name], майстар павінен перазапусціць ваш камп'ютар.%n%nПеразапусціць цяпер?
ShowReadmeCheck=Так, я хачу прагледзець файл README
YesRadio=Так, перазапусціць камп'ютар цяпер
NoRadio=Не, я перазапушчу камп'ютар пазней
; used for example as 'Run MyProg.exe'
RunEntryExec=Запусціць %1
; used for example as 'View Readme.txt'
RunEntryShellExec=Прагледзець %1

; *** "Setup Needs the Next Disk" stuff
ChangeDiskTitle=Майстру патрэбны наступны дыск
SelectDiskLabel2=Устаўце дыск %1 і націсніце «ОК».%n%nКалі файлы з гэтага дыска знаходзяцца ў іншай папцы, чым паказана ніжэй, увядзіце правільны шлях або націсніце «Агляд».
PathLabel=Шлях:
FileNotInDir2=Файл "%1" не знойдзены ў "%2". Устаўце правільны дыск або выберыце іншую папку.
SelectDirectoryLabel=Пакажыце размяшчэнне наступнага дыска.

; *** Installation phase messages
SetupAborted=Усталёўка не завершана.%n%nВыправіце праблему і запусціце ўсталёўку зноў.
AbortRetryIgnoreSelectAction=Выберыце дзеянне
AbortRetryIgnoreRetry=Паўтарыць спробу
AbortRetryIgnoreIgnore=Ігнараваць памылку і працягнуць
AbortRetryIgnoreCancel=Скасаваць усталёўку
RetryCancelSelectAction=Выберыце дзеянне
RetryCancelRetry=Паўтарыць спробу
RetryCancelCancel=Скасаваць

; *** Installation status messages
StatusClosingApplications=Закрыццё праграм...
StatusCreateDirs=Стварэнне папак...
StatusExtractFiles=Распакоўка файлаў...
StatusDownloadFiles=Загрузка файлаў...
StatusCreateIcons=Стварэнне ярлыкоў...
StatusCreateIniEntries=Стварэнне запісаў INI...
StatusCreateRegistryEntries=Стварэнне запісаў рэестра...
StatusRegisterFiles=Рэгістрацыя файлаў...
StatusSavingUninstall=Захаванне звестак для выдалення...
StatusRunProgram=Завяршэнне ўсталёўкі...
StatusRestartingApplications=Перазапуск праграм...
StatusRollback=Адмена змен...

; *** Misc. errors
ErrorInternal2=Унутраная памылка: %1
ErrorFunctionFailedNoCode=%1: збой
ErrorFunctionFailed=%1: збой; код %2
ErrorFunctionFailedWithMessage=%1: збой; код %2.%n%3
ErrorExecutingProgram=Не ўдалося запусціць файл:%n%1

; *** Registry errors
ErrorRegOpenKey=Памылка адкрыцця раздзела рэестра:%n%1\%2
ErrorRegCreateKey=Памылка стварэння раздзела рэестра:%n%1\%2
ErrorRegWriteKey=Памылка запісу ў раздзел рэестра:%n%1\%2

; *** INI errors
ErrorIniEntry=Памылка стварэння запісу INI ў файле "%1".

; *** File copying errors
FileAbortRetryIgnoreSkipNotRecommended=Прапусціць гэты файл (не рэкамендуецца)
FileAbortRetryIgnoreIgnoreNotRecommended=Ігнараваць памылку і працягнуць (не рэкамендуецца)
SourceIsCorrupted=Зыходны файл пашкоджаны
SourceDoesntExist=Зыходны файл "%1" не існуе
SourceVerificationFailed=Не ўдалося праверыць зыходны файл: %1
VerificationSignatureDoesntExist=Файл подпісу "%1" не існуе
VerificationSignatureInvalid=Файл подпісу "%1" недапушчальны
VerificationKeyNotFound=Файл подпісу "%1" выкарыстоўвае невядомы ключ
VerificationFileNameIncorrect=Імя файла няправільнае
VerificationFileTagIncorrect=Тэг файла няправільны
VerificationFileSizeIncorrect=Памер файла няправільны
VerificationFileHashIncorrect=Хэш файла няправільны
ExistingFileReadOnly2=Наяўны файл не ўдалося замяніць, бо ён пазначаны як даступны толькі для чытання.
ExistingFileReadOnlyRetry=Зняць атрыбут «толькі для чытання» і паўтарыць спробу
ExistingFileReadOnlyKeepExisting=Пакінуць наяўны файл
ErrorReadingExistingDest=Памылка пры чытанні наяўнага файла:
FileExistsSelectAction=Выберыце дзеянне
FileExists2=Файл ужо існуе.
FileExistsOverwriteExisting=Перазапісаць наяўны файл
FileExistsKeepExisting=Пакінуць наяўны файл
FileExistsOverwriteOrKeepAll=Рабіць так і для наступных канфліктаў
ExistingFileNewerSelectAction=Выберыце дзеянне
ExistingFileNewer2=Наяўны файл навейшы за той, які майстар спрабуе ўсталяваць.
ExistingFileNewerOverwriteExisting=Перазапісаць наяўны файл
ExistingFileNewerKeepExisting=Пакінуць наяўны файл (рэкамендуецца)
ExistingFileNewerOverwriteOrKeepAll=Рабіць так і для наступных канфліктаў
ErrorChangingAttr=Памылка пры змене атрыбутаў наяўнага файла:
ErrorCreatingTemp=Памылка пры стварэнні файла ў папцы прызначэння:
ErrorReadingSource=Памылка пры чытанні зыходнага файла:
ErrorCopying=Памылка пры капіраванні файла:
ErrorDownloading=Памылка пры загрузцы файла:
ErrorExtracting=Памылка пры распакоўцы архіва:
ErrorReplacingExistingFile=Памылка пры замене наяўнага файла:
ErrorRestartReplace=Збой RestartReplace:
ErrorRenamingTemp=Памылка пры перайменаванні файла ў папцы прызначэння:
ErrorRegisterServer=Не ўдалося зарэгістраваць DLL/OCX: %1
ErrorRegSvr32Failed=Збой RegSvr32, код завяршэння %1
ErrorRegisterTypeLib=Не ўдалося зарэгістраваць бібліятэку тыпаў: %1

; *** Uninstall display name markings
; used for example as 'My Program (32-bit)'
UninstallDisplayNameMark=%1 (%2)
; used for example as 'My Program (32-bit, All users)'
UninstallDisplayNameMarks=%1 (%2, %3)
UninstallDisplayNameMark32Bit=32-разрадная
UninstallDisplayNameMark64Bit=64-разрадная
UninstallDisplayNameMarkAllUsers=Усе карыстальнікі
UninstallDisplayNameMarkCurrentUser=Бягучы карыстальнік

; *** Post-installation errors
ErrorOpeningReadme=Памылка пры спробе адкрыць файл README.
ErrorRestartingComputer=Майстру не ўдалося перазапусціць камп'ютар. Зрабіце гэта ўручную.

; *** Uninstaller messages
UninstallNotFound=Файл "%1" не існуе. Выдаленне немагчыма.
UninstallOpenError=Файл "%1" не ўдалося адкрыць. Выдаленне немагчыма
UninstallUnsupportedVer=Файл журнала выдалення "%1" мае фармат, які не распазнае гэтая версія праграмы выдалення. Выдаленне немагчыма
UninstallUnknownEntry=У журнале выдалення знойдзены невядомы запіс (%1)
ConfirmUninstall=Вы ўпэўнены, што хочаце цалкам выдаліць %1 і ўсе яго кампаненты?
UninstallOnlyOnWin64=Гэту ўсталёўку можна выдаліць толькі ў 64-разраднай Windows.
OnlyAdminCanUninstall=Гэту ўсталёўку можа выдаліць толькі карыстальнік з правамі адміністратара.
UninstallStatusLabel=Пачакайце, пакуль %1 выдаляецца з вашага камп'ютара.
UninstalledAll=%1 паспяхова выдалена з вашага камп'ютара.
UninstalledMost=Выдаленне %1 завершана.%n%nНекаторыя элементы выдаліць не ўдалося. Іх можна выдаліць уручную.
UninstalledAndNeedsRestart=Каб завяршыць выдаленне %1, трэба перазапусціць камп'ютар.%n%nПеразапусціць цяпер?
UninstallDataCorrupted=Файл "%1" пашкоджаны. Выдаленне немагчыма

; *** Uninstallation phase messages
ConfirmDeleteSharedFileTitle=Выдаліць агульны файл?
ConfirmDeleteSharedFile2=Сістэма паведамляе, што наступны агульны файл больш не выкарыстоўваецца ніводнай праграмай. Выдаліць гэты агульны файл?%n%nКалі якія-небудзь праграмы ўсё яшчэ выкарыстоўваюць гэты файл, пасля яго выдалення яны могуць працаваць няправільна. Калі вы не ўпэўнены, абярыце «Не». Файл, які застанецца ў сістэме, не прынясе шкоды.
SharedFileNameLabel=Імя файла:
SharedFileLocationLabel=Размяшчэнне:
WizardUninstalling=Стан выдалення
StatusUninstalling=Выдаленне %1...

; *** Shutdown block reasons
ShutdownBlockReasonInstallingApp=Усталёўка %1.
ShutdownBlockReasonUninstallingApp=Выдаленне %1.

; The custom messages below aren't used by Setup itself, but if you make
; use of them in your scripts, you'll want to translate them.

[CustomMessages]

NameAndVersion=%1, версія %2
AdditionalIcons=Дадатковыя ярлыкі:
CreateDesktopIcon=Стварыць ярлык на працоўным стале
CreateQuickLaunchIcon=Стварыць ярлык у панэлі хуткага запуску
ProgramOnTheWeb=%1 у Інтэрнэце
UninstallProgram=Выдаліць %1
LaunchProgram=Запусціць %1
AssocFileExtension=Звязаць %1 з пашырэннем файлаў %2
AssocingFileExtension=Звязванне %1 з пашырэннем файлаў %2...
AutoStartProgramGroupDescription=Аўтазапуск:
AutoStartProgram=Аўтаматычна запускаць %1
AddonHostProgramNotFound=%1 не знойдзена ў выбранай вамі папцы.%n%nПрацягнуць усё роўна?

; *** The messages of the installer that are Lang Switcher's own (the sample is English.isl)
LocalCertReady=Праграма ўсталёўкі створыць на гэтым камп'ютары сертыфікат і падпіша ім Lang Switcher, каб праграма працавала і ў вокнах праграм, запушчаных ад імя адміністратара.
LocalCertFailed=Lang Switcher усталяваны, але не будзе працаваць у вокнах праграм, запушчаных ад імя адміністратара: Windows не прыняла сертыфікат, створаны на гэтым камп'ютары. Падрабязнасці — у журнале ўсталёўкі.
RemoveSettingsQuestion=Выдаліць таксама файл налад Lang Switcher?
RemoveSettingsKeep=Абярыце «Не», каб захаваць налады для паўторнай усталёўкі.
AuthorSlogan=праграмы, зробленыя з душой
