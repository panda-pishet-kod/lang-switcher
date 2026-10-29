; Lang Switcher -- the messages of the installer in Telugu (stage E108, group A).
;
; Inno Setup has no official Telugu translation (inno_official = false in res\lang\languages.toml),
; so this file carries the whole of it: [LangOptions], every message of the compiler:Default.isl
; (6.5.0+), and the messages of the installer that are Lang Switcher's own. It is read after
; compiler:Default.isl as the second MessagesFile of the "telugu" entry of [Languages] in
; ..\LangSwitcher.iss, and so replaces the English text name for name.
; English.isl is the sample the own messages translate; the glossary of the language is
; l10n\glossary\te.tsv of the control contour. The name of the program is not translated.
; No accelerators (&): a file without an official translation puts none, so that one window never
; shows two equal letters (RULES 16). The names of buttons inside a message stand in the quotation
; marks of the language. As in Default.isl: a message that has no full stop there has none here
; (Setup adds its own).

[LangOptions]
LanguageName=తెలుగు
LanguageID=$044A
LanguageCodePage=0

[Messages]

; *** Application titles
SetupAppTitle=సెటప్
SetupWindowTitle=సెటప్ - %1
UninstallAppTitle=అన్‌ఇన్‌స్టాల్
UninstallAppFullTitle=%1 అన్‌ఇన్‌స్టాల్

; *** Misc. common
InformationTitle=సమాచారం
ConfirmTitle=నిర్ధారించండి
ErrorTitle=లోపం

; *** SetupLdr messages
SetupLdrStartupMessage=ఇది %1 ను ఇన్‌స్టాల్ చేస్తుంది. కొనసాగించాలనుకుంటున్నారా?
LdrCannotCreateTemp=తాత్కాలిక ఫైల్‌ను సృష్టించలేకపోయింది. సెటప్ ఆపివేయబడింది
LdrCannotExecTemp=తాత్కాలిక ఫోల్డర్‌లోని ఫైల్‌ను అమలు చేయలేకపోయింది. సెటప్ ఆపివేయబడింది
HelpTextNote=

; *** Startup error messages
LastErrorMessage=%1.%n%nలోపం %2: %3
SetupFileMissing=ఇన్‌స్టాలేషన్ ఫోల్డర్‌లో %1 ఫైల్ లేదు. దయచేసి సమస్యను సరిచేయండి లేదా ప్రోగ్రామ్ కొత్త కాపీని పొందండి.
SetupFileCorrupt=సెటప్ ఫైళ్లు పాడైపోయాయి. దయచేసి ప్రోగ్రామ్ కొత్త కాపీని పొందండి.
SetupFileCorruptOrWrongVer=సెటప్ ఫైళ్లు పాడైపోయాయి లేదా ఈ సెటప్ సంస్కరణకు అనుకూలంగా లేవు. దయచేసి సమస్యను సరిచేయండి లేదా ప్రోగ్రామ్ కొత్త కాపీని పొందండి.
InvalidParameter=కమాండ్ లైన్‌లో చెల్లని పారామీటర్ ఇవ్వబడింది:%n%n%1
SetupAlreadyRunning=సెటప్ ఇప్పటికే నడుస్తోంది.
WindowsVersionNotSupported=ఈ ప్రోగ్రామ్ మీ కంప్యూటర్‌లోని Windows సంస్కరణకు మద్దతు ఇవ్వదు.
WindowsServicePackRequired=ఈ ప్రోగ్రామ్‌కు %1 సర్వీస్ ప్యాక్ %2 లేదా తరువాతిది అవసరం.
NotOnThisPlatform=ఈ ప్రోగ్రామ్ %1 లో నడవదు.
OnlyOnThisPlatform=ఈ ప్రోగ్రామ్‌ను %1 లో మాత్రమే నడపాలి.
OnlyOnTheseArchitectures=ఈ ప్రోగ్రామ్‌ను క్రింది ప్రాసెసర్ ఆర్కిటెక్చర్‌ల కోసం రూపొందించిన Windows సంస్కరణలలో మాత్రమే ఇన్‌స్టాల్ చేయవచ్చు:%n%n%1
WinVersionTooLowError=ఈ ప్రోగ్రామ్‌కు %1 సంస్కరణ %2 లేదా తరువాతిది అవసరం.
WinVersionTooHighError=ఈ ప్రోగ్రామ్‌ను %1 సంస్కరణ %2 లేదా తరువాతి దానిపై ఇన్‌స్టాల్ చేయలేరు.
AdminPrivilegesRequired=ఈ ప్రోగ్రామ్‌ను ఇన్‌స్టాల్ చేసేటప్పుడు మీరు అడ్మినిస్ట్రేటర్‌గా సైన్ ఇన్ అయి ఉండాలి.
PowerUserPrivilegesRequired=ఈ ప్రోగ్రామ్‌ను ఇన్‌స్టాల్ చేసేటప్పుడు మీరు అడ్మినిస్ట్రేటర్‌గా లేదా Power Users సమూహ సభ్యునిగా సైన్ ఇన్ అయి ఉండాలి.
SetupAppRunningError=%1 ప్రస్తుతం నడుస్తున్నట్లు సెటప్ గుర్తించింది.%n%nదయచేసి దాని అన్ని ఇన్‌స్టెన్స్‌లను ఇప్పుడు మూసివేసి, కొనసాగించడానికి “సరే” లేదా నిష్క్రమించడానికి “రద్దు” నొక్కండి.
UninstallAppRunningError=%1 ప్రస్తుతం నడుస్తున్నట్లు అన్‌ఇన్‌స్టాల్ గుర్తించింది.%n%nదయచేసి దాని అన్ని ఇన్‌స్టెన్స్‌లను ఇప్పుడు మూసివేసి, కొనసాగించడానికి “సరే” లేదా నిష్క్రమించడానికి “రద్దు” నొక్కండి.

; *** Startup questions
PrivilegesRequiredOverrideTitle=సెటప్ ఇన్‌స్టాల్ మోడ్‌ను ఎంచుకోండి
PrivilegesRequiredOverrideInstruction=ఇన్‌స్టాల్ మోడ్‌ను ఎంచుకోండి
PrivilegesRequiredOverrideText1=%1 ను వినియోగదారులందరి కోసం (అడ్మినిస్ట్రేటివ్ హక్కులు అవసరం) లేదా మీ కోసం మాత్రమే ఇన్‌స్టాల్ చేయవచ్చు.
PrivilegesRequiredOverrideText2=%1 ను మీ కోసం మాత్రమే లేదా వినియోగదారులందరి కోసం (అడ్మినిస్ట్రేటివ్ హక్కులు అవసరం) ఇన్‌స్టాల్ చేయవచ్చు.
PrivilegesRequiredOverrideAllUsers=వినియోగదారులందరి కోసం ఇన్‌స్టాల్ చేయండి
PrivilegesRequiredOverrideAllUsersRecommended=వినియోగదారులందరి కోసం ఇన్‌స్టాల్ చేయండి (సిఫార్సు చేయబడింది)
PrivilegesRequiredOverrideCurrentUser=నా కోసం మాత్రమే ఇన్‌స్టాల్ చేయండి
PrivilegesRequiredOverrideCurrentUserRecommended=నా కోసం మాత్రమే ఇన్‌స్టాల్ చేయండి (సిఫార్సు చేయబడింది)

; *** Misc. errors
ErrorCreatingDir=సెటప్ "%1" డైరెక్టరీని సృష్టించలేకపోయింది
ErrorTooManyFilesInDir="%1" డైరెక్టరీలో చాలా ఫైళ్లు ఉన్నందున అందులో ఫైల్‌ను సృష్టించలేకపోయింది

; *** Setup common messages
ExitSetupTitle=సెటప్ నుండి నిష్క్రమించండి
ExitSetupMessage=సెటప్ పూర్తి కాలేదు. ఇప్పుడు నిష్క్రమిస్తే, ప్రోగ్రామ్ ఇన్‌స్టాల్ కాదు.%n%nఇన్‌స్టాలేషన్ పూర్తి చేయడానికి మీరు సెటప్‌ను మరొక సమయంలో మళ్లీ నడపవచ్చు.%n%nసెటప్ నుండి నిష్క్రమించాలా?
AboutSetupMenuItem=సెటప్ గురించి...
AboutSetupTitle=సెటప్ గురించి
AboutSetupMessage=%1 సంస్కరణ %2%n%3%n%n%1 హోమ్ పేజీ:%n%4
AboutSetupNote=
TranslatorNote=

; *** Buttons
ButtonBack=< వెనుకకు
ButtonNext=తదుపరి >
ButtonInstall=ఇన్‌స్టాల్ చేయండి
ButtonOK=సరే
ButtonCancel=రద్దు
ButtonYes=అవును
ButtonYesToAll=అన్నిటికీ అవును
ButtonNo=కాదు
ButtonNoToAll=అన్నిటికీ కాదు
ButtonFinish=ముగించండి
ButtonBrowse=బ్రౌజ్ చేయండి...
ButtonWizardBrowse=బ్రౌజ్ చేయండి...
ButtonNewFolder=కొత్త ఫోల్డర్‌ను సృష్టించండి

; *** "Select Language" dialog messages
SelectLanguageTitle=సెటప్ భాషను ఎంచుకోండి
SelectLanguageLabel=ఇన్‌స్టాలేషన్ సమయంలో ఉపయోగించాల్సిన భాషను ఎంచుకోండి.

; *** Common wizard text
ClickNext=కొనసాగించడానికి “తదుపరి” నొక్కండి, లేదా సెటప్ నుండి నిష్క్రమించడానికి “రద్దు” నొక్కండి.
BeveledLabel=
BrowseDialogTitle=ఫోల్డర్ కోసం బ్రౌజ్ చేయండి
BrowseDialogLabel=క్రింది జాబితాలో ఒక ఫోల్డర్‌ను ఎంచుకుని, ఆపై “సరే” నొక్కండి.
NewFolderName=కొత్త ఫోల్డర్

; *** "Welcome" wizard page
WelcomeLabel1=[name] సెటప్ విజార్డ్‌కు స్వాగతం
WelcomeLabel2=ఇది మీ కంప్యూటర్‌లో [name/ver] ను ఇన్‌స్టాల్ చేస్తుంది.%n%nకొనసాగే ముందు ఇతర అన్ని అప్లికేషన్‌లను మూసివేయడం మంచిది.

; *** "Password" wizard page
WizardPassword=పాస్‌వర్డ్
PasswordLabel1=ఈ ఇన్‌స్టాలేషన్ పాస్‌వర్డ్‌తో రక్షించబడింది.
PasswordLabel3=దయచేసి పాస్‌వర్డ్ ఇచ్చి, కొనసాగించడానికి “తదుపరి” నొక్కండి. పాస్‌వర్డ్‌లలో పెద్ద, చిన్న అక్షరాలు వేరుగా పరిగణించబడతాయి.
PasswordEditLabel=పాస్‌వర్డ్:
IncorrectPassword=మీరు ఇచ్చిన పాస్‌వర్డ్ సరైనది కాదు. దయచేసి మళ్లీ ప్రయత్నించండి.

; *** "License Agreement" wizard page
WizardLicense=లైసెన్స్ ఒప్పందం
LicenseLabel=కొనసాగే ముందు దయచేసి ఈ ముఖ్యమైన సమాచారాన్ని చదవండి.
LicenseLabel3=దయచేసి క్రింది లైసెన్స్ ఒప్పందాన్ని చదవండి. ఇన్‌స్టాలేషన్‌ను కొనసాగించే ముందు మీరు ఈ ఒప్పంద నిబంధనలను అంగీకరించాలి.
LicenseAccepted=నేను ఒప్పందాన్ని అంగీకరిస్తున్నాను
LicenseNotAccepted=నేను ఒప్పందాన్ని అంగీకరించను

; *** "Information" wizard pages
WizardInfoBefore=సమాచారం
InfoBeforeLabel=కొనసాగే ముందు దయచేసి ఈ ముఖ్యమైన సమాచారాన్ని చదవండి.
InfoBeforeClickLabel=సెటప్‌ను కొనసాగించడానికి సిద్ధమైనప్పుడు “తదుపరి” నొక్కండి.
WizardInfoAfter=సమాచారం
InfoAfterLabel=కొనసాగే ముందు దయచేసి ఈ ముఖ్యమైన సమాచారాన్ని చదవండి.
InfoAfterClickLabel=సెటప్‌ను కొనసాగించడానికి సిద్ధమైనప్పుడు “తదుపరి” నొక్కండి.

; *** "User Information" wizard page
WizardUserInfo=వినియోగదారు సమాచారం
UserInfoDesc=దయచేసి మీ వివరాలను నమోదు చేయండి.
UserInfoName=వినియోగదారు పేరు:
UserInfoOrg=సంస్థ:
UserInfoSerial=సీరియల్ నంబర్:
UserInfoNameRequired=మీరు పేరు నమోదు చేయాలి.

; *** "Select Destination Location" wizard page
WizardSelectDir=గమ్యస్థానాన్ని ఎంచుకోండి
SelectDirDesc=[name] ను ఎక్కడ ఇన్‌స్టాల్ చేయాలి?
SelectDirLabel3=సెటప్ [name] ను క్రింది ఫోల్డర్‌లో ఇన్‌స్టాల్ చేస్తుంది.
SelectDirBrowseLabel=కొనసాగించడానికి “తదుపరి” నొక్కండి. వేరే ఫోల్డర్‌ను ఎంచుకోవాలనుకుంటే “బ్రౌజ్ చేయండి” నొక్కండి.
DiskSpaceGBLabel=కనీసం [gb] GB ఖాళీ డిస్క్ స్థలం అవసరం.
DiskSpaceMBLabel=కనీసం [mb] MB ఖాళీ డిస్క్ స్థలం అవసరం.
CannotInstallToNetworkDrive=సెటప్ నెట్‌వర్క్ డ్రైవ్‌లో ఇన్‌స్టాల్ చేయలేదు.
CannotInstallToUNCPath=సెటప్ UNC పాత్‌లో ఇన్‌స్టాల్ చేయలేదు.
InvalidPath=మీరు డ్రైవ్ అక్షరంతో పూర్తి పాత్‌ను నమోదు చేయాలి; ఉదాహరణకు:%n%nC:\APP%n%nలేదా ఈ రూపంలో UNC పాత్:%n%n\\server\share
InvalidDrive=మీరు ఎంచుకున్న డ్రైవ్ లేదా UNC షేర్ లేదు లేదా అందుబాటులో లేదు. దయచేసి వేరొకటి ఎంచుకోండి.
DiskSpaceWarningTitle=డిస్క్ స్థలం సరిపోదు
DiskSpaceWarning=ఇన్‌స్టాల్ చేయడానికి సెటప్‌కు కనీసం %1 KB ఖాళీ స్థలం అవసరం, కానీ ఎంచుకున్న డ్రైవ్‌లో %2 KB మాత్రమే అందుబాటులో ఉంది.%n%nఅయినా కొనసాగించాలనుకుంటున్నారా?
DirNameTooLong=ఫోల్డర్ పేరు లేదా పాత్ చాలా పొడవుగా ఉంది.
InvalidDirName=ఫోల్డర్ పేరు చెల్లదు.
BadDirName32=ఫోల్డర్ పేర్లలో క్రింది అక్షరాలు ఉండకూడదు:%n%n%1
DirExistsTitle=ఫోల్డర్ ఇప్పటికే ఉంది
DirExists=ఈ ఫోల్డర్:%n%n%1%n%nఇప్పటికే ఉంది. అయినా ఆ ఫోల్డర్‌లోనే ఇన్‌స్టాల్ చేయాలనుకుంటున్నారా?
DirDoesntExistTitle=ఫోల్డర్ లేదు
DirDoesntExist=ఈ ఫోల్డర్:%n%n%1%n%nలేదు. ఫోల్డర్‌ను సృష్టించాలనుకుంటున్నారా?

; *** "Select Components" wizard page
WizardSelectComponents=భాగాలను ఎంచుకోండి
SelectComponentsDesc=ఏ భాగాలను ఇన్‌స్టాల్ చేయాలి?
SelectComponentsLabel2=మీరు ఇన్‌స్టాల్ చేయాలనుకునే భాగాలను ఎంచుకోండి; వద్దనుకునే భాగాల ఎంపికను తీసివేయండి. సిద్ధమైనప్పుడు “తదుపరి” నొక్కండి.
FullInstallation=పూర్తి ఇన్‌స్టాలేషన్
; if possible don't translate 'Compact' as 'Minimal' (I mean 'Minimal' in your language)
CompactInstallation=సంక్షిప్త ఇన్‌స్టాలేషన్
CustomInstallation=అనుకూల ఇన్‌స్టాలేషన్
NoUninstallWarningTitle=భాగాలు ఇప్పటికే ఉన్నాయి
NoUninstallWarning=క్రింది భాగాలు మీ కంప్యూటర్‌లో ఇప్పటికే ఇన్‌స్టాల్ అయి ఉన్నట్లు సెటప్ గుర్తించింది:%n%n%1%n%nఈ భాగాల ఎంపికను తీసివేసినంత మాత్రాన అవి అన్‌ఇన్‌స్టాల్ కావు.%n%nఅయినా కొనసాగించాలనుకుంటున్నారా?
ComponentSize1=%1 KB
ComponentSize2=%1 MB
ComponentsDiskSpaceGBLabel=ప్రస్తుత ఎంపికకు కనీసం [gb] GB డిస్క్ స్థలం అవసరం.
ComponentsDiskSpaceMBLabel=ప్రస్తుత ఎంపికకు కనీసం [mb] MB డిస్క్ స్థలం అవసరం.

; *** "Select Additional Tasks" wizard page
WizardSelectTasks=అదనపు పనులను ఎంచుకోండి
SelectTasksDesc=ఏ అదనపు పనులు చేయాలి?
SelectTasksLabel2=[name] ను ఇన్‌స్టాల్ చేసేటప్పుడు సెటప్ చేయాలని మీరు కోరుకునే అదనపు పనులను ఎంచుకుని, ఆపై “తదుపరి” నొక్కండి.

; *** "Select Start Menu Folder" wizard page
WizardSelectProgramGroup=స్టార్ట్ మెనూ ఫోల్డర్‌ను ఎంచుకోండి
SelectStartMenuFolderDesc=ప్రోగ్రామ్ షార్ట్‌కట్‌లను సెటప్ ఎక్కడ ఉంచాలి?
SelectStartMenuFolderLabel3=సెటప్ ప్రోగ్రామ్ షార్ట్‌కట్‌లను క్రింది స్టార్ట్ మెనూ ఫోల్డర్‌లో సృష్టిస్తుంది.
SelectStartMenuFolderBrowseLabel=కొనసాగించడానికి “తదుపరి” నొక్కండి. వేరే ఫోల్డర్‌ను ఎంచుకోవాలనుకుంటే “బ్రౌజ్ చేయండి” నొక్కండి.
MustEnterGroupName=మీరు ఫోల్డర్ పేరు నమోదు చేయాలి.
GroupNameTooLong=ఫోల్డర్ పేరు లేదా పాత్ చాలా పొడవుగా ఉంది.
InvalidGroupName=ఫోల్డర్ పేరు చెల్లదు.
BadGroupName=ఫోల్డర్ పేరులో క్రింది అక్షరాలు ఉండకూడదు:%n%n%1
NoProgramGroupCheck2=స్టార్ట్ మెనూ ఫోల్డర్‌ను సృష్టించవద్దు

; *** "Ready to Install" wizard page
WizardReady=ఇన్‌స్టాల్ చేయడానికి సిద్ధం
ReadyLabel1=సెటప్ ఇప్పుడు మీ కంప్యూటర్‌లో [name] ను ఇన్‌స్టాల్ చేయడం ప్రారంభించడానికి సిద్ధంగా ఉంది.
ReadyLabel2a=ఇన్‌స్టాలేషన్‌ను కొనసాగించడానికి “ఇన్‌స్టాల్ చేయండి” నొక్కండి, లేదా ఏవైనా సెట్టింగ్‌లను సమీక్షించాలన్నా, మార్చాలన్నా “వెనుకకు” నొక్కండి.
ReadyLabel2b=ఇన్‌స్టాలేషన్‌ను కొనసాగించడానికి “ఇన్‌స్టాల్ చేయండి” నొక్కండి.
ReadyMemoUserInfo=వినియోగదారు సమాచారం:
ReadyMemoDir=గమ్యస్థానం:
ReadyMemoType=సెటప్ రకం:
ReadyMemoComponents=ఎంచుకున్న భాగాలు:
ReadyMemoGroup=స్టార్ట్ మెనూ ఫోల్డర్:
ReadyMemoTasks=అదనపు పనులు:

; *** TDownloadWizardPage wizard page and DownloadTemporaryFile
DownloadingLabel2=ఫైళ్లను డౌన్‌లోడ్ చేస్తోంది...
ButtonStopDownload=డౌన్‌లోడ్ ఆపండి
StopDownload=డౌన్‌లోడ్ ఆపాలని మీకు ఖచ్చితంగా ఉందా?
ErrorDownloadAborted=డౌన్‌లోడ్ ఆపివేయబడింది
ErrorDownloadFailed=డౌన్‌లోడ్ విఫలమైంది: %1 %2
ErrorDownloadSizeFailed=పరిమాణం తెలుసుకోవడం విఫలమైంది: %1 %2
ErrorProgress=చెల్లని పురోగతి: %2 లో %1
ErrorFileSize=చెల్లని ఫైల్ పరిమాణం: %1 ఆశించగా, %2 కనిపించింది

; *** TExtractionWizardPage wizard page and ExtractArchive
ExtractingLabel=ఫైళ్లను వెలికితీస్తోంది...
ButtonStopExtraction=వెలికితీత ఆపండి
StopExtraction=వెలికితీత ఆపాలని మీకు ఖచ్చితంగా ఉందా?
ErrorExtractionAborted=వెలికితీత ఆపివేయబడింది
ErrorExtractionFailed=వెలికితీత విఫలమైంది: %1

; *** Archive extraction failure details
ArchiveIncorrectPassword=పాస్‌వర్డ్ తప్పు
ArchiveIsCorrupted=ఆర్కైవ్ పాడైపోయింది
ArchiveUnsupportedFormat=ఆర్కైవ్ ఫార్మాట్‌కు మద్దతు లేదు

; *** "Preparing to Install" wizard page
WizardPreparing=ఇన్‌స్టాల్ చేయడానికి సిద్ధమవుతోంది
PreparingDesc=సెటప్ మీ కంప్యూటర్‌లో [name] ను ఇన్‌స్టాల్ చేయడానికి సిద్ధమవుతోంది.
PreviousInstallNotCompleted=మునుపటి ప్రోగ్రామ్ ఇన్‌స్టాలేషన్ లేదా తొలగింపు పూర్తి కాలేదు. ఆ ఇన్‌స్టాలేషన్‌ను పూర్తి చేయడానికి మీరు కంప్యూటర్‌ను పునఃప్రారంభించాలి.%n%nకంప్యూటర్‌ను పునఃప్రారంభించిన తర్వాత, [name] ఇన్‌స్టాలేషన్‌ను పూర్తి చేయడానికి సెటప్‌ను మళ్లీ నడపండి.
CannotContinue=సెటప్ కొనసాగలేదు. నిష్క్రమించడానికి దయచేసి “రద్దు” నొక్కండి.
ApplicationsFound=సెటప్ నవీకరించాల్సిన ఫైళ్లను క్రింది అప్లికేషన్‌లు ఉపయోగిస్తున్నాయి. ఈ అప్లికేషన్‌లను సెటప్ స్వయంచాలకంగా మూసివేయడానికి అనుమతించడం మంచిది.
ApplicationsFound2=సెటప్ నవీకరించాల్సిన ఫైళ్లను క్రింది అప్లికేషన్‌లు ఉపయోగిస్తున్నాయి. ఈ అప్లికేషన్‌లను సెటప్ స్వయంచాలకంగా మూసివేయడానికి అనుమతించడం మంచిది. ఇన్‌స్టాలేషన్ పూర్తయిన తర్వాత, సెటప్ ఆ అప్లికేషన్‌లను మళ్లీ ప్రారంభించడానికి ప్రయత్నిస్తుంది.
CloseApplications=అప్లికేషన్‌లను స్వయంచాలకంగా మూసివేయండి
DontCloseApplications=అప్లికేషన్‌లను మూసివేయవద్దు
ErrorCloseApplications=అన్ని అప్లికేషన్‌లను స్వయంచాలకంగా మూసివేయడం సెటప్‌కు సాధ్యం కాలేదు. కొనసాగే ముందు సెటప్ నవీకరించాల్సిన ఫైళ్లను ఉపయోగిస్తున్న అన్ని అప్లికేషన్‌లను మూసివేయడం మంచిది.
PrepareToInstallNeedsRestart=సెటప్ మీ కంప్యూటర్‌ను పునఃప్రారంభించాలి. కంప్యూటర్‌ను పునఃప్రారంభించిన తర్వాత, [name] ఇన్‌స్టాలేషన్‌ను పూర్తి చేయడానికి సెటప్‌ను మళ్లీ నడపండి.%n%nఇప్పుడు పునఃప్రారంభించాలా?

; *** "Installing" wizard page
WizardInstalling=ఇన్‌స్టాల్ అవుతోంది
InstallingLabel=సెటప్ మీ కంప్యూటర్‌లో [name] ను ఇన్‌స్టాల్ చేస్తోంది, దయచేసి వేచి ఉండండి.

; *** "Setup Completed" wizard page
FinishedHeadingLabel=[name] సెటప్ విజార్డ్ పూర్తవుతోంది
FinishedLabelNoIcons=సెటప్ మీ కంప్యూటర్‌లో [name] ఇన్‌స్టాలేషన్‌ను పూర్తి చేసింది.
FinishedLabel=సెటప్ మీ కంప్యూటర్‌లో [name] ఇన్‌స్టాలేషన్‌ను పూర్తి చేసింది. ఇన్‌స్టాల్ అయిన షార్ట్‌కట్‌లను ఎంచుకోవడం ద్వారా అప్లికేషన్‌ను ప్రారంభించవచ్చు.
ClickFinish=సెటప్ నుండి నిష్క్రమించడానికి “ముగించండి” నొక్కండి.
FinishedRestartLabel=[name] ఇన్‌స్టాలేషన్‌ను పూర్తి చేయడానికి సెటప్ మీ కంప్యూటర్‌ను పునఃప్రారంభించాలి. ఇప్పుడు పునఃప్రారంభించాలా?
FinishedRestartMessage=[name] ఇన్‌స్టాలేషన్‌ను పూర్తి చేయడానికి సెటప్ మీ కంప్యూటర్‌ను పునఃప్రారంభించాలి.%n%nఇప్పుడు పునఃప్రారంభించాలా?
ShowReadmeCheck=అవును, README ఫైల్‌ను చూడాలనుకుంటున్నాను
YesRadio=అవును, కంప్యూటర్‌ను ఇప్పుడే పునఃప్రారంభించండి
NoRadio=కాదు, కంప్యూటర్‌ను నేను తర్వాత పునఃప్రారంభిస్తాను
; used for example as 'Run MyProg.exe'
RunEntryExec=%1 ను నడపండి
; used for example as 'View Readme.txt'
RunEntryShellExec=%1 ను చూడండి

; *** "Setup Needs the Next Disk" stuff
ChangeDiskTitle=సెటప్‌కు తదుపరి డిస్క్ అవసరం
SelectDiskLabel2=దయచేసి డిస్క్ %1 ను చొప్పించి, “సరే” నొక్కండి.%n%nఈ డిస్క్‌లోని ఫైళ్లు క్రింద చూపిన ఫోల్డర్ కాకుండా వేరే ఫోల్డర్‌లో ఉంటే, సరైన పాత్‌ను నమోదు చేయండి లేదా “బ్రౌజ్ చేయండి” నొక్కండి.
PathLabel=పాత్:
FileNotInDir2="%2" లో "%1" ఫైల్ కనుగొనబడలేదు. దయచేసి సరైన డిస్క్‌ను చొప్పించండి లేదా వేరే ఫోల్డర్‌ను ఎంచుకోండి.
SelectDirectoryLabel=దయచేసి తదుపరి డిస్క్ స్థానాన్ని పేర్కొనండి.

; *** Installation phase messages
SetupAborted=సెటప్ పూర్తి కాలేదు.%n%nదయచేసి సమస్యను సరిచేసి సెటప్‌ను మళ్లీ నడపండి.
AbortRetryIgnoreSelectAction=చర్యను ఎంచుకోండి
AbortRetryIgnoreRetry=మళ్లీ ప్రయత్నించండి
AbortRetryIgnoreIgnore=లోపాన్ని విస్మరించి కొనసాగించండి
AbortRetryIgnoreCancel=ఇన్‌స్టాలేషన్‌ను రద్దు చేయండి
RetryCancelSelectAction=చర్యను ఎంచుకోండి
RetryCancelRetry=మళ్లీ ప్రయత్నించండి
RetryCancelCancel=రద్దు

; *** Installation status messages
StatusClosingApplications=అప్లికేషన్‌లను మూసివేస్తోంది...
StatusCreateDirs=డైరెక్టరీలను సృష్టిస్తోంది...
StatusExtractFiles=ఫైళ్లను వెలికితీస్తోంది...
StatusDownloadFiles=ఫైళ్లను డౌన్‌లోడ్ చేస్తోంది...
StatusCreateIcons=షార్ట్‌కట్‌లను సృష్టిస్తోంది...
StatusCreateIniEntries=INI నమోదులను సృష్టిస్తోంది...
StatusCreateRegistryEntries=రిజిస్ట్రీ నమోదులను సృష్టిస్తోంది...
StatusRegisterFiles=ఫైళ్లను నమోదు చేస్తోంది...
StatusSavingUninstall=అన్‌ఇన్‌స్టాల్ సమాచారాన్ని సేవ్ చేస్తోంది...
StatusRunProgram=ఇన్‌స్టాలేషన్‌ను ముగిస్తోంది...
StatusRestartingApplications=అప్లికేషన్‌లను పునఃప్రారంభిస్తోంది...
StatusRollback=మార్పులను వెనక్కి తీసుకుంటోంది...

; *** Misc. errors
ErrorInternal2=అంతర్గత లోపం: %1
ErrorFunctionFailedNoCode=%1 విఫలమైంది
ErrorFunctionFailed=%1 విఫలమైంది; కోడ్ %2
ErrorFunctionFailedWithMessage=%1 విఫలమైంది; కోడ్ %2.%n%3
ErrorExecutingProgram=ఫైల్‌ను అమలు చేయలేకపోయింది:%n%1

; *** Registry errors
ErrorRegOpenKey=రిజిస్ట్రీ కీని తెరవడంలో లోపం:%n%1\%2
ErrorRegCreateKey=రిజిస్ట్రీ కీని సృష్టించడంలో లోపం:%n%1\%2
ErrorRegWriteKey=రిజిస్ట్రీ కీలో రాయడంలో లోపం:%n%1\%2

; *** INI errors
ErrorIniEntry="%1" ఫైల్‌లో INI నమోదును సృష్టించడంలో లోపం.

; *** File copying errors
FileAbortRetryIgnoreSkipNotRecommended=ఈ ఫైల్‌ను దాటవేయండి (సిఫార్సు చేయబడదు)
FileAbortRetryIgnoreIgnoreNotRecommended=లోపాన్ని విస్మరించి కొనసాగించండి (సిఫార్సు చేయబడదు)
SourceIsCorrupted=మూల ఫైల్ పాడైపోయింది
SourceDoesntExist="%1" మూల ఫైల్ లేదు
SourceVerificationFailed=మూల ఫైల్ ధృవీకరణ విఫలమైంది: %1
VerificationSignatureDoesntExist="%1" సంతకం ఫైల్ లేదు
VerificationSignatureInvalid="%1" సంతకం ఫైల్ చెల్లనిది
VerificationKeyNotFound="%1" సంతకం ఫైల్ తెలియని కీని ఉపయోగిస్తోంది
VerificationFileNameIncorrect=ఫైల్ పేరు తప్పు
VerificationFileTagIncorrect=ఫైల్ ట్యాగ్ తప్పు
VerificationFileSizeIncorrect=ఫైల్ పరిమాణం తప్పు
VerificationFileHashIncorrect=ఫైల్ హ్యాష్ తప్పు
ExistingFileReadOnly2=ఇప్పటికే ఉన్న ఫైల్ చదవడానికి మాత్రమే అని గుర్తించబడినందున దాన్ని భర్తీ చేయలేకపోయింది.
ExistingFileReadOnlyRetry=చదవడానికి మాత్రమే లక్షణాన్ని తీసివేసి మళ్లీ ప్రయత్నించండి
ExistingFileReadOnlyKeepExisting=ఇప్పటికే ఉన్న ఫైల్‌ను ఉంచండి
ErrorReadingExistingDest=ఇప్పటికే ఉన్న ఫైల్‌ను చదవడానికి ప్రయత్నిస్తున్నప్పుడు లోపం ఏర్పడింది:
FileExistsSelectAction=చర్యను ఎంచుకోండి
FileExists2=ఫైల్ ఇప్పటికే ఉంది.
FileExistsOverwriteExisting=ఇప్పటికే ఉన్న ఫైల్‌ను భర్తీ చేయండి
FileExistsKeepExisting=ఇప్పటికే ఉన్న ఫైల్‌ను ఉంచండి
FileExistsOverwriteOrKeepAll=తదుపరి సంఘర్షణలకు కూడా ఇదే చేయండి
ExistingFileNewerSelectAction=చర్యను ఎంచుకోండి
ExistingFileNewer2=ఇప్పటికే ఉన్న ఫైల్ సెటప్ ఇన్‌స్టాల్ చేయడానికి ప్రయత్నిస్తున్న దాని కంటే కొత్తది.
ExistingFileNewerOverwriteExisting=ఇప్పటికే ఉన్న ఫైల్‌ను భర్తీ చేయండి
ExistingFileNewerKeepExisting=ఇప్పటికే ఉన్న ఫైల్‌ను ఉంచండి (సిఫార్సు చేయబడింది)
ExistingFileNewerOverwriteOrKeepAll=తదుపరి సంఘర్షణలకు కూడా ఇదే చేయండి
ErrorChangingAttr=ఇప్పటికే ఉన్న ఫైల్ లక్షణాలను మార్చడానికి ప్రయత్నిస్తున్నప్పుడు లోపం ఏర్పడింది:
ErrorCreatingTemp=గమ్యస్థాన డైరెక్టరీలో ఫైల్‌ను సృష్టించడానికి ప్రయత్నిస్తున్నప్పుడు లోపం ఏర్పడింది:
ErrorReadingSource=మూల ఫైల్‌ను చదవడానికి ప్రయత్నిస్తున్నప్పుడు లోపం ఏర్పడింది:
ErrorCopying=ఫైల్‌ను కాపీ చేయడానికి ప్రయత్నిస్తున్నప్పుడు లోపం ఏర్పడింది:
ErrorDownloading=ఫైల్‌ను డౌన్‌లోడ్ చేయడానికి ప్రయత్నిస్తున్నప్పుడు లోపం ఏర్పడింది:
ErrorExtracting=ఆర్కైవ్‌ను వెలికితీయడానికి ప్రయత్నిస్తున్నప్పుడు లోపం ఏర్పడింది:
ErrorReplacingExistingFile=ఇప్పటికే ఉన్న ఫైల్‌ను భర్తీ చేయడానికి ప్రయత్నిస్తున్నప్పుడు లోపం ఏర్పడింది:
ErrorRestartReplace=RestartReplace విఫలమైంది:
ErrorRenamingTemp=గమ్యస్థాన డైరెక్టరీలో ఫైల్ పేరు మార్చడానికి ప్రయత్నిస్తున్నప్పుడు లోపం ఏర్పడింది:
ErrorRegisterServer=DLL/OCX ను నమోదు చేయలేకపోయింది: %1
ErrorRegSvr32Failed=RegSvr32 నిష్క్రమణ కోడ్ %1 తో విఫలమైంది
ErrorRegisterTypeLib=టైప్ లైబ్రరీని నమోదు చేయలేకపోయింది: %1

; *** Uninstall display name markings
; used for example as 'My Program (32-bit)'
UninstallDisplayNameMark=%1 (%2)
; used for example as 'My Program (32-bit, All users)'
UninstallDisplayNameMarks=%1 (%2, %3)
UninstallDisplayNameMark32Bit=32-బిట్
UninstallDisplayNameMark64Bit=64-బిట్
UninstallDisplayNameMarkAllUsers=వినియోగదారులందరూ
UninstallDisplayNameMarkCurrentUser=ప్రస్తుత వినియోగదారు

; *** Post-installation errors
ErrorOpeningReadme=README ఫైల్‌ను తెరవడానికి ప్రయత్నిస్తున్నప్పుడు లోపం ఏర్పడింది.
ErrorRestartingComputer=సెటప్ కంప్యూటర్‌ను పునఃప్రారంభించలేకపోయింది. దయచేసి ఇది చేతితో చేయండి.

; *** Uninstaller messages
UninstallNotFound="%1" ఫైల్ లేదు. అన్‌ఇన్‌స్టాల్ చేయలేము.
UninstallOpenError="%1" ఫైల్‌ను తెరవలేకపోయింది. అన్‌ఇన్‌స్టాల్ చేయలేము
UninstallUnsupportedVer="%1" అన్‌ఇన్‌స్టాల్ లాగ్ ఫైల్ ఈ అన్‌ఇన్‌స్టాలర్ సంస్కరణ గుర్తించని ఫార్మాట్‌లో ఉంది. అన్‌ఇన్‌స్టాల్ చేయలేము
UninstallUnknownEntry=అన్‌ఇన్‌స్టాల్ లాగ్‌లో తెలియని నమోదు (%1) ఎదురైంది
ConfirmUninstall=%1 ను, దాని అన్ని భాగాలను పూర్తిగా తొలగించాలని మీకు ఖచ్చితంగా ఉందా?
UninstallOnlyOnWin64=ఈ ఇన్‌స్టాలేషన్‌ను 64-బిట్ Windows లో మాత్రమే అన్‌ఇన్‌స్టాల్ చేయవచ్చు.
OnlyAdminCanUninstall=ఈ ఇన్‌స్టాలేషన్‌ను అడ్మినిస్ట్రేటివ్ హక్కులు ఉన్న వినియోగదారు మాత్రమే అన్‌ఇన్‌స్టాల్ చేయగలరు.
UninstallStatusLabel=మీ కంప్యూటర్ నుండి %1 తొలగించబడుతోంది, దయచేసి వేచి ఉండండి.
UninstalledAll=%1 మీ కంప్యూటర్ నుండి విజయవంతంగా తొలగించబడింది.
UninstalledMost=%1 అన్‌ఇన్‌స్టాల్ పూర్తయింది.%n%nకొన్ని అంశాలను తొలగించలేకపోయింది. వాటిని చేతితో తొలగించవచ్చు.
UninstalledAndNeedsRestart=%1 అన్‌ఇన్‌స్టాల్‌ను పూర్తి చేయడానికి మీ కంప్యూటర్‌ను పునఃప్రారంభించాలి.%n%nఇప్పుడు పునఃప్రారంభించాలా?
UninstallDataCorrupted="%1" ఫైల్ పాడైపోయింది. అన్‌ఇన్‌స్టాల్ చేయలేము

; *** Uninstallation phase messages
ConfirmDeleteSharedFileTitle=షేర్డ్ ఫైల్‌ను తొలగించాలా?
ConfirmDeleteSharedFile2=క్రింది షేర్డ్ ఫైల్‌ను ఇకపై ఏ ప్రోగ్రామ్ ఉపయోగించడం లేదని సిస్టమ్ సూచిస్తోంది. ఈ షేర్డ్ ఫైల్‌ను అన్‌ఇన్‌స్టాల్ తొలగించాలని మీరు కోరుకుంటున్నారా?%n%nఏదైనా ప్రోగ్రామ్ ఇంకా ఈ ఫైల్‌ను ఉపయోగిస్తూ ఉండి, దీన్ని తొలగిస్తే, ఆ ప్రోగ్రామ్‌లు సరిగా పని చేయకపోవచ్చు. ఖచ్చితంగా తెలియకపోతే “కాదు” ఎంచుకోండి. ఫైల్‌ను సిస్టమ్‌లో ఉంచడం వల్ల ఎలాంటి హాని ఉండదు.
SharedFileNameLabel=ఫైల్ పేరు:
SharedFileLocationLabel=స్థానం:
WizardUninstalling=అన్‌ఇన్‌స్టాల్ స్థితి
StatusUninstalling=%1 ను అన్‌ఇన్‌స్టాల్ చేస్తోంది...

; *** Shutdown block reasons
ShutdownBlockReasonInstallingApp=%1 ను ఇన్‌స్టాల్ చేస్తోంది.
ShutdownBlockReasonUninstallingApp=%1 ను అన్‌ఇన్‌స్టాల్ చేస్తోంది.

; The custom messages below aren't used by Setup itself, but if you make
; use of them in your scripts, you'll want to translate them.

[CustomMessages]

NameAndVersion=%1 సంస్కరణ %2
AdditionalIcons=అదనపు షార్ట్‌కట్‌లు:
CreateDesktopIcon=డెస్క్‌టాప్ షార్ట్‌కట్‌ను సృష్టించండి
CreateQuickLaunchIcon=క్విక్ లాంచ్ షార్ట్‌కట్‌ను సృష్టించండి
ProgramOnTheWeb=వెబ్‌లో %1
UninstallProgram=%1 ను అన్‌ఇన్‌స్టాల్ చేయండి
LaunchProgram=%1 ను ప్రారంభించండి
AssocFileExtension=%1 ను %2 ఫైల్ ఎక్స్‌టెన్షన్‌తో అనుబంధించండి
AssocingFileExtension=%1 ను %2 ఫైల్ ఎక్స్‌టెన్షన్‌తో అనుబంధిస్తోంది...
AutoStartProgramGroupDescription=స్టార్టప్:
AutoStartProgram=%1 ను స్వయంచాలకంగా ప్రారంభించండి
AddonHostProgramNotFound=మీరు ఎంచుకున్న ఫోల్డర్‌లో %1 కనుగొనబడలేదు.%n%nఅయినా కొనసాగించాలనుకుంటున్నారా?

; The messages that are Lang Switcher's own (the sample is English.isl).
LocalCertReady=సెటప్ ఈ కంప్యూటర్‌లో ఒక సర్టిఫికెట్‌ను సృష్టించి, దానితో Lang Switcher పై సంతకం చేస్తుంది; దీనివల్ల ప్రోగ్రామ్ అడ్మినిస్ట్రేటర్‌గా నడుస్తున్న ప్రోగ్రామ్‌ల విండోలలో కూడా పని చేస్తుంది.
LocalCertFailed=Lang Switcher ఇన్‌స్టాల్ అయింది, కానీ అది అడ్మినిస్ట్రేటర్‌గా నడుస్తున్న ప్రోగ్రామ్‌ల విండోలలో పని చేయదు: ఈ కంప్యూటర్‌లో సృష్టించిన సర్టిఫికెట్‌ను Windows అంగీకరించలేదు. వివరాల కోసం సెటప్ లాగ్ చూడండి.
RemoveSettingsQuestion=Lang Switcher సెట్టింగ్‌ల ఫైల్‌ను కూడా తొలగించాలా?
RemoveSettingsKeep=తర్వాత మళ్లీ ఇన్‌స్టాల్ చేయడానికి మీ సెట్టింగ్‌లను ఉంచాలంటే “కాదు” ఎంచుకోండి.
AuthorSlogan=మనసుతో చేసిన ప్రోగ్రామ్‌లు
