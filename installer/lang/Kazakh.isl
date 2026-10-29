; Lang Switcher -- the messages of the installer in Kazakh (stage E108, group A).
;
; Inno Setup has no official Kazakh file, so this file is whole: [LangOptions], all the messages of
; compiler:Default.isl (version 6.5.0 sample) translated, then the stock [CustomMessages] of that
; file and the messages of the installer that are its own (the sample is English.isl). Read as the
; only MessagesFile of the "kazakh" entry of [Languages] in ..\LangSwitcher.iss; Default.isl may
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
LanguageName=Қазақ тілі
LanguageID=$043F
LanguageCodePage=0
;DialogFontName=
;DialogFontSize=9
;DialogFontBaseScaleWidth=7
;DialogFontBaseScaleHeight=15
;WelcomeFontName=Segoe UI
;WelcomeFontSize=14

[Messages]

; *** Application titles
SetupAppTitle=Орнату
SetupWindowTitle=Орнату - %1
UninstallAppTitle=Жою
UninstallAppFullTitle=%1 бағдарламасын жою

; *** Misc. common
InformationTitle=Ақпарат
ConfirmTitle=Растау
ErrorTitle=Қате

; *** SetupLdr messages
SetupLdrStartupMessage=Бұл %1 бағдарламасын орнатады. Жалғастырғыңыз келе ме?
LdrCannotCreateTemp=Уақытша файлды жасау мүмкін болмады. Орнату тоқтатылды
LdrCannotExecTemp=Уақытша қалтадағы файлды іске қосу мүмкін болмады. Орнату тоқтатылды
HelpTextNote=

; *** Startup error messages
LastErrorMessage=%1.%n%nҚате %2: %3
SetupFileMissing=%1 файлы орнату қалтасында жоқ. Мәселені түзетіңіз немесе бағдарламаның жаңа көшірмесін алыңыз.
SetupFileCorrupt=Орнату файлдары бүлінген. Бағдарламаның жаңа көшірмесін алыңыз.
SetupFileCorruptOrWrongVer=Орнату файлдары бүлінген немесе Орнатудың осы нұсқасымен үйлеспейді. Мәселені түзетіңіз немесе бағдарламаның жаңа көшірмесін алыңыз.
InvalidParameter=Пәрмен жолында жарамсыз параметр берілді:%n%n%1
SetupAlreadyRunning=Орнату әлдеқашан жұмыс істеп тұр.
WindowsVersionNotSupported=Бұл бағдарлама компьютеріңізде жұмыс істеп тұрған Windows нұсқасын қолдамайды.
WindowsServicePackRequired=Бұл бағдарламаға %1 Service Pack %2 немесе одан кейінгісі қажет.
NotOnThisPlatform=Бұл бағдарлама %1 жүйесінде жұмыс істемейді.
OnlyOnThisPlatform=Бұл бағдарламаны %1 жүйесінде іске қосу керек.
OnlyOnTheseArchitectures=Бұл бағдарламаны тек мына процессор архитектураларына арналған Windows нұсқаларына орнатуға болады:%n%n%1
WinVersionTooLowError=Бұл бағдарламаға %1 %2 нұсқасы немесе одан кейінгісі қажет.
WinVersionTooHighError=Бұл бағдарламаны %1 %2 нұсқасына немесе одан кейінгісіне орнатуға болмайды.
AdminPrivilegesRequired=Бұл бағдарламаны орнату кезінде әкімші ретінде жүйеге кірген болуыңыз керек.
PowerUserPrivilegesRequired=Бұл бағдарламаны орнату кезінде әкімші ретінде немесе Power Users тобының мүшесі ретінде жүйеге кірген болуыңыз керек.
SetupAppRunningError=Орнату %1 бағдарламасының қазір жұмыс істеп тұрғанын анықтады.%n%nОның барлық данасын жауып, жалғастыру үшін «ОК» түймесін немесе шығу үшін «Бас тарту» түймесін басыңыз.
UninstallAppRunningError=Жою %1 бағдарламасының қазір жұмыс істеп тұрғанын анықтады.%n%nОның барлық данасын жауып, жалғастыру үшін «ОК» түймесін немесе шығу үшін «Бас тарту» түймесін басыңыз.

; *** Startup questions
PrivilegesRequiredOverrideTitle=Орнату режимін таңдаңыз
PrivilegesRequiredOverrideInstruction=Орнату режимін таңдаңыз
PrivilegesRequiredOverrideText1=%1 бағдарламасын барлық пайдаланушылар үшін (әкімші құқықтары қажет) немесе тек өзіңіз үшін орнатуға болады.
PrivilegesRequiredOverrideText2=%1 бағдарламасын тек өзіңіз үшін немесе барлық пайдаланушылар үшін (әкімші құқықтары қажет) орнатуға болады.
PrivilegesRequiredOverrideAllUsers=Барлық пайдаланушылар үшін орнату
PrivilegesRequiredOverrideAllUsersRecommended=Барлық пайдаланушылар үшін орнату (ұсынылады)
PrivilegesRequiredOverrideCurrentUser=Тек мен үшін орнату
PrivilegesRequiredOverrideCurrentUserRecommended=Тек мен үшін орнату (ұсынылады)

; *** Misc. errors
ErrorCreatingDir=Орнату "%1" қалтасын жасай алмады
ErrorTooManyFilesInDir="%1" қалтасында файл жасау мүмкін болмады, себебі онда файлдар тым көп

; *** Setup common messages
ExitSetupTitle=Орнатудан шығу
ExitSetupMessage=Орнату аяқталмады. Қазір шықсаңыз, бағдарлама орнатылмайды.%n%nОрнатуды аяқтау үшін Орнатуды кейінірек қайта іске қосуға болады.%n%nОрнатудан шығасыз ба?
AboutSetupMenuItem=Орнату туралы...
AboutSetupTitle=Орнату туралы
AboutSetupMessage=%1 %2 нұсқасы%n%3%n%n%1 басты беті:%n%4
AboutSetupNote=
TranslatorNote=

; *** Buttons
ButtonBack=< Артқа
ButtonNext=Келесі >
ButtonInstall=Орнату
ButtonOK=ОК
ButtonCancel=Бас тарту
ButtonYes=Иә
ButtonYesToAll=Барлығына иә
ButtonNo=Жоқ
ButtonNoToAll=Барлығына жоқ
ButtonFinish=Аяқтау
ButtonBrowse=Шолу...
ButtonWizardBrowse=Шолу...
ButtonNewFolder=Жаңа қалта жасау

; *** "Select Language" dialog messages
SelectLanguageTitle=Орнату тілін таңдаңыз
SelectLanguageLabel=Орнату кезінде қолданылатын тілді таңдаңыз.

; *** Common wizard text
ClickNext=Жалғастыру үшін «Келесі» түймесін немесе Орнатудан шығу үшін «Бас тарту» түймесін басыңыз.
BeveledLabel=
BrowseDialogTitle=Қалтаны шолу
BrowseDialogLabel=Төмендегі тізімнен қалтаны таңдап, «ОК» түймесін басыңыз.
NewFolderName=Жаңа қалта

; *** "Welcome" wizard page
WelcomeLabel1=[name] орнату шеберіне қош келдіңіз
WelcomeLabel2=Бұл [name/ver] бағдарламасын компьютеріңізге орнатады.%n%nЖалғастырмас бұрын барлық басқа бағдарламаларды жабу ұсынылады.

; *** "Password" wizard page
WizardPassword=Құпия сөз
PasswordLabel1=Бұл орнату құпия сөзбен қорғалған.
PasswordLabel3=Құпия сөзді енгізіп, жалғастыру үшін «Келесі» түймесін басыңыз. Құпия сөздер регистрді ескереді.
PasswordEditLabel=Құпия сөз:
IncorrectPassword=Енгізілген құпия сөз дұрыс емес. Қайталап көріңіз.

; *** "License Agreement" wizard page
WizardLicense=Лицензиялық келісім
LicenseLabel=Жалғастырмас бұрын мына маңызды ақпаратты оқып шығыңыз.
LicenseLabel3=Мына лицензиялық келісімді оқып шығыңыз. Орнатуды жалғастырмас бұрын келісім шарттарын қабылдауыңыз керек.
LicenseAccepted=Келісімді қабылдаймын
LicenseNotAccepted=Келісімді қабылдамаймын

; *** "Information" wizard pages
WizardInfoBefore=Ақпарат
InfoBeforeLabel=Жалғастырмас бұрын мына маңызды ақпаратты оқып шығыңыз.
InfoBeforeClickLabel=Орнатуды жалғастыруға дайын болғанда «Келесі» түймесін басыңыз.
WizardInfoAfter=Ақпарат
InfoAfterLabel=Жалғастырмас бұрын мына маңызды ақпаратты оқып шығыңыз.
InfoAfterClickLabel=Орнатуды жалғастыруға дайын болғанда «Келесі» түймесін басыңыз.

; *** "User Information" wizard page
WizardUserInfo=Пайдаланушы туралы ақпарат
UserInfoDesc=Деректеріңізді енгізіңіз.
UserInfoName=Пайдаланушы аты:
UserInfoOrg=Ұйым:
UserInfoSerial=Сериялық нөмір:
UserInfoNameRequired=Атыңызды енгізуіңіз керек.

; *** "Select Destination Location" wizard page
WizardSelectDir=Орнату орнын таңдаңыз
SelectDirDesc=[name] қайда орнатылсын?
SelectDirLabel3=Орнату [name] бағдарламасын мына қалтаға орнатады.
SelectDirBrowseLabel=Жалғастыру үшін «Келесі» түймесін басыңыз. Басқа қалтаны таңдағыңыз келсе, «Шолу» түймесін басыңыз.
DiskSpaceGBLabel=Кемінде [gb] ГБ бос дискі орны қажет.
DiskSpaceMBLabel=Кемінде [mb] МБ бос дискі орны қажет.
CannotInstallToNetworkDrive=Орнату желілік дискіге орната алмайды.
CannotInstallToUNCPath=Орнату UNC жолына орната алмайды.
InvalidPath=Диск әрпімен толық жолды енгізуіңіз керек, мысалы:%n%nC:\APP%n%nнемесе мына түрдегі UNC жолын:%n%n\\server\share
InvalidDrive=Таңдалған диск немесе UNC ортақ қалтасы жоқ немесе қолжетімсіз. Басқасын таңдаңыз.
DiskSpaceWarningTitle=Дискіде орын жеткіліксіз
DiskSpaceWarning=Орнату үшін кемінде %1 КБ бос орын қажет, ал таңдалған дискіде тек %2 КБ бар.%n%nСонда да жалғастырасыз ба?
DirNameTooLong=Қалта атауы немесе жолы тым ұзын.
InvalidDirName=Қалта атауы жарамсыз.
BadDirName32=Қалта атауларында мына таңбалар болмауы керек:%n%n%1
DirExistsTitle=Қалта бар
DirExists=Мына қалта:%n%n%1%n%nәлдеқашан бар. Сонда да осы қалтаға орнатқыңыз келе ме?
DirDoesntExistTitle=Қалта жоқ
DirDoesntExist=Мына қалта:%n%n%1%n%nжоқ. Қалтаны жасағыңыз келе ме?

; *** "Select Components" wizard page
WizardSelectComponents=Құрамдастарды таңдаңыз
SelectComponentsDesc=Қандай құрамдастар орнатылсын?
SelectComponentsLabel2=Орнатқыңыз келетін құрамдастарды таңдаңыз; орнатқыңыз келмейтіндердің құсбелгісін алып тастаңыз. Дайын болғанда «Келесі» түймесін басыңыз.
FullInstallation=Толық орнату
; if possible don't translate 'Compact' as 'Minimal' (I mean 'Minimal' in your language)
CompactInstallation=Ықшам орнату
CustomInstallation=Таңдау бойынша орнату
NoUninstallWarningTitle=Құрамдастар бар
NoUninstallWarning=Орнату мына құрамдастардың компьютеріңізде әлдеқашан орнатылғанын анықтады:%n%n%1%n%nОлардың құсбелгісін алып тастау оларды жоймайды.%n%nСонда да жалғастырасыз ба?
ComponentSize1=%1 КБ
ComponentSize2=%1 МБ
ComponentsDiskSpaceGBLabel=Ағымдағы таңдау үшін кемінде [gb] ГБ дискі орны қажет.
ComponentsDiskSpaceMBLabel=Ағымдағы таңдау үшін кемінде [mb] МБ дискі орны қажет.

; *** "Select Additional Tasks" wizard page
WizardSelectTasks=Қосымша тапсырмаларды таңдаңыз
SelectTasksDesc=Қандай қосымша тапсырмалар орындалсын?
SelectTasksLabel2=[name] орнату кезінде Орнату орындауы тиіс қосымша тапсырмаларды таңдап, «Келесі» түймесін басыңыз.

; *** "Select Start Menu Folder" wizard page
WizardSelectProgramGroup=«Бастау» мәзірінің қалтасын таңдаңыз
SelectStartMenuFolderDesc=Орнату бағдарламаның төте жолдарын қайда орналастырсын?
SelectStartMenuFolderLabel3=Орнату бағдарламаның төте жолдарын «Бастау» мәзірінің мына қалтасында жасайды.
SelectStartMenuFolderBrowseLabel=Жалғастыру үшін «Келесі» түймесін басыңыз. Басқа қалтаны таңдағыңыз келсе, «Шолу» түймесін басыңыз.
MustEnterGroupName=Қалта атауын енгізуіңіз керек.
GroupNameTooLong=Қалта атауы немесе жолы тым ұзын.
InvalidGroupName=Қалта атауы жарамсыз.
BadGroupName=Қалта атауында мына таңбалар болмауы керек:%n%n%1
NoProgramGroupCheck2=«Бастау» мәзірінің қалтасын жасамау

; *** "Ready to Install" wizard page
WizardReady=Орнатуға дайын
ReadyLabel1=Орнату [name] бағдарламасын компьютеріңізге орнатуға дайын.
ReadyLabel2a=Орнатуды жалғастыру үшін «Орнату» түймесін басыңыз немесе параметрлерді қарап шығу не өзгерту үшін «Артқа» түймесін басыңыз.
ReadyLabel2b=Орнатуды жалғастыру үшін «Орнату» түймесін басыңыз.
ReadyMemoUserInfo=Пайдаланушы туралы ақпарат:
ReadyMemoDir=Орнату орны:
ReadyMemoType=Орнату түрі:
ReadyMemoComponents=Таңдалған құрамдастар:
ReadyMemoGroup=«Бастау» мәзірінің қалтасы:
ReadyMemoTasks=Қосымша тапсырмалар:

; *** TDownloadWizardPage wizard page and DownloadTemporaryFile
DownloadingLabel2=Файлдар жүктелуде...
ButtonStopDownload=Жүктеуді тоқтату
StopDownload=Жүктеуді тоқтатқыңыз келетініне сенімдісіз бе?
ErrorDownloadAborted=Жүктеу тоқтатылды
ErrorDownloadFailed=Жүктеу сәтсіз аяқталды: %1 %2
ErrorDownloadSizeFailed=Өлшемді алу сәтсіз аяқталды: %1 %2
ErrorProgress=Жарамсыз барыс: %2 ішінен %1
ErrorFileSize=Жарамсыз файл өлшемі: күтілгені %1, табылғаны %2

; *** TExtractionWizardPage wizard page and ExtractArchive
ExtractingLabel=Файлдар шығарылуда...
ButtonStopExtraction=Шығаруды тоқтату
StopExtraction=Шығаруды тоқтатқыңыз келетініне сенімдісіз бе?
ErrorExtractionAborted=Шығару тоқтатылды
ErrorExtractionFailed=Шығару сәтсіз аяқталды: %1

; *** Archive extraction failure details
ArchiveIncorrectPassword=Құпия сөз дұрыс емес
ArchiveIsCorrupted=Мұрағат бүлінген
ArchiveUnsupportedFormat=Мұрағат пішіміне қолдау көрсетілмейді

; *** "Preparing to Install" wizard page
WizardPreparing=Орнатуға дайындалу
PreparingDesc=Орнату [name] бағдарламасын компьютеріңізге орнатуға дайындалуда.
PreviousInstallNotCompleted=Алдыңғы бағдарламаны орнату/жою аяқталмады. Ол орнатуды аяқтау үшін компьютерді қайта іске қосу керек.%n%nКомпьютерді қайта іске қосқаннан кейін [name] орнатуын аяқтау үшін Орнатуды қайта іске қосыңыз.
CannotContinue=Орнатуды жалғастыру мүмкін емес. Шығу үшін «Бас тарту» түймесін басыңыз.
ApplicationsFound=Мына бағдарламалар Орнату жаңартуы тиіс файлдарды пайдаланып тұр. Орнатуға осы бағдарламаларды автоматты түрде жабуға рұқсат беру ұсынылады.
ApplicationsFound2=Мына бағдарламалар Орнату жаңартуы тиіс файлдарды пайдаланып тұр. Орнатуға осы бағдарламаларды автоматты түрде жабуға рұқсат беру ұсынылады. Орнату аяқталғаннан кейін Орнату бағдарламаларды қайта іске қосуға тырысады.
CloseApplications=Бағдарламаларды автоматты түрде жабу
DontCloseApplications=Бағдарламаларды жаппау
ErrorCloseApplications=Орнату барлық бағдарламаны автоматты түрде жаба алмады. Жалғастырмас бұрын Орнату жаңартуы тиіс файлдарды пайдаланатын барлық бағдарламаны жабу ұсынылады.
PrepareToInstallNeedsRestart=Орнату компьютерді қайта іске қосуы керек. Компьютерді қайта іске қосқаннан кейін [name] орнатуын аяқтау үшін Орнатуды қайта іске қосыңыз.%n%nҚазір қайта іске қосқыңыз келе ме?

; *** "Installing" wizard page
WizardInstalling=Орнату
InstallingLabel=Орнату [name] бағдарламасын компьютеріңізге орнатып жатқанда күте тұрыңыз.

; *** "Setup Completed" wizard page
FinishedHeadingLabel=[name] орнату шеберін аяқтау
FinishedLabelNoIcons=Орнату [name] бағдарламасын компьютеріңізге орнатуды аяқтады.
FinishedLabel=Орнату [name] бағдарламасын компьютеріңізге орнатуды аяқтады. Бағдарламаны орнатылған төте жолдар арқылы іске қосуға болады.
ClickFinish=Орнатудан шығу үшін «Аяқтау» түймесін басыңыз.
FinishedRestartLabel=[name] орнатуын аяқтау үшін Орнату компьютерді қайта іске қосуы керек. Қазір қайта іске қосқыңыз келе ме?
FinishedRestartMessage=[name] орнатуын аяқтау үшін Орнату компьютерді қайта іске қосуы керек.%n%nҚазір қайта іске қосқыңыз келе ме?
ShowReadmeCheck=Иә, README файлын көргім келеді
YesRadio=Иә, компьютерді қазір қайта іске қосу
NoRadio=Жоқ, компьютерді кейінірек қайта іске қосамын
; used for example as 'Run MyProg.exe'
RunEntryExec=%1 іске қосу
; used for example as 'View Readme.txt'
RunEntryShellExec=%1 көру

; *** "Setup Needs the Next Disk" stuff
ChangeDiskTitle=Орнатуға келесі диск қажет
SelectDiskLabel2=%1 дискісін салып, «ОК» түймесін басыңыз.%n%nЕгер осы дисктегі файлдарды төменде көрсетілгеннен басқа қалтадан табуға болса, дұрыс жолды енгізіңіз немесе «Шолу» түймесін басыңыз.
PathLabel=Жол:
FileNotInDir2="%1" файлы "%2" ішінен табылмады. Дұрыс дискіні салыңыз немесе басқа қалтаны таңдаңыз.
SelectDirectoryLabel=Келесі дискінің орнын көрсетіңіз.

; *** Installation phase messages
SetupAborted=Орнату аяқталмады.%n%nМәселені түзетіп, Орнатуды қайта іске қосыңыз.
AbortRetryIgnoreSelectAction=Әрекетті таңдаңыз
AbortRetryIgnoreRetry=Қайталап көру
AbortRetryIgnoreIgnore=Қатені елемей жалғастыру
AbortRetryIgnoreCancel=Орнатуды тоқтату
RetryCancelSelectAction=Әрекетті таңдаңыз
RetryCancelRetry=Қайталап көру
RetryCancelCancel=Бас тарту

; *** Installation status messages
StatusClosingApplications=Бағдарламалар жабылуда...
StatusCreateDirs=Қалталар жасалуда...
StatusExtractFiles=Файлдар шығарылуда...
StatusDownloadFiles=Файлдар жүктелуде...
StatusCreateIcons=Төте жолдар жасалуда...
StatusCreateIniEntries=INI жазбалары жасалуда...
StatusCreateRegistryEntries=Тізілім жазбалары жасалуда...
StatusRegisterFiles=Файлдар тіркелуде...
StatusSavingUninstall=Жою туралы ақпарат сақталуда...
StatusRunProgram=Орнату аяқталуда...
StatusRestartingApplications=Бағдарламалар қайта іске қосылуда...
StatusRollback=Өзгерістер кері қайтарылуда...

; *** Misc. errors
ErrorInternal2=Ішкі қате: %1
ErrorFunctionFailedNoCode=%1 сәтсіз аяқталды
ErrorFunctionFailed=%1 сәтсіз аяқталды; коды %2
ErrorFunctionFailedWithMessage=%1 сәтсіз аяқталды; коды %2.%n%3
ErrorExecutingProgram=Файлды іске қосу мүмкін болмады:%n%1

; *** Registry errors
ErrorRegOpenKey=Тізілім кілтін ашу қатесі:%n%1\%2
ErrorRegCreateKey=Тізілім кілтін жасау қатесі:%n%1\%2
ErrorRegWriteKey=Тізілім кілтіне жазу қатесі:%n%1\%2

; *** INI errors
ErrorIniEntry="%1" файлында INI жазбасын жасау қатесі.

; *** File copying errors
FileAbortRetryIgnoreSkipNotRecommended=Бұл файлды өткізіп жіберу (ұсынылмайды)
FileAbortRetryIgnoreIgnoreNotRecommended=Қатені елемей жалғастыру (ұсынылмайды)
SourceIsCorrupted=Бастапқы файл бүлінген
SourceDoesntExist=Бастапқы "%1" файлы жоқ
SourceVerificationFailed=Бастапқы файлды тексеру сәтсіз аяқталды: %1
VerificationSignatureDoesntExist="%1" қолтаңба файлы жоқ
VerificationSignatureInvalid="%1" қолтаңба файлы жарамсыз
VerificationKeyNotFound="%1" қолтаңба файлы белгісіз кілтті пайдаланады
VerificationFileNameIncorrect=Файл атауы дұрыс емес
VerificationFileTagIncorrect=Файл тегі дұрыс емес
VerificationFileSizeIncorrect=Файл өлшемі дұрыс емес
VerificationFileHashIncorrect=Файл хэші дұрыс емес
ExistingFileReadOnly2=Бар файлды ауыстыру мүмкін болмады, себебі ол тек оқуға арналған деп белгіленген.
ExistingFileReadOnlyRetry=Тек оқу төлсипатын алып тастап, қайталап көру
ExistingFileReadOnlyKeepExisting=Бар файлды сақтау
ErrorReadingExistingDest=Бар файлды оқу кезінде қате болды:
FileExistsSelectAction=Әрекетті таңдаңыз
FileExists2=Файл әлдеқашан бар.
FileExistsOverwriteExisting=Бар файлды қайта жазу
FileExistsKeepExisting=Бар файлды сақтау
FileExistsOverwriteOrKeepAll=Келесі қайшылықтар үшін де солай істеу
ExistingFileNewerSelectAction=Әрекетті таңдаңыз
ExistingFileNewer2=Бар файл Орнату орнатқысы келген файлдан жаңарақ.
ExistingFileNewerOverwriteExisting=Бар файлды қайта жазу
ExistingFileNewerKeepExisting=Бар файлды сақтау (ұсынылады)
ExistingFileNewerOverwriteOrKeepAll=Келесі қайшылықтар үшін де солай істеу
ErrorChangingAttr=Бар файлдың төлсипаттарын өзгерту кезінде қате болды:
ErrorCreatingTemp=Мақсатты қалтада файл жасау кезінде қате болды:
ErrorReadingSource=Бастапқы файлды оқу кезінде қате болды:
ErrorCopying=Файлды көшіру кезінде қате болды:
ErrorDownloading=Файлды жүктеу кезінде қате болды:
ErrorExtracting=Мұрағатты шығару кезінде қате болды:
ErrorReplacingExistingFile=Бар файлды ауыстыру кезінде қате болды:
ErrorRestartReplace=RestartReplace сәтсіз аяқталды:
ErrorRenamingTemp=Мақсатты қалтадағы файлдың атын өзгерту кезінде қате болды:
ErrorRegisterServer=DLL/OCX тіркеу мүмкін болмады: %1
ErrorRegSvr32Failed=RegSvr32 %1 шығу кодымен сәтсіз аяқталды
ErrorRegisterTypeLib=Түр кітапханасын тіркеу мүмкін болмады: %1

; *** Uninstall display name markings
; used for example as 'My Program (32-bit)'
UninstallDisplayNameMark=%1 (%2)
; used for example as 'My Program (32-bit, All users)'
UninstallDisplayNameMarks=%1 (%2, %3)
UninstallDisplayNameMark32Bit=32 биттік
UninstallDisplayNameMark64Bit=64 биттік
UninstallDisplayNameMarkAllUsers=Барлық пайдаланушылар
UninstallDisplayNameMarkCurrentUser=Ағымдағы пайдаланушы

; *** Post-installation errors
ErrorOpeningReadme=README файлын ашу кезінде қате болды.
ErrorRestartingComputer=Орнату компьютерді қайта іске қоса алмады. Мұны қолмен жасаңыз.

; *** Uninstaller messages
UninstallNotFound="%1" файлы жоқ. Жою мүмкін емес.
UninstallOpenError="%1" файлын ашу мүмкін болмады. Жою мүмкін емес
UninstallUnsupportedVer="%1" жою журналы файлы Жоюдың осы нұсқасы танымайтын пішімде. Жою мүмкін емес
UninstallUnknownEntry=Жою журналында белгісіз жазба (%1) кездесті
ConfirmUninstall=%1 бағдарламасын және оның барлық құрамдастарын толық жойғыңыз келетініне сенімдісіз бе?
UninstallOnlyOnWin64=Бұл орнатуды тек 64 биттік Windows жүйесінде жоюға болады.
OnlyAdminCanUninstall=Бұл орнатуды тек әкімші құқықтары бар пайдаланушы жоя алады.
UninstallStatusLabel=%1 компьютеріңізден жойылып жатқанда күте тұрыңыз.
UninstalledAll=%1 компьютеріңізден сәтті жойылды.
UninstalledMost=%1 жою аяқталды.%n%nКейбір элементтерді жою мүмкін болмады. Оларды қолмен жоюға болады.
UninstalledAndNeedsRestart=%1 жоюды аяқтау үшін компьютерді қайта іске қосу керек.%n%nҚазір қайта іске қосқыңыз келе ме?
UninstallDataCorrupted="%1" файлы бүлінген. Жою мүмкін емес

; *** Uninstallation phase messages
ConfirmDeleteSharedFileTitle=Ортақ файл жойылсын ба?
ConfirmDeleteSharedFile2=Жүйе мына ортақ файлды енді ешбір бағдарлама пайдаланбайтынын көрсетуде. Жою осы ортақ файлды жойсын ба?%n%nЕгер қандай да бір бағдарлама бұл файлды әлі пайдаланып жатса және ол жойылса, ол бағдарламалар дұрыс жұмыс істемеуі мүмкін. Сенімді болмасаңыз, «Жоқ» түймесін басыңыз. Файлды жүйеде қалдыру зиян келтірмейді.
SharedFileNameLabel=Файл атауы:
SharedFileLocationLabel=Орны:
WizardUninstalling=Жою күйі
StatusUninstalling=%1 жойылуда...

; *** Shutdown block reasons
ShutdownBlockReasonInstallingApp=%1 орнатылуда.
ShutdownBlockReasonUninstallingApp=%1 жойылуда.

; The custom messages below aren't used by Setup itself, but if you make
; use of them in your scripts, you'll want to translate them.

[CustomMessages]

NameAndVersion=%1 %2 нұсқасы
AdditionalIcons=Қосымша төте жолдар:
CreateDesktopIcon=Жұмыс үстелінде төте жол жасау
CreateQuickLaunchIcon=Жылдам іске қосу төте жолын жасау
ProgramOnTheWeb=%1 — вебте
UninstallProgram=%1 жою
LaunchProgram=%1 іске қосу
AssocFileExtension=%1 бағдарламасын %2 файл кеңейтімімен байланыстыру
AssocingFileExtension=%1 бағдарламасы %2 файл кеңейтімімен байланыстырылуда...
AutoStartProgramGroupDescription=Автоіске қосу:
AutoStartProgram=%1 бағдарламасын автоматты түрде іске қосу
AddonHostProgramNotFound=%1 таңдалған қалтада табылмады.%n%nСонда да жалғастырасыз ба?

; The messages that are Lang Switcher's own (the sample is English.isl).
LocalCertReady=Орнату осы компьютерде сертификат жасап, онымен Lang Switcher бағдарламасына қол қояды, сонда бағдарлама әкімші атынан іске қосылған бағдарламалардың терезелерінде де жұмыс істей алады.
LocalCertFailed=Lang Switcher орнатылды, бірақ әкімші атынан іске қосылған бағдарламалардың терезелерінде жұмыс істемейді: Windows осы компьютерде жасалған сертификатты қабылдамады. Толығырақ ақпарат орнату журналында.
RemoveSettingsQuestion=Lang Switcher параметрлер файлын да жою керек пе?
RemoveSettingsKeep=Қайта орнату үшін параметрлеріңізді сақтау керек болса, «Жоқ» түймесін басыңыз.
AuthorSlogan=жүрекпен жасалған бағдарламалар
