; Lang Switcher -- the messages of the installer in Nepali (stage E108, group A).
;
; Inno Setup has no official Nepali translation (inno_official = false in res\lang\languages.toml),
; so this file carries the whole of it: [LangOptions], every message of the compiler:Default.isl
; (6.5.0+), and the messages of the installer that are Lang Switcher's own. It is read after
; compiler:Default.isl as the second MessagesFile of the "nepali" entry of [Languages] in
; ..\LangSwitcher.iss, and so replaces the English text name for name.
; English.isl is the sample the own messages translate; the glossary of the language is
; l10n\glossary\ne.tsv of the control contour. The name of the program is not translated.
; As in Default.isl: a message that has no full stop there has no danda here (Setup adds its own).
; No "&" accelerators (RULES 16): the file has no official Inno translation, and two equal
; letters in one window would be worse than none. The names of the buttons inside a message
; stand in the quotation marks of the language.

[LangOptions]
LanguageName=नेपाली
LanguageID=$0461
LanguageCodePage=0

[Messages]

; *** Application titles
SetupAppTitle=सेटअप
SetupWindowTitle=सेटअप - %1
UninstallAppTitle=स्थापना रद्द
UninstallAppFullTitle=%1 स्थापना रद्द

; *** Misc. common
InformationTitle=जानकारी
ConfirmTitle=पुष्टि गर्नुहोस्
ErrorTitle=त्रुटि

; *** SetupLdr messages
SetupLdrStartupMessage=यसले %1 स्थापना गर्नेछ। के तपाईं जारी राख्न चाहनुहुन्छ?
LdrCannotCreateTemp=अस्थायी फाइल बनाउन सकिएन। सेटअप रद्द गरियो
LdrCannotExecTemp=अस्थायी फोल्डरमा फाइल चलाउन सकिएन। सेटअप रद्द गरियो
HelpTextNote=

; *** Startup error messages
LastErrorMessage=%1.%n%nत्रुटि %2: %3
SetupFileMissing=स्थापना फोल्डरमा %1 फाइल छैन। कृपया समस्या सच्याउनुहोस् वा प्रोग्रामको नयाँ प्रति प्राप्त गर्नुहोस्।
SetupFileCorrupt=सेटअप फाइलहरू बिग्रिएका छन्। कृपया प्रोग्रामको नयाँ प्रति प्राप्त गर्नुहोस्।
SetupFileCorruptOrWrongVer=सेटअप फाइलहरू बिग्रिएका छन् वा सेटअपको यस संस्करणसँग मिल्दैनन्। कृपया समस्या सच्याउनुहोस् वा प्रोग्रामको नयाँ प्रति प्राप्त गर्नुहोस्।
InvalidParameter=कमाण्ड लाइनमा अमान्य प्यारामिटर दिइयो:%n%n%1
SetupAlreadyRunning=सेटअप पहिले नै चलिरहेको छ।
WindowsVersionNotSupported=यो प्रोग्रामले तपाईंको कम्प्युटरमा चलिरहेको Windows को संस्करणलाई समर्थन गर्दैन।
WindowsServicePackRequired=यो प्रोग्रामलाई %1 सर्भिस प्याक %2 वा त्यसभन्दा पछिको चाहिन्छ।
NotOnThisPlatform=यो प्रोग्राम %1 मा चल्दैन।
OnlyOnThisPlatform=यो प्रोग्राम %1 मा चलाउनुपर्छ।
OnlyOnTheseArchitectures=यो प्रोग्राम निम्न प्रोसेसर आर्किटेक्चरका लागि बनाइएका Windows संस्करणमा मात्र स्थापना गर्न सकिन्छ:%n%n%1
WinVersionTooLowError=यो प्रोग्रामलाई %1 संस्करण %2 वा त्यसभन्दा पछिको चाहिन्छ।
WinVersionTooHighError=यो प्रोग्राम %1 संस्करण %2 वा त्यसभन्दा पछिको संस्करणमा स्थापना गर्न सकिँदैन।
AdminPrivilegesRequired=यो प्रोग्राम स्थापना गर्दा तपाईं प्रशासकका रूपमा साइन इन भएको हुनुपर्छ।
PowerUserPrivilegesRequired=यो प्रोग्राम स्थापना गर्दा तपाईं प्रशासकका रूपमा वा Power Users समूहको सदस्यका रूपमा साइन इन भएको हुनुपर्छ।
SetupAppRunningError=सेटअपले %1 अहिले चलिरहेको फेला पारेको छ।%n%nकृपया यसका सबै विन्डो अहिले बन्द गर्नुहोस्, अनि जारी राख्न “ठिक छ” वा बाहिर निस्कन “रद्द गर्नुहोस्” थिच्नुहोस्।
UninstallAppRunningError=स्थापना रद्द गर्ने प्रोग्रामले %1 अहिले चलिरहेको फेला पारेको छ।%n%nकृपया यसका सबै विन्डो अहिले बन्द गर्नुहोस्, अनि जारी राख्न “ठिक छ” वा बाहिर निस्कन “रद्द गर्नुहोस्” थिच्नुहोस्।

; *** Startup questions
PrivilegesRequiredOverrideTitle=सेटअप स्थापना मोड छान्नुहोस्
PrivilegesRequiredOverrideInstruction=स्थापना मोड छान्नुहोस्
PrivilegesRequiredOverrideText1=%1 सबै प्रयोगकर्ताका लागि (प्रशासकीय अधिकार चाहिन्छ) वा तपाईंका लागि मात्र स्थापना गर्न सकिन्छ।
PrivilegesRequiredOverrideText2=%1 तपाईंका लागि मात्र वा सबै प्रयोगकर्ताका लागि (प्रशासकीय अधिकार चाहिन्छ) स्थापना गर्न सकिन्छ।
PrivilegesRequiredOverrideAllUsers=सबै प्रयोगकर्ताका लागि स्थापना गर्नुहोस्
PrivilegesRequiredOverrideAllUsersRecommended=सबै प्रयोगकर्ताका लागि स्थापना गर्नुहोस् (सिफारिस गरिएको)
PrivilegesRequiredOverrideCurrentUser=मेरा लागि मात्र स्थापना गर्नुहोस्
PrivilegesRequiredOverrideCurrentUserRecommended=मेरा लागि मात्र स्थापना गर्नुहोस् (सिफारिस गरिएको)

; *** Misc. errors
ErrorCreatingDir=सेटअपले “%1” डाइरेक्टरी बनाउन सकेन
ErrorTooManyFilesInDir=“%1” डाइरेक्टरीमा धेरै फाइल भएकाले त्यहाँ फाइल बनाउन सकिएन

; *** Setup common messages
ExitSetupTitle=सेटअपबाट बाहिर निस्कनुहोस्
ExitSetupMessage=सेटअप पूरा भएको छैन। अहिले बाहिर निस्कनुभयो भने प्रोग्राम स्थापना हुनेछैन।%n%nस्थापना पूरा गर्न तपाईं सेटअप पछि फेरि चलाउन सक्नुहुन्छ।%n%nसेटअपबाट बाहिर निस्कने?
AboutSetupMenuItem=सेटअपको बारेमा...
AboutSetupTitle=सेटअपको बारेमा
AboutSetupMessage=%1 संस्करण %2%n%3%n%n%1 को गृहपृष्ठ:%n%4
AboutSetupNote=
TranslatorNote=

; *** Buttons
ButtonBack=< पछाडि
ButtonNext=अर्को >
ButtonInstall=स्थापना गर्नुहोस्
ButtonOK=ठिक छ
ButtonCancel=रद्द गर्नुहोस्
ButtonYes=हो
ButtonYesToAll=सबैका लागि हो
ButtonNo=होइन
ButtonNoToAll=सबैका लागि होइन
ButtonFinish=समाप्त गर्नुहोस्
ButtonBrowse=ब्राउज गर्नुहोस्...
ButtonWizardBrowse=ब्राउज गर्नुहोस्...
ButtonNewFolder=नयाँ फोल्डर बनाउनुहोस्

; *** "Select Language" dialog messages
SelectLanguageTitle=सेटअपको भाषा छान्नुहोस्
SelectLanguageLabel=स्थापनाका बेला प्रयोग गर्ने भाषा छान्नुहोस्।

; *** Common wizard text
ClickNext=जारी राख्न “अर्को” वा सेटअपबाट बाहिर निस्कन “रद्द गर्नुहोस्” थिच्नुहोस्।
BeveledLabel=
BrowseDialogTitle=फोल्डर खोज्नुहोस्
BrowseDialogLabel=तलको सूचीबाट फोल्डर छान्नुहोस्, अनि “ठिक छ” थिच्नुहोस्।
NewFolderName=नयाँ फोल्डर

; *** "Welcome" wizard page
WelcomeLabel1=[name] सेटअप विजार्डमा स्वागत छ
WelcomeLabel2=यसले तपाईंको कम्प्युटरमा [name/ver] स्थापना गर्नेछ।%n%nजारी राख्नुअघि अन्य सबै प्रोग्राम बन्द गर्न सिफारिस गरिन्छ।

; *** "Password" wizard page
WizardPassword=पासवर्ड
PasswordLabel1=यो स्थापना पासवर्डद्वारा सुरक्षित छ।
PasswordLabel3=कृपया पासवर्ड दिनुहोस्, अनि जारी राख्न “अर्को” थिच्नुहोस्। पासवर्डमा ठूला र साना अक्षर फरक मानिन्छन्।
PasswordEditLabel=पासवर्ड:
IncorrectPassword=तपाईंले दिनुभएको पासवर्ड मिलेन। कृपया फेरि प्रयास गर्नुहोस्।

; *** "License Agreement" wizard page
WizardLicense=इजाजतपत्र सम्झौता
LicenseLabel=जारी राख्नुअघि कृपया निम्न महत्त्वपूर्ण जानकारी पढ्नुहोस्।
LicenseLabel3=कृपया निम्न इजाजतपत्र सम्झौता पढ्नुहोस्। स्थापना जारी राख्नुअघि तपाईंले यस सम्झौताका सर्तहरू स्वीकार गर्नुपर्छ।
LicenseAccepted=सम्झौता स्वीकार गरेको छु
LicenseNotAccepted=सम्झौता स्वीकार गरेको छैन

; *** "Information" wizard pages
WizardInfoBefore=जानकारी
InfoBeforeLabel=जारी राख्नुअघि कृपया निम्न महत्त्वपूर्ण जानकारी पढ्नुहोस्।
InfoBeforeClickLabel=सेटअपमा अघि बढ्न तयार हुनुभएपछि “अर्को” थिच्नुहोस्।
WizardInfoAfter=जानकारी
InfoAfterLabel=जारी राख्नुअघि कृपया निम्न महत्त्वपूर्ण जानकारी पढ्नुहोस्।
InfoAfterClickLabel=सेटअपमा अघि बढ्न तयार हुनुभएपछि “अर्को” थिच्नुहोस्।

; *** "User Information" wizard page
WizardUserInfo=प्रयोगकर्ताको जानकारी
UserInfoDesc=कृपया आफ्नो जानकारी भर्नुहोस्।
UserInfoName=प्रयोगकर्ताको नाम:
UserInfoOrg=संस्था:
UserInfoSerial=क्रम संख्या:
UserInfoNameRequired=नाम भर्नुपर्छ।

; *** "Select Destination Location" wizard page
WizardSelectDir=गन्तव्य स्थान छान्नुहोस्
SelectDirDesc=[name] कहाँ स्थापना गर्ने?
SelectDirLabel3=सेटअपले [name] निम्न फोल्डरमा स्थापना गर्नेछ।
SelectDirBrowseLabel=जारी राख्न “अर्को” थिच्नुहोस्। अर्को फोल्डर छान्न चाहनुहुन्छ भने “ब्राउज गर्नुहोस्” थिच्नुहोस्।
DiskSpaceGBLabel=कम्तीमा [gb] GB खाली डिस्क ठाउँ चाहिन्छ।
DiskSpaceMBLabel=कम्तीमा [mb] MB खाली डिस्क ठाउँ चाहिन्छ।
CannotInstallToNetworkDrive=सेटअपले नेटवर्क ड्राइभमा स्थापना गर्न सक्दैन।
CannotInstallToUNCPath=सेटअपले UNC पाथमा स्थापना गर्न सक्दैन।
InvalidPath=ड्राइभ अक्षरसहित पूरा पाथ दिनुपर्छ; उदाहरणका लागि:%n%nC:\APP%n%nवा यस रूपको UNC पाथ:%n%n\\server\share
InvalidDrive=तपाईंले छानेको ड्राइभ वा UNC साझेदारी छैन वा पहुँचयोग्य छैन। कृपया अर्को छान्नुहोस्।
DiskSpaceWarningTitle=डिस्कमा पर्याप्त ठाउँ छैन
DiskSpaceWarning=स्थापनाका लागि सेटअपलाई कम्तीमा %1 KB खाली ठाउँ चाहिन्छ, तर छानिएको ड्राइभमा %2 KB मात्र उपलब्ध छ।%n%nतैपनि जारी राख्ने?
DirNameTooLong=फोल्डरको नाम वा पाथ धेरै लामो छ।
InvalidDirName=फोल्डरको नाम मान्य छैन।
BadDirName32=फोल्डरको नाममा निम्न अक्षर हुन सक्दैनन्:%n%n%1
DirExistsTitle=फोल्डर पहिले नै छ
DirExists=यो फोल्डर:%n%n%1%n%nपहिले नै छ। तैपनि त्यही फोल्डरमा स्थापना गर्ने?
DirDoesntExistTitle=फोल्डर छैन
DirDoesntExist=यो फोल्डर:%n%n%1%n%nछैन। फोल्डर बनाउने?

; *** "Select Components" wizard page
WizardSelectComponents=कम्पोनेन्ट छान्नुहोस्
SelectComponentsDesc=कुन कम्पोनेन्ट स्थापना गर्ने?
SelectComponentsLabel2=स्थापना गर्न चाहेका कम्पोनेन्ट छान्नुहोस्; नचाहिएकाको चिनो हटाउनुहोस्। तयार भएपछि “अर्को” थिच्नुहोस्।
FullInstallation=पूर्ण स्थापना
; if possible don't translate 'Compact' as 'Minimal' (I mean 'Minimal' in your language)
CompactInstallation=संक्षिप्त स्थापना
CustomInstallation=अनुकूलित स्थापना
NoUninstallWarningTitle=कम्पोनेन्ट पहिले नै छन्
NoUninstallWarning=सेटअपले तपाईंको कम्प्युटरमा निम्न कम्पोनेन्ट पहिले नै स्थापना गरिएको फेला पारेको छ:%n%n%1%n%nयी कम्पोनेन्टको चिनो हटाउँदा तिनको स्थापना रद्द हुने छैन।%n%nतैपनि जारी राख्ने?
ComponentSize1=%1 KB
ComponentSize2=%1 MB
ComponentsDiskSpaceGBLabel=हालको चयनलाई कम्तीमा [gb] GB डिस्क ठाउँ चाहिन्छ।
ComponentsDiskSpaceMBLabel=हालको चयनलाई कम्तीमा [mb] MB डिस्क ठाउँ चाहिन्छ।

; *** "Select Additional Tasks" wizard page
WizardSelectTasks=थप कामहरू छान्नुहोस्
SelectTasksDesc=कुन थप कामहरू गर्ने?
SelectTasksLabel2=[name] स्थापना गर्दा सेटअपले गर्नुपर्ने थप कामहरू छान्नुहोस्, अनि “अर्को” थिच्नुहोस्।

; *** "Select Start Menu Folder" wizard page
WizardSelectProgramGroup=स्टार्ट मेनु फोल्डर छान्नुहोस्
SelectStartMenuFolderDesc=सेटअपले प्रोग्रामका सर्टकट कहाँ राख्ने?
SelectStartMenuFolderLabel3=सेटअपले प्रोग्रामका सर्टकट निम्न स्टार्ट मेनु फोल्डरमा बनाउनेछ।
SelectStartMenuFolderBrowseLabel=जारी राख्न “अर्को” थिच्नुहोस्। अर्को फोल्डर छान्न चाहनुहुन्छ भने “ब्राउज गर्नुहोस्” थिच्नुहोस्।
MustEnterGroupName=फोल्डरको नाम भर्नुपर्छ।
GroupNameTooLong=फोल्डरको नाम वा पाथ धेरै लामो छ।
InvalidGroupName=फोल्डरको नाम मान्य छैन।
BadGroupName=फोल्डरको नाममा निम्न अक्षर हुन सक्दैनन्:%n%n%1
NoProgramGroupCheck2=स्टार्ट मेनु फोल्डर नबनाउनुहोस्

; *** "Ready to Install" wizard page
WizardReady=स्थापना गर्न तयार
ReadyLabel1=सेटअप अब तपाईंको कम्प्युटरमा [name] स्थापना गर्न तयार छ।
ReadyLabel2a=स्थापना जारी राख्न “स्थापना गर्नुहोस्” थिच्नुहोस्, वा कुनै सेटिङ हेर्न वा बदल्न चाहनुहुन्छ भने “पछाडि” थिच्नुहोस्।
ReadyLabel2b=स्थापना जारी राख्न “स्थापना गर्नुहोस्” थिच्नुहोस्।
ReadyMemoUserInfo=प्रयोगकर्ताको जानकारी:
ReadyMemoDir=गन्तव्य स्थान:
ReadyMemoType=सेटअपको प्रकार:
ReadyMemoComponents=छानिएका कम्पोनेन्ट:
ReadyMemoGroup=स्टार्ट मेनु फोल्डर:
ReadyMemoTasks=थप कामहरू:

; *** TDownloadWizardPage wizard page and DownloadTemporaryFile
DownloadingLabel2=फाइलहरू डाउनलोड हुँदैछन्...
ButtonStopDownload=डाउनलोड रोक्नुहोस्
StopDownload=के तपाईं साँच्चै डाउनलोड रोक्न चाहनुहुन्छ?
ErrorDownloadAborted=डाउनलोड रद्द गरियो
ErrorDownloadFailed=डाउनलोड असफल: %1 %2
ErrorDownloadSizeFailed=आकार पाउन सकिएन: %1 %2
ErrorProgress=अमान्य प्रगति: %2 मध्ये %1
ErrorFileSize=फाइलको आकार अमान्य: अपेक्षित %1, भेटिएको %2

; *** TExtractionWizardPage wizard page and ExtractArchive
ExtractingLabel=फाइलहरू निकालिँदैछन्...
ButtonStopExtraction=निकाल्न रोक्नुहोस्
StopExtraction=के तपाईं साँच्चै फाइल निकाल्न रोक्न चाहनुहुन्छ?
ErrorExtractionAborted=फाइल निकाल्ने काम रद्द गरियो
ErrorExtractionFailed=फाइल निकाल्न असफल: %1

; *** Archive extraction failure details
ArchiveIncorrectPassword=पासवर्ड मिलेन
ArchiveIsCorrupted=आर्काइभ बिग्रिएको छ
ArchiveUnsupportedFormat=आर्काइभको ढाँचा समर्थित छैन

; *** "Preparing to Install" wizard page
WizardPreparing=स्थापनाको तयारी
PreparingDesc=सेटअप तपाईंको कम्प्युटरमा [name] स्थापना गर्ने तयारी गर्दैछ।
PreviousInstallNotCompleted=अघिल्लो प्रोग्रामको स्थापना/हटाइ पूरा भएको छैन। त्यो पूरा गर्न तपाईंले कम्प्युटर फेरि सुरु गर्नुपर्छ।%n%nकम्प्युटर फेरि सुरु गरेपछि [name] को स्थापना पूरा गर्न सेटअप फेरि चलाउनुहोस्।
CannotContinue=सेटअप जारी राख्न सक्दैन। बाहिर निस्कन “रद्द गर्नुहोस्” थिच्नुहोस्।
ApplicationsFound=सेटअपले अद्यावधिक गर्नुपर्ने फाइलहरू निम्न प्रोग्रामले प्रयोग गरिरहेका छन्। यी प्रोग्राम सेटअपले आफैँ बन्द गर्न दिन सिफारिस गरिन्छ।
ApplicationsFound2=सेटअपले अद्यावधिक गर्नुपर्ने फाइलहरू निम्न प्रोग्रामले प्रयोग गरिरहेका छन्। यी प्रोग्राम सेटअपले आफैँ बन्द गर्न दिन सिफारिस गरिन्छ। स्थापना सकिएपछि सेटअपले यी प्रोग्राम फेरि सुरु गर्ने प्रयास गर्नेछ।
CloseApplications=प्रोग्राम आफैँ बन्द गर्नुहोस्
DontCloseApplications=प्रोग्राम बन्द नगर्नुहोस्
ErrorCloseApplications=सेटअपले सबै प्रोग्राम आफैँ बन्द गर्न सकेन। जारी राख्नुअघि सेटअपले अद्यावधिक गर्नुपर्ने फाइल प्रयोग गर्ने सबै प्रोग्राम बन्द गर्न सिफारिस गरिन्छ।
PrepareToInstallNeedsRestart=सेटअपले तपाईंको कम्प्युटर फेरि सुरु गर्नुपर्छ। कम्प्युटर फेरि सुरु गरेपछि [name] को स्थापना पूरा गर्न सेटअप फेरि चलाउनुहोस्।%n%nअहिले फेरि सुरु गर्ने?

; *** "Installing" wizard page
WizardInstalling=स्थापना हुँदैछ
InstallingLabel=सेटअपले तपाईंको कम्प्युटरमा [name] स्थापना गर्दैछ, कृपया पर्खनुहोस्।

; *** "Setup Completed" wizard page
FinishedHeadingLabel=[name] सेटअप विजार्ड पूरा हुँदैछ
FinishedLabelNoIcons=सेटअपले तपाईंको कम्प्युटरमा [name] स्थापना गरिसकेको छ।
FinishedLabel=सेटअपले तपाईंको कम्प्युटरमा [name] स्थापना गरिसकेको छ। स्थापना गरिएका सर्टकट छानेर प्रोग्राम सुरु गर्न सकिन्छ।
ClickFinish=सेटअपबाट बाहिर निस्कन “समाप्त गर्नुहोस्” थिच्नुहोस्।
FinishedRestartLabel=[name] को स्थापना पूरा गर्न सेटअपले तपाईंको कम्प्युटर फेरि सुरु गर्नुपर्छ। अहिले फेरि सुरु गर्ने?
FinishedRestartMessage=[name] को स्थापना पूरा गर्न सेटअपले तपाईंको कम्प्युटर फेरि सुरु गर्नुपर्छ।%n%nअहिले फेरि सुरु गर्ने?
ShowReadmeCheck=हो, म README फाइल हेर्न चाहन्छु
YesRadio=हो, कम्प्युटर अहिले फेरि सुरु गर्नुहोस्
NoRadio=होइन, म कम्प्युटर पछि फेरि सुरु गर्नेछु
; used for example as 'Run MyProg.exe'
RunEntryExec=%1 चलाउनुहोस्
; used for example as 'View Readme.txt'
RunEntryShellExec=%1 हेर्नुहोस्

; *** "Setup Needs the Next Disk" stuff
ChangeDiskTitle=सेटअपलाई अर्को डिस्क चाहिन्छ
SelectDiskLabel2=कृपया डिस्क %1 हाल्नुहोस् र “ठिक छ” थिच्नुहोस्।%n%nयो डिस्कका फाइलहरू तल देखाइएको फोल्डरभन्दा फरक फोल्डरमा भेटिन्छन् भने सही पाथ लेख्नुहोस् वा “ब्राउज गर्नुहोस्” थिच्नुहोस्।
PathLabel=पाथ:
FileNotInDir2=“%1” फाइल “%2” मा भेटिएन। कृपया सही डिस्क हाल्नुहोस् वा अर्को फोल्डर छान्नुहोस्।
SelectDirectoryLabel=कृपया अर्को डिस्कको स्थान खुलाउनुहोस्।

; *** Installation phase messages
SetupAborted=सेटअप पूरा भएन।%n%nकृपया समस्या सच्याउनुहोस् र सेटअप फेरि चलाउनुहोस्।
AbortRetryIgnoreSelectAction=कार्य छान्नुहोस्
AbortRetryIgnoreRetry=फेरि प्रयास गर्नुहोस्
AbortRetryIgnoreIgnore=त्रुटिलाई बेवास्ता गरी जारी राख्नुहोस्
AbortRetryIgnoreCancel=स्थापना रद्द गर्नुहोस्
RetryCancelSelectAction=कार्य छान्नुहोस्
RetryCancelRetry=फेरि प्रयास गर्नुहोस्
RetryCancelCancel=रद्द गर्नुहोस्

; *** Installation status messages
StatusClosingApplications=प्रोग्राम बन्द गरिँदैछ...
StatusCreateDirs=फोल्डर बनाइँदैछ...
StatusExtractFiles=फाइलहरू निकालिँदैछन्...
StatusDownloadFiles=फाइलहरू डाउनलोड हुँदैछन्...
StatusCreateIcons=सर्टकट बनाइँदैछ...
StatusCreateIniEntries=INI प्रविष्टि बनाइँदैछ...
StatusCreateRegistryEntries=रजिस्ट्री प्रविष्टि बनाइँदैछ...
StatusRegisterFiles=फाइलहरू दर्ता गरिँदैछ...
StatusSavingUninstall=स्थापना रद्द गर्ने जानकारी सुरक्षित गरिँदैछ...
StatusRunProgram=स्थापना पूरा गरिँदैछ...
StatusRestartingApplications=प्रोग्राम फेरि सुरु गरिँदैछ...
StatusRollback=परिवर्तन फिर्ता गरिँदैछ...

; *** Misc. errors
ErrorInternal2=आन्तरिक त्रुटि: %1
ErrorFunctionFailedNoCode=%1 असफल भयो
ErrorFunctionFailed=%1 असफल भयो; कोड %2
ErrorFunctionFailedWithMessage=%1 असफल भयो; कोड %2.%n%3
ErrorExecutingProgram=फाइल चलाउन सकिएन:%n%1

; *** Registry errors
ErrorRegOpenKey=रजिस्ट्री कुञ्जी खोल्दा त्रुटि:%n%1\%2
ErrorRegCreateKey=रजिस्ट्री कुञ्जी बनाउँदा त्रुटि:%n%1\%2
ErrorRegWriteKey=रजिस्ट्री कुञ्जीमा लेख्दा त्रुटि:%n%1\%2

; *** INI errors
ErrorIniEntry=“%1” फाइलमा INI प्रविष्टि बनाउँदा त्रुटि भयो।

; *** File copying errors
FileAbortRetryIgnoreSkipNotRecommended=यो फाइल छोड्नुहोस् (सिफारिस गरिएको छैन)
FileAbortRetryIgnoreIgnoreNotRecommended=त्रुटिलाई बेवास्ता गरी जारी राख्नुहोस् (सिफारिस गरिएको छैन)
SourceIsCorrupted=स्रोत फाइल बिग्रिएको छ
SourceDoesntExist=“%1” स्रोत फाइल छैन
SourceVerificationFailed=स्रोत फाइलको प्रमाणीकरण असफल: %1
VerificationSignatureDoesntExist=“%1” हस्ताक्षर फाइल छैन
VerificationSignatureInvalid=“%1” हस्ताक्षर फाइल अमान्य छ
VerificationKeyNotFound=“%1” हस्ताक्षर फाइलले अज्ञात कुञ्जी प्रयोग गर्छ
VerificationFileNameIncorrect=फाइलको नाम गलत छ
VerificationFileTagIncorrect=फाइलको ट्याग गलत छ
VerificationFileSizeIncorrect=फाइलको आकार गलत छ
VerificationFileHashIncorrect=फाइलको ह्यास गलत छ
ExistingFileReadOnly2=पहिले नै भएको फाइल केवल-पढ्नका लागि चिनो लगाइएको हुँदा त्यसलाई बदल्न सकिएन।
ExistingFileReadOnlyRetry=केवल-पढ्ने गुण हटाएर फेरि प्रयास गर्नुहोस्
ExistingFileReadOnlyKeepExisting=पहिलेको फाइल राख्नुहोस्
ErrorReadingExistingDest=पहिले नै भएको फाइल पढ्दा त्रुटि भयो:
FileExistsSelectAction=कार्य छान्नुहोस्
FileExists2=फाइल पहिले नै छ।
FileExistsOverwriteExisting=पहिलेको फाइलमा लेख्नुहोस्
FileExistsKeepExisting=पहिलेको फाइल राख्नुहोस्
FileExistsOverwriteOrKeepAll=अर्का द्वन्द्वका लागि पनि यही गर्नुहोस्
ExistingFileNewerSelectAction=कार्य छान्नुहोस्
ExistingFileNewer2=पहिले नै भएको फाइल सेटअपले स्थापना गर्न खोजेको फाइलभन्दा नयाँ छ।
ExistingFileNewerOverwriteExisting=पहिलेको फाइलमा लेख्नुहोस्
ExistingFileNewerKeepExisting=पहिलेको फाइल राख्नुहोस् (सिफारिस गरिएको)
ExistingFileNewerOverwriteOrKeepAll=अर्का द्वन्द्वका लागि पनि यही गर्नुहोस्
ErrorChangingAttr=पहिले नै भएको फाइलका गुण बदल्दा त्रुटि भयो:
ErrorCreatingTemp=गन्तव्य फोल्डरमा फाइल बनाउँदा त्रुटि भयो:
ErrorReadingSource=स्रोत फाइल पढ्दा त्रुटि भयो:
ErrorCopying=फाइल कपी गर्दा त्रुटि भयो:
ErrorDownloading=फाइल डाउनलोड गर्दा त्रुटि भयो:
ErrorExtracting=आर्काइभबाट फाइल निकाल्दा त्रुटि भयो:
ErrorReplacingExistingFile=पहिले नै भएको फाइल बदल्दा त्रुटि भयो:
ErrorRestartReplace=RestartReplace असफल:
ErrorRenamingTemp=गन्तव्य फोल्डरमा फाइलको नाम बदल्दा त्रुटि भयो:
ErrorRegisterServer=DLL/OCX दर्ता गर्न सकिएन: %1
ErrorRegSvr32Failed=RegSvr32 एक्जिट कोड %1 सँग असफल भयो
ErrorRegisterTypeLib=टाइप लाइब्रेरी दर्ता गर्न सकिएन: %1

; *** Uninstall display name markings
; used for example as 'My Program (32-bit)'
UninstallDisplayNameMark=%1 (%2)
; used for example as 'My Program (32-bit, All users)'
UninstallDisplayNameMarks=%1 (%2, %3)
UninstallDisplayNameMark32Bit=32-बिट
UninstallDisplayNameMark64Bit=64-बिट
UninstallDisplayNameMarkAllUsers=सबै प्रयोगकर्ता
UninstallDisplayNameMarkCurrentUser=हालको प्रयोगकर्ता

; *** Post-installation errors
ErrorOpeningReadme=README फाइल खोल्दा त्रुटि भयो।
ErrorRestartingComputer=सेटअपले कम्प्युटर फेरि सुरु गर्न सकेन। कृपया यो आफैँ गर्नुहोस्।

; *** Uninstaller messages
UninstallNotFound=“%1” फाइल छैन। स्थापना रद्द गर्न सकिँदैन।
UninstallOpenError=“%1” फाइल खोल्न सकिएन। स्थापना रद्द गर्न सकिँदैन
UninstallUnsupportedVer=“%1” स्थापना रद्द गर्ने लग फाइल यो स्थापना रद्द गर्ने प्रोग्रामको संस्करणले नचिन्ने ढाँचामा छ। स्थापना रद्द गर्न सकिँदैन
UninstallUnknownEntry=स्थापना रद्द गर्ने लगमा अज्ञात प्रविष्टि (%1) भेटियो
ConfirmUninstall=के तपाईं %1 र यसका सबै कम्पोनेन्ट पूर्ण रूपमा हटाउन चाहनुहुन्छ?
UninstallOnlyOnWin64=यो स्थापना 64-बिट Windows मा मात्र रद्द गर्न सकिन्छ।
OnlyAdminCanUninstall=यो स्थापना प्रशासकीय अधिकार भएको प्रयोगकर्ताले मात्र रद्द गर्न सक्छ।
UninstallStatusLabel=%1 तपाईंको कम्प्युटरबाट हटाइँदैछ, कृपया पर्खनुहोस्।
UninstalledAll=%1 तपाईंको कम्प्युटरबाट सफलतापूर्वक हटाइयो।
UninstalledMost=%1 को स्थापना रद्द पूरा भयो।%n%nकेही तत्त्व हटाउन सकिएन। तिनलाई हातैले हटाउन सकिन्छ।
UninstalledAndNeedsRestart=%1 को स्थापना रद्द पूरा गर्न तपाईंको कम्प्युटर फेरि सुरु गर्नुपर्छ।%n%nअहिले फेरि सुरु गर्ने?
UninstallDataCorrupted=“%1” फाइल बिग्रिएको छ। स्थापना रद्द गर्न सकिँदैन

; *** Uninstallation phase messages
ConfirmDeleteSharedFileTitle=साझा फाइल हटाउने?
ConfirmDeleteSharedFile2=निम्न साझा फाइल अब कुनै प्रोग्रामले प्रयोग गरिरहेको छैन भनी प्रणालीले जनाउँछ। स्थापना रद्द गर्दा यो साझा फाइल हटाउने?%n%nकुनै प्रोग्रामले अझै यो फाइल प्रयोग गरिरहेको छ र यो हटाइयो भने ती प्रोग्राम राम्ररी नचल्न सक्छन्। पक्का छैन भने “होइन” छान्नुहोस्। फाइल प्रणालीमा राख्दा कुनै हानि हुँदैन।
SharedFileNameLabel=फाइलको नाम:
SharedFileLocationLabel=स्थान:
WizardUninstalling=स्थापना रद्दको अवस्था
StatusUninstalling=%1 को स्थापना रद्द हुँदैछ...

; *** Shutdown block reasons
ShutdownBlockReasonInstallingApp=%1 स्थापना हुँदैछ।
ShutdownBlockReasonUninstallingApp=%1 को स्थापना रद्द हुँदैछ।

; The custom messages below aren't used by Setup itself, but if you make
; use of them in your scripts, you'll want to translate them.

[CustomMessages]

NameAndVersion=%1 संस्करण %2
AdditionalIcons=थप सर्टकट:
CreateDesktopIcon=डेस्कटप सर्टकट बनाउनुहोस्
CreateQuickLaunchIcon=क्विक लन्च सर्टकट बनाउनुहोस्
ProgramOnTheWeb=वेबमा %1
UninstallProgram=%1 को स्थापना रद्द गर्नुहोस्
LaunchProgram=%1 सुरु गर्नुहोस्
AssocFileExtension=%1 लाई %2 फाइल एक्सटेन्सनसँग जोड्नुहोस्
AssocingFileExtension=%1 लाई %2 फाइल एक्सटेन्सनसँग जोडिँदैछ...
AutoStartProgramGroupDescription=स्टार्टअप:
AutoStartProgram=%1 आफैँ सुरु गर्नुहोस्
AddonHostProgramNotFound=तपाईंले छानेको फोल्डरमा %1 भेटिएन।%n%nतैपनि जारी राख्ने?

; The messages that are Lang Switcher's own (the sample is English.isl).
LocalCertReady=सेटअपले यो कम्प्युटरमा एउटा प्रमाणपत्र बनाउनेछ र त्यसद्वारा Lang Switcher मा हस्ताक्षर गर्नेछ, जसले गर्दा यो प्रशासकका रूपमा चलाइएका प्रोग्रामका विन्डोमा पनि काम गर्न सक्नेछ।
LocalCertFailed=Lang Switcher स्थापना भयो, तर यो प्रशासकका रूपमा चलाइएका प्रोग्रामका विन्डोमा काम गर्ने छैन: यो कम्प्युटरमा बनाइएको प्रमाणपत्र Windows ले स्वीकार गरेन। विवरणका लागि सेटअप लग हेर्नुहोस्।
RemoveSettingsQuestion=के Lang Switcher को सेटिङ फाइल पनि हटाउने?
RemoveSettingsKeep=पछि फेरि स्थापना गर्दा सेटिङ जोगाउन “होइन” छान्नुहोस्।
AuthorSlogan=मन लगाएर बनाइएका प्रोग्राम
