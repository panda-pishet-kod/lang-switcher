; Lang Switcher -- the messages of the installer in Hindi (stage E108, group A).
;
; Inno Setup has no official Hindi translation (inno_official = false in res\lang\languages.toml),
; so this file carries the whole of it: [LangOptions], every message of the compiler:Default.isl
; (6.5.0+), and the messages of the installer that are Lang Switcher's own. It is read after
; compiler:Default.isl as the second MessagesFile of the "hindi" entry of [Languages] in
; ..\LangSwitcher.iss, and so replaces the English text name for name.
; English.isl is the sample the own messages translate; the glossary of the language is
; l10n\glossary\hi.tsv of the control contour. The name of the program is not translated.
; As in Default.isl: a message that has no full stop there has no danda here (Setup adds its own).

[LangOptions]
LanguageName=हिन्दी
LanguageID=$0439
LanguageCodePage=0

[Messages]

; *** Application titles
SetupAppTitle=सेटअप
SetupWindowTitle=सेटअप - %1
UninstallAppTitle=अनइंस्टॉल
UninstallAppFullTitle=%1 अनइंस्टॉल

; *** Misc. common
InformationTitle=जानकारी
ConfirmTitle=पुष्टि करें
ErrorTitle=त्रुटि

; *** SetupLdr messages
SetupLdrStartupMessage=यह %1 को इंस्टॉल करेगा। क्या आप जारी रखना चाहते हैं?
LdrCannotCreateTemp=अस्थायी फ़ाइल नहीं बनाई जा सकी। सेटअप रद्द किया गया
LdrCannotExecTemp=अस्थायी फ़ोल्डर में फ़ाइल नहीं चलाई जा सकी। सेटअप रद्द किया गया
HelpTextNote=

; *** Startup error messages
LastErrorMessage=%1.%n%nत्रुटि %2: %3
SetupFileMissing=इंस्टॉलेशन फ़ोल्डर में फ़ाइल %1 नहीं मिली। कृपया समस्या ठीक करें या प्रोग्राम की नई प्रति प्राप्त करें।
SetupFileCorrupt=सेटअप फ़ाइलें क्षतिग्रस्त हैं। कृपया प्रोग्राम की नई प्रति प्राप्त करें।
SetupFileCorruptOrWrongVer=सेटअप फ़ाइलें क्षतिग्रस्त हैं या सेटअप के इस संस्करण के अनुकूल नहीं हैं। कृपया समस्या ठीक करें या प्रोग्राम की नई प्रति प्राप्त करें।
InvalidParameter=कमांड लाइन में अमान्य पैरामीटर दिया गया:%n%n%1
SetupAlreadyRunning=सेटअप पहले से चल रहा है।
WindowsVersionNotSupported=यह प्रोग्राम आपके कंप्यूटर पर चल रहे Windows संस्करण का समर्थन नहीं करता।
WindowsServicePackRequired=इस प्रोग्राम के लिए %1 सर्विस पैक %2 या उसके बाद का संस्करण चाहिए।
NotOnThisPlatform=यह प्रोग्राम %1 पर नहीं चलेगा।
OnlyOnThisPlatform=इस प्रोग्राम को %1 पर चलाना होगा।
OnlyOnTheseArchitectures=यह प्रोग्राम केवल Windows के उन संस्करणों पर इंस्टॉल हो सकता है जो इन प्रोसेसर आर्किटेक्चर के लिए बने हैं:%n%n%1
WinVersionTooLowError=इस प्रोग्राम के लिए %1 का संस्करण %2 या उसके बाद का चाहिए।
WinVersionTooHighError=इस प्रोग्राम को %1 के संस्करण %2 या उसके बाद के संस्करण पर इंस्टॉल नहीं किया जा सकता।
AdminPrivilegesRequired=इस प्रोग्राम को इंस्टॉल करते समय आपको व्यवस्थापक के रूप में साइन इन होना चाहिए।
PowerUserPrivilegesRequired=इस प्रोग्राम को इंस्टॉल करते समय आपको व्यवस्थापक के रूप में या Power Users समूह के सदस्य के रूप में साइन इन होना चाहिए।
SetupAppRunningError=सेटअप ने पाया कि %1 अभी चल रहा है।%n%nकृपया उसके सभी इंस्टेंस अभी बंद करें, फिर जारी रखने के लिए “ठीक है” पर, या बाहर निकलने के लिए “रद्द करें” पर क्लिक करें।
UninstallAppRunningError=अनइंस्टॉल ने पाया कि %1 अभी चल रहा है।%n%nकृपया उसके सभी इंस्टेंस अभी बंद करें, फिर जारी रखने के लिए “ठीक है” पर, या बाहर निकलने के लिए “रद्द करें” पर क्लिक करें।

; *** Startup questions
PrivilegesRequiredOverrideTitle=सेटअप का इंस्टॉल मोड चुनें
PrivilegesRequiredOverrideInstruction=इंस्टॉल मोड चुनें
PrivilegesRequiredOverrideText1=%1 को सभी उपयोगकर्ताओं के लिए (व्यवस्थापक अधिकार चाहिए) या केवल आपके लिए इंस्टॉल किया जा सकता है।
PrivilegesRequiredOverrideText2=%1 को केवल आपके लिए या सभी उपयोगकर्ताओं के लिए (व्यवस्थापक अधिकार चाहिए) इंस्टॉल किया जा सकता है।
PrivilegesRequiredOverrideAllUsers=सभी उपयोगकर्ताओं के लिए इंस्टॉल करें
PrivilegesRequiredOverrideAllUsersRecommended=सभी उपयोगकर्ताओं के लिए इंस्टॉल करें (अनुशंसित)
PrivilegesRequiredOverrideCurrentUser=केवल मेरे लिए इंस्टॉल करें
PrivilegesRequiredOverrideCurrentUserRecommended=केवल मेरे लिए इंस्टॉल करें (अनुशंसित)

; *** Misc. errors
ErrorCreatingDir=सेटअप फ़ोल्डर "%1" नहीं बना सका
ErrorTooManyFilesInDir=फ़ोल्डर "%1" में फ़ाइल नहीं बनाई जा सकी, क्योंकि उसमें बहुत ज़्यादा फ़ाइलें हैं

; *** Setup common messages
ExitSetupTitle=सेटअप से बाहर निकलें
ExitSetupMessage=सेटअप पूरा नहीं हुआ है। अभी बाहर निकलने पर प्रोग्राम इंस्टॉल नहीं होगा।%n%nइंस्टॉलेशन पूरा करने के लिए आप सेटअप को बाद में फिर चला सकते हैं।%n%nसेटअप से बाहर निकलें?
AboutSetupMenuItem=सेटअप के बारे में...
AboutSetupTitle=सेटअप के बारे में
AboutSetupMessage=%1 संस्करण %2%n%3%n%n%1 का होम पेज:%n%4
AboutSetupNote=
TranslatorNote=

; *** Buttons
ButtonBack=< पीछे
ButtonNext=आगे >
ButtonInstall=इंस्टॉल करें
ButtonOK=ठीक है
ButtonCancel=रद्द करें
ButtonYes=हाँ
ButtonYesToAll=सभी के लिए हाँ
ButtonNo=नहीं
ButtonNoToAll=सभी के लिए नहीं
ButtonFinish=समाप्त करें
ButtonBrowse=ब्राउज़ करें...
ButtonWizardBrowse=ब्राउज़ करें...
ButtonNewFolder=नया फ़ोल्डर बनाएँ

; *** "Select Language" dialog messages
SelectLanguageTitle=सेटअप की भाषा चुनें
SelectLanguageLabel=इंस्टॉलेशन के दौरान उपयोग की जाने वाली भाषा चुनें।

; *** Common wizard text
ClickNext=जारी रखने के लिए “आगे” पर, या सेटअप से बाहर निकलने के लिए “रद्द करें” पर क्लिक करें।
BeveledLabel=
BrowseDialogTitle=फ़ोल्डर चुनें
BrowseDialogLabel=नीचे दी गई सूची में से कोई फ़ोल्डर चुनें, फिर “ठीक है” पर क्लिक करें।
NewFolderName=नया फ़ोल्डर

; *** "Welcome" wizard page
WelcomeLabel1=[name] सेटअप विज़ार्ड में आपका स्वागत है
WelcomeLabel2=यह आपके कंप्यूटर पर [name/ver] इंस्टॉल करेगा।%n%nजारी रखने से पहले अन्य सभी एप्लिकेशन बंद करने की सलाह दी जाती है।

; *** "Password" wizard page
WizardPassword=पासवर्ड
PasswordLabel1=यह इंस्टॉलेशन पासवर्ड से सुरक्षित है।
PasswordLabel3=कृपया पासवर्ड दर्ज करें, फिर जारी रखने के लिए “आगे” पर क्लिक करें। पासवर्ड में बड़े और छोटे अक्षर अलग माने जाते हैं।
PasswordEditLabel=पासवर्ड:
IncorrectPassword=आपका दर्ज किया पासवर्ड सही नहीं है। कृपया फिर कोशिश करें।

; *** "License Agreement" wizard page
WizardLicense=लाइसेंस समझौता
LicenseLabel=जारी रखने से पहले कृपया यह महत्वपूर्ण जानकारी पढ़ें।
LicenseLabel3=कृपया यह लाइसेंस समझौता पढ़ें। इंस्टॉलेशन जारी रखने से पहले आपको इस समझौते की शर्तें स्वीकार करनी होंगी।
LicenseAccepted=मुझे समझौता स्वीकार है
LicenseNotAccepted=मुझे समझौता स्वीकार नहीं है

; *** "Information" wizard pages
WizardInfoBefore=जानकारी
InfoBeforeLabel=जारी रखने से पहले कृपया यह महत्वपूर्ण जानकारी पढ़ें।
InfoBeforeClickLabel=जब आप सेटअप जारी रखने के लिए तैयार हों, तो “आगे” पर क्लिक करें।
WizardInfoAfter=जानकारी
InfoAfterLabel=जारी रखने से पहले कृपया यह महत्वपूर्ण जानकारी पढ़ें।
InfoAfterClickLabel=जब आप सेटअप जारी रखने के लिए तैयार हों, तो “आगे” पर क्लिक करें।

; *** "User Information" wizard page
WizardUserInfo=उपयोगकर्ता की जानकारी
UserInfoDesc=कृपया अपनी जानकारी दर्ज करें।
UserInfoName=उपयोगकर्ता का नाम:
UserInfoOrg=संगठन:
UserInfoSerial=सीरियल नंबर:
UserInfoNameRequired=आपको नाम दर्ज करना होगा।

; *** "Select Destination Location" wizard page
WizardSelectDir=गंतव्य स्थान चुनें
SelectDirDesc=[name] कहाँ इंस्टॉल किया जाए?
SelectDirLabel3=सेटअप [name] को इस फ़ोल्डर में इंस्टॉल करेगा।
SelectDirBrowseLabel=जारी रखने के लिए “आगे” पर क्लिक करें। कोई दूसरा फ़ोल्डर चुनना हो, तो “ब्राउज़ करें” पर क्लिक करें।
DiskSpaceGBLabel=कम से कम [gb] GB खाली डिस्क स्थान चाहिए।
DiskSpaceMBLabel=कम से कम [mb] MB खाली डिस्क स्थान चाहिए।
CannotInstallToNetworkDrive=सेटअप नेटवर्क ड्राइव पर इंस्टॉल नहीं कर सकता।
CannotInstallToUNCPath=सेटअप UNC पथ पर इंस्टॉल नहीं कर सकता।
InvalidPath=आपको ड्राइव अक्षर के साथ पूरा पथ दर्ज करना होगा; उदाहरण के लिए:%n%nC:\APP%n%nया इस रूप में UNC पथ:%n%n\\server\share
InvalidDrive=आपका चुना हुआ ड्राइव या UNC शेयर मौजूद नहीं है या उपलब्ध नहीं है। कृपया कोई दूसरा चुनें।
DiskSpaceWarningTitle=डिस्क स्थान पर्याप्त नहीं
DiskSpaceWarning=इंस्टॉल करने के लिए सेटअप को कम से कम %1 KB खाली स्थान चाहिए, लेकिन चुने हुए ड्राइव पर केवल %2 KB उपलब्ध है।%n%nक्या आप फिर भी जारी रखना चाहते हैं?
DirNameTooLong=फ़ोल्डर का नाम या पथ बहुत लंबा है।
InvalidDirName=फ़ोल्डर का नाम मान्य नहीं है।
BadDirName32=फ़ोल्डर के नाम में इनमें से कोई वर्ण नहीं हो सकता:%n%n%1
DirExistsTitle=फ़ोल्डर पहले से मौजूद है
DirExists=फ़ोल्डर:%n%n%1%n%nपहले से मौजूद है। क्या आप फिर भी उसी फ़ोल्डर में इंस्टॉल करना चाहेंगे?
DirDoesntExistTitle=फ़ोल्डर मौजूद नहीं है
DirDoesntExist=फ़ोल्डर:%n%n%1%n%nमौजूद नहीं है। क्या आप चाहेंगे कि फ़ोल्डर बनाया जाए?

; *** "Select Components" wizard page
WizardSelectComponents=घटक चुनें
SelectComponentsDesc=कौन-से घटक इंस्टॉल किए जाएँ?
SelectComponentsLabel2=जो घटक इंस्टॉल करना चाहते हैं उन्हें चुनें; जिन्हें नहीं करना चाहते उनका चयन हटा दें। तैयार होने पर “आगे” पर क्लिक करें।
FullInstallation=पूर्ण इंस्टॉलेशन
; if possible don't translate 'Compact' as 'Minimal' (I mean 'Minimal' in your language)
CompactInstallation=संक्षिप्त इंस्टॉलेशन
CustomInstallation=अनुकूलित इंस्टॉलेशन
NoUninstallWarningTitle=घटक पहले से मौजूद हैं
NoUninstallWarning=सेटअप ने पाया कि ये घटक आपके कंप्यूटर पर पहले से इंस्टॉल हैं:%n%n%1%n%nइनका चयन हटाने से ये अनइंस्टॉल नहीं होंगे।%n%nक्या आप फिर भी जारी रखना चाहते हैं?
ComponentSize1=%1 KB
ComponentSize2=%1 MB
ComponentsDiskSpaceGBLabel=वर्तमान चयन के लिए कम से कम [gb] GB डिस्क स्थान चाहिए।
ComponentsDiskSpaceMBLabel=वर्तमान चयन के लिए कम से कम [mb] MB डिस्क स्थान चाहिए।

; *** "Select Additional Tasks" wizard page
WizardSelectTasks=अतिरिक्त कार्य चुनें
SelectTasksDesc=कौन-से अतिरिक्त कार्य किए जाएँ?
SelectTasksLabel2=वे अतिरिक्त कार्य चुनें जो आप चाहते हैं कि सेटअप [name] इंस्टॉल करते समय करे, फिर “आगे” पर क्लिक करें।

; *** "Select Start Menu Folder" wizard page
WizardSelectProgramGroup=प्रारंभ मेनू फ़ोल्डर चुनें
SelectStartMenuFolderDesc=सेटअप प्रोग्राम के शॉर्टकट कहाँ रखे?
SelectStartMenuFolderLabel3=सेटअप प्रोग्राम के शॉर्टकट इस प्रारंभ मेनू फ़ोल्डर में बनाएगा।
SelectStartMenuFolderBrowseLabel=जारी रखने के लिए “आगे” पर क्लिक करें। कोई दूसरा फ़ोल्डर चुनना हो, तो “ब्राउज़ करें” पर क्लिक करें।
MustEnterGroupName=आपको फ़ोल्डर का नाम दर्ज करना होगा।
GroupNameTooLong=फ़ोल्डर का नाम या पथ बहुत लंबा है।
InvalidGroupName=फ़ोल्डर का नाम मान्य नहीं है।
BadGroupName=फ़ोल्डर के नाम में इनमें से कोई वर्ण नहीं हो सकता:%n%n%1
NoProgramGroupCheck2=प्रारंभ मेनू फ़ोल्डर न बनाएँ

; *** "Ready to Install" wizard page
WizardReady=इंस्टॉल करने के लिए तैयार
ReadyLabel1=सेटअप अब आपके कंप्यूटर पर [name] इंस्टॉल करने के लिए तैयार है।
ReadyLabel2a=इंस्टॉलेशन जारी रखने के लिए “इंस्टॉल करें” पर क्लिक करें, या कोई सेटिंग देखनी या बदलनी हो, तो “पीछे” पर क्लिक करें।
ReadyLabel2b=इंस्टॉलेशन जारी रखने के लिए “इंस्टॉल करें” पर क्लिक करें।
ReadyMemoUserInfo=उपयोगकर्ता की जानकारी:
ReadyMemoDir=गंतव्य स्थान:
ReadyMemoType=सेटअप का प्रकार:
ReadyMemoComponents=चुने हुए घटक:
ReadyMemoGroup=प्रारंभ मेनू फ़ोल्डर:
ReadyMemoTasks=अतिरिक्त कार्य:

; *** TDownloadWizardPage wizard page and DownloadTemporaryFile
DownloadingLabel2=फ़ाइलें डाउनलोड हो रही हैं...
ButtonStopDownload=डाउनलोड रोकें
StopDownload=क्या आप सचमुच डाउनलोड रोकना चाहते हैं?
ErrorDownloadAborted=डाउनलोड रद्द किया गया
ErrorDownloadFailed=डाउनलोड विफल: %1 %2
ErrorDownloadSizeFailed=आकार प्राप्त नहीं हो सका: %1 %2
ErrorProgress=अमान्य प्रगति: %2 में से %1
ErrorFileSize=अमान्य फ़ाइल आकार: अपेक्षित %1, मिला %2

; *** TExtractionWizardPage wizard page and ExtractArchive
ExtractingLabel=फ़ाइलें निकाली जा रही हैं...
ButtonStopExtraction=निकालना रोकें
StopExtraction=क्या आप सचमुच फ़ाइलें निकालना रोकना चाहते हैं?
ErrorExtractionAborted=फ़ाइलें निकालना रद्द किया गया
ErrorExtractionFailed=फ़ाइलें निकालना विफल: %1

; *** Archive extraction failure details
ArchiveIncorrectPassword=पासवर्ड गलत है
ArchiveIsCorrupted=आर्काइव क्षतिग्रस्त है
ArchiveUnsupportedFormat=आर्काइव का फ़ॉर्मैट समर्थित नहीं है

; *** "Preparing to Install" wizard page
WizardPreparing=इंस्टॉलेशन की तैयारी
PreparingDesc=सेटअप आपके कंप्यूटर पर [name] इंस्टॉल करने की तैयारी कर रहा है।
PreviousInstallNotCompleted=पिछले प्रोग्राम का इंस्टॉलेशन या हटाना पूरा नहीं हुआ था। उसे पूरा करने के लिए आपको कंप्यूटर पुनः प्रारंभ करना होगा।%n%nकंप्यूटर पुनः प्रारंभ करने के बाद [name] का इंस्टॉलेशन पूरा करने के लिए सेटअप फिर चलाएँ।
CannotContinue=सेटअप जारी नहीं रह सकता। बाहर निकलने के लिए कृपया “रद्द करें” पर क्लिक करें।
ApplicationsFound=ये एप्लिकेशन ऐसी फ़ाइलें उपयोग कर रहे हैं जिन्हें सेटअप को अपडेट करना है। सलाह दी जाती है कि आप सेटअप को इन एप्लिकेशन को अपने-आप बंद करने दें।
ApplicationsFound2=ये एप्लिकेशन ऐसी फ़ाइलें उपयोग कर रहे हैं जिन्हें सेटअप को अपडेट करना है। सलाह दी जाती है कि आप सेटअप को इन एप्लिकेशन को अपने-आप बंद करने दें। इंस्टॉलेशन पूरा होने के बाद सेटअप इन एप्लिकेशन को फिर शुरू करने का प्रयास करेगा।
CloseApplications=एप्लिकेशन अपने-आप बंद करें
DontCloseApplications=एप्लिकेशन बंद न करें
ErrorCloseApplications=सेटअप सभी एप्लिकेशन अपने-आप बंद नहीं कर सका। जारी रखने से पहले उन सभी एप्लिकेशन को बंद करने की सलाह दी जाती है जो ऐसी फ़ाइलें उपयोग कर रहे हैं जिन्हें सेटअप को अपडेट करना है।
PrepareToInstallNeedsRestart=सेटअप को आपका कंप्यूटर पुनः प्रारंभ करना होगा। कंप्यूटर पुनः प्रारंभ होने के बाद [name] का इंस्टॉलेशन पूरा करने के लिए सेटअप फिर चलाएँ।%n%nक्या आप अभी पुनः प्रारंभ करना चाहते हैं?

; *** "Installing" wizard page
WizardInstalling=इंस्टॉल हो रहा है
InstallingLabel=कृपया प्रतीक्षा करें, जब तक सेटअप आपके कंप्यूटर पर [name] इंस्टॉल करता है।

; *** "Setup Completed" wizard page
FinishedHeadingLabel=[name] सेटअप विज़ार्ड पूरा हो रहा है
FinishedLabelNoIcons=सेटअप ने आपके कंप्यूटर पर [name] का इंस्टॉलेशन पूरा कर लिया है।
FinishedLabel=सेटअप ने आपके कंप्यूटर पर [name] का इंस्टॉलेशन पूरा कर लिया है। इंस्टॉल किए गए शॉर्टकट चुनकर एप्लिकेशन चलाया जा सकता है।
ClickFinish=सेटअप से बाहर निकलने के लिए “समाप्त करें” पर क्लिक करें।
FinishedRestartLabel=[name] का इंस्टॉलेशन पूरा करने के लिए सेटअप को आपका कंप्यूटर पुनः प्रारंभ करना होगा। क्या आप अभी पुनः प्रारंभ करना चाहते हैं?
FinishedRestartMessage=[name] का इंस्टॉलेशन पूरा करने के लिए सेटअप को आपका कंप्यूटर पुनः प्रारंभ करना होगा।%n%nक्या आप अभी पुनः प्रारंभ करना चाहते हैं?
ShowReadmeCheck=README फ़ाइल देखें
YesRadio=हाँ, कंप्यूटर अभी पुनः प्रारंभ करें
NoRadio=नहीं, कंप्यूटर बाद में पुनः प्रारंभ किया जाएगा
; used for example as 'Run MyProg.exe'
RunEntryExec=%1 चलाएँ
; used for example as 'View Readme.txt'
RunEntryShellExec=%1 देखें

; *** "Setup Needs the Next Disk" stuff
ChangeDiskTitle=सेटअप को अगली डिस्क चाहिए
SelectDiskLabel2=कृपया डिस्क %1 डालें और “ठीक है” पर क्लिक करें।%n%nअगर इस डिस्क की फ़ाइलें नीचे दिखाए गए फ़ोल्डर से अलग किसी फ़ोल्डर में हैं, तो सही पथ दर्ज करें या “ब्राउज़ करें” पर क्लिक करें।
PathLabel=पथ:
FileNotInDir2=फ़ाइल "%1" "%2" में नहीं मिली। कृपया सही डिस्क डालें या कोई दूसरा फ़ोल्डर चुनें।
SelectDirectoryLabel=कृपया अगली डिस्क का स्थान बताएँ।

; *** Installation phase messages
SetupAborted=सेटअप पूरा नहीं हुआ।%n%nकृपया समस्या ठीक करें और सेटअप फिर चलाएँ।
AbortRetryIgnoreSelectAction=कार्रवाई चुनें
AbortRetryIgnoreRetry=फिर कोशिश करें
AbortRetryIgnoreIgnore=त्रुटि अनदेखी करके जारी रखें
AbortRetryIgnoreCancel=इंस्टॉलेशन रद्द करें
RetryCancelSelectAction=कार्रवाई चुनें
RetryCancelRetry=फिर कोशिश करें
RetryCancelCancel=रद्द करें

; *** Installation status messages
StatusClosingApplications=एप्लिकेशन बंद किए जा रहे हैं...
StatusCreateDirs=फ़ोल्डर बनाए जा रहे हैं...
StatusExtractFiles=फ़ाइलें निकाली जा रही हैं...
StatusDownloadFiles=फ़ाइलें डाउनलोड हो रही हैं...
StatusCreateIcons=शॉर्टकट बनाए जा रहे हैं...
StatusCreateIniEntries=INI प्रविष्टियाँ बनाई जा रही हैं...
StatusCreateRegistryEntries=रजिस्ट्री प्रविष्टियाँ बनाई जा रही हैं...
StatusRegisterFiles=फ़ाइलें पंजीकृत की जा रही हैं...
StatusSavingUninstall=अनइंस्टॉल की जानकारी सहेजी जा रही है...
StatusRunProgram=इंस्टॉलेशन पूरा किया जा रहा है...
StatusRestartingApplications=एप्लिकेशन फिर शुरू किए जा रहे हैं...
StatusRollback=बदलाव वापस लिए जा रहे हैं...

; *** Misc. errors
ErrorInternal2=आंतरिक त्रुटि: %1
ErrorFunctionFailedNoCode=%1 विफल रहा
ErrorFunctionFailed=%1 विफल रहा; कोड %2
ErrorFunctionFailedWithMessage=%1 विफल रहा; कोड %2.%n%3
ErrorExecutingProgram=फ़ाइल नहीं चलाई जा सकी:%n%1

; *** Registry errors
ErrorRegOpenKey=रजिस्ट्री कुंजी खोलने में त्रुटि:%n%1\%2
ErrorRegCreateKey=रजिस्ट्री कुंजी बनाने में त्रुटि:%n%1\%2
ErrorRegWriteKey=रजिस्ट्री कुंजी में लिखने में त्रुटि:%n%1\%2

; *** INI errors
ErrorIniEntry=फ़ाइल "%1" में INI प्रविष्टि बनाने में त्रुटि।

; *** File copying errors
FileAbortRetryIgnoreSkipNotRecommended=इस फ़ाइल को छोड़ें (अनुशंसित नहीं)
FileAbortRetryIgnoreIgnoreNotRecommended=त्रुटि अनदेखी करके जारी रखें (अनुशंसित नहीं)
SourceIsCorrupted=स्रोत फ़ाइल क्षतिग्रस्त है
SourceDoesntExist=स्रोत फ़ाइल "%1" मौजूद नहीं है
SourceVerificationFailed=स्रोत फ़ाइल का सत्यापन विफल रहा: %1
VerificationSignatureDoesntExist=हस्ताक्षर फ़ाइल "%1" मौजूद नहीं है
VerificationSignatureInvalid=हस्ताक्षर फ़ाइल "%1" अमान्य है
VerificationKeyNotFound=हस्ताक्षर फ़ाइल "%1" एक अज्ञात कुंजी का उपयोग करती है
VerificationFileNameIncorrect=फ़ाइल का नाम गलत है
VerificationFileTagIncorrect=फ़ाइल का टैग गलत है
VerificationFileSizeIncorrect=फ़ाइल का आकार गलत है
VerificationFileHashIncorrect=फ़ाइल का हैश गलत है
ExistingFileReadOnly2=मौजूदा फ़ाइल बदली नहीं जा सकी, क्योंकि वह केवल-पढ़ने के लिए चिह्नित है।
ExistingFileReadOnlyRetry=केवल-पढ़ने का गुण हटाएँ और फिर कोशिश करें
ExistingFileReadOnlyKeepExisting=मौजूदा फ़ाइल रखें
ErrorReadingExistingDest=मौजूदा फ़ाइल पढ़ने का प्रयास करते समय त्रुटि हुई:
FileExistsSelectAction=कार्रवाई चुनें
FileExists2=फ़ाइल पहले से मौजूद है।
FileExistsOverwriteExisting=मौजूदा फ़ाइल को अधिलेखित करें
FileExistsKeepExisting=मौजूदा फ़ाइल रखें
FileExistsOverwriteOrKeepAll=अगले टकरावों के लिए भी यही करें
ExistingFileNewerSelectAction=कार्रवाई चुनें
ExistingFileNewer2=मौजूदा फ़ाइल उस फ़ाइल से नई है जिसे सेटअप इंस्टॉल करने का प्रयास कर रहा है।
ExistingFileNewerOverwriteExisting=मौजूदा फ़ाइल को अधिलेखित करें
ExistingFileNewerKeepExisting=मौजूदा फ़ाइल रखें (अनुशंसित)
ExistingFileNewerOverwriteOrKeepAll=अगले टकरावों के लिए भी यही करें
ErrorChangingAttr=मौजूदा फ़ाइल के गुण बदलने का प्रयास करते समय त्रुटि हुई:
ErrorCreatingTemp=गंतव्य फ़ोल्डर में फ़ाइल बनाने का प्रयास करते समय त्रुटि हुई:
ErrorReadingSource=स्रोत फ़ाइल पढ़ने का प्रयास करते समय त्रुटि हुई:
ErrorCopying=फ़ाइल कॉपी करने का प्रयास करते समय त्रुटि हुई:
ErrorDownloading=फ़ाइल डाउनलोड करने का प्रयास करते समय त्रुटि हुई:
ErrorExtracting=आर्काइव निकालने का प्रयास करते समय त्रुटि हुई:
ErrorReplacingExistingFile=मौजूदा फ़ाइल बदलने का प्रयास करते समय त्रुटि हुई:
ErrorRestartReplace=RestartReplace विफल रहा:
ErrorRenamingTemp=गंतव्य फ़ोल्डर में फ़ाइल का नाम बदलने का प्रयास करते समय त्रुटि हुई:
ErrorRegisterServer=DLL/OCX पंजीकृत नहीं किया जा सका: %1
ErrorRegSvr32Failed=RegSvr32 निकास कोड %1 के साथ विफल रहा
ErrorRegisterTypeLib=टाइप लाइब्रेरी पंजीकृत नहीं की जा सकी: %1

; *** Uninstall display name markings
; used for example as 'My Program (32-bit)'
UninstallDisplayNameMark=%1 (%2)
; used for example as 'My Program (32-bit, All users)'
UninstallDisplayNameMarks=%1 (%2, %3)
UninstallDisplayNameMark32Bit=32-बिट
UninstallDisplayNameMark64Bit=64-बिट
UninstallDisplayNameMarkAllUsers=सभी उपयोगकर्ता
UninstallDisplayNameMarkCurrentUser=वर्तमान उपयोगकर्ता

; *** Post-installation errors
ErrorOpeningReadme=README फ़ाइल खोलने का प्रयास करते समय त्रुटि हुई।
ErrorRestartingComputer=सेटअप कंप्यूटर पुनः प्रारंभ नहीं कर सका। कृपया यह हाथ से करें।

; *** Uninstaller messages
UninstallNotFound=फ़ाइल "%1" मौजूद नहीं है। अनइंस्टॉल नहीं किया जा सकता।
UninstallOpenError=फ़ाइल "%1" खोली नहीं जा सकी। अनइंस्टॉल नहीं किया जा सकता
UninstallUnsupportedVer=अनइंस्टॉल लॉग फ़ाइल "%1" ऐसे फ़ॉर्मैट में है जिसे अनइंस्टॉलर का यह संस्करण नहीं पहचानता। अनइंस्टॉल नहीं किया जा सकता
UninstallUnknownEntry=अनइंस्टॉल लॉग में एक अज्ञात प्रविष्टि (%1) मिली
ConfirmUninstall=क्या आप सचमुच %1 और उसके सभी घटकों को पूरी तरह हटाना चाहते हैं?
UninstallOnlyOnWin64=इस इंस्टॉलेशन को केवल 64-बिट Windows पर अनइंस्टॉल किया जा सकता है।
OnlyAdminCanUninstall=इस इंस्टॉलेशन को केवल व्यवस्थापक अधिकार वाला उपयोगकर्ता अनइंस्टॉल कर सकता है।
UninstallStatusLabel=कृपया प्रतीक्षा करें, जब तक %1 आपके कंप्यूटर से हटाया जाता है।
UninstalledAll=%1 आपके कंप्यूटर से सफलतापूर्वक हटा दिया गया।
UninstalledMost=%1 का अनइंस्टॉल पूरा हुआ।%n%nकुछ तत्व हटाए नहीं जा सके। इन्हें हाथ से हटाया जा सकता है।
UninstalledAndNeedsRestart=%1 का अनइंस्टॉल पूरा करने के लिए आपका कंप्यूटर पुनः प्रारंभ करना होगा।%n%nक्या आप अभी पुनः प्रारंभ करना चाहते हैं?
UninstallDataCorrupted="%1" फ़ाइल क्षतिग्रस्त है। अनइंस्टॉल नहीं किया जा सकता

; *** Uninstallation phase messages
ConfirmDeleteSharedFileTitle=साझा फ़ाइल हटाएँ?
ConfirmDeleteSharedFile2=सिस्टम के अनुसार निम्नलिखित साझा फ़ाइल का अब कोई प्रोग्राम उपयोग नहीं कर रहा है। क्या आप चाहेंगे कि अनइंस्टॉल इस साझा फ़ाइल को हटा दे?%n%nअगर कोई प्रोग्राम अब भी इस फ़ाइल का उपयोग कर रहा है और इसे हटा दिया गया, तो वह प्रोग्राम ठीक से काम नहीं कर सकता। अगर आप निश्चित नहीं हैं, तो “नहीं” चुनें। फ़ाइल को सिस्टम पर रहने देने से कोई नुकसान नहीं होगा।
SharedFileNameLabel=फ़ाइल का नाम:
SharedFileLocationLabel=स्थान:
WizardUninstalling=अनइंस्टॉल की स्थिति
StatusUninstalling=%1 अनइंस्टॉल हो रहा है...

; *** Shutdown block reasons
ShutdownBlockReasonInstallingApp=%1 इंस्टॉल हो रहा है।
ShutdownBlockReasonUninstallingApp=%1 अनइंस्टॉल हो रहा है।

; The custom messages below aren't used by Setup itself, but if you make
; use of them in your scripts, you'll want to translate them.

[CustomMessages]

NameAndVersion=%1 संस्करण %2
AdditionalIcons=अतिरिक्त शॉर्टकट:
CreateDesktopIcon=डेस्कटॉप शॉर्टकट बनाएँ
CreateQuickLaunchIcon=क्विक लॉन्च शॉर्टकट बनाएँ
ProgramOnTheWeb=वेब पर %1
UninstallProgram=%1 अनइंस्टॉल करें
LaunchProgram=%1 शुरू करें
AssocFileExtension=%1 को %2 फ़ाइल एक्सटेंशन से जोड़ें
AssocingFileExtension=%1 को %2 फ़ाइल एक्सटेंशन से जोड़ा जा रहा है...
AutoStartProgramGroupDescription=स्टार्टअप:
AutoStartProgram=%1 अपने-आप शुरू करें
AddonHostProgramNotFound=आपके चुने हुए फ़ोल्डर में %1 नहीं मिला।%n%nक्या आप फिर भी जारी रखना चाहते हैं?

; The messages that are Lang Switcher's own (the sample is English.isl).
LocalCertReady=सेटअप इस कंप्यूटर पर एक प्रमाणपत्र बनाएगा और उससे Lang Switcher पर हस्ताक्षर करेगा, ताकि यह व्यवस्थापक के रूप में चलाए गए प्रोग्रामों की विंडो में भी काम कर सके।
LocalCertFailed=Lang Switcher इंस्टॉल हो गया, लेकिन यह व्यवस्थापक के रूप में चलाए गए प्रोग्रामों की विंडो में काम नहीं करेगा: Windows ने इस कंप्यूटर पर बनाया गया प्रमाणपत्र स्वीकार नहीं किया। विवरण के लिए सेटअप लॉग देखें।
RemoveSettingsQuestion=क्या Lang Switcher की सेटिंग्स फ़ाइल भी हटानी है?
RemoveSettingsKeep=दोबारा इंस्टॉल करने के लिए अपनी सेटिंग्स रखनी हों, तो “नहीं” चुनें।
AuthorSlogan=दिल से बने प्रोग्राम
