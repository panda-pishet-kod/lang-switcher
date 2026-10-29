; Lang Switcher -- the messages of the installer in Uzbek, Cyrillic script (stage E108, group A).
;
; Inno Setup has no official Uzbek translation (inno_official = false in res\lang\languages.toml),
; so this file carries the whole of it: [LangOptions], every message of the compiler:Default.isl
; (6.5.0+), and the messages of the installer that are Lang Switcher's own. It is read after
; compiler:Default.isl as the second MessagesFile of the "uzbekcyrillic" entry of [Languages] in
; ..\LangSwitcher.iss, and so replaces the English text name for name.
; The words are those of Uzbek.isl (the Latin script of the same language), letter for letter in
; Cyrillic; English.isl is the sample the own messages translate; the glossary of the language is
; l10n\glossary\uz-Cyrl.tsv of the control contour. The name of the program is not translated.
; The names of the buttons inside a message stand in the quotation marks of the language (RULES 16).
; The accelerators (&) are not used, as in Tamil.isl: no two equal letters in one window.
; As in Default.isl: a message that has no full stop there has none here (Setup adds its own).

[LangOptions]
LanguageName=Ўзбекча
LanguageID=$0843
LanguageCodePage=0
;DialogFontName=
;DialogFontSize=9
;DialogFontBaseScaleWidth=7
;DialogFontBaseScaleHeight=15
;WelcomeFontName=Segoe UI
;WelcomeFontSize=14

[Messages]

; *** Application titles
SetupAppTitle=Ўрнатиш
SetupWindowTitle=Ўрнатиш - %1
UninstallAppTitle=Ўчириш
UninstallAppFullTitle=%1 дастурини ўчириш

; *** Misc. common
InformationTitle=Маълумот
ConfirmTitle=Тасдиқлаш
ErrorTitle=Хато

; *** SetupLdr messages
SetupLdrStartupMessage=Бу %1 дастурини ўрнатади. Давом этасизми?
LdrCannotCreateTemp=Вақтинчалик файл яратиб бўлмади. Ўрнатиш бекор қилинди
LdrCannotExecTemp=Вақтинчалик жилддаги файлни ишга тушириб бўлмади. Ўрнатиш бекор қилинди
HelpTextNote=

; *** Startup error messages
LastErrorMessage=%1.%n%nХато %2: %3
SetupFileMissing=Ўрнатиш жилдида %1 файли йўқ. Муаммони бартараф қилинг ёки дастурнинг янги нусхасини олинг.
SetupFileCorrupt=Ўрнатиш файллари бузилган. Дастурнинг янги нусхасини олинг.
SetupFileCorruptOrWrongVer=Ўрнатиш файллари бузилган ёки Ўрнатувчининг бу версиясига мос келмайди. Муаммони бартараф қилинг ёки дастурнинг янги нусхасини олинг.
InvalidParameter=Буйруқ сатрига нотўғри параметр берилди:%n%n%1
SetupAlreadyRunning=Ўрнатувчи аллақачон ишламоқда.
WindowsVersionNotSupported=Бу дастур компьютерингиздаги Windows версиясини қўллаб-қувватламайди.
WindowsServicePackRequired=Бу дастур учун %1 Service Pack %2 ёки ундан янгиси керак.
NotOnThisPlatform=Бу дастур %1 муҳитида ишламайди.
OnlyOnThisPlatform=Бу дастурни %1 муҳитида ишга тушириш керак.
OnlyOnTheseArchitectures=Бу дастурни фақат қуйидаги процессор архитектуралари учун мўлжалланган Windows версияларига ўрнатиш мумкин:%n%n%1
WinVersionTooLowError=Бу дастур учун %1 версияси %2 ёки ундан янгиси керак.
WinVersionTooHighError=Бу дастурни %1 версияси %2 ёки ундан янгисига ўрнатиб бўлмайди.
AdminPrivilegesRequired=Бу дастурни ўрнатишда администратор сифатида тизимга кирган бўлишингиз керак.
PowerUserPrivilegesRequired=Бу дастурни ўрнатишда администратор ёки Power Users гуруҳи аъзоси сифатида тизимга кирган бўлишингиз керак.
SetupAppRunningError=Ўрнатувчи %1 ҳозир ишлаётганини аниқлади.%n%nУнинг барча нусхаларини ёпинг, сўнг давом этиш учун «OK» тугмасини ёки чиқиш учун «Бекор қилиш» тугмасини босинг.
UninstallAppRunningError=Ўчириш дастури %1 ҳозир ишлаётганини аниқлади.%n%nУнинг барча нусхаларини ёпинг, сўнг давом этиш учун «OK» тугмасини ёки чиқиш учун «Бекор қилиш» тугмасини босинг.

; *** Startup questions
PrivilegesRequiredOverrideTitle=Ўрнатиш режимини танланг
PrivilegesRequiredOverrideInstruction=Ўрнатиш режимини танланг
PrivilegesRequiredOverrideText1=%1 дастурини барча фойдаланувчилар учун (администратор ҳуқуқлари керак) ёки фақат сиз учун ўрнатиш мумкин.
PrivilegesRequiredOverrideText2=%1 дастурини фақат сиз учун ёки барча фойдаланувчилар учун (администратор ҳуқуқлари керак) ўрнатиш мумкин.
PrivilegesRequiredOverrideAllUsers=Барча фойдаланувчилар учун ўрнатиш
PrivilegesRequiredOverrideAllUsersRecommended=Барча фойдаланувчилар учун ўрнатиш (тавсия этилади)
PrivilegesRequiredOverrideCurrentUser=Фақат мен учун ўрнатиш
PrivilegesRequiredOverrideCurrentUserRecommended=Фақат мен учун ўрнатиш (тавсия этилади)

; *** Misc. errors
ErrorCreatingDir=Ўрнатувчи "%1" жилдини ярата олмади
ErrorTooManyFilesInDir="%1" жилдида файллар жуда кўп бўлгани учун унда файл яратиб бўлмади

; *** Setup common messages
ExitSetupTitle=Ўрнатишдан чиқиш
ExitSetupMessage=Ўрнатиш тугалланмади. Ҳозир чиқсангиз, дастур ўрнатилмайди.%n%nЎрнатишни тугатиш учун Ўрнатувчини кейинроқ яна ишга туширишингиз мумкин.%n%nЎрнатишдан чиқилсинми?
AboutSetupMenuItem=Ўрнатувчи ҳақида...
AboutSetupTitle=Ўрнатувчи ҳақида
AboutSetupMessage=%1 версия %2%n%3%n%n%1 бош саҳифаси:%n%4
AboutSetupNote=
TranslatorNote=

; *** Buttons
ButtonBack=< Орқага
ButtonNext=Кейингиси >
ButtonInstall=Ўрнатиш
ButtonOK=OK
ButtonCancel=Бекор қилиш
ButtonYes=Ҳа
ButtonYesToAll=Ҳаммасига ҳа
ButtonNo=Йўқ
ButtonNoToAll=Ҳаммасига йўқ
ButtonFinish=Якунлаш
ButtonBrowse=Кўриб чиқиш...
ButtonWizardBrowse=Кўриб чиқиш...
ButtonNewFolder=Янги жилд яратиш

; *** "Select Language" dialog messages
SelectLanguageTitle=Ўрнатиш тилини танланг
SelectLanguageLabel=Ўрнатиш вақтида фойдаланиладиган тилни танланг.

; *** Common wizard text
ClickNext=Давом этиш учун «Кейингиси» тугмасини, чиқиш учун «Бекор қилиш» тугмасини босинг.
BeveledLabel=
BrowseDialogTitle=Жилдни танлаш
BrowseDialogLabel=Қуйидаги рўйхатдан жилдни танланг, сўнг «OK» тугмасини босинг.
NewFolderName=Янги жилд

; *** "Welcome" wizard page
WelcomeLabel1=[name] ўрнатиш устасига хуш келибсиз
WelcomeLabel2=Бу [name/ver] дастурини компьютерингизга ўрнатади.%n%nДавом этишдан олдин барча бошқа дастурларни ёпиш тавсия этилади.

; *** "Password" wizard page
WizardPassword=Пароль
PasswordLabel1=Бу ўрнатиш пароль билан ҳимояланган.
PasswordLabel3=Паролни киритинг, сўнг давом этиш учун «Кейингиси» тугмасини босинг. Паролда бош ва кичик ҳарфлар фарқланади.
PasswordEditLabel=Пароль:
IncorrectPassword=Киритилган пароль нотўғри. Қайтадан уриниб кўринг.

; *** "License Agreement" wizard page
WizardLicense=Лицензия шартномаси
LicenseLabel=Давом этишдан олдин қуйидаги муҳим маълумотни ўқинг.
LicenseLabel3=Қуйидаги лицензия шартномасини ўқинг. Ўрнатишни давом эттиришдан олдин шартнома шартларини қабул қилишингиз керак.
LicenseAccepted=Шартномани қабул қиламан
LicenseNotAccepted=Шартномани қабул қилмайман

; *** "Information" wizard pages
WizardInfoBefore=Маълумот
InfoBeforeLabel=Давом этишдан олдин қуйидаги муҳим маълумотни ўқинг.
InfoBeforeClickLabel=Ўрнатишни давом эттиришга тайёр бўлганингизда «Кейингиси» тугмасини босинг.
WizardInfoAfter=Маълумот
InfoAfterLabel=Давом этишдан олдин қуйидаги муҳим маълумотни ўқинг.
InfoAfterClickLabel=Ўрнатишни давом эттиришга тайёр бўлганингизда «Кейингиси» тугмасини босинг.

; *** "User Information" wizard page
WizardUserInfo=Фойдаланувчи маълумотлари
UserInfoDesc=Маълумотларингизни киритинг.
UserInfoName=Фойдаланувчи номи:
UserInfoOrg=Ташкилот:
UserInfoSerial=Серия рақами:
UserInfoNameRequired=Исм киритишингиз шарт.

; *** "Select Destination Location" wizard page
WizardSelectDir=Ўрнатиш жилдини танланг
SelectDirDesc=[name] қаерга ўрнатилсин?
SelectDirLabel3=Ўрнатувчи [name] дастурини қуйидаги жилдга ўрнатади.
SelectDirBrowseLabel=Давом этиш учун «Кейингиси» тугмасини босинг. Бошқа жилдни танламоқчи бўлсангиз, «Кўриб чиқиш» тугмасини босинг.
DiskSpaceGBLabel=Камида [gb] GB бўш диск жойи керак.
DiskSpaceMBLabel=Камида [mb] MB бўш диск жойи керак.
CannotInstallToNetworkDrive=Ўрнатувчи тармоқ дискига ўрната олмайди.
CannotInstallToUNCPath=Ўрнатувчи UNC йўлига ўрната олмайди.
InvalidPath=Диск ҳарфи билан тўлиқ йўлни киритинг, масалан:%n%nC:\APP%n%nёки UNC йўлини қуйидаги кўринишда:%n%n\\server\share
InvalidDrive=Танланган диск ёки UNC улуши мавжуд эмас ёки унга кириб бўлмайди. Бошқасини танланг.
DiskSpaceWarningTitle=Диск жойи етарли эмас
DiskSpaceWarning=Ўрнатиш учун камида %1 KB бўш жой керак, танланган дискда эса фақат %2 KB бор.%n%nБарибир давом этасизми?
DirNameTooLong=Жилд номи ёки йўли жуда узун.
InvalidDirName=Жилд номи яроқсиз.
BadDirName32=Жилд номида қуйидаги белгилар бўлиши мумкин эмас:%n%n%1
DirExistsTitle=Жилд мавжуд
DirExists=Қуйидаги жилд:%n%n%1%n%nаллақачон мавжуд. Барибир шу жилдга ўрнатилсинми?
DirDoesntExistTitle=Жилд мавжуд эмас
DirDoesntExist=Қуйидаги жилд:%n%n%1%n%nмавжуд эмас. Жилд яратилсинми?

; *** "Select Components" wizard page
WizardSelectComponents=Компонентларни танланг
SelectComponentsDesc=Қайси компонентлар ўрнатилсин?
SelectComponentsLabel2=Ўрнатмоқчи бўлган компонентларни белгиланг; ўрнатмоқчи бўлмаганларининг белгисини олиб ташланг. Давом этишга тайёр бўлганингизда «Кейингиси» тугмасини босинг.
FullInstallation=Тўлиқ ўрнатиш
; if possible don't translate 'Compact' as 'Minimal' (I mean 'Minimal' in your language)
CompactInstallation=Ихчам ўрнатиш
CustomInstallation=Мослаштирилган ўрнатиш
NoUninstallWarningTitle=Компонентлар мавжуд
NoUninstallWarning=Ўрнатувчи қуйидаги компонентлар компьютерингизда аллақачон ўрнатилганини аниқлади:%n%n%1%n%nБу компонентларнинг белгисини олиб ташлаш уларни ўчирмайди.%n%nБарибир давом этасизми?
ComponentSize1=%1 KB
ComponentSize2=%1 MB
ComponentsDiskSpaceGBLabel=Танланган компонентлар учун камида [gb] GB диск жойи керак.
ComponentsDiskSpaceMBLabel=Танланган компонентлар учун камида [mb] MB диск жойи керак.

; *** "Select Additional Tasks" wizard page
WizardSelectTasks=Қўшимча вазифаларни танланг
SelectTasksDesc=Қайси қўшимча вазифалар бажарилсин?
SelectTasksLabel2=[name] ўрнатилаётганда Ўрнатувчи бажариши керак бўлган қўшимча вазифаларни танланг, сўнг «Кейингиси» тугмасини босинг.

; *** "Select Start Menu Folder" wizard page
WizardSelectProgramGroup=«Бошлаш» менюси жилдини танланг
SelectStartMenuFolderDesc=Ўрнатувчи дастур ёрлиқларини қаерга жойлаштирсин?
SelectStartMenuFolderLabel3=Ўрнатувчи дастур ёрлиқларини «Бошлаш» менюсининг қуйидаги жилдида яратади.
SelectStartMenuFolderBrowseLabel=Давом этиш учун «Кейингиси» тугмасини босинг. Бошқа жилдни танламоқчи бўлсангиз, «Кўриб чиқиш» тугмасини босинг.
MustEnterGroupName=Жилд номини киритишингиз шарт.
GroupNameTooLong=Жилд номи ёки йўли жуда узун.
InvalidGroupName=Жилд номи яроқсиз.
BadGroupName=Жилд номида қуйидаги белгилар бўлиши мумкин эмас:%n%n%1
NoProgramGroupCheck2=«Бошлаш» менюсида жилд яратилмасин

; *** "Ready to Install" wizard page
WizardReady=Ўрнатишга тайёр
ReadyLabel1=Ўрнатувчи [name] дастурини компьютерингизга ўрнатишни бошлашга тайёр.
ReadyLabel2a=Ўрнатишни давом эттириш учун «Ўрнатиш» тугмасини босинг ёки созламаларни кўриб чиқиш ёхуд ўзгартириш учун «Орқага» тугмасини босинг.
ReadyLabel2b=Ўрнатишни давом эттириш учун «Ўрнатиш» тугмасини босинг.
ReadyMemoUserInfo=Фойдаланувчи маълумотлари:
ReadyMemoDir=Ўрнатиш жилди:
ReadyMemoType=Ўрнатиш тури:
ReadyMemoComponents=Танланган компонентлар:
ReadyMemoGroup=«Бошлаш» менюси жилди:
ReadyMemoTasks=Қўшимча вазифалар:

; *** TDownloadWizardPage wizard page and DownloadTemporaryFile
DownloadingLabel2=Файллар юклаб олинмоқда...
ButtonStopDownload=Юклаб олишни тўхтатиш
StopDownload=Юклаб олишни тўхтатмоқчимисиз?
ErrorDownloadAborted=Юклаб олиш тўхтатилди
ErrorDownloadFailed=Юклаб олиб бўлмади: %1 %2
ErrorDownloadSizeFailed=Ҳажмни аниқлаб бўлмади: %1 %2
ErrorProgress=Бажарилиш кўрсаткичи нотўғри: %1 / %2
ErrorFileSize=Файл ҳажми нотўғри: кутилгани %1, топилгани %2

; *** TExtractionWizardPage wizard page and ExtractArchive
ExtractingLabel=Файллар чиқарилмоқда...
ButtonStopExtraction=Чиқаришни тўхтатиш
StopExtraction=Чиқаришни тўхтатмоқчимисиз?
ErrorExtractionAborted=Чиқариш тўхтатилди
ErrorExtractionFailed=Чиқариб бўлмади: %1

; *** Archive extraction failure details
ArchiveIncorrectPassword=Пароль нотўғри
ArchiveIsCorrupted=Архив бузилган
ArchiveUnsupportedFormat=Архив формати қўллаб-қувватланмайди

; *** "Preparing to Install" wizard page
WizardPreparing=Ўрнатишга тайёрланмоқда
PreparingDesc=Ўрнатувчи [name] дастурини компьютерингизга ўрнатишга тайёрланмоқда.
PreviousInstallNotCompleted=Олдинги дастурни ўрнатиш ёки ўчириш тугалланмаган. Ўрнатишни тугатиш учун компьютерни қайта ишга туширишингиз керак.%n%nКомпьютер қайта ишга тушгач, [name] ўрнатишни тугатиш учун Ўрнатувчини яна ишга туширинг.
CannotContinue=Ўрнатишни давом эттириб бўлмайди. Чиқиш учун «Бекор қилиш» тугмасини босинг.
ApplicationsFound=Қуйидаги дастурлар Ўрнатувчи янгилаши керак бўлган файллардан фойдаланмоқда. Ўрнатувчига бу дастурларни автоматик ёпишга рухсат бериш тавсия этилади.
ApplicationsFound2=Қуйидаги дастурлар Ўрнатувчи янгилаши керак бўлган файллардан фойдаланмоқда. Ўрнатувчига бу дастурларни автоматик ёпишга рухсат бериш тавсия этилади. Ўрнатиш тугагач, Ўрнатувчи дастурларни қайта ишга туширишга уринади.
CloseApplications=Дастурларни автоматик ёпиш
DontCloseApplications=Дастурларни ёпмаслик
ErrorCloseApplications=Ўрнатувчи барча дастурларни автоматик ёпа олмади. Давом этишдан олдин Ўрнатувчи янгилаши керак бўлган файллардан фойдаланаётган барча дастурларни ёпиш тавсия этилади.
PrepareToInstallNeedsRestart=Ўрнатувчи компьютерингизни қайта ишга тушириши керак. Компьютер қайта ишга тушгач, [name] ўрнатишни тугатиш учун Ўрнатувчини яна ишга туширинг.%n%nҲозир қайта ишга туширилсинми?

; *** "Installing" wizard page
WizardInstalling=Ўрнатилмоқда
InstallingLabel=Ўрнатувчи [name] дастурини компьютерингизга ўрнатгунча кутинг.

; *** "Setup Completed" wizard page
FinishedHeadingLabel=[name] ўрнатиш устаси якунланмоқда
FinishedLabelNoIcons=Ўрнатувчи [name] дастурини компьютерингизга ўрнатиб бўлди.
FinishedLabel=Ўрнатувчи [name] дастурини компьютерингизга ўрнатиб бўлди. Дастурни ўрнатилган ёрлиқлар орқали ишга тушириш мумкин.
ClickFinish=Ўрнатувчидан чиқиш учун «Якунлаш» тугмасини босинг.
FinishedRestartLabel=[name] ўрнатишни тугатиш учун Ўрнатувчи компьютерингизни қайта ишга тушириши керак. Ҳозир қайта ишга туширилсинми?
FinishedRestartMessage=[name] ўрнатишни тугатиш учун Ўрнатувчи компьютерингизни қайта ишга тушириши керак.%n%nҲозир қайта ишга туширилсинми?
ShowReadmeCheck=Ҳа, README файлини кўрмоқчиман
YesRadio=Ҳа, компьютер ҳозир қайта ишга туширилсин
NoRadio=Йўқ, компьютерни кейинроқ ўзим қайта ишга тушираман
; used for example as 'Run MyProg.exe'
RunEntryExec=Ишга тушириш: %1
; used for example as 'View Readme.txt'
RunEntryShellExec=Кўриш: %1

; *** "Setup Needs the Next Disk" stuff
ChangeDiskTitle=Ўрнатувчига навбатдаги диск керак
SelectDiskLabel2=%1-дискни жойланг ва «OK» тугмасини босинг.%n%nАгар бу дискдаги файллар қуйида кўрсатилган жилддан бошқа жилдда бўлса, тўғри йўлни киритинг ёки «Кўриб чиқиш» тугмасини босинг.
PathLabel=Йўл:
FileNotInDir2="%1" файли "%2" ичидан топилмади. Тўғри дискни жойланг ёки бошқа жилдни танланг.
SelectDirectoryLabel=Навбатдаги дискнинг жойлашувини кўрсатинг.

; *** Installation phase messages
SetupAborted=Ўрнатиш тугалланмади.%n%nМуаммони бартараф қилинг ва Ўрнатувчини қайтадан ишга туширинг.
AbortRetryIgnoreSelectAction=Амални танланг
AbortRetryIgnoreRetry=Қайтадан уриниш
AbortRetryIgnoreIgnore=Хатога эътибор бермай давом этиш
AbortRetryIgnoreCancel=Ўрнатишни бекор қилиш
RetryCancelSelectAction=Амални танланг
RetryCancelRetry=Қайтадан уриниш
RetryCancelCancel=Бекор қилиш

; *** Installation status messages
StatusClosingApplications=Дастурлар ёпилмоқда...
StatusCreateDirs=Жилдлар яратилмоқда...
StatusExtractFiles=Файллар чиқарилмоқда...
StatusDownloadFiles=Файллар юклаб олинмоқда...
StatusCreateIcons=Ёрлиқлар яратилмоқда...
StatusCreateIniEntries=INI ёзувлари яратилмоқда...
StatusCreateRegistryEntries=Реестр ёзувлари яратилмоқда...
StatusRegisterFiles=Файллар рўйхатдан ўтказилмоқда...
StatusSavingUninstall=Ўчириш маълумотлари сақланмоқда...
StatusRunProgram=Ўрнатиш якунланмоқда...
StatusRestartingApplications=Дастурлар қайта ишга туширилмоқда...
StatusRollback=Ўзгаришлар бекор қилинмоқда...

; *** Misc. errors
ErrorInternal2=Ички хато: %1
ErrorFunctionFailedNoCode=%1 бажарилмади
ErrorFunctionFailed=%1 бажарилмади; код %2
ErrorFunctionFailedWithMessage=%1 бажарилмади; код %2.%n%3
ErrorExecutingProgram=Файлни ишга тушириб бўлмади:%n%1

; *** Registry errors
ErrorRegOpenKey=Реестр калитини очишда хато:%n%1\%2
ErrorRegCreateKey=Реестр калитини яратишда хато:%n%1\%2
ErrorRegWriteKey=Реестр калитига ёзишда хато:%n%1\%2

; *** INI errors
ErrorIniEntry="%1" файлида INI ёзувини яратишда хато.

; *** File copying errors
FileAbortRetryIgnoreSkipNotRecommended=Бу файлни ўтказиб юбориш (тавсия этилмайди)
FileAbortRetryIgnoreIgnoreNotRecommended=Хатога эътибор бермай давом этиш (тавсия этилмайди)
SourceIsCorrupted=Манба файл бузилган
SourceDoesntExist="%1" манба файли мавжуд эмас
SourceVerificationFailed=Манба файлни текшириш муваффақиятсиз тугади: %1
VerificationSignatureDoesntExist="%1" имзо файли мавжуд эмас
VerificationSignatureInvalid="%1" имзо файли яроқсиз
VerificationKeyNotFound="%1" имзо файли номаълум калитдан фойдаланади
VerificationFileNameIncorrect=Файл номи нотўғри
VerificationFileTagIncorrect=Файл теги нотўғри
VerificationFileSizeIncorrect=Файл ҳажми нотўғри
VerificationFileHashIncorrect=Файл хеши нотўғри
ExistingFileReadOnly2=Мавжуд файлни алмаштириб бўлмади, чунки у «фақат ўқиш учун» деб белгиланган.
ExistingFileReadOnlyRetry=«Фақат ўқиш учун» хусусиятини олиб ташлаб, қайтадан уриниш
ExistingFileReadOnlyKeepExisting=Мавжуд файлни сақлаб қолиш
ErrorReadingExistingDest=Мавжуд файлни ўқишда хато юз берди:
FileExistsSelectAction=Амални танланг
FileExists2=Файл аллақачон мавжуд.
FileExistsOverwriteExisting=Мавжуд файлни алмаштириш
FileExistsKeepExisting=Мавжуд файлни сақлаб қолиш
FileExistsOverwriteOrKeepAll=Кейинги тўқнашувларда ҳам шундай қилиш
ExistingFileNewerSelectAction=Амални танланг
ExistingFileNewer2=Мавжуд файл Ўрнатувчи ўрнатмоқчи бўлган файлдан янгироқ.
ExistingFileNewerOverwriteExisting=Мавжуд файлни алмаштириш
ExistingFileNewerKeepExisting=Мавжуд файлни сақлаб қолиш (тавсия этилади)
ExistingFileNewerOverwriteOrKeepAll=Кейинги тўқнашувларда ҳам шундай қилиш
ErrorChangingAttr=Мавжуд файл хусусиятларини ўзгартиришда хато юз берди:
ErrorCreatingTemp=Манзил жилдида файл яратишда хато юз берди:
ErrorReadingSource=Манба файлни ўқишда хато юз берди:
ErrorCopying=Файлни нусхалашда хато юз берди:
ErrorDownloading=Файлни юклаб олишда хато юз берди:
ErrorExtracting=Архивни чиқаришда хато юз берди:
ErrorReplacingExistingFile=Мавжуд файлни алмаштиришда хато юз берди:
ErrorRestartReplace=RestartReplace бажарилмади:
ErrorRenamingTemp=Манзил жилдидаги файлнинг номини ўзгартиришда хато юз берди:
ErrorRegisterServer=DLL/OCX ни рўйхатдан ўтказиб бўлмади: %1
ErrorRegSvr32Failed=RegSvr32 %1 чиқиш коди билан тугади
ErrorRegisterTypeLib=Тур кутубхонасини рўйхатдан ўтказиб бўлмади: %1

; *** Uninstall display name markings
; used for example as 'My Program (32-bit)'
UninstallDisplayNameMark=%1 (%2)
; used for example as 'My Program (32-bit, All users)'
UninstallDisplayNameMarks=%1 (%2, %3)
UninstallDisplayNameMark32Bit=32 битли
UninstallDisplayNameMark64Bit=64 битли
UninstallDisplayNameMarkAllUsers=Барча фойдаланувчилар
UninstallDisplayNameMarkCurrentUser=Жорий фойдаланувчи

; *** Post-installation errors
ErrorOpeningReadme=README файлини очишда хато юз берди.
ErrorRestartingComputer=Ўрнатувчи компьютерни қайта ишга тушира олмади. Буни қўлда бажаринг.

; *** Uninstaller messages
UninstallNotFound="%1" файли мавжуд эмас. Ўчириб бўлмайди.
UninstallOpenError="%1" файлини очиб бўлмади. Ўчириб бўлмайди
UninstallUnsupportedVer="%1" ўчириш журнали файли Ўчириш дастурининг бу версияси танимайдиган форматда. Ўчириб бўлмайди
UninstallUnknownEntry=Ўчириш журналида номаълум ёзув (%1) учради
ConfirmUninstall=%1 ва унинг барча компонентларини бутунлай ўчириб ташламоқчимисиз?
UninstallOnlyOnWin64=Бу ўрнатишни фақат 64 битли Windows муҳитида ўчириш мумкин.
OnlyAdminCanUninstall=Бу ўрнатишни фақат администратор ҳуқуқларига эга фойдаланувчи ўчира олади.
UninstallStatusLabel=%1 компьютерингиздан ўчирилгунча кутинг.
UninstalledAll=%1 компьютерингиздан муваффақиятли ўчирилди.
UninstalledMost=%1 ўчирилди.%n%nБаъзи элементларни ўчириб бўлмади. Уларни қўлда ўчириш мумкин.
UninstalledAndNeedsRestart=%1 ўчиришни тугатиш учун компьютерни қайта ишга тушириш керак.%n%nҲозир қайта ишга туширилсинми?
UninstallDataCorrupted="%1" файли бузилган. Ўчириб бўлмайди

; *** Uninstallation phase messages
ConfirmDeleteSharedFileTitle=Умумий файл ўчирилсинми?
ConfirmDeleteSharedFile2=Тизим қуйидаги умумий файлдан энди ҳеч қандай дастур фойдаланмаётганини билдирмоқда. Ўчириш дастури бу умумий файлни ўчирсинми?%n%nАгар баъзи дастурлар бу файлдан ҳамон фойдаланаётган бўлса ва у ўчирилса, улар тўғри ишламай қолиши мумкин. Ишончингиз комил бўлмаса, «Йўқ» тугмасини танланг. Файлни тизимда қолдириш ҳеч қандай зарар келтирмайди.
SharedFileNameLabel=Файл номи:
SharedFileLocationLabel=Жойлашуви:
WizardUninstalling=Ўчириш ҳолати
StatusUninstalling=%1 ўчирилмоқда...

; *** Shutdown block reasons
ShutdownBlockReasonInstallingApp=%1 ўрнатилмоқда.
ShutdownBlockReasonUninstallingApp=%1 ўчирилмоқда.

; The custom messages below aren't used by Setup itself, but if you make
; use of them in your scripts, you'll want to translate them.

[CustomMessages]

NameAndVersion=%1 версия %2
AdditionalIcons=Қўшимча ёрлиқлар:
CreateDesktopIcon=Иш столида ёрлиқ яратиш
CreateQuickLaunchIcon=Тез ишга тушириш панелида ёрлиқ яратиш
ProgramOnTheWeb=%1 интернетда
UninstallProgram=Ўчириш: %1
LaunchProgram=Ишга тушириш: %1
AssocFileExtension=%1 дастурини %2 файл кенгайтмаси билан боғлаш
AssocingFileExtension=%1 дастури %2 файл кенгайтмаси билан боғланмоқда...
AutoStartProgramGroupDescription=Автоюклаш:
AutoStartProgram=Автоматик ишга тушириш: %1
AddonHostProgramNotFound=%1 сиз танлаган жилдда топилмади.%n%nБарибир давом этасизми?

; The messages of the installer of Lang Switcher that are its own (the sample is English.isl).
LocalCertReady=Ўрнатувчи ушбу компьютерда сертификат яратади ва Lang Switcher дастурини у билан имзолайди, шунда дастур администратор номидан ишга туширилган дастурлар ойналарида ҳам ишлай олади.
LocalCertFailed=Lang Switcher ўрнатилди, лекин администратор номидан ишга туширилган дастурлар ойналарида ишламайди: Windows ушбу компьютерда яратилган сертификатни қабул қилмади. Тафсилотлар — ўрнатиш журналида.
RemoveSettingsQuestion=Lang Switcher созламалар файли ҳам ўчирилсинми?
RemoveSettingsKeep=Созламалар кейинроқ қайта ўрнатиш учун сақлансин десангиз, «Йўқ» тугмасини танланг.
AuthorSlogan=дил билан яратилган дастурлар
