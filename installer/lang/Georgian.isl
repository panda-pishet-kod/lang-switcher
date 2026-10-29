; Lang Switcher -- the messages of the installer in Georgian (stage E108, group A).
;
; Inno Setup has no official Georgian translation (inno_official = false in res\lang\languages.toml),
; so this file carries the whole of it: [LangOptions], every message of the compiler:Default.isl
; (6.5.0+), and the messages of the installer that are Lang Switcher's own. It is read after
; compiler:Default.isl as the second MessagesFile of the "georgian" entry of [Languages] in
; ..\LangSwitcher.iss, and so replaces the English text name for name.
; English.isl is the sample the own messages translate; the glossary of the language is
; l10n\glossary\ka.tsv of the control contour. The name of the program is not translated.
; As in Default.isl: a message that has no full stop there has none here (Setup adds its own).
; No "&" accelerators (RULES 16): the names of the buttons inside a text stand in the quotes „ “.

[LangOptions]
LanguageName=ქართული
LanguageID=$0437
LanguageCodePage=0

[Messages]

; *** Application titles
SetupAppTitle=ინსტალაცია
SetupWindowTitle=ინსტალაცია - %1
UninstallAppTitle=დეინსტალაცია
UninstallAppFullTitle=%1 — დეინსტალაცია

; *** Misc. common
InformationTitle=ინფორმაცია
ConfirmTitle=დადასტურება
ErrorTitle=შეცდომა

; *** SetupLdr messages
SetupLdrStartupMessage=დაინსტალირდება %1. გსურთ გაგრძელება?
LdrCannotCreateTemp=დროებითი ფაილის შექმნა ვერ მოხერხდა. ინსტალაცია შეწყდა
LdrCannotExecTemp=დროებით საქაღალდეში ფაილის გაშვება ვერ მოხერხდა. ინსტალაცია შეწყდა
HelpTextNote=

; *** Startup error messages
LastErrorMessage=%1.%n%nშეცდომა %2: %3
SetupFileMissing=ინსტალაციის საქაღალდეში ფაილი %1 აღარ არის. გაასწორეთ პრობლემა ან მოიძიეთ პროგრამის ახალი ასლი.
SetupFileCorrupt=ინსტალაციის ფაილები დაზიანებულია. მოიძიეთ პროგრამის ახალი ასლი.
SetupFileCorruptOrWrongVer=ინსტალაციის ფაილები დაზიანებულია ან ინსტალაციის ამ ვერსიას არ ერგება. გაასწორეთ პრობლემა ან მოიძიეთ პროგრამის ახალი ასლი.
InvalidParameter=ბრძანების ხაზში გადაცემულია არასწორი პარამეტრი:%n%n%1
SetupAlreadyRunning=ინსტალაცია უკვე გაშვებულია.
WindowsVersionNotSupported=ეს პროგრამა თქვენს კომპიუტერზე დაყენებული Windows-ის ვერსიას არ უჭერს მხარს.
WindowsServicePackRequired=ამ პროგრამას სჭირდება %1 Service Pack %2 ან უფრო ახალი.
NotOnThisPlatform=ეს პროგრამა %1-ზე არ იმუშავებს.
OnlyOnThisPlatform=ეს პროგრამა უნდა გაეშვას %1-ზე.
OnlyOnTheseArchitectures=ეს პროგრამა შეიძლება დაინსტალირდეს მხოლოდ Windows-ის იმ ვერსიებზე, რომლებიც შექმნილია შემდეგი პროცესორის არქიტექტურებისთვის:%n%n%1
WinVersionTooLowError=ამ პროგრამას სჭირდება %1 ვერსია %2 ან უფრო ახალი.
WinVersionTooHighError=ეს პროგრამა ვერ დაინსტალირდება %1 ვერსია %2-ზე ან უფრო ახალზე.
AdminPrivilegesRequired=ამ პროგრამის დასაინსტალირებლად ადმინისტრატორის სახელით უნდა იყოთ შესული.
PowerUserPrivilegesRequired=ამ პროგრამის დასაინსტალირებლად უნდა იყოთ შესული ადმინისტრატორის სახელით ან Power Users ჯგუფის წევრად.
SetupAppRunningError=ინსტალაციამ აღმოაჩინა, რომ %1 ამჟამად გაშვებულია.%n%nდახურეთ მისი ყველა ეგზემპლარი, შემდეგ დააჭირეთ „OK“ გასაგრძელებლად ან „გაუქმება“ გასასვლელად.
UninstallAppRunningError=დეინსტალაციამ აღმოაჩინა, რომ %1 ამჟამად გაშვებულია.%n%nდახურეთ მისი ყველა ეგზემპლარი, შემდეგ დააჭირეთ „OK“ გასაგრძელებლად ან „გაუქმება“ გასასვლელად.

; *** Startup questions
PrivilegesRequiredOverrideTitle=ინსტალაციის რეჟიმის არჩევა
PrivilegesRequiredOverrideInstruction=აირჩიეთ ინსტალაციის რეჟიმი
PrivilegesRequiredOverrideText1=%1 შეიძლება დაინსტალირდეს ყველა მომხმარებლისთვის (საჭიროა ადმინისტრატორის უფლებები) ან მხოლოდ თქვენთვის.
PrivilegesRequiredOverrideText2=%1 შეიძლება დაინსტალირდეს მხოლოდ თქვენთვის ან ყველა მომხმარებლისთვის (საჭიროა ადმინისტრატორის უფლებები).
PrivilegesRequiredOverrideAllUsers=ყველა მომხმარებლისთვის
PrivilegesRequiredOverrideAllUsersRecommended=ყველა მომხმარებლისთვის (რეკომენდებულია)
PrivilegesRequiredOverrideCurrentUser=მხოლოდ ჩემთვის
PrivilegesRequiredOverrideCurrentUserRecommended=მხოლოდ ჩემთვის (რეკომენდებულია)

; *** Misc. errors
ErrorCreatingDir=საქაღალდის „%1“ შექმნა ვერ მოხერხდა
ErrorTooManyFilesInDir=საქაღალდეში „%1“ ფაილის შექმნა ვერ მოხერხდა, რადგან მასში ძალიან ბევრი ფაილია

; *** Setup common messages
ExitSetupTitle=ინსტალაციიდან გასვლა
ExitSetupMessage=ინსტალაცია არ დასრულებულა. თუ ახლა გახვალთ, პროგრამა არ დაინსტალირდება.%n%nინსტალაციის დასასრულებლად მოგვიანებით შეგიძლიათ ხელახლა გაუშვათ.%n%nგსურთ ინსტალაციიდან გასვლა?
AboutSetupMenuItem=ინსტალაციის შესახებ...
AboutSetupTitle=ინსტალაციის შესახებ
AboutSetupMessage=%1, ვერსია %2%n%3%n%n%1 — ვებგვერდი:%n%4
AboutSetupNote=
TranslatorNote=

; *** Buttons
ButtonBack=< უკან
ButtonNext=შემდეგი >
ButtonInstall=ინსტალაცია
ButtonOK=OK
ButtonCancel=გაუქმება
ButtonYes=დიახ
ButtonYesToAll=დიახ, ყველასთვის
ButtonNo=არა
ButtonNoToAll=არა, ყველასთვის
ButtonFinish=დასრულება
ButtonBrowse=დათვალიერება...
ButtonWizardBrowse=დათვალიერება...
ButtonNewFolder=ახალი საქაღალდე

; *** "Select Language" dialog messages
SelectLanguageTitle=ინსტალაციის ენის არჩევა
SelectLanguageLabel=აირჩიეთ ენა, რომელსაც ინსტალაციისას გამოიყენებთ.

; *** Common wizard text
ClickNext=გასაგრძელებლად დააჭირეთ „შემდეგი“, ინსტალაციიდან გასასვლელად — „გაუქმება“.
BeveledLabel=
BrowseDialogTitle=საქაღალდის არჩევა
BrowseDialogLabel=აირჩიეთ საქაღალდე ქვემოთ მოცემული სიიდან, შემდეგ დააჭირეთ „OK“.
NewFolderName=ახალი საქაღალდე

; *** "Welcome" wizard page
WelcomeLabel1=მოგესალმებათ [name]-ის ინსტალაციის ოსტატი
WelcomeLabel2=თქვენს კომპიუტერზე დაინსტალირდება [name/ver].%n%nგაგრძელებამდე რეკომენდებულია დახუროთ ყველა სხვა პროგრამა.

; *** "Password" wizard page
WizardPassword=პაროლი
PasswordLabel1=ეს ინსტალაცია პაროლით არის დაცული.
PasswordLabel3=შეიყვანეთ პაროლი, შემდეგ გასაგრძელებლად დააჭირეთ „შემდეგი“. პაროლში მნიშვნელობა აქვს დიდ და პატარა ასოებს.
PasswordEditLabel=პაროლი:
IncorrectPassword=შეყვანილი პაროლი არასწორია. სცადეთ ხელახლა.

; *** "License Agreement" wizard page
WizardLicense=სალიცენზიო შეთანხმება
LicenseLabel=გაგრძელებამდე წაიკითხეთ შემდეგი მნიშვნელოვანი ინფორმაცია.
LicenseLabel3=წაიკითხეთ სალიცენზიო შეთანხმება. ინსტალაციის გასაგრძელებლად უნდა დაეთანხმოთ ამ შეთანხმების პირობებს.
LicenseAccepted=ვეთანხმები შეთანხმებას
LicenseNotAccepted=არ ვეთანხმები შეთანხმებას

; *** "Information" wizard pages
WizardInfoBefore=ინფორმაცია
InfoBeforeLabel=გაგრძელებამდე წაიკითხეთ შემდეგი მნიშვნელოვანი ინფორმაცია.
InfoBeforeClickLabel=როცა მზად იქნებით ინსტალაციის გასაგრძელებლად, დააჭირეთ „შემდეგი“.
WizardInfoAfter=ინფორმაცია
InfoAfterLabel=გაგრძელებამდე წაიკითხეთ შემდეგი მნიშვნელოვანი ინფორმაცია.
InfoAfterClickLabel=როცა მზად იქნებით ინსტალაციის გასაგრძელებლად, დააჭირეთ „შემდეგი“.

; *** "User Information" wizard page
WizardUserInfo=მომხმარებლის ინფორმაცია
UserInfoDesc=შეიყვანეთ თქვენი მონაცემები.
UserInfoName=მომხმარებლის სახელი:
UserInfoOrg=ორგანიზაცია:
UserInfoSerial=სერიული ნომერი:
UserInfoNameRequired=სახელის შეყვანა აუცილებელია.

; *** "Select Destination Location" wizard page
WizardSelectDir=დანიშნულების ადგილის არჩევა
SelectDirDesc=სად უნდა დაინსტალირდეს [name]?
SelectDirLabel3=[name] დაინსტალირდება შემდეგ საქაღალდეში.
SelectDirBrowseLabel=გასაგრძელებლად დააჭირეთ „შემდეგი“. სხვა საქაღალდის ასარჩევად დააჭირეთ „დათვალიერება“.
DiskSpaceGBLabel=საჭიროა მინიმუმ [gb] გბ თავისუფალი ადგილი დისკზე.
DiskSpaceMBLabel=საჭიროა მინიმუმ [mb] მბ თავისუფალი ადგილი დისკზე.
CannotInstallToNetworkDrive=ინსტალაცია ქსელურ დისკზე შეუძლებელია.
CannotInstallToUNCPath=ინსტალაცია UNC გზაზე შეუძლებელია.
InvalidPath=მიუთითეთ სრული გზა დისკის ასოთი, მაგალითად:%n%nC:\APP%n%nან UNC გზა ასეთი სახით:%n%n\\server\share
InvalidDrive=არჩეული დისკი ან UNC რესურსი არ არსებობს ან მიუწვდომელია. აირჩიეთ სხვა.
DiskSpaceWarningTitle=დისკზე საკმარისი ადგილი არ არის
DiskSpaceWarning=ინსტალაციას სჭირდება მინიმუმ %1 კბ თავისუფალი ადგილი, მაგრამ არჩეულ დისკზე მხოლოდ %2 კბ არის.%n%nგსურთ მაინც გაგრძელება?
DirNameTooLong=საქაღალდის სახელი ან გზა ძალიან გრძელია.
InvalidDirName=საქაღალდის სახელი არასწორია.
BadDirName32=საქაღალდის სახელში ვერ გამოიყენება შემდეგი სიმბოლოები:%n%n%1
DirExistsTitle=საქაღალდე უკვე არსებობს
DirExists=საქაღალდე:%n%n%1%n%nუკვე არსებობს. გსურთ მაინც ამ საქაღალდეში დაინსტალირება?
DirDoesntExistTitle=საქაღალდე არ არსებობს
DirDoesntExist=საქაღალდე:%n%n%1%n%nარ არსებობს. გსურთ მისი შექმნა?

; *** "Select Components" wizard page
WizardSelectComponents=კომპონენტების არჩევა
SelectComponentsDesc=რომელი კომპონენტები უნდა დაინსტალირდეს?
SelectComponentsLabel2=მონიშნეთ კომპონენტები, რომელთა დაინსტალირებაც გსურთ; მოხსენით ის კომპონენტები, რომლებიც არ გჭირდებათ. მზად რომ იქნებით, დააჭირეთ „შემდეგი“.
FullInstallation=სრული ინსტალაცია
; if possible don't translate 'Compact' as 'Minimal' (I mean 'Minimal' in your language)
CompactInstallation=კომპაქტური ინსტალაცია
CustomInstallation=არჩევითი ინსტალაცია
NoUninstallWarningTitle=კომპონენტები უკვე არსებობს
NoUninstallWarning=ინსტალაციამ აღმოაჩინა, რომ შემდეგი კომპონენტები თქვენს კომპიუტერზე უკვე დაინსტალირებულია:%n%n%1%n%nამ კომპონენტების მონიშვნის მოხსნა მათ არ წაშლის.%n%nგსურთ მაინც გაგრძელება?
ComponentSize1=%1 კბ
ComponentSize2=%1 მბ
ComponentsDiskSpaceGBLabel=არჩეულ კომპონენტებს სჭირდება მინიმუმ [gb] გბ ადგილი დისკზე.
ComponentsDiskSpaceMBLabel=არჩეულ კომპონენტებს სჭირდება მინიმუმ [mb] მბ ადგილი დისკზე.

; *** "Select Additional Tasks" wizard page
WizardSelectTasks=დამატებითი ამოცანების არჩევა
SelectTasksDesc=რომელი დამატებითი ამოცანები უნდა შესრულდეს?
SelectTasksLabel2=აირჩიეთ დამატებითი ამოცანები, რომლებიც ინსტალაციამ უნდა შეასრულოს [name]-ის დაინსტალირებისას, შემდეგ დააჭირეთ „შემდეგი“.

; *** "Select Start Menu Folder" wizard page
WizardSelectProgramGroup=დაწყების მენიუს საქაღალდის არჩევა
SelectStartMenuFolderDesc=სად უნდა შექმნას ინსტალაციამ პროგრამის მალსახმობები?
SelectStartMenuFolderLabel3=ინსტალაცია პროგრამის მალსახმობებს შექმნის დაწყების მენიუს შემდეგ საქაღალდეში.
SelectStartMenuFolderBrowseLabel=გასაგრძელებლად დააჭირეთ „შემდეგი“. სხვა საქაღალდის ასარჩევად დააჭირეთ „დათვალიერება“.
MustEnterGroupName=საქაღალდის სახელის შეყვანა აუცილებელია.
GroupNameTooLong=საქაღალდის სახელი ან გზა ძალიან გრძელია.
InvalidGroupName=საქაღალდის სახელი არასწორია.
BadGroupName=საქაღალდის სახელში ვერ გამოიყენება შემდეგი სიმბოლოები:%n%n%1
NoProgramGroupCheck2=დაწყების მენიუს საქაღალდე არ შეიქმნას

; *** "Ready to Install" wizard page
WizardReady=მზადაა ინსტალაციისთვის
ReadyLabel1=ინსტალაცია მზადაა, რომ თქვენს კომპიუტერზე დააინსტალიროს [name].
ReadyLabel2a=ინსტალაციის გასაგრძელებლად დააჭირეთ „ინსტალაცია“, ან დააჭირეთ „უკან“, თუ გსურთ პარამეტრების გადახედვა ან შეცვლა.
ReadyLabel2b=ინსტალაციის გასაგრძელებლად დააჭირეთ „ინსტალაცია“.
ReadyMemoUserInfo=მომხმარებლის ინფორმაცია:
ReadyMemoDir=დანიშნულების ადგილი:
ReadyMemoType=ინსტალაციის ტიპი:
ReadyMemoComponents=არჩეული კომპონენტები:
ReadyMemoGroup=დაწყების მენიუს საქაღალდე:
ReadyMemoTasks=დამატებითი ამოცანები:

; *** TDownloadWizardPage wizard page and DownloadTemporaryFile
DownloadingLabel2=ფაილები ჩამოიტვირთება...
ButtonStopDownload=ჩამოტვირთვის შეჩერება
StopDownload=ნამდვილად გსურთ ჩამოტვირთვის შეჩერება?
ErrorDownloadAborted=ჩამოტვირთვა შეწყდა
ErrorDownloadFailed=ჩამოტვირთვა ვერ მოხერხდა: %1 %2
ErrorDownloadSizeFailed=ზომის მიღება ვერ მოხერხდა: %1 %2
ErrorProgress=არასწორი პროგრესი: %1 / %2
ErrorFileSize=ფაილის ზომა არასწორია: მოსალოდნელი იყო %1, აღმოჩნდა %2

; *** TExtractionWizardPage wizard page and ExtractArchive
ExtractingLabel=ფაილები იხსნება...
ButtonStopExtraction=ამოხსნის შეჩერება
StopExtraction=ნამდვილად გსურთ ამოხსნის შეჩერება?
ErrorExtractionAborted=ამოხსნა შეწყდა
ErrorExtractionFailed=ამოხსნა ვერ მოხერხდა: %1

; *** Archive extraction failure details
ArchiveIncorrectPassword=პაროლი არასწორია
ArchiveIsCorrupted=არქივი დაზიანებულია
ArchiveUnsupportedFormat=არქივის ფორმატი მხარდაჭერილი არ არის

; *** "Preparing to Install" wizard page
WizardPreparing=ინსტალაციისთვის მზადება
PreparingDesc=ინსტალაცია ემზადება [name]-ის დასაინსტალირებლად თქვენს კომპიუტერზე.
PreviousInstallNotCompleted=წინა პროგრამის ინსტალაცია ან წაშლა არ დასრულებულა. მისი დასასრულებლად კომპიუტერი უნდა გადაიტვირთოს.%n%nგადატვირთვის შემდეგ ისევ გაუშვით ინსტალაცია, რომ დასრულდეს [name]-ის ინსტალაცია.
CannotContinue=ინსტალაციის გაგრძელება შეუძლებელია. გასასვლელად დააჭირეთ „გაუქმება“.
ApplicationsFound=შემდეგი პროგრამები იყენებენ ფაილებს, რომლებიც ინსტალაციამ უნდა განაახლოს. რეკომენდებულია, ინსტალაციას ნება დართოთ, ეს პროგრამები თავად დახუროს.
ApplicationsFound2=შემდეგი პროგრამები იყენებენ ფაილებს, რომლებიც ინსტალაციამ უნდა განაახლოს. რეკომენდებულია, ინსტალაციას ნება დართოთ, ეს პროგრამები თავად დახუროს. ინსტალაციის დასრულების შემდეგ ის ამ პროგრამების ხელახლა გაშვებას შეეცდება.
CloseApplications=პროგრამები თავად დაიხუროს
DontCloseApplications=პროგრამები არ დაიხუროს
ErrorCloseApplications=ინსტალაციამ ყველა პროგრამის თავისით დახურვა ვერ შეძლო. გაგრძელებამდე რეკომენდებულია, დახუროთ ყველა პროგრამა, რომელიც იყენებს ფაილებს, რომლებიც ინსტალაციამ უნდა განაახლოს.
PrepareToInstallNeedsRestart=ინსტალაციამ უნდა გადატვირთოს თქვენი კომპიუტერი. გადატვირთვის შემდეგ ისევ გაუშვით ინსტალაცია, რომ დასრულდეს [name]-ის ინსტალაცია.%n%nგსურთ ახლა გადატვირთვა?

; *** "Installing" wizard page
WizardInstalling=მიმდინარეობს ინსტალაცია
InstallingLabel=დაელოდეთ, სანამ ინსტალაცია თქვენს კომპიუტერზე დააინსტალირებს [name]-ს.

; *** "Setup Completed" wizard page
FinishedHeadingLabel=[name]-ის ინსტალაციის ოსტატის დასრულება
FinishedLabelNoIcons=ინსტალაციამ დაასრულა [name]-ის დაინსტალირება თქვენს კომპიუტერზე.
FinishedLabel=ინსტალაციამ დაასრულა [name]-ის დაინსტალირება თქვენს კომპიუტერზე. პროგრამის გაშვება შეგიძლიათ დაინსტალირებული მალსახმობებით.
ClickFinish=ინსტალაციიდან გასასვლელად დააჭირეთ „დასრულება“.
FinishedRestartLabel=[name]-ის ინსტალაციის დასასრულებლად ინსტალაციამ უნდა გადატვირთოს თქვენი კომპიუტერი. გსურთ ახლა გადატვირთვა?
FinishedRestartMessage=[name]-ის ინსტალაციის დასასრულებლად ინსტალაციამ უნდა გადატვირთოს თქვენი კომპიუტერი.%n%nგსურთ ახლა გადატვირთვა?
ShowReadmeCheck=დიახ, მსურს README ფაილის ნახვა
YesRadio=დიახ, კომპიუტერი ახლავე გადაიტვირთოს
NoRadio=არა, კომპიუტერს მოგვიანებით თავად გადავტვირთავ
; used for example as 'Run MyProg.exe'
RunEntryExec=გაშვება: %1
; used for example as 'View Readme.txt'
RunEntryShellExec=ნახვა: %1

; *** "Setup Needs the Next Disk" stuff
ChangeDiskTitle=ინსტალაციას სჭირდება შემდეგი დისკი
SelectDiskLabel2=ჩადეთ დისკი %1 და დააჭირეთ „OK“.%n%nთუ ამ დისკზე არსებული ფაილები ქვემოთ ნაჩვენებისგან განსხვავებულ საქაღალდეშია, შეიყვანეთ სწორი გზა ან დააჭირეთ „დათვალიერება“.
PathLabel=გზა:
FileNotInDir2=ფაილი „%1“ ვერ მოიძებნა საქაღალდეში „%2“. ჩადეთ სწორი დისკი ან აირჩიეთ სხვა საქაღალდე.
SelectDirectoryLabel=მიუთითეთ შემდეგი დისკის მდებარეობა.

; *** Installation phase messages
SetupAborted=ინსტალაცია არ დასრულებულა.%n%nგაასწორეთ პრობლემა და გაუშვით ინსტალაცია ხელახლა.
AbortRetryIgnoreSelectAction=აირჩიეთ მოქმედება
AbortRetryIgnoreRetry=ხელახლა ცდა
AbortRetryIgnoreIgnore=შეცდომის იგნორირება და გაგრძელება
AbortRetryIgnoreCancel=ინსტალაციის გაუქმება
RetryCancelSelectAction=აირჩიეთ მოქმედება
RetryCancelRetry=ხელახლა ცდა
RetryCancelCancel=გაუქმება

; *** Installation status messages
StatusClosingApplications=პროგრამები იხურება...
StatusCreateDirs=საქაღალდეები იქმნება...
StatusExtractFiles=ფაილები იხსნება...
StatusDownloadFiles=ფაილები ჩამოიტვირთება...
StatusCreateIcons=მალსახმობები იქმნება...
StatusCreateIniEntries=INI ჩანაწერები იქმნება...
StatusCreateRegistryEntries=რეესტრის ჩანაწერები იქმნება...
StatusRegisterFiles=ფაილები რეგისტრირდება...
StatusSavingUninstall=ინახება დეინსტალაციის ინფორმაცია...
StatusRunProgram=ინსტალაცია სრულდება...
StatusRestartingApplications=პროგრამები ხელახლა იშვება...
StatusRollback=ცვლილებები უქმდება...

; *** Misc. errors
ErrorInternal2=შიდა შეცდომა: %1
ErrorFunctionFailedNoCode=%1 ვერ შესრულდა
ErrorFunctionFailed=%1 ვერ შესრულდა; კოდი %2
ErrorFunctionFailedWithMessage=%1 ვერ შესრულდა; კოდი %2.%n%3
ErrorExecutingProgram=ფაილის გაშვება ვერ მოხერხდა:%n%1

; *** Registry errors
ErrorRegOpenKey=რეესტრის გასაღების გახსნის შეცდომა:%n%1\%2
ErrorRegCreateKey=რეესტრის გასაღების შექმნის შეცდომა:%n%1\%2
ErrorRegWriteKey=რეესტრის გასაღებში ჩაწერის შეცდომა:%n%1\%2

; *** INI errors
ErrorIniEntry=INI ჩანაწერის შექმნის შეცდომა ფაილში „%1“.

; *** File copying errors
FileAbortRetryIgnoreSkipNotRecommended=ამ ფაილის გამოტოვება (არ არის რეკომენდებული)
FileAbortRetryIgnoreIgnoreNotRecommended=შეცდომის იგნორირება და გაგრძელება (არ არის რეკომენდებული)
SourceIsCorrupted=საწყისი ფაილი დაზიანებულია
SourceDoesntExist=საწყისი ფაილი „%1“ არ არსებობს
SourceVerificationFailed=საწყისი ფაილის შემოწმება ვერ მოხერხდა: %1
VerificationSignatureDoesntExist=ხელმოწერის ფაილი „%1“ არ არსებობს
VerificationSignatureInvalid=ხელმოწერის ფაილი „%1“ არასწორია
VerificationKeyNotFound=ხელმოწერის ფაილი „%1“ უცნობ გასაღებს იყენებს
VerificationFileNameIncorrect=ფაილის სახელი არასწორია
VerificationFileTagIncorrect=ფაილის ტეგი არასწორია
VerificationFileSizeIncorrect=ფაილის ზომა არასწორია
VerificationFileHashIncorrect=ფაილის ჰეში არასწორია
ExistingFileReadOnly2=არსებული ფაილის ჩანაცვლება ვერ მოხერხდა, რადგან ის მხოლოდ წასაკითხადაა მონიშნული.
ExistingFileReadOnlyRetry=ატრიბუტის „მხოლოდ წასაკითხად“ მოხსნა და ხელახლა ცდა
ExistingFileReadOnlyKeepExisting=არსებული ფაილის დატოვება
ErrorReadingExistingDest=არსებული ფაილის წაკითხვისას მოხდა შეცდომა:
FileExistsSelectAction=აირჩიეთ მოქმედება
FileExists2=ფაილი უკვე არსებობს.
FileExistsOverwriteExisting=არსებული ფაილის გადაწერა
FileExistsKeepExisting=არსებული ფაილის დატოვება
FileExistsOverwriteOrKeepAll=იგივე გაკეთდეს შემდეგი კონფლიქტებისთვისაც
ExistingFileNewerSelectAction=აირჩიეთ მოქმედება
ExistingFileNewer2=არსებული ფაილი უფრო ახალია, ვიდრე ის, რომლის დაინსტალირებასაც ინსტალაცია ცდილობს.
ExistingFileNewerOverwriteExisting=არსებული ფაილის გადაწერა
ExistingFileNewerKeepExisting=არსებული ფაილის დატოვება (რეკომენდებულია)
ExistingFileNewerOverwriteOrKeepAll=იგივე გაკეთდეს შემდეგი კონფლიქტებისთვისაც
ErrorChangingAttr=არსებული ფაილის ატრიბუტების შეცვლისას მოხდა შეცდომა:
ErrorCreatingTemp=დანიშნულების საქაღალდეში ფაილის შექმნისას მოხდა შეცდომა:
ErrorReadingSource=საწყისი ფაილის წაკითხვისას მოხდა შეცდომა:
ErrorCopying=ფაილის კოპირებისას მოხდა შეცდომა:
ErrorDownloading=ფაილის ჩამოტვირთვისას მოხდა შეცდომა:
ErrorExtracting=არქივის ამოხსნისას მოხდა შეცდომა:
ErrorReplacingExistingFile=არსებული ფაილის ჩანაცვლებისას მოხდა შეცდომა:
ErrorRestartReplace=RestartReplace ვერ შესრულდა:
ErrorRenamingTemp=დანიშნულების საქაღალდეში ფაილის გადარქმევისას მოხდა შეცდომა:
ErrorRegisterServer=DLL/OCX-ის რეგისტრაცია ვერ მოხერხდა: %1
ErrorRegSvr32Failed=RegSvr32 დასრულდა შეცდომით, გასვლის კოდი %1
ErrorRegisterTypeLib=ტიპების ბიბლიოთეკის რეგისტრაცია ვერ მოხერხდა: %1

; *** Uninstall display name markings
; used for example as 'My Program (32-bit)'
UninstallDisplayNameMark=%1 (%2)
; used for example as 'My Program (32-bit, All users)'
UninstallDisplayNameMarks=%1 (%2, %3)
UninstallDisplayNameMark32Bit=32-ბიტიანი
UninstallDisplayNameMark64Bit=64-ბიტიანი
UninstallDisplayNameMarkAllUsers=ყველა მომხმარებელი
UninstallDisplayNameMarkCurrentUser=მიმდინარე მომხმარებელი

; *** Post-installation errors
ErrorOpeningReadme=README ფაილის გახსნისას მოხდა შეცდომა.
ErrorRestartingComputer=ინსტალაციამ კომპიუტერის გადატვირთვა ვერ შეძლო. გადატვირთეთ ხელით.

; *** Uninstaller messages
UninstallNotFound=ფაილი „%1“ არ არსებობს. დეინსტალაცია შეუძლებელია.
UninstallOpenError=ფაილის „%1“ გახსნა ვერ მოხერხდა. დეინსტალაცია შეუძლებელია
UninstallUnsupportedVer=დეინსტალაციის ჟურნალის ფაილი „%1“ ისეთ ფორმატშია, რომელსაც დეინსტალატორის ეს ვერსია ვერ ცნობს. დეინსტალაცია შეუძლებელია
UninstallUnknownEntry=დეინსტალაციის ჟურნალში აღმოჩენილია უცნობი ჩანაწერი (%1)
ConfirmUninstall=ნამდვილად გსურთ %1-ის და მისი ყველა კომპონენტის სრულად წაშლა?
UninstallOnlyOnWin64=ამ ინსტალაციის დეინსტალაცია შესაძლებელია მხოლოდ 64-ბიტიან Windows-ზე.
OnlyAdminCanUninstall=ამ ინსტალაციის დეინსტალაცია შეუძლია მხოლოდ ადმინისტრატორის უფლებების მქონე მომხმარებელს.
UninstallStatusLabel=დაელოდეთ, სანამ %1 თქვენი კომპიუტერიდან წაიშლება.
UninstalledAll=%1 წარმატებით წაიშალა თქვენი კომპიუტერიდან.
UninstalledMost=%1-ის დეინსტალაცია დასრულდა.%n%nზოგი ელემენტის წაშლა ვერ მოხერხდა. მათი ხელით წაშლა შეგიძლიათ.
UninstalledAndNeedsRestart=%1-ის დეინსტალაციის დასასრულებლად კომპიუტერი უნდა გადაიტვირთოს.%n%nგსურთ ახლა გადატვირთვა?
UninstallDataCorrupted=ფაილი „%1“ დაზიანებულია. დეინსტალაცია შეუძლებელია

; *** Uninstallation phase messages
ConfirmDeleteSharedFileTitle=გაზიარებული ფაილის წაშლა?
ConfirmDeleteSharedFile2=სისტემის მონაცემებით, შემდეგ გაზიარებულ ფაილს აღარც ერთი პროგრამა არ იყენებს. გსურთ, დეინსტალაციამ ეს ფაილი წაშალოს?%n%nთუ რომელიმე პროგრამა ჯერ კიდევ იყენებს ამ ფაილს და ის წაიშლება, ამ პროგრამებმა შეიძლება სწორად ვეღარ იმუშაოს. თუ დარწმუნებული არ ხართ, აირჩიეთ „არა“. ფაილის დატოვება სისტემას ზიანს არ მიაყენებს.
SharedFileNameLabel=ფაილის სახელი:
SharedFileLocationLabel=მდებარეობა:
WizardUninstalling=დეინსტალაციის მდგომარეობა
StatusUninstalling=იშლება %1...

; *** Shutdown block reasons
ShutdownBlockReasonInstallingApp=ინსტალაცია: %1.
ShutdownBlockReasonUninstallingApp=დეინსტალაცია: %1.

; The custom messages below aren't used by Setup itself, but if you make
; use of them in your scripts, you'll want to translate them.

[CustomMessages]

NameAndVersion=%1, ვერსია %2
AdditionalIcons=დამატებითი მალსახმობები:
CreateDesktopIcon=სამუშაო მაგიდის მალსახმობის შექმნა
CreateQuickLaunchIcon=სწრაფი გაშვების მალსახმობის შექმნა
ProgramOnTheWeb=%1 ვებში
UninstallProgram=%1-ის დეინსტალაცია
LaunchProgram=%1-ის გაშვება
AssocFileExtension=%1-ის ასოცირება %2 გაფართოების ფაილებთან
AssocingFileExtension=%1 ასოცირდება %2 გაფართოების ფაილებთან...
AutoStartProgramGroupDescription=ავტოგაშვება:
AutoStartProgram=%1-ის ავტომატური გაშვება
AddonHostProgramNotFound=%1 არჩეულ საქაღალდეში ვერ მოიძებნა.%n%nგსურთ მაინც გაგრძელება?

; The messages that are Lang Switcher's own (the sample is English.isl).
LocalCertReady=ინსტალაცია ამ კომპიუტერზე სერტიფიკატს შექმნის და მისით ხელს მოაწერს Lang Switcher-ს, რათა პროგრამამ ადმინისტრატორის სახელით გაშვებული პროგრამების ფანჯრებშიც იმუშაოს.
LocalCertFailed=Lang Switcher დაინსტალირდა, მაგრამ ადმინისტრატორის სახელით გაშვებული პროგრამების ფანჯრებში არ იმუშავებს: Windows-მა ამ კომპიუტერზე შექმნილი სერტიფიკატი არ მიიღო. დეტალები იხილეთ ინსტალაციის ჟურნალში.
RemoveSettingsQuestion=წაიშალოს Lang Switcher-ის პარამეტრების ფაილიც?
RemoveSettingsKeep=პარამეტრების შესანარჩუნებლად, მომავალში ხელახალი ინსტალაციისთვის, აირჩიეთ „არა“.
AuthorSlogan=გულით შექმნილი პროგრამები
