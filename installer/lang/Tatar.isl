; Lang Switcher -- the messages of the installer in Tatar (stage E108, group A).
;
; Inno Setup has no official Tatar file, so this file is whole: [LangOptions], all the messages of
; compiler:Default.isl (version 6.5.0 sample) translated, then the stock [CustomMessages] of that
; file and the messages of the installer that are its own (the sample is English.isl). Read as the
; only MessagesFile of the "tatar" entry of [Languages] in ..\LangSwitcher.iss; Default.isl may
; stand before it: what is missing here falls back to English.
; Cyrillic, left to right. There are no accelerators (&): in a file without an official Inno
; translation they are left out, so that one window never carries two equal letters. The names of
; the buttons inside a message stand in the quotation marks of the language. Messages that end
; without a full stop in Default.isl end without one here: Setup adds a full stop itself. The
; name of the program is not translated.
; AuthorSlogan (stage E99, decision 167.1): the slogan of the signature of the author at the
; bottom of every page of the wizard -- after "Panda Koder" and a middle dot; light, without
; pathos. The name of the author and the address of the site stay as they are in every language.

[LangOptions]
LanguageName=Татар
LanguageID=$0444
LanguageCodePage=0
;DialogFontName=
;DialogFontSize=9
;DialogFontBaseScaleWidth=7
;DialogFontBaseScaleHeight=15
;WelcomeFontName=Segoe UI
;WelcomeFontSize=14

[Messages]

; *** Application titles
SetupAppTitle=Урнаштыру
SetupWindowTitle=Урнаштыру - %1
UninstallAppTitle=Бетерү
UninstallAppFullTitle=%1 программасын бетерү

; *** Misc. common
InformationTitle=Мәгълүмат
ConfirmTitle=Раслау
ErrorTitle=Хата

; *** SetupLdr messages
SetupLdrStartupMessage=Бу %1 программасын урнаштырачак. Дәвам итәргә телисезме?
LdrCannotCreateTemp=Вакытлыча файл булдырып булмады. Урнаштыру туктатылды
LdrCannotExecTemp=Вакытлыча папкадагы файлны эшләтеп булмады. Урнаштыру туктатылды
HelpTextNote=

; *** Startup error messages
LastErrorMessage=%1.%n%nХата %2: %3
SetupFileMissing=%1 файлы урнаштыру папкасында юк. Проблеманы төзәтегез яки программаның яңа күчермәсен алыгыз.
SetupFileCorrupt=Урнаштыру файллары зарарланган. Программаның яңа күчермәсен алыгыз.
SetupFileCorruptOrWrongVer=Урнаштыру файллары зарарланган яки Урнаштыруның бу версиясе белән туры килми. Проблеманы төзәтегез яки программаның яңа күчермәсен алыгыз.
InvalidParameter=Команда юлына дөрес булмаган параметр бирелгән:%n%n%1
SetupAlreadyRunning=Урнаштыру инде эшли.
WindowsVersionNotSupported=Бу программа сезнең компьютерда эшләгән Windows версиясен хупламый.
WindowsServicePackRequired=Бу программага %1 Service Pack %2 яки аннан соңгысы кирәк.
NotOnThisPlatform=Бу программа %1 системасында эшләмәячәк.
OnlyOnThisPlatform=Бу программаны %1 системасында эшләтергә кирәк.
OnlyOnTheseArchitectures=Бу программаны процессорның түбәндәге архитектураларына исәпләнгән Windows версияләренә генә урнаштырып була:%n%n%1
WinVersionTooLowError=Бу программага %1 %2 версиясе яки аннан соңгысы кирәк.
WinVersionTooHighError=Бу программаны %1 %2 версиясенә яки аннан соңгысына урнаштырып булмый.
AdminPrivilegesRequired=Бу программаны урнаштырганда сез администратор буларак системага кергән булырга тиеш.
PowerUserPrivilegesRequired=Бу программаны урнаштырганда сез администратор буларак яки Power Users төркеме әгъзасы буларак системага кергән булырга тиеш.
SetupAppRunningError=Урнаштыру %1 программасы хәзер эшләвен ачыклады.%n%nАның барлык күчермәләрен ябыгыз, аннары дәвам итү өчен «ОК» төймәсенә яки чыгу өчен «Баш тарту» төймәсенә басыгыз.
UninstallAppRunningError=Бетерү %1 программасы хәзер эшләвен ачыклады.%n%nАның барлык күчермәләрен ябыгыз, аннары дәвам итү өчен «ОК» төймәсенә яки чыгу өчен «Баш тарту» төймәсенә басыгыз.

; *** Startup questions
PrivilegesRequiredOverrideTitle=Урнаштыру режимын сайлагыз
PrivilegesRequiredOverrideInstruction=Урнаштыру режимын сайлагыз
PrivilegesRequiredOverrideText1=%1 программасын барлык кулланучылар өчен (администратор хокуклары кирәк) яки үзегез генә өчен урнаштырып була.
PrivilegesRequiredOverrideText2=%1 программасын үзегез генә өчен яки барлык кулланучылар өчен (администратор хокуклары кирәк) урнаштырып була.
PrivilegesRequiredOverrideAllUsers=Барлык кулланучылар өчен урнаштыру
PrivilegesRequiredOverrideAllUsersRecommended=Барлык кулланучылар өчен урнаштыру (тәкъдим ителә)
PrivilegesRequiredOverrideCurrentUser=Үзем өчен генә урнаштыру
PrivilegesRequiredOverrideCurrentUserRecommended=Үзем өчен генә урнаштыру (тәкъдим ителә)

; *** Misc. errors
ErrorCreatingDir=Урнаштыру "%1" папкасын булдыра алмады
ErrorTooManyFilesInDir="%1" папкасында файл булдырып булмады, чөнки анда файллар бик күп

; *** Setup common messages
ExitSetupTitle=Урнаштырудан чыгу
ExitSetupMessage=Урнаштыру тәмамланмады. Хәзер чыксагыз, программа урнаштырылмаячак.%n%nУрнаштыруны тәмамлау өчен аны соңрак кабат эшләтеп җибәрергә мөмкин.%n%nУрнаштырудан чыгарга телисезме?
AboutSetupMenuItem=Урнаштыру турында...
AboutSetupTitle=Урнаштыру турында
AboutSetupMessage=%1 %2 версиясе%n%3%n%n%1 баш бите:%n%4
AboutSetupNote=
TranslatorNote=

; *** Buttons
ButtonBack=< Артка
ButtonNext=Алга >
ButtonInstall=Урнаштыру
ButtonOK=ОК
ButtonCancel=Баш тарту
ButtonYes=Әйе
ButtonYesToAll=Барысына да әйе
ButtonNo=Юк
ButtonNoToAll=Барысына да юк
ButtonFinish=Тәмамлау
ButtonBrowse=Күзәтү...
ButtonWizardBrowse=Күзәтү...
ButtonNewFolder=Яңа папка булдыру

; *** "Select Language" dialog messages
SelectLanguageTitle=Урнаштыру телен сайлагыз
SelectLanguageLabel=Урнаштыру вакытында кулланыла торган телне сайлагыз.

; *** Common wizard text
ClickNext=Дәвам итү өчен «Алга» төймәсенә яки Урнаштырудан чыгу өчен «Баш тарту» төймәсенә басыгыз.
BeveledLabel=
BrowseDialogTitle=Папканы күзәтү
BrowseDialogLabel=Түбәндәге исемлектән папканы сайлагыз һәм «ОК» төймәсенә басыгыз.
NewFolderName=Яңа папка

; *** "Welcome" wizard page
WelcomeLabel1=[name] урнаштыру мастерына рәхим итегез
WelcomeLabel2=Бу [name/ver] программасын сезнең компьютерга урнаштырачак.%n%nДәвам итәр алдыннан башка барлык программаларны ябарга киңәш ителә.

; *** "Password" wizard page
WizardPassword=Серсүз
PasswordLabel1=Бу урнаштыру серсүз белән сакланган.
PasswordLabel3=Серсүзне кертегез һәм дәвам итү өчен «Алга» төймәсенә басыгыз. Серсүздә баш һәм юл хәрефләре аерыла.
PasswordEditLabel=Серсүз:
IncorrectPassword=Кертелгән серсүз дөрес түгел. Тагын бер тапкыр сынап карагыз.

; *** "License Agreement" wizard page
WizardLicense=Лицензия килешүе
LicenseLabel=Дәвам итәр алдыннан түбәндәге мөһим мәгълүматны укыгыз.
LicenseLabel3=Түбәндәге лицензия килешүен укыгыз. Урнаштыруны дәвам иткәнче килешү шартларын кабул итәргә тиеш буласыз.
LicenseAccepted=Килешүне кабул итәм
LicenseNotAccepted=Килешүне кабул итмим

; *** "Information" wizard pages
WizardInfoBefore=Мәгълүмат
InfoBeforeLabel=Дәвам итәр алдыннан түбәндәге мөһим мәгълүматны укыгыз.
InfoBeforeClickLabel=Урнаштыруны дәвам итәргә әзер булгач, «Алга» төймәсенә басыгыз.
WizardInfoAfter=Мәгълүмат
InfoAfterLabel=Дәвам итәр алдыннан түбәндәге мөһим мәгълүматны укыгыз.
InfoAfterClickLabel=Урнаштыруны дәвам итәргә әзер булгач, «Алга» төймәсенә басыгыз.

; *** "User Information" wizard page
WizardUserInfo=Кулланучы турында мәгълүмат
UserInfoDesc=Үзегез турында мәгълүмат кертегез.
UserInfoName=Кулланучы исеме:
UserInfoOrg=Оешма:
UserInfoSerial=Сериялек номер:
UserInfoNameRequired=Исемегезне кертергә тиеш буласыз.

; *** "Select Destination Location" wizard page
WizardSelectDir=Урнаштыру урынын сайлагыз
SelectDirDesc=[name] кайда урнаштырылсын?
SelectDirLabel3=Урнаштыру [name] программасын түбәндәге папкага урнаштырачак.
SelectDirBrowseLabel=Дәвам итү өчен «Алга» төймәсенә басыгыз. Башка папка сайларга телисез икән, «Күзәтү» төймәсенә басыгыз.
DiskSpaceGBLabel=Ким дигәндә [gb] ГБ буш диск урыны кирәк.
DiskSpaceMBLabel=Ким дигәндә [mb] МБ буш диск урыны кирәк.
CannotInstallToNetworkDrive=Урнаштыру челтәр дискына урнаштыра алмый.
CannotInstallToUNCPath=Урнаштыру UNC юлына урнаштыра алмый.
InvalidPath=Диск хәрефе белән тулы юлны кертергә тиеш буласыз, мәсәлән:%n%nC:\APP%n%nяки түбәндәге рәвештәге UNC юлын:%n%n\\server\share
InvalidDrive=Сайланган диск яки UNC уртак папкасы юк яки аңа ирешеп булмый. Башкасын сайлагыз.
DiskSpaceWarningTitle=Дискта урын җитми
DiskSpaceWarning=Урнаштыру өчен ким дигәндә %1 КБ буш урын кирәк, ә сайланган дискта %2 КБ гына бар.%n%nШулай да дәвам итәсезме?
DirNameTooLong=Папка исеме яки юлы бик озын.
InvalidDirName=Папка исеме дөрес түгел.
BadDirName32=Папка исемендә түбәндәге символлар булырга тиеш түгел:%n%n%1
DirExistsTitle=Папка бар
DirExists=Түбәндәге папка:%n%n%1%n%nинде бар. Шулай да шул папкага урнаштырырга телисезме?
DirDoesntExistTitle=Папка юк
DirDoesntExist=Түбәндәге папка:%n%n%1%n%nюк. Папканы булдырырга телисезме?

; *** "Select Components" wizard page
WizardSelectComponents=Компонентларны сайлагыз
SelectComponentsDesc=Нинди компонентлар урнаштырылсын?
SelectComponentsLabel2=Урнаштырырга теләгән компонентларны билгеләгез; урнаштырырга теләмәгәннәрдән билгене алыгыз. Әзер булгач «Алга» төймәсенә басыгыз.
FullInstallation=Тулы урнаштыру
; if possible don't translate 'Compact' as 'Minimal' (I mean 'Minimal' in your language)
CompactInstallation=Кыскартылган урнаштыру
CustomInstallation=Сайлау буенча урнаштыру
NoUninstallWarningTitle=Компонентлар бар
NoUninstallWarning=Урнаштыру түбәндәге компонентларның сезнең компьютерда инде урнаштырылганын ачыклады:%n%n%1%n%nАлардан билгене алу аларны бетермәячәк.%n%nШулай да дәвам итәсезме?
ComponentSize1=%1 КБ
ComponentSize2=%1 МБ
ComponentsDiskSpaceGBLabel=Хәзерге сайлау өчен ким дигәндә [gb] ГБ диск урыны кирәк.
ComponentsDiskSpaceMBLabel=Хәзерге сайлау өчен ким дигәндә [mb] МБ диск урыны кирәк.

; *** "Select Additional Tasks" wizard page
WizardSelectTasks=Өстәмә биремнәрне сайлагыз
SelectTasksDesc=Нинди өстәмә биремнәр үтәлсен?
SelectTasksLabel2=[name] урнаштырганда Урнаштыру үтәргә тиешле өстәмә биремнәрне сайлагыз, аннары «Алга» төймәсенә басыгыз.

; *** "Select Start Menu Folder" wizard page
WizardSelectProgramGroup=«Башлау» менюсы папкасын сайлагыз
SelectStartMenuFolderDesc=Урнаштыру программаның ярлыкларын кайда урнаштырсын?
SelectStartMenuFolderLabel3=Урнаштыру программаның ярлыкларын «Башлау» менюсының түбәндәге папкасында булдырачак.
SelectStartMenuFolderBrowseLabel=Дәвам итү өчен «Алга» төймәсенә басыгыз. Башка папка сайларга телисез икән, «Күзәтү» төймәсенә басыгыз.
MustEnterGroupName=Папка исемен кертергә тиеш буласыз.
GroupNameTooLong=Папка исеме яки юлы бик озын.
InvalidGroupName=Папка исеме дөрес түгел.
BadGroupName=Папка исемендә түбәндәге символлар булырга тиеш түгел:%n%n%1
NoProgramGroupCheck2=«Башлау» менюсы папкасын булдырмау

; *** "Ready to Install" wizard page
WizardReady=Урнаштыруга әзер
ReadyLabel1=Урнаштыру [name] программасын сезнең компьютерга урнаштыруга әзер.
ReadyLabel2a=Урнаштыруны дәвам итү өчен «Урнаштыру» төймәсенә басыгыз яки көйләүләрне карау я үзгәртү өчен «Артка» төймәсенә басыгыз.
ReadyLabel2b=Урнаштыруны дәвам итү өчен «Урнаштыру» төймәсенә басыгыз.
ReadyMemoUserInfo=Кулланучы турында мәгълүмат:
ReadyMemoDir=Урнаштыру урыны:
ReadyMemoType=Урнаштыру төре:
ReadyMemoComponents=Сайланган компонентлар:
ReadyMemoGroup=«Башлау» менюсы папкасы:
ReadyMemoTasks=Өстәмә биремнәр:

; *** TDownloadWizardPage wizard page and DownloadTemporaryFile
DownloadingLabel2=Файллар йөкләп алына...
ButtonStopDownload=Йөкләп алуны туктату
StopDownload=Йөкләп алуны туктатырга чыннан да телисезме?
ErrorDownloadAborted=Йөкләп алу өзелде
ErrorDownloadFailed=Йөкләп алу барып чыкмады: %1 %2
ErrorDownloadSizeFailed=Күләмне алу барып чыкмады: %1 %2
ErrorProgress=Дөрес булмаган барыш: %2 эченнән %1
ErrorFileSize=Файл күләме дөрес түгел: көтелгәне %1, табылганы %2

; *** TExtractionWizardPage wizard page and ExtractArchive
ExtractingLabel=Файллар чыгарыла...
ButtonStopExtraction=Чыгаруны туктату
StopExtraction=Чыгаруны туктатырга чыннан да телисезме?
ErrorExtractionAborted=Чыгару өзелде
ErrorExtractionFailed=Чыгару барып чыкмады: %1

; *** Archive extraction failure details
ArchiveIncorrectPassword=Серсүз дөрес түгел
ArchiveIsCorrupted=Архив зарарланган
ArchiveUnsupportedFormat=Архив форматы хупланмый

; *** "Preparing to Install" wizard page
WizardPreparing=Урнаштыруга әзерләнү
PreparingDesc=Урнаштыру [name] программасын сезнең компьютерга урнаштыруга әзерләнә.
PreviousInstallNotCompleted=Алдагы программаны урнаштыру/бетерү тәмамланмады. Ул урнаштыруны тәмамлау өчен компьютерны яңадан йөкләргә кирәк.%n%nКомпьютерны яңадан йөкләгәч, [name] урнаштыруын тәмамлау өчен Урнаштыруны кабат эшләтегез.
CannotContinue=Урнаштыруны дәвам итеп булмый. Чыгу өчен «Баш тарту» төймәсенә басыгыз.
ApplicationsFound=Түбәндәге программалар Урнаштыру яңартырга тиешле файллардан файдалана. Урнаштыруга бу программаларны автоматик рәвештә ябарга рөхсәт итәргә киңәш ителә.
ApplicationsFound2=Түбәндәге программалар Урнаштыру яңартырга тиешле файллардан файдалана. Урнаштыруга бу программаларны автоматик рәвештә ябарга рөхсәт итәргә киңәш ителә. Урнаштыру тәмамлангач, Урнаштыру программаларны яңадан эшләтеп җибәрергә тырышачак.
CloseApplications=Программаларны автоматик рәвештә ябу
DontCloseApplications=Программаларны ябмау
ErrorCloseApplications=Урнаштыру барлык программаларны да автоматик рәвештә яба алмады. Дәвам итәр алдыннан Урнаштыру яңартырга тиешле файллардан файдаланучы барлык программаларны ябарга киңәш ителә.
PrepareToInstallNeedsRestart=Урнаштыру компьютерны яңадан йөкләргә тиеш. Компьютерны яңадан йөкләгәч, [name] урнаштыруын тәмамлау өчен Урнаштыруны кабат эшләтегез.%n%nХәзер яңадан йөкләргә телисезме?

; *** "Installing" wizard page
WizardInstalling=Урнаштыру
InstallingLabel=Урнаштыру [name] программасын сезнең компьютерга урнаштырганда көтеп торыгыз.

; *** "Setup Completed" wizard page
FinishedHeadingLabel=[name] урнаштыру мастерын тәмамлау
FinishedLabelNoIcons=Урнаштыру [name] программасын сезнең компьютерга урнаштыруны тәмамлады.
FinishedLabel=Урнаштыру [name] программасын сезнең компьютерга урнаштыруны тәмамлады. Программаны урнаштырылган ярлыклар ярдәмендә эшләтеп җибәреп була.
ClickFinish=Урнаштырудан чыгу өчен «Тәмамлау» төймәсенә басыгыз.
FinishedRestartLabel=[name] урнаштыруын тәмамлау өчен Урнаштыру компьютерны яңадан йөкләргә тиеш. Хәзер яңадан йөкләргә телисезме?
FinishedRestartMessage=[name] урнаштыруын тәмамлау өчен Урнаштыру компьютерны яңадан йөкләргә тиеш.%n%nХәзер яңадан йөкләргә телисезме?
ShowReadmeCheck=Әйе, README файлын карарга телим
YesRadio=Әйе, компьютерны хәзер яңадан йөкләргә
NoRadio=Юк, компьютерны соңрак яңадан йөкләрмен
; used for example as 'Run MyProg.exe'
RunEntryExec=%1 эшләтеп җибәрү
; used for example as 'View Readme.txt'
RunEntryShellExec=%1 карау

; *** "Setup Needs the Next Disk" stuff
ChangeDiskTitle=Урнаштыруга киләсе диск кирәк
SelectDiskLabel2=%1 дискын куегыз һәм «ОК» төймәсенә басыгыз.%n%nӘгәр бу диск файлларын түбәндә күрсәтелгәннән башка папкада табып булса, дөрес юлны кертегез яки «Күзәтү» төймәсенә басыгыз.
PathLabel=Юл:
FileNotInDir2="%1" файлы "%2" эчендә табылмады. Дөрес дискны куегыз яки башка папка сайлагыз.
SelectDirectoryLabel=Киләсе диск урнашкан урынны күрсәтегез.

; *** Installation phase messages
SetupAborted=Урнаштыру тәмамланмады.%n%nПроблеманы төзәтегез һәм Урнаштыруны кабат эшләтегез.
AbortRetryIgnoreSelectAction=Гамәлне сайлагыз
AbortRetryIgnoreRetry=Кабат сынап карау
AbortRetryIgnoreIgnore=Хатаны санга сукмыйча дәвам итү
AbortRetryIgnoreCancel=Урнаштыруны туктату
RetryCancelSelectAction=Гамәлне сайлагыз
RetryCancelRetry=Кабат сынап карау
RetryCancelCancel=Баш тарту

; *** Installation status messages
StatusClosingApplications=Программалар ябыла...
StatusCreateDirs=Папкалар булдырыла...
StatusExtractFiles=Файллар чыгарыла...
StatusDownloadFiles=Файллар йөкләп алына...
StatusCreateIcons=Ярлыклар булдырыла...
StatusCreateIniEntries=INI язмалары булдырыла...
StatusCreateRegistryEntries=Реестр язмалары булдырыла...
StatusRegisterFiles=Файллар теркәлә...
StatusSavingUninstall=Бетерү турында мәгълүмат саклана...
StatusRunProgram=Урнаштыру тәмамлана...
StatusRestartingApplications=Программалар яңадан эшләтеп җибәрелә...
StatusRollback=Үзгәрешләр кире кайтарыла...

; *** Misc. errors
ErrorInternal2=Эчке хата: %1
ErrorFunctionFailedNoCode=%1 барып чыкмады
ErrorFunctionFailed=%1 барып чыкмады; коды %2
ErrorFunctionFailedWithMessage=%1 барып чыкмады; коды %2.%n%3
ErrorExecutingProgram=Файлны эшләтеп җибәреп булмады:%n%1

; *** Registry errors
ErrorRegOpenKey=Реестр ачкычын ачу хатасы:%n%1\%2
ErrorRegCreateKey=Реестр ачкычын булдыру хатасы:%n%1\%2
ErrorRegWriteKey=Реестр ачкычына язу хатасы:%n%1\%2

; *** INI errors
ErrorIniEntry="%1" файлында INI язмасын булдыру хатасы.

; *** File copying errors
FileAbortRetryIgnoreSkipNotRecommended=Бу файлны төшереп калдыру (тәкъдим ителми)
FileAbortRetryIgnoreIgnoreNotRecommended=Хатаны санга сукмыйча дәвам итү (тәкъдим ителми)
SourceIsCorrupted=Чыганак файл зарарланган
SourceDoesntExist=Чыганак "%1" файлы юк
SourceVerificationFailed=Чыганак файлны тикшерү барып чыкмады: %1
VerificationSignatureDoesntExist="%1" имза файлы юк
VerificationSignatureInvalid="%1" имза файлы дөрес түгел
VerificationKeyNotFound="%1" имза файлы билгесез ачкыч куллана
VerificationFileNameIncorrect=Файл исеме дөрес түгел
VerificationFileTagIncorrect=Файл тегы дөрес түгел
VerificationFileSizeIncorrect=Файл күләме дөрес түгел
VerificationFileHashIncorrect=Файл хэше дөрес түгел
ExistingFileReadOnly2=Булган файлны алыштырып булмады, чөнки ул «тик укыр өчен» дип билгеләнгән.
ExistingFileReadOnlyRetry=«Тик укыр өчен» атрибутын алып, кабат сынап карау
ExistingFileReadOnlyKeepExisting=Булган файлны калдыру
ErrorReadingExistingDest=Булган файлны укыганда хата килеп чыкты:
FileExistsSelectAction=Гамәлне сайлагыз
FileExists2=Файл инде бар.
FileExistsOverwriteExisting=Булган файлның өстенә язу
FileExistsKeepExisting=Булган файлны калдыру
FileExistsOverwriteOrKeepAll=Киләсе каршылыклар өчен дә шулай эшләү
ExistingFileNewerSelectAction=Гамәлне сайлагыз
ExistingFileNewer2=Булган файл Урнаштыру урнаштырырга теләгән файлдан яңарак.
ExistingFileNewerOverwriteExisting=Булган файлның өстенә язу
ExistingFileNewerKeepExisting=Булган файлны калдыру (тәкъдим ителә)
ExistingFileNewerOverwriteOrKeepAll=Киләсе каршылыклар өчен дә шулай эшләү
ErrorChangingAttr=Булган файлның атрибутларын үзгәрткәндә хата килеп чыкты:
ErrorCreatingTemp=Максат папкада файл булдырганда хата килеп чыкты:
ErrorReadingSource=Чыганак файлны укыганда хата килеп чыкты:
ErrorCopying=Файлны күчереп алганда хата килеп чыкты:
ErrorDownloading=Файлны йөкләп алганда хата килеп чыкты:
ErrorExtracting=Архивны чыгарганда хата килеп чыкты:
ErrorReplacingExistingFile=Булган файлны алыштырганда хата килеп чыкты:
ErrorRestartReplace=RestartReplace барып чыкмады:
ErrorRenamingTemp=Максат папкадагы файлның исемен алыштырганда хата килеп чыкты:
ErrorRegisterServer=DLL/OCX теркәп булмады: %1
ErrorRegSvr32Failed=RegSvr32 %1 чыгу коды белән барып чыкмады
ErrorRegisterTypeLib=Тип китапханәсен теркәп булмады: %1

; *** Uninstall display name markings
; used for example as 'My Program (32-bit)'
UninstallDisplayNameMark=%1 (%2)
; used for example as 'My Program (32-bit, All users)'
UninstallDisplayNameMarks=%1 (%2, %3)
UninstallDisplayNameMark32Bit=32 разрядлы
UninstallDisplayNameMark64Bit=64 разрядлы
UninstallDisplayNameMarkAllUsers=Барлык кулланучылар
UninstallDisplayNameMarkCurrentUser=Хәзерге кулланучы

; *** Post-installation errors
ErrorOpeningReadme=README файлын ачканда хата килеп чыкты.
ErrorRestartingComputer=Урнаштыру компьютерны яңадан йөкли алмады. Моны кул белән эшләгез.

; *** Uninstaller messages
UninstallNotFound="%1" файлы юк. Бетереп булмый.
UninstallOpenError="%1" файлын ачып булмады. Бетереп булмый
UninstallUnsupportedVer="%1" бетерү журналы файлы Бетерүнең бу версиясе танымаган форматта. Бетереп булмый
UninstallUnknownEntry=Бетерү журналында билгесез язма (%1) очрады
ConfirmUninstall=%1 программасын һәм аның барлык компонентларын тулысынча бетерергә телисезме?
UninstallOnlyOnWin64=Бу урнаштыруны 64 разрядлы Windows системасында гына бетереп була.
OnlyAdminCanUninstall=Бу урнаштыруны администратор хокуклары булган кулланучы гына бетерә ала.
UninstallStatusLabel=%1 компьютерыгыздан бетерелгәндә көтеп торыгыз.
UninstalledAll=%1 компьютерыгыздан уңышлы бетерелде.
UninstalledMost=%1 бетерү тәмамланды.%n%nКайбер элементларны бетереп булмады. Аларны кул белән бетереп була.
UninstalledAndNeedsRestart=%1 бетерүне тәмамлау өчен компьютерны яңадан йөкләргә кирәк.%n%nХәзер яңадан йөкләргә телисезме?
UninstallDataCorrupted="%1" файлы зарарланган. Бетереп булмый

; *** Uninstallation phase messages
ConfirmDeleteSharedFileTitle=Уртак файл бетерелсенме?
ConfirmDeleteSharedFile2=Система түбәндәге уртак файлны хәзер бернинди программа да кулланмавын күрсәтә. Бетерү бу уртак файлны бетерсенме?%n%nӘгәр нинди дә булса программа әле бу файлдан файдалана һәм ул бетерелсә, ул программалар дөрес эшләмәскә мөмкин. Ышанычыгыз булмаса, «Юк» төймәсенә басыгыз. Файлны системада калдыру зыян китермәячәк.
SharedFileNameLabel=Файл исеме:
SharedFileLocationLabel=Урыны:
WizardUninstalling=Бетерү хәле
StatusUninstalling=%1 бетерелә...

; *** Shutdown block reasons
ShutdownBlockReasonInstallingApp=%1 урнаштырыла.
ShutdownBlockReasonUninstallingApp=%1 бетерелә.

; The custom messages below aren't used by Setup itself, but if you make
; use of them in your scripts, you'll want to translate them.

[CustomMessages]

NameAndVersion=%1 %2 версиясе
AdditionalIcons=Өстәмә ярлыклар:
CreateDesktopIcon=Эш өстәлендә ярлык булдыру
CreateQuickLaunchIcon=Тиз эшләтү ярлыгын булдыру
ProgramOnTheWeb=%1 — Интернетта
UninstallProgram=%1 программасын бетерү
LaunchProgram=%1 эшләтеп җибәрү
AssocFileExtension=%1 программасын %2 файл киңәйтмәсе белән бәйләү
AssocingFileExtension=%1 программасы %2 файл киңәйтмәсе белән бәйләнә...
AutoStartProgramGroupDescription=Автоматик эшләтү:
AutoStartProgram=%1 программасын автоматик рәвештә эшләтеп җибәрү
AddonHostProgramNotFound=%1 сез сайлаган папкада табылмады.%n%nШулай да дәвам итәсезме?

; The messages that are Lang Switcher's own (the sample is English.isl).
LocalCertReady=Урнаштыру бу компьютерда сертификат булдырачак һәм аның белән Lang Switcher программасына кул куячак, шуның аркасында программа администратор исеменнән эшләтелгән программаларның тәрәзәләрендә дә эшли алачак.
LocalCertFailed=Lang Switcher урнаштырылды, ләкин администратор исеменнән эшләтелгән программаларның тәрәзәләрендә эшләмәячәк: Windows бу компьютерда булдырылган сертификатны кабул итмәде. Тулырак мәгълүмат урнаштыру журналында.
RemoveSettingsQuestion=Lang Switcher көйләүләр файлын да бетерергәме?
RemoveSettingsKeep=Яңадан урнаштыру өчен көйләүләрегезне калдырырга теләсәгез, «Юк» төймәсенә басыгыз.
AuthorSlogan=йөрәк белән ясалган программалар
