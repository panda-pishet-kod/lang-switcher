; Lang Switcher -- the messages of the installer in Macedonian (stage E108, group A).
;
; Inno Setup has no official Macedonian translation (inno_official = false in res\lang\languages.toml),
; so this file carries the whole of it: [LangOptions], every message of the compiler:Default.isl
; (6.5.0+), and the messages of the installer that are Lang Switcher's own. It is read after
; compiler:Default.isl as the second MessagesFile of the "macedonian" entry of [Languages] in
; ..\LangSwitcher.iss, and so replaces the English text name for name.
; English.isl is the sample the own messages translate; the glossary of the language is
; l10n\glossary\mk.tsv of the control contour. The name of the program is not translated.
; As in Default.isl: a message that has no full stop there has none here (Setup adds its own).
; No accelerators (&) and the names of buttons inside a message are in „ “ (RULES 16).

[LangOptions]
LanguageName=Македонски
LanguageID=$042F
LanguageCodePage=0

[Messages]

; *** Application titles
SetupAppTitle=Инсталација
SetupWindowTitle=Инсталација - %1
UninstallAppTitle=Деинсталација
UninstallAppFullTitle=Деинсталација на %1

; *** Misc. common
InformationTitle=Информација
ConfirmTitle=Потврда
ErrorTitle=Грешка

; *** SetupLdr messages
SetupLdrStartupMessage=Ова ќе го инсталира %1. Дали сакате да продолжите?
LdrCannotCreateTemp=Не може да се создаде привремена датотека. Инсталацијата е прекината
LdrCannotExecTemp=Не може да се изврши датотека во привремената папка. Инсталацијата е прекината
HelpTextNote=

; *** Startup error messages
LastErrorMessage=%1.%n%nГрешка %2: %3
SetupFileMissing=Датотеката %1 недостига во папката за инсталација. Отстранете го проблемот или набавете нов примерок од програмата.
SetupFileCorrupt=Датотеките на инсталацијата се оштетени. Набавете нов примерок од програмата.
SetupFileCorruptOrWrongVer=Датотеките на инсталацијата се оштетени или не се совместливи со оваа верзија на инсталаторот. Отстранете го проблемот или набавете нов примерок од програмата.
InvalidParameter=На командната линија е предаден неважечки параметар:%n%n%1
SetupAlreadyRunning=Инсталаторот веќе работи.
WindowsVersionNotSupported=Оваа програма не ја поддржува верзијата на Windows што работи на вашиот компјутер.
WindowsServicePackRequired=Оваа програма бара %1 Service Pack %2 или понов.
NotOnThisPlatform=Оваа програма нема да работи на %1.
OnlyOnThisPlatform=Оваа програма мора да се стартува на %1.
OnlyOnTheseArchitectures=Оваа програма може да се инсталира само на верзии на Windows наменети за следниве архитектури на процесор:%n%n%1
WinVersionTooLowError=Оваа програма бара %1 верзија %2 или понова.
WinVersionTooHighError=Оваа програма не може да се инсталира на %1 верзија %2 или понова.
AdminPrivilegesRequired=При инсталирањето на оваа програма мора да сте најавени како администратор.
PowerUserPrivilegesRequired=При инсталирањето на оваа програма мора да сте најавени како администратор или како член на групата Power Users.
SetupAppRunningError=Инсталаторот откри дека %1 моментално работи.%n%nЗатворете ги сите негови примероци, па кликнете на „ОК“ за да продолжите или на „Откажи“ за да излезете.
UninstallAppRunningError=Деинсталаторот откри дека %1 моментално работи.%n%nЗатворете ги сите негови примероци, па кликнете на „ОК“ за да продолжите или на „Откажи“ за да излезете.

; *** Startup questions
PrivilegesRequiredOverrideTitle=Избор на начин на инсталирање
PrivilegesRequiredOverrideInstruction=Изберете начин на инсталирање
PrivilegesRequiredOverrideText1=%1 може да се инсталира за сите корисници (бара администраторски привилегии) или само за вас.
PrivilegesRequiredOverrideText2=%1 може да се инсталира само за вас или за сите корисници (бара администраторски привилегии).
PrivilegesRequiredOverrideAllUsers=Инсталирај за сите корисници
PrivilegesRequiredOverrideAllUsersRecommended=Инсталирај за сите корисници (препорачливо)
PrivilegesRequiredOverrideCurrentUser=Инсталирај само за мене
PrivilegesRequiredOverrideCurrentUserRecommended=Инсталирај само за мене (препорачливо)

; *** Misc. errors
ErrorCreatingDir=Инсталаторот не можеше да ја создаде папката „%1“
ErrorTooManyFilesInDir=Не може да се создаде датотека во папката „%1“ бидејќи содржи премногу датотеки

; *** Setup common messages
ExitSetupTitle=Излез од инсталацијата
ExitSetupMessage=Инсталацијата не е завршена. Ако излезете сега, програмата нема да биде инсталирана.%n%nИнсталацијата можете повторно да ја стартувате подоцна за да ја завршите.%n%nДали сакате да излезете од инсталацијата?
AboutSetupMenuItem=За инсталацијата...
AboutSetupTitle=За инсталацијата
AboutSetupMessage=%1 верзија %2%n%3%n%n%1 почетна страница:%n%4
AboutSetupNote=
TranslatorNote=

; *** Buttons
ButtonBack=< Назад
ButtonNext=Следно >
ButtonInstall=Инсталирај
ButtonOK=ОК
ButtonCancel=Откажи
ButtonYes=Да
ButtonYesToAll=Да за сите
ButtonNo=Не
ButtonNoToAll=Не за сите
ButtonFinish=Заврши
ButtonBrowse=Прелистај...
ButtonWizardBrowse=Прелистај...
ButtonNewFolder=Нова папка

; *** "Select Language" dialog messages
SelectLanguageTitle=Избор на јазик на инсталацијата
SelectLanguageLabel=Изберете го јазикот што ќе се користи при инсталацијата.

; *** Common wizard text
ClickNext=Кликнете на „Следно“ за да продолжите или на „Откажи“ за да излезете од инсталацијата.
BeveledLabel=
BrowseDialogTitle=Избор на папка
BrowseDialogLabel=Изберете папка од списокот подолу, па кликнете на „ОК“.
NewFolderName=Нова папка

; *** "Welcome" wizard page
WelcomeLabel1=Добре дојдовте во волшебникот за инсталација на [name]
WelcomeLabel2=Ова ќе го инсталира [name/ver] на вашиот компјутер.%n%nПрепорачливо е да ги затворите сите други програми пред да продолжите.

; *** "Password" wizard page
WizardPassword=Лозинка
PasswordLabel1=Оваа инсталација е заштитена со лозинка.
PasswordLabel3=Внесете ја лозинката, па кликнете на „Следно“ за да продолжите. Лозинките разликуваат големи и мали букви.
PasswordEditLabel=Лозинка:
IncorrectPassword=Внесената лозинка не е точна. Обидете се повторно.

; *** "License Agreement" wizard page
WizardLicense=Лиценцен договор
LicenseLabel=Прочитајте ги следниве важни информации пред да продолжите.
LicenseLabel3=Прочитајте го следниов лиценцен договор. Мора да ги прифатите условите на договорот за да продолжите со инсталацијата.
LicenseAccepted=Го прифаќам договорот
LicenseNotAccepted=Не го прифаќам договорот

; *** "Information" wizard pages
WizardInfoBefore=Информации
InfoBeforeLabel=Прочитајте ги следниве важни информации пред да продолжите.
InfoBeforeClickLabel=Кога ќе бидете подготвени да продолжите со инсталацијата, кликнете на „Следно“.
WizardInfoAfter=Информации
InfoAfterLabel=Прочитајте ги следниве важни информации пред да продолжите.
InfoAfterClickLabel=Кога ќе бидете подготвени да продолжите со инсталацијата, кликнете на „Следно“.

; *** "User Information" wizard page
WizardUserInfo=Информации за корисникот
UserInfoDesc=Внесете ги вашите податоци.
UserInfoName=Корисничко име:
UserInfoOrg=Организација:
UserInfoSerial=Сериски број:
UserInfoNameRequired=Мора да внесете име.

; *** "Select Destination Location" wizard page
WizardSelectDir=Избор на одредишна локација
SelectDirDesc=Каде да се инсталира [name]?
SelectDirLabel3=Инсталаторот ќе го инсталира [name] во следнава папка.
SelectDirBrowseLabel=За да продолжите, кликнете на „Следно“. Ако сакате да изберете друга папка, кликнете на „Прелистај“.
DiskSpaceGBLabel=Потребни се најмалку [gb] GB слободен простор на дискот.
DiskSpaceMBLabel=Потребни се најмалку [mb] MB слободен простор на дискот.
CannotInstallToNetworkDrive=Инсталаторот не може да инсталира на мрежен диск.
CannotInstallToUNCPath=Инсталаторот не може да инсталира на UNC-патека.
InvalidPath=Мора да внесете целосна патека со буква на дискот, на пример:%n%nC:\APP%n%nили UNC-патека во облик:%n%n\\server\share
InvalidDrive=Избраниот диск или UNC-споделување не постои или не е достапно. Изберете друго.
DiskSpaceWarningTitle=Нема доволно простор на дискот
DiskSpaceWarning=Инсталаторот бара најмалку %1 KB слободен простор, но на избраниот диск има само %2 KB.%n%nДали сепак сакате да продолжите?
DirNameTooLong=Името на папката или патеката е предолго.
InvalidDirName=Името на папката не е важечко.
BadDirName32=Имињата на папките не смеат да содржат ниту еден од следниве знаци:%n%n%1
DirExistsTitle=Папката постои
DirExists=Папката:%n%n%1%n%nвеќе постои. Дали сепак сакате да инсталирате во таа папка?
DirDoesntExistTitle=Папката не постои
DirDoesntExist=Папката:%n%n%1%n%nне постои. Дали сакате да се создаде папката?

; *** "Select Components" wizard page
WizardSelectComponents=Избор на компоненти
SelectComponentsDesc=Кои компоненти да се инсталираат?
SelectComponentsLabel2=Означете ги компонентите што сакате да ги инсталирате, а од оние што не сакате да ги инсталирате тргнете ја ознаката. Кликнете на „Следно“ кога ќе бидете подготвени да продолжите.
FullInstallation=Целосна инсталација
; if possible don't translate 'Compact' as 'Minimal' (I mean 'Minimal' in your language)
CompactInstallation=Компактна инсталација
CustomInstallation=Прилагодена инсталација
NoUninstallWarningTitle=Компонентите веќе постојат
NoUninstallWarning=Инсталаторот откри дека следниве компоненти веќе се инсталирани на вашиот компјутер:%n%n%1%n%nАко им ја тргнете ознаката, тие нема да се деинсталираат.%n%nДали сепак сакате да продолжите?
ComponentSize1=%1 KB
ComponentSize2=%1 MB
ComponentsDiskSpaceGBLabel=Тековниот избор бара најмалку [gb] GB простор на дискот.
ComponentsDiskSpaceMBLabel=Тековниот избор бара најмалку [mb] MB простор на дискот.

; *** "Select Additional Tasks" wizard page
WizardSelectTasks=Избор на дополнителни задачи
SelectTasksDesc=Кои дополнителни задачи да се извршат?
SelectTasksLabel2=Изберете ги дополнителните задачи што сакате инсталаторот да ги изврши при инсталирањето на [name], па кликнете на „Следно“.

; *** "Select Start Menu Folder" wizard page
WizardSelectProgramGroup=Избор на папка во менито „Старт“
SelectStartMenuFolderDesc=Каде инсталаторот да ги постави кратенките на програмата?
SelectStartMenuFolderLabel3=Инсталаторот ќе ги создаде кратенките на програмата во следнава папка од менито „Старт“.
SelectStartMenuFolderBrowseLabel=За да продолжите, кликнете на „Следно“. Ако сакате да изберете друга папка, кликнете на „Прелистај“.
MustEnterGroupName=Мора да внесете име на папка.
GroupNameTooLong=Името на папката или патеката е предолго.
InvalidGroupName=Името на папката не е важечко.
BadGroupName=Името на папката не смее да содржи ниту еден од следниве знаци:%n%n%1
NoProgramGroupCheck2=Не создавај папка во менито „Старт“

; *** "Ready to Install" wizard page
WizardReady=Подготвено за инсталирање
ReadyLabel1=Инсталаторот е подготвен да започне со инсталирањето на [name] на вашиот компјутер.
ReadyLabel2a=Кликнете на „Инсталирај“ за да продолжите со инсталацијата или на „Назад“ ако сакате да ги прегледате или промените поставките.
ReadyLabel2b=Кликнете на „Инсталирај“ за да продолжите со инсталацијата.
ReadyMemoUserInfo=Информации за корисникот:
ReadyMemoDir=Одредишна локација:
ReadyMemoType=Вид на инсталација:
ReadyMemoComponents=Избрани компоненти:
ReadyMemoGroup=Папка во менито „Старт“:
ReadyMemoTasks=Дополнителни задачи:

; *** TDownloadWizardPage wizard page and DownloadTemporaryFile
DownloadingLabel2=Се преземаат датотеките...
ButtonStopDownload=Прекини го преземањето
StopDownload=Дали навистина сакате да го прекинете преземањето?
ErrorDownloadAborted=Преземањето е прекинато
ErrorDownloadFailed=Преземањето не успеа: %1 %2
ErrorDownloadSizeFailed=Добивањето на големината не успеа: %1 %2
ErrorProgress=Неважечки напредок: %1 од %2
ErrorFileSize=Неважечка големина на датотеката: очекувано %1, најдено %2

; *** TExtractionWizardPage wizard page and ExtractArchive
ExtractingLabel=Се распакуваат датотеките...
ButtonStopExtraction=Прекини го распакувањето
StopExtraction=Дали навистина сакате да го прекинете распакувањето?
ErrorExtractionAborted=Распакувањето е прекинато
ErrorExtractionFailed=Распакувањето не успеа: %1

; *** Archive extraction failure details
ArchiveIncorrectPassword=Лозинката не е точна
ArchiveIsCorrupted=Архивата е оштетена
ArchiveUnsupportedFormat=Форматот на архивата не е поддржан

; *** "Preparing to Install" wizard page
WizardPreparing=Подготовка за инсталирање
PreparingDesc=Инсталаторот се подготвува да го инсталира [name] на вашиот компјутер.
PreviousInstallNotCompleted=Инсталирањето или отстранувањето на претходна програма не е завршено. Мора да го рестартирате компјутерот за да се заврши таа инсталација.%n%nОткако ќе го рестартирате компјутерот, повторно стартувајте го инсталаторот за да се заврши инсталирањето на [name].
CannotContinue=Инсталаторот не може да продолжи. Кликнете на „Откажи“ за да излезете.
ApplicationsFound=Следниве програми користат датотеки што инсталаторот треба да ги ажурира. Препорачливо е да му дозволите на инсталаторот автоматски да ги затвори.
ApplicationsFound2=Следниве програми користат датотеки што инсталаторот треба да ги ажурира. Препорачливо е да му дозволите на инсталаторот автоматски да ги затвори. По завршувањето на инсталацијата, инсталаторот ќе се обиде повторно да ги стартува.
CloseApplications=Автоматски затвори ги програмите
DontCloseApplications=Не ги затворај програмите
ErrorCloseApplications=Инсталаторот не успеа автоматски да ги затвори сите програми. Препорачливо е да ги затворите сите програми што користат датотеки што инсталаторот треба да ги ажурира, пред да продолжите.
PrepareToInstallNeedsRestart=Инсталаторот мора да го рестартира компјутерот. Откако ќе се рестартира, повторно стартувајте го инсталаторот за да се заврши инсталирањето на [name].%n%nДали сакате да рестартирате сега?

; *** "Installing" wizard page
WizardInstalling=Инсталирање
InstallingLabel=Почекајте додека инсталаторот го инсталира [name] на вашиот компјутер.

; *** "Setup Completed" wizard page
FinishedHeadingLabel=Завршување на волшебникот за инсталација на [name]
FinishedLabelNoIcons=Инсталаторот заврши со инсталирањето на [name] на вашиот компјутер.
FinishedLabel=Инсталаторот заврши со инсталирањето на [name] на вашиот компјутер. Програмата може да се стартува преку инсталираните кратенки.
ClickFinish=Кликнете на „Заврши“ за да излезете од инсталацијата.
FinishedRestartLabel=За да се заврши инсталирањето на [name], инсталаторот мора да го рестартира компјутерот. Дали сакате да рестартирате сега?
FinishedRestartMessage=За да се заврши инсталирањето на [name], инсталаторот мора да го рестартира компјутерот.%n%nДали сакате да рестартирате сега?
ShowReadmeCheck=Да, сакам да ја прегледам датотеката README
YesRadio=Да, рестартирај го компјутерот сега
NoRadio=Не, ќе го рестартирам компјутерот подоцна
; used for example as 'Run MyProg.exe'
RunEntryExec=Стартувај %1
; used for example as 'View Readme.txt'
RunEntryShellExec=Прегледај %1

; *** "Setup Needs the Next Disk" stuff
ChangeDiskTitle=Инсталаторот бара следен диск
SelectDiskLabel2=Вметнете го дискот %1 и кликнете на „ОК“.%n%nАко датотеките од овој диск се наоѓаат во папка различна од прикажаната подолу, внесете ја точната патека или кликнете на „Прелистај“.
PathLabel=Патека:
FileNotInDir2=Датотеката „%1“ не беше пронајдена во „%2“. Вметнете го вистинскиот диск или изберете друга папка.
SelectDirectoryLabel=Наведете ја локацијата на следниот диск.

; *** Installation phase messages
SetupAborted=Инсталацијата не е завршена.%n%nОтстранете го проблемот и повторно стартувајте го инсталаторот.
AbortRetryIgnoreSelectAction=Изберете дејство
AbortRetryIgnoreRetry=Обиди се повторно
AbortRetryIgnoreIgnore=Игнорирај ја грешката и продолжи
AbortRetryIgnoreCancel=Откажи ја инсталацијата
RetryCancelSelectAction=Изберете дејство
RetryCancelRetry=Обиди се повторно
RetryCancelCancel=Откажи

; *** Installation status messages
StatusClosingApplications=Се затвораат програмите...
StatusCreateDirs=Се создаваат папките...
StatusExtractFiles=Се распакуваат датотеките...
StatusDownloadFiles=Се преземаат датотеките...
StatusCreateIcons=Се создаваат кратенките...
StatusCreateIniEntries=Се создаваат INI-записите...
StatusCreateRegistryEntries=Се создаваат записите во регистарот...
StatusRegisterFiles=Се регистрираат датотеките...
StatusSavingUninstall=Се зачувуваат податоците за деинсталирање...
StatusRunProgram=Инсталацијата се завршува...
StatusRestartingApplications=Програмите повторно се стартуваат...
StatusRollback=Промените се враќаат...

; *** Misc. errors
ErrorInternal2=Внатрешна грешка: %1
ErrorFunctionFailedNoCode=%1 не успеа
ErrorFunctionFailed=%1 не успеа; код %2
ErrorFunctionFailedWithMessage=%1 не успеа; код %2.%n%3
ErrorExecutingProgram=Не може да се изврши датотеката:%n%1

; *** Registry errors
ErrorRegOpenKey=Грешка при отворање на клучот од регистарот:%n%1\%2
ErrorRegCreateKey=Грешка при создавање на клучот од регистарот:%n%1\%2
ErrorRegWriteKey=Грешка при запишување во клучот од регистарот:%n%1\%2

; *** INI errors
ErrorIniEntry=Грешка при создавање на INI-запис во датотеката „%1“.

; *** File copying errors
FileAbortRetryIgnoreSkipNotRecommended=Прескокни ја оваа датотека (не се препорачува)
FileAbortRetryIgnoreIgnoreNotRecommended=Игнорирај ја грешката и продолжи (не се препорачува)
SourceIsCorrupted=Изворната датотека е оштетена
SourceDoesntExist=Изворната датотека „%1“ не постои
SourceVerificationFailed=Проверката на изворната датотека не успеа: %1
VerificationSignatureDoesntExist=Датотеката со потпис „%1“ не постои
VerificationSignatureInvalid=Датотеката со потпис „%1“ е неважечка
VerificationKeyNotFound=Датотеката со потпис „%1“ користи непознат клуч
VerificationFileNameIncorrect=Името на датотеката е погрешно
VerificationFileTagIncorrect=Ознаката на датотеката е погрешна
VerificationFileSizeIncorrect=Големината на датотеката е погрешна
VerificationFileHashIncorrect=Хеш-вредноста на датотеката е погрешна
ExistingFileReadOnly2=Постојната датотека не можеше да се замени бидејќи е означена само за читање.
ExistingFileReadOnlyRetry=Отстрани го атрибутот „само за читање“ и обиди се повторно
ExistingFileReadOnlyKeepExisting=Задржи ја постојната датотека
ErrorReadingExistingDest=Настана грешка при обид да се прочита постојната датотека:
FileExistsSelectAction=Изберете дејство
FileExists2=Датотеката веќе постои.
FileExistsOverwriteExisting=Пребриши ја постојната датотека
FileExistsKeepExisting=Задржи ја постојната датотека
FileExistsOverwriteOrKeepAll=Направи го тоа за следните конфликти
ExistingFileNewerSelectAction=Изберете дејство
ExistingFileNewer2=Постојната датотека е понова од онаа што инсталаторот се обидува да ја инсталира.
ExistingFileNewerOverwriteExisting=Пребриши ја постојната датотека
ExistingFileNewerKeepExisting=Задржи ја постојната датотека (препорачливо)
ExistingFileNewerOverwriteOrKeepAll=Направи го тоа за следните конфликти
ErrorChangingAttr=Настана грешка при обид да се променат атрибутите на постојната датотека:
ErrorCreatingTemp=Настана грешка при обид да се создаде датотека во одредишната папка:
ErrorReadingSource=Настана грешка при обид да се прочита изворната датотека:
ErrorCopying=Настана грешка при обид да се копира датотека:
ErrorDownloading=Настана грешка при обид да се преземе датотека:
ErrorExtracting=Настана грешка при обид да се распакува архива:
ErrorReplacingExistingFile=Настана грешка при обид да се замени постојната датотека:
ErrorRestartReplace=RestartReplace не успеа:
ErrorRenamingTemp=Настана грешка при обид да се преименува датотека во одредишната папка:
ErrorRegisterServer=Не може да се регистрира DLL/OCX: %1
ErrorRegSvr32Failed=RegSvr32 заврши со излезен код %1
ErrorRegisterTypeLib=Не може да се регистрира библиотеката на типови: %1

; *** Uninstall display name markings
; used for example as 'My Program (32-bit)'
UninstallDisplayNameMark=%1 (%2)
; used for example as 'My Program (32-bit, All users)'
UninstallDisplayNameMarks=%1 (%2, %3)
UninstallDisplayNameMark32Bit=32-битна
UninstallDisplayNameMark64Bit=64-битна
UninstallDisplayNameMarkAllUsers=Сите корисници
UninstallDisplayNameMarkCurrentUser=Тековниот корисник

; *** Post-installation errors
ErrorOpeningReadme=Настана грешка при обид да се отвори датотеката README.
ErrorRestartingComputer=Инсталаторот не успеа да го рестартира компјутерот. Направете го тоа рачно.

; *** Uninstaller messages
UninstallNotFound=Датотеката „%1“ не постои. Не може да се деинсталира.
UninstallOpenError=Датотеката „%1“ не можеше да се отвори. Не може да се деинсталира
UninstallUnsupportedVer=Датотеката со дневникот за деинсталирање „%1“ е во формат што не го препознава оваа верзија на деинсталаторот. Не може да се деинсталира
UninstallUnknownEntry=Во дневникот за деинсталирање е пронајден непознат запис (%1)
ConfirmUninstall=Дали навистина сакате целосно да го отстраните %1 заедно со сите компоненти?
UninstallOnlyOnWin64=Оваа инсталација може да се деинсталира само на 64-битен Windows.
OnlyAdminCanUninstall=Оваа инсталација може да ја деинсталира само корисник со администраторски привилегии.
UninstallStatusLabel=Почекајте додека %1 се отстранува од вашиот компјутер.
UninstalledAll=Отстранувањето на %1 од вашиот компјутер заврши успешно.
UninstalledMost=Деинсталацијата на %1 заврши.%n%nНекои елементи не можеа да се отстранат. Тие може да се отстранат рачно.
UninstalledAndNeedsRestart=За да се заврши деинсталацијата на %1, компјутерот мора да се рестартира.%n%nДали сакате да рестартирате сега?
UninstallDataCorrupted=Датотеката „%1“ е оштетена. Не може да се деинсталира

; *** Uninstallation phase messages
ConfirmDeleteSharedFileTitle=Да се отстрани ли споделената датотека?
ConfirmDeleteSharedFile2=Системот покажува дека следнава споделена датотека повеќе не ја користи ниедна програма. Дали сакате деинсталаторот да ја отстрани оваа споделена датотека?%n%nАко некои програми сѐ уште ја користат, а таа се отстрани, тие програми може да не работат правилно. Ако не сте сигурни, изберете „Не“. Задржувањето на датотеката во системот нема да предизвика штета.
SharedFileNameLabel=Име на датотеката:
SharedFileLocationLabel=Локација:
WizardUninstalling=Состојба на деинсталацијата
StatusUninstalling=Се деинсталира %1...

; *** Shutdown block reasons
ShutdownBlockReasonInstallingApp=Се инсталира %1.
ShutdownBlockReasonUninstallingApp=Се деинсталира %1.

; The custom messages below aren't used by Setup itself, but if you make
; use of them in your scripts, you'll want to translate them.

[CustomMessages]

NameAndVersion=%1 верзија %2
AdditionalIcons=Дополнителни кратенки:
CreateDesktopIcon=Создај кратенка на работната површина
CreateQuickLaunchIcon=Создај кратенка во лентата за брзо стартување
ProgramOnTheWeb=%1 на интернет
UninstallProgram=Деинсталирај %1
LaunchProgram=Стартувај %1
AssocFileExtension=Поврзи го %1 со наставката за датотеки %2
AssocingFileExtension=%1 се поврзува со наставката за датотеки %2...
AutoStartProgramGroupDescription=Стартување:
AutoStartProgram=Автоматски стартувај го %1
AddonHostProgramNotFound=%1 не беше пронајден во избраната папка.%n%nДали сепак сакате да продолжите?

; The messages that are Lang Switcher's own (the sample is English.isl).
LocalCertReady=Инсталаторот ќе создаде сертификат на овој компјутер и ќе ја потпише програмата Lang Switcher со него, за да работи и во прозорците на програмите стартувани како администратор.
LocalCertFailed=Lang Switcher е инсталиран, но нема да работи во прозорците на програмите стартувани како администратор: Windows не го прифати сертификатот создаден на овој компјутер. Подробности — во дневникот на инсталацијата.
RemoveSettingsQuestion=Да се отстрани ли и датотеката со поставки на Lang Switcher?
RemoveSettingsKeep=Изберете „Не“ за да ги задржите поставките за подоцнежна повторна инсталација.
AuthorSlogan=програми направени со срце
