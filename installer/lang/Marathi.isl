; Lang Switcher -- the messages of the installer in Marathi (stage E108, group A).
;
; Inno Setup has no official Marathi translation (inno_official = false in res\lang\languages.toml),
; so this file carries the whole of it: [LangOptions], every message of the compiler:Default.isl
; (6.5.0+), and the messages of the installer that are Lang Switcher's own. It is read after
; compiler:Default.isl as the second MessagesFile of the "marathi" entry of [Languages] in
; ..\LangSwitcher.iss, and so replaces the English text name for name.
; English.isl is the sample the own messages translate; the glossary of the language is
; l10n\glossary\mr.tsv of the control contour. The name of the program is not translated.
; As in Default.isl: a message that has no full stop there has none here (Setup adds its own).
; No "&" accelerators (RULES 16): the file has no official Inno translation, and two equal
; letters in one window would be worse than none. The names of the buttons inside a message
; stand in the quotation marks of the language.

[LangOptions]
LanguageName=मराठी
LanguageID=$044E
LanguageCodePage=0

[Messages]

; *** Application titles
SetupAppTitle=सेटअप
SetupWindowTitle=सेटअप - %1
UninstallAppTitle=अनइन्स्टॉल
UninstallAppFullTitle=%1 अनइन्स्टॉल

; *** Misc. common
InformationTitle=माहिती
ConfirmTitle=खात्री करा
ErrorTitle=त्रुटी

; *** SetupLdr messages
SetupLdrStartupMessage=हे %1 इन्स्टॉल करेल. तुम्हाला पुढे जायचे आहे का?
LdrCannotCreateTemp=तात्पुरती फाइल तयार करता आली नाही. सेटअप रद्द केला
LdrCannotExecTemp=तात्पुरत्या फोल्डरमधील फाइल चालवता आली नाही. सेटअप रद्द केला
HelpTextNote=

; *** Startup error messages
LastErrorMessage=%1.%n%nत्रुटी %2: %3
SetupFileMissing=इन्स्टॉलेशन फोल्डरमध्ये %1 फाइल सापडत नाही. कृपया समस्या दुरुस्त करा किंवा प्रोग्रामची नवी प्रत मिळवा.
SetupFileCorrupt=सेटअप फाइल्स दूषित आहेत. कृपया प्रोग्रामची नवी प्रत मिळवा.
SetupFileCorruptOrWrongVer=सेटअप फाइल्स दूषित आहेत किंवा सेटअपच्या या आवृत्तीशी सुसंगत नाहीत. कृपया समस्या दुरुस्त करा किंवा प्रोग्रामची नवी प्रत मिळवा.
InvalidParameter=कमांड लाइनवर अवैध पॅरामीटर दिला गेला:%n%n%1
SetupAlreadyRunning=सेटअप आधीच सुरू आहे.
WindowsVersionNotSupported=हा प्रोग्राम तुमच्या संगणकावरील Windows च्या आवृत्तीला समर्थन देत नाही.
WindowsServicePackRequired=या प्रोग्रामसाठी %1 सर्व्हिस पॅक %2 किंवा नंतरचा आवश्यक आहे.
NotOnThisPlatform=हा प्रोग्राम %1 वर चालणार नाही.
OnlyOnThisPlatform=हा प्रोग्राम %1 वर चालवला पाहिजे.
OnlyOnTheseArchitectures=हा प्रोग्राम फक्त खालील प्रोसेसर आर्किटेक्चरसाठी बनवलेल्या Windows च्या आवृत्त्यांवर इन्स्टॉल करता येतो:%n%n%1
WinVersionTooLowError=या प्रोग्रामसाठी %1 आवृत्ती %2 किंवा नंतरची आवश्यक आहे.
WinVersionTooHighError=हा प्रोग्राम %1 आवृत्ती %2 किंवा नंतरच्या आवृत्तीवर इन्स्टॉल करता येत नाही.
AdminPrivilegesRequired=हा प्रोग्राम इन्स्टॉल करताना तुम्ही प्रशासक म्हणून साइन इन केलेले असणे आवश्यक आहे.
PowerUserPrivilegesRequired=हा प्रोग्राम इन्स्टॉल करताना तुम्ही प्रशासक म्हणून किंवा Power Users गटाचे सदस्य म्हणून साइन इन केलेले असणे आवश्यक आहे.
SetupAppRunningError=%1 सध्या सुरू असल्याचे सेटअपला आढळले आहे.%n%nकृपया त्याच्या सर्व चालू प्रती आता बंद करा, मग पुढे जाण्यासाठी “ठीक आहे” किंवा बाहेर पडण्यासाठी “रद्द करा” दाबा.
UninstallAppRunningError=%1 सध्या सुरू असल्याचे अनइन्स्टॉलला आढळले आहे.%n%nकृपया त्याच्या सर्व चालू प्रती आता बंद करा, मग पुढे जाण्यासाठी “ठीक आहे” किंवा बाहेर पडण्यासाठी “रद्द करा” दाबा.

; *** Startup questions
PrivilegesRequiredOverrideTitle=सेटअप इन्स्टॉल मोड निवडा
PrivilegesRequiredOverrideInstruction=इन्स्टॉल मोड निवडा
PrivilegesRequiredOverrideText1=%1 सर्व वापरकर्त्यांसाठी (प्रशासकीय अधिकार आवश्यक) किंवा फक्त तुमच्यासाठी इन्स्टॉल करता येते.
PrivilegesRequiredOverrideText2=%1 फक्त तुमच्यासाठी किंवा सर्व वापरकर्त्यांसाठी (प्रशासकीय अधिकार आवश्यक) इन्स्टॉल करता येते.
PrivilegesRequiredOverrideAllUsers=सर्व वापरकर्त्यांसाठी इन्स्टॉल करा
PrivilegesRequiredOverrideAllUsersRecommended=सर्व वापरकर्त्यांसाठी इन्स्टॉल करा (शिफारस केलेले)
PrivilegesRequiredOverrideCurrentUser=फक्त माझ्यासाठी इन्स्टॉल करा
PrivilegesRequiredOverrideCurrentUserRecommended=फक्त माझ्यासाठी इन्स्टॉल करा (शिफारस केलेले)

; *** Misc. errors
ErrorCreatingDir=सेटअपला “%1” ही डिरेक्टरी तयार करता आली नाही
ErrorTooManyFilesInDir=“%1” डिरेक्टरीमध्ये खूप फाइल्स असल्याने त्यात फाइल तयार करता आली नाही

; *** Setup common messages
ExitSetupTitle=सेटअपमधून बाहेर पडा
ExitSetupMessage=सेटअप पूर्ण झालेला नाही. आता बाहेर पडल्यास प्रोग्राम इन्स्टॉल होणार नाही.%n%nइन्स्टॉलेशन पूर्ण करण्यासाठी तुम्ही सेटअप नंतर पुन्हा चालवू शकता.%n%nसेटअपमधून बाहेर पडायचे का?
AboutSetupMenuItem=सेटअपविषयी...
AboutSetupTitle=सेटअपविषयी
AboutSetupMessage=%1 आवृत्ती %2%n%3%n%n%1 मुख्यपृष्ठ:%n%4
AboutSetupNote=
TranslatorNote=

; *** Buttons
ButtonBack=< मागे
ButtonNext=पुढे >
ButtonInstall=इन्स्टॉल करा
ButtonOK=ठीक आहे
ButtonCancel=रद्द करा
ButtonYes=होय
ButtonYesToAll=सर्वांसाठी होय
ButtonNo=नाही
ButtonNoToAll=सर्वांसाठी नाही
ButtonFinish=पूर्ण करा
ButtonBrowse=ब्राउझ करा...
ButtonWizardBrowse=ब्राउझ करा...
ButtonNewFolder=नवीन फोल्डर तयार करा

; *** "Select Language" dialog messages
SelectLanguageTitle=सेटअपची भाषा निवडा
SelectLanguageLabel=इन्स्टॉलेशनदरम्यान वापरायची भाषा निवडा.

; *** Common wizard text
ClickNext=पुढे जाण्यासाठी “पुढे” किंवा सेटअपमधून बाहेर पडण्यासाठी “रद्द करा” दाबा.
BeveledLabel=
BrowseDialogTitle=फोल्डर शोधा
BrowseDialogLabel=खालील यादीतून फोल्डर निवडा, मग “ठीक आहे” दाबा.
NewFolderName=नवीन फोल्डर

; *** "Welcome" wizard page
WelcomeLabel1=[name] सेटअप विझार्डमध्ये आपले स्वागत आहे
WelcomeLabel2=हे तुमच्या संगणकावर [name/ver] इन्स्टॉल करेल.%n%nपुढे जाण्यापूर्वी इतर सर्व प्रोग्राम बंद करण्याची शिफारस केली जाते.

; *** "Password" wizard page
WizardPassword=पासवर्ड
PasswordLabel1=हे इन्स्टॉलेशन पासवर्डने संरक्षित आहे.
PasswordLabel3=कृपया पासवर्ड द्या, मग पुढे जाण्यासाठी “पुढे” दाबा. पासवर्डमध्ये लहान-मोठी अक्षरे वेगळी मानली जातात.
PasswordEditLabel=पासवर्ड:
IncorrectPassword=तुम्ही दिलेला पासवर्ड चुकीचा आहे. कृपया पुन्हा प्रयत्न करा.

; *** "License Agreement" wizard page
WizardLicense=परवाना करार
LicenseLabel=पुढे जाण्यापूर्वी कृपया ही महत्त्वाची माहिती वाचा.
LicenseLabel3=कृपया खालील परवाना करार वाचा. इन्स्टॉलेशन पुढे नेण्यापूर्वी तुम्हाला या कराराच्या अटी मान्य कराव्या लागतील.
LicenseAccepted=करार मान्य आहे
LicenseNotAccepted=करार मान्य नाही

; *** "Information" wizard pages
WizardInfoBefore=माहिती
InfoBeforeLabel=पुढे जाण्यापूर्वी कृपया ही महत्त्वाची माहिती वाचा.
InfoBeforeClickLabel=सेटअपमध्ये पुढे जाण्यास तयार असाल तेव्हा “पुढे” दाबा.
WizardInfoAfter=माहिती
InfoAfterLabel=पुढे जाण्यापूर्वी कृपया ही महत्त्वाची माहिती वाचा.
InfoAfterClickLabel=सेटअपमध्ये पुढे जाण्यास तयार असाल तेव्हा “पुढे” दाबा.

; *** "User Information" wizard page
WizardUserInfo=वापरकर्त्याची माहिती
UserInfoDesc=कृपया तुमची माहिती भरा.
UserInfoName=वापरकर्त्याचे नाव:
UserInfoOrg=संस्था:
UserInfoSerial=अनुक्रमांक:
UserInfoNameRequired=नाव भरणे आवश्यक आहे.

; *** "Select Destination Location" wizard page
WizardSelectDir=गंतव्य स्थान निवडा
SelectDirDesc=[name] कुठे इन्स्टॉल करायचे?
SelectDirLabel3=सेटअप [name] खालील फोल्डरमध्ये इन्स्टॉल करेल.
SelectDirBrowseLabel=पुढे जाण्यासाठी “पुढे” दाबा. दुसरे फोल्डर निवडायचे असल्यास “ब्राउझ करा” दाबा.
DiskSpaceGBLabel=किमान [gb] GB मोकळी डिस्क जागा आवश्यक आहे.
DiskSpaceMBLabel=किमान [mb] MB मोकळी डिस्क जागा आवश्यक आहे.
CannotInstallToNetworkDrive=सेटअप नेटवर्क ड्राइव्हवर इन्स्टॉल करू शकत नाही.
CannotInstallToUNCPath=सेटअप UNC पाथवर इन्स्टॉल करू शकत नाही.
InvalidPath=ड्राइव्ह अक्षरासह पूर्ण पाथ द्यावा लागेल; उदाहरणार्थ:%n%nC:\APP%n%nकिंवा या स्वरूपातील UNC पाथ:%n%n\\server\share
InvalidDrive=तुम्ही निवडलेला ड्राइव्ह किंवा UNC शेअर अस्तित्वात नाही किंवा उपलब्ध नाही. कृपया दुसरा निवडा.
DiskSpaceWarningTitle=डिस्कवर पुरेशी जागा नाही
DiskSpaceWarning=इन्स्टॉल करण्यासाठी सेटअपला किमान %1 KB मोकळी जागा हवी आहे, पण निवडलेल्या ड्राइव्हवर फक्त %2 KB उपलब्ध आहे.%n%nतरीही पुढे जायचे का?
DirNameTooLong=फोल्डरचे नाव किंवा पाथ खूप लांब आहे.
InvalidDirName=फोल्डरचे नाव वैध नाही.
BadDirName32=फोल्डरच्या नावात खालील अक्षरे असू शकत नाहीत:%n%n%1
DirExistsTitle=फोल्डर अस्तित्वात आहे
DirExists=हे फोल्डर:%n%n%1%n%nआधीच अस्तित्वात आहे. तरीही त्या फोल्डरमध्ये इन्स्टॉल करायचे का?
DirDoesntExistTitle=फोल्डर अस्तित्वात नाही
DirDoesntExist=हे फोल्डर:%n%n%1%n%nअस्तित्वात नाही. फोल्डर तयार करायचे का?

; *** "Select Components" wizard page
WizardSelectComponents=घटक निवडा
SelectComponentsDesc=कोणते घटक इन्स्टॉल करायचे?
SelectComponentsLabel2=इन्स्टॉल करायचे घटक निवडा; नको असलेल्या घटकांची निवड काढा. तयार असाल तेव्हा “पुढे” दाबा.
FullInstallation=पूर्ण इन्स्टॉलेशन
; if possible don't translate 'Compact' as 'Minimal' (I mean 'Minimal' in your language)
CompactInstallation=संक्षिप्त इन्स्टॉलेशन
CustomInstallation=सानुकूल इन्स्टॉलेशन
NoUninstallWarningTitle=घटक आधीपासून आहेत
NoUninstallWarning=खालील घटक तुमच्या संगणकावर आधीच इन्स्टॉल असल्याचे सेटअपला आढळले आहे:%n%n%1%n%nया घटकांची निवड काढल्याने ते अनइन्स्टॉल होणार नाहीत.%n%nतरीही पुढे जायचे का?
ComponentSize1=%1 KB
ComponentSize2=%1 MB
ComponentsDiskSpaceGBLabel=सध्याच्या निवडीसाठी किमान [gb] GB डिस्क जागा आवश्यक आहे.
ComponentsDiskSpaceMBLabel=सध्याच्या निवडीसाठी किमान [mb] MB डिस्क जागा आवश्यक आहे.

; *** "Select Additional Tasks" wizard page
WizardSelectTasks=अतिरिक्त कामे निवडा
SelectTasksDesc=कोणती अतिरिक्त कामे करायची?
SelectTasksLabel2=[name] इन्स्टॉल करताना सेटअपने कोणती अतिरिक्त कामे करावीत ते निवडा, मग “पुढे” दाबा.

; *** "Select Start Menu Folder" wizard page
WizardSelectProgramGroup=स्टार्ट मेनू फोल्डर निवडा
SelectStartMenuFolderDesc=सेटअपने प्रोग्रामचे शॉर्टकट कुठे ठेवावेत?
SelectStartMenuFolderLabel3=सेटअप प्रोग्रामचे शॉर्टकट खालील स्टार्ट मेनू फोल्डरमध्ये तयार करेल.
SelectStartMenuFolderBrowseLabel=पुढे जाण्यासाठी “पुढे” दाबा. दुसरे फोल्डर निवडायचे असल्यास “ब्राउझ करा” दाबा.
MustEnterGroupName=फोल्डरचे नाव भरणे आवश्यक आहे.
GroupNameTooLong=फोल्डरचे नाव किंवा पाथ खूप लांब आहे.
InvalidGroupName=फोल्डरचे नाव वैध नाही.
BadGroupName=फोल्डरच्या नावात खालील अक्षरे असू शकत नाहीत:%n%n%1
NoProgramGroupCheck2=स्टार्ट मेनू फोल्डर तयार करू नका

; *** "Ready to Install" wizard page
WizardReady=इन्स्टॉल करण्यास तयार
ReadyLabel1=सेटअप आता तुमच्या संगणकावर [name] इन्स्टॉल करण्यास तयार आहे.
ReadyLabel2a=इन्स्टॉलेशन सुरू ठेवण्यासाठी “इन्स्टॉल करा” दाबा, किंवा कोणतीही सेटिंग्ज पाहायची वा बदलायची असल्यास “मागे” दाबा.
ReadyLabel2b=इन्स्टॉलेशन सुरू ठेवण्यासाठी “इन्स्टॉल करा” दाबा.
ReadyMemoUserInfo=वापरकर्त्याची माहिती:
ReadyMemoDir=गंतव्य स्थान:
ReadyMemoType=सेटअपचा प्रकार:
ReadyMemoComponents=निवडलेले घटक:
ReadyMemoGroup=स्टार्ट मेनू फोल्डर:
ReadyMemoTasks=अतिरिक्त कामे:

; *** TDownloadWizardPage wizard page and DownloadTemporaryFile
DownloadingLabel2=फाइल्स डाउनलोड होत आहेत...
ButtonStopDownload=डाउनलोड थांबवा
StopDownload=तुम्हाला खरोखर डाउनलोड थांबवायचे आहे का?
ErrorDownloadAborted=डाउनलोड रद्द केले
ErrorDownloadFailed=डाउनलोड अयशस्वी: %1 %2
ErrorDownloadSizeFailed=आकार मिळवता आला नाही: %1 %2
ErrorProgress=अवैध प्रगती: %2 पैकी %1
ErrorFileSize=फाइलचा आकार अवैध: अपेक्षित %1, आढळला %2

; *** TExtractionWizardPage wizard page and ExtractArchive
ExtractingLabel=फाइल्स काढल्या जात आहेत...
ButtonStopExtraction=काढणे थांबवा
StopExtraction=तुम्हाला खरोखर फाइल्स काढणे थांबवायचे आहे का?
ErrorExtractionAborted=फाइल्स काढणे रद्द केले
ErrorExtractionFailed=फाइल्स काढणे अयशस्वी: %1

; *** Archive extraction failure details
ArchiveIncorrectPassword=पासवर्ड चुकीचा आहे
ArchiveIsCorrupted=आर्काइव्ह दूषित आहे
ArchiveUnsupportedFormat=आर्काइव्हचे स्वरूप समर्थित नाही

; *** "Preparing to Install" wizard page
WizardPreparing=इन्स्टॉलेशनची तयारी
PreparingDesc=सेटअप तुमच्या संगणकावर [name] इन्स्टॉल करण्याची तयारी करत आहे.
PreviousInstallNotCompleted=आधीच्या प्रोग्रामचे इन्स्टॉलेशन/काढणे पूर्ण झाले नाही. ते पूर्ण करण्यासाठी तुम्हाला संगणक रीस्टार्ट करावा लागेल.%n%nसंगणक रीस्टार्ट केल्यानंतर [name] चे इन्स्टॉलेशन पूर्ण करण्यासाठी सेटअप पुन्हा चालवा.
CannotContinue=सेटअप पुढे जाऊ शकत नाही. बाहेर पडण्यासाठी “रद्द करा” दाबा.
ApplicationsFound=सेटअपला अद्यतनित कराव्या लागणाऱ्या फाइल्स खालील प्रोग्राम वापरत आहेत. हे प्रोग्राम सेटअपने आपोआप बंद करू देण्याची शिफारस केली जाते.
ApplicationsFound2=सेटअपला अद्यतनित कराव्या लागणाऱ्या फाइल्स खालील प्रोग्राम वापरत आहेत. हे प्रोग्राम सेटअपने आपोआप बंद करू देण्याची शिफारस केली जाते. इन्स्टॉलेशन पूर्ण झाल्यावर सेटअप हे प्रोग्राम पुन्हा सुरू करण्याचा प्रयत्न करेल.
CloseApplications=प्रोग्राम आपोआप बंद करा
DontCloseApplications=प्रोग्राम बंद करू नका
ErrorCloseApplications=सेटअपला सर्व प्रोग्राम आपोआप बंद करता आले नाहीत. पुढे जाण्यापूर्वी सेटअपला अद्यतनित कराव्या लागणाऱ्या फाइल्स वापरणारे सर्व प्रोग्राम बंद करण्याची शिफारस केली जाते.
PrepareToInstallNeedsRestart=सेटअपला तुमचा संगणक रीस्टार्ट करावा लागेल. संगणक रीस्टार्ट केल्यानंतर [name] चे इन्स्टॉलेशन पूर्ण करण्यासाठी सेटअप पुन्हा चालवा.%n%nआता रीस्टार्ट करायचे का?

; *** "Installing" wizard page
WizardInstalling=इन्स्टॉल होत आहे
InstallingLabel=सेटअप तुमच्या संगणकावर [name] इन्स्टॉल करत आहे, कृपया थांबा.

; *** "Setup Completed" wizard page
FinishedHeadingLabel=[name] सेटअप विझार्ड पूर्ण होत आहे
FinishedLabelNoIcons=सेटअपने तुमच्या संगणकावर [name] इन्स्टॉल करणे पूर्ण केले आहे.
FinishedLabel=सेटअपने तुमच्या संगणकावर [name] इन्स्टॉल करणे पूर्ण केले आहे. इन्स्टॉल केलेले शॉर्टकट निवडून प्रोग्राम सुरू करता येईल.
ClickFinish=सेटअपमधून बाहेर पडण्यासाठी “पूर्ण करा” दाबा.
FinishedRestartLabel=[name] चे इन्स्टॉलेशन पूर्ण करण्यासाठी सेटअपला तुमचा संगणक रीस्टार्ट करावा लागेल. आता रीस्टार्ट करायचे का?
FinishedRestartMessage=[name] चे इन्स्टॉलेशन पूर्ण करण्यासाठी सेटअपला तुमचा संगणक रीस्टार्ट करावा लागेल.%n%nआता रीस्टार्ट करायचे का?
ShowReadmeCheck=होय, मला README फाइल पाहायची आहे
YesRadio=होय, संगणक आता रीस्टार्ट करा
NoRadio=नाही, मी संगणक नंतर रीस्टार्ट करेन
; used for example as 'Run MyProg.exe'
RunEntryExec=%1 चालवा
; used for example as 'View Readme.txt'
RunEntryShellExec=%1 पहा

; *** "Setup Needs the Next Disk" stuff
ChangeDiskTitle=सेटअपला पुढील डिस्क हवी आहे
SelectDiskLabel2=कृपया डिस्क %1 घाला आणि “ठीक आहे” दाबा.%n%nया डिस्कवरील फाइल्स खाली दाखवलेल्या फोल्डरव्यतिरिक्त दुसऱ्या फोल्डरमध्ये सापडत असतील, तर योग्य पाथ भरा किंवा “ब्राउझ करा” दाबा.
PathLabel=पाथ:
FileNotInDir2=“%1” ही फाइल “%2” मध्ये सापडली नाही. कृपया योग्य डिस्क घाला किंवा दुसरे फोल्डर निवडा.
SelectDirectoryLabel=कृपया पुढील डिस्कचे स्थान सांगा.

; *** Installation phase messages
SetupAborted=सेटअप पूर्ण झाले नाही.%n%nकृपया समस्या दुरुस्त करा आणि सेटअप पुन्हा चालवा.
AbortRetryIgnoreSelectAction=कृती निवडा
AbortRetryIgnoreRetry=पुन्हा प्रयत्न करा
AbortRetryIgnoreIgnore=त्रुटीकडे दुर्लक्ष करून पुढे जा
AbortRetryIgnoreCancel=इन्स्टॉलेशन रद्द करा
RetryCancelSelectAction=कृती निवडा
RetryCancelRetry=पुन्हा प्रयत्न करा
RetryCancelCancel=रद्द करा

; *** Installation status messages
StatusClosingApplications=प्रोग्राम बंद केले जात आहेत...
StatusCreateDirs=फोल्डर तयार केले जात आहेत...
StatusExtractFiles=फाइल्स काढल्या जात आहेत...
StatusDownloadFiles=फाइल्स डाउनलोड होत आहेत...
StatusCreateIcons=शॉर्टकट तयार केले जात आहेत...
StatusCreateIniEntries=INI नोंदी तयार केल्या जात आहेत...
StatusCreateRegistryEntries=रजिस्ट्री नोंदी तयार केल्या जात आहेत...
StatusRegisterFiles=फाइल्स नोंदवल्या जात आहेत...
StatusSavingUninstall=अनइन्स्टॉलची माहिती जतन केली जात आहे...
StatusRunProgram=इन्स्टॉलेशन पूर्ण केले जात आहे...
StatusRestartingApplications=प्रोग्राम पुन्हा सुरू केले जात आहेत...
StatusRollback=बदल मागे घेतले जात आहेत...

; *** Misc. errors
ErrorInternal2=अंतर्गत त्रुटी: %1
ErrorFunctionFailedNoCode=%1 अयशस्वी झाले
ErrorFunctionFailed=%1 अयशस्वी झाले; कोड %2
ErrorFunctionFailedWithMessage=%1 अयशस्वी झाले; कोड %2.%n%3
ErrorExecutingProgram=फाइल चालवता आली नाही:%n%1

; *** Registry errors
ErrorRegOpenKey=रजिस्ट्री की उघडताना त्रुटी:%n%1\%2
ErrorRegCreateKey=रजिस्ट्री की तयार करताना त्रुटी:%n%1\%2
ErrorRegWriteKey=रजिस्ट्री कीमध्ये लिहिताना त्रुटी:%n%1\%2

; *** INI errors
ErrorIniEntry=“%1” फाइलमध्ये INI नोंद तयार करताना त्रुटी आली.

; *** File copying errors
FileAbortRetryIgnoreSkipNotRecommended=ही फाइल वगळा (शिफारस केलेली नाही)
FileAbortRetryIgnoreIgnoreNotRecommended=त्रुटीकडे दुर्लक्ष करून पुढे जा (शिफारस केलेली नाही)
SourceIsCorrupted=स्रोत फाइल दूषित आहे
SourceDoesntExist=“%1” ही स्रोत फाइल अस्तित्वात नाही
SourceVerificationFailed=स्रोत फाइलची पडताळणी अयशस्वी: %1
VerificationSignatureDoesntExist=“%1” ही स्वाक्षरी फाइल अस्तित्वात नाही
VerificationSignatureInvalid=“%1” ही स्वाक्षरी फाइल अवैध आहे
VerificationKeyNotFound=“%1” ही स्वाक्षरी फाइल अज्ञात की वापरते
VerificationFileNameIncorrect=फाइलचे नाव चुकीचे आहे
VerificationFileTagIncorrect=फाइलचा टॅग चुकीचा आहे
VerificationFileSizeIncorrect=फाइलचा आकार चुकीचा आहे
VerificationFileHashIncorrect=फाइलचा हॅश चुकीचा आहे
ExistingFileReadOnly2=आधीची फाइल केवळ-वाचनीय म्हणून चिन्हांकित असल्याने ती बदलता आली नाही.
ExistingFileReadOnlyRetry=केवळ-वाचनीय गुणधर्म काढून पुन्हा प्रयत्न करा
ExistingFileReadOnlyKeepExisting=आधीची फाइल ठेवा
ErrorReadingExistingDest=आधीची फाइल वाचताना त्रुटी आली:
FileExistsSelectAction=कृती निवडा
FileExists2=फाइल आधीच अस्तित्वात आहे.
FileExistsOverwriteExisting=आधीच्या फाइलवर लिहा
FileExistsKeepExisting=आधीची फाइल ठेवा
FileExistsOverwriteOrKeepAll=पुढील संघर्षांसाठीही हेच करा
ExistingFileNewerSelectAction=कृती निवडा
ExistingFileNewer2=आधीची फाइल सेटअप इन्स्टॉल करू पाहत असलेल्या फाइलपेक्षा नवी आहे.
ExistingFileNewerOverwriteExisting=आधीच्या फाइलवर लिहा
ExistingFileNewerKeepExisting=आधीची फाइल ठेवा (शिफारस केलेले)
ExistingFileNewerOverwriteOrKeepAll=पुढील संघर्षांसाठीही हेच करा
ErrorChangingAttr=आधीच्या फाइलचे गुणधर्म बदलताना त्रुटी आली:
ErrorCreatingTemp=गंतव्य फोल्डरमध्ये फाइल तयार करताना त्रुटी आली:
ErrorReadingSource=स्रोत फाइल वाचताना त्रुटी आली:
ErrorCopying=फाइल कॉपी करताना त्रुटी आली:
ErrorDownloading=फाइल डाउनलोड करताना त्रुटी आली:
ErrorExtracting=आर्काइव्हमधून फाइल्स काढताना त्रुटी आली:
ErrorReplacingExistingFile=आधीची फाइल बदलताना त्रुटी आली:
ErrorRestartReplace=RestartReplace अयशस्वी:
ErrorRenamingTemp=गंतव्य फोल्डरमधील फाइलचे नाव बदलताना त्रुटी आली:
ErrorRegisterServer=DLL/OCX नोंदवता आले नाही: %1
ErrorRegSvr32Failed=RegSvr32 एक्झिट कोड %1 सह अयशस्वी झाले
ErrorRegisterTypeLib=टाइप लायब्ररी नोंदवता आली नाही: %1

; *** Uninstall display name markings
; used for example as 'My Program (32-bit)'
UninstallDisplayNameMark=%1 (%2)
; used for example as 'My Program (32-bit, All users)'
UninstallDisplayNameMarks=%1 (%2, %3)
UninstallDisplayNameMark32Bit=32-बिट
UninstallDisplayNameMark64Bit=64-बिट
UninstallDisplayNameMarkAllUsers=सर्व वापरकर्ते
UninstallDisplayNameMarkCurrentUser=सध्याचा वापरकर्ता

; *** Post-installation errors
ErrorOpeningReadme=README फाइल उघडताना त्रुटी आली.
ErrorRestartingComputer=सेटअपला संगणक रीस्टार्ट करता आला नाही. कृपया ते स्वतः करा.

; *** Uninstaller messages
UninstallNotFound=“%1” ही फाइल अस्तित्वात नाही. अनइन्स्टॉल करता येत नाही.
UninstallOpenError=“%1” ही फाइल उघडता आली नाही. अनइन्स्टॉल करता येत नाही
UninstallUnsupportedVer=“%1” हा अनइन्स्टॉल लॉग अनइन्स्टॉलरच्या या आवृत्तीला अपरिचित स्वरूपात आहे. अनइन्स्टॉल करता येत नाही
UninstallUnknownEntry=अनइन्स्टॉल लॉगमध्ये अज्ञात नोंद (%1) आढळली
ConfirmUninstall=तुम्हाला %1 आणि त्याचे सर्व घटक पूर्णपणे काढून टाकायचे आहेत का?
UninstallOnlyOnWin64=हे इन्स्टॉलेशन फक्त 64-बिट Windows वर अनइन्स्टॉल करता येते.
OnlyAdminCanUninstall=हे इन्स्टॉलेशन फक्त प्रशासकीय अधिकार असलेला वापरकर्ताच अनइन्स्टॉल करू शकतो.
UninstallStatusLabel=%1 तुमच्या संगणकावरून काढले जात आहे, कृपया थांबा.
UninstalledAll=%1 तुमच्या संगणकावरून यशस्वीरित्या काढले गेले.
UninstalledMost=%1 अनइन्स्टॉल पूर्ण झाले.%n%nकाही घटक काढता आले नाहीत. ते स्वतः काढता येतील.
UninstalledAndNeedsRestart=%1 चे अनइन्स्टॉल पूर्ण करण्यासाठी तुमचा संगणक रीस्टार्ट करावा लागेल.%n%nआता रीस्टार्ट करायचे का?
UninstallDataCorrupted=“%1” फाइल दूषित आहे. अनइन्स्टॉल करता येत नाही

; *** Uninstallation phase messages
ConfirmDeleteSharedFileTitle=सामायिक फाइल काढून टाकायची का?
ConfirmDeleteSharedFile2=खालील सामायिक फाइल आता कोणताही प्रोग्राम वापरत नसल्याचे सिस्टीम सूचित करते. अनइन्स्टॉलने ही सामायिक फाइल काढून टाकावी का?%n%nकाही प्रोग्राम अजूनही ही फाइल वापरत असतील आणि ती काढली गेली, तर ते प्रोग्राम नीट चालणार नाहीत. खात्री नसल्यास “नाही” निवडा. फाइल सिस्टीमवर ठेवल्याने कोणतेही नुकसान होत नाही.
SharedFileNameLabel=फाइलचे नाव:
SharedFileLocationLabel=स्थान:
WizardUninstalling=अनइन्स्टॉलची स्थिती
StatusUninstalling=%1 अनइन्स्टॉल होत आहे...

; *** Shutdown block reasons
ShutdownBlockReasonInstallingApp=%1 इन्स्टॉल होत आहे.
ShutdownBlockReasonUninstallingApp=%1 अनइन्स्टॉल होत आहे.

; The custom messages below aren't used by Setup itself, but if you make
; use of them in your scripts, you'll want to translate them.

[CustomMessages]

NameAndVersion=%1 आवृत्ती %2
AdditionalIcons=अतिरिक्त शॉर्टकट:
CreateDesktopIcon=डेस्कटॉप शॉर्टकट तयार करा
CreateQuickLaunchIcon=क्विक लाँच शॉर्टकट तयार करा
ProgramOnTheWeb=%1 वेबवर
UninstallProgram=%1 अनइन्स्टॉल करा
LaunchProgram=%1 सुरू करा
AssocFileExtension=%1 ला %2 फाइल एक्स्टेंशनशी जोडा
AssocingFileExtension=%1 ला %2 फाइल एक्स्टेंशनशी जोडले जात आहे...
AutoStartProgramGroupDescription=स्टार्टअप:
AutoStartProgram=%1 आपोआप सुरू करा
AddonHostProgramNotFound=तुम्ही निवडलेल्या फोल्डरमध्ये %1 सापडले नाही.%n%nतरीही पुढे जायचे का?

; The messages that are Lang Switcher's own (the sample is English.isl).
LocalCertReady=सेटअप या संगणकावर एक प्रमाणपत्र तयार करेल आणि त्याने Lang Switcher वर स्वाक्षरी करेल, जेणेकरून तो प्रशासक म्हणून चालवलेल्या प्रोग्रामच्या विंडोमध्येही काम करू शकेल.
LocalCertFailed=Lang Switcher इन्स्टॉल झाला, पण तो प्रशासक म्हणून चालवलेल्या प्रोग्रामच्या विंडोमध्ये काम करणार नाही: या संगणकावर तयार केलेले प्रमाणपत्र Windows ने स्वीकारले नाही. तपशिलांसाठी सेटअप लॉग पहा.
RemoveSettingsQuestion=Lang Switcher ची सेटिंग्ज फाइलही काढून टाकायची का?
RemoveSettingsKeep=नंतर पुन्हा इन्स्टॉल करताना सेटिंग्ज जपून ठेवण्यासाठी “नाही” निवडा.
AuthorSlogan=मनापासून बनवलेले प्रोग्राम
