; Lang Switcher -- the messages of the installer in Odia (stage E108, group A).
;
; Inno Setup has no official Odia translation (inno_official = false in res\lang\languages.toml),
; so this file carries the whole of it: [LangOptions], every message of the compiler:Default.isl
; (6.5.0+), and the messages of the installer that are Lang Switcher's own. It is read after
; compiler:Default.isl as the second MessagesFile of the "odia" entry of [Languages] in
; ..\LangSwitcher.iss, and so replaces the English text name for name.
; English.isl is the sample the own messages translate; the glossary of the language is
; l10n\glossary\or.tsv of the control contour. The name of the program is not translated.
; As in Default.isl: a message that has no full stop there has no danda here (Setup adds its own).
; No "&" accelerators: Odia has no official file, so one window never carries two equal letters;
; the names of buttons inside a message stand in “” (they are written as the buttons say them).

[LangOptions]
LanguageName=ଓଡ଼ିଆ
LanguageID=$0448
LanguageCodePage=0

[Messages]

; *** Application titles
SetupAppTitle=ସେଟଅପ୍
SetupWindowTitle=ସେଟଅପ୍ - %1
UninstallAppTitle=ଅନଇନଷ୍ଟଲ୍
UninstallAppFullTitle=%1 ଅନଇନଷ୍ଟଲ୍

; *** Misc. common
InformationTitle=ସୂଚନା
ConfirmTitle=ନିଶ୍ଚିତ କରନ୍ତୁ
ErrorTitle=ତ୍ରୁଟି

; *** SetupLdr messages
SetupLdrStartupMessage=ଏହା %1 ଇନଷ୍ଟଲ୍ କରିବ। ଆପଣ ଜାରି ରଖିବାକୁ ଚାହାଁନ୍ତି କି?
LdrCannotCreateTemp=ଅସ୍ଥାୟୀ ଫାଇଲ ତିଆରି କରାଯାଇପାରିଲା ନାହିଁ। ସେଟଅପ୍ ବାତିଲ୍ ହେଲା
LdrCannotExecTemp=ଅସ୍ଥାୟୀ ଡାଇରେକ୍ଟୋରୀରେ ଫାଇଲ ଚଲାଇପାରିଲା ନାହିଁ। ସେଟଅପ୍ ବାତିଲ୍ ହେଲା
HelpTextNote=

; *** Startup error messages
LastErrorMessage=%1.%n%nତ୍ରୁଟି %2: %3
SetupFileMissing=ଇନଷ୍ଟଲେସନ୍ ଡାଇରେକ୍ଟୋରୀରେ %1 ଫାଇଲ ନାହିଁ। ଦୟାକରି ସମସ୍ୟା ଠିକ୍ କରନ୍ତୁ ବା ପ୍ରୋଗ୍ରାମର ଏକ ନୂଆ କପି ସଂଗ୍ରହ କରନ୍ତୁ।
SetupFileCorrupt=ସେଟଅପ୍ ଫାଇଲଗୁଡ଼ିକ ନଷ୍ଟ ହୋଇଯାଇଛି। ଦୟାକରି ପ୍ରୋଗ୍ରାମର ଏକ ନୂଆ କପି ସଂଗ୍ରହ କରନ୍ତୁ।
SetupFileCorruptOrWrongVer=ସେଟଅପ୍ ଫାଇଲଗୁଡ଼ିକ ନଷ୍ଟ ହୋଇଯାଇଛି, ବା ଏହି ସେଟଅପ୍ ସଂସ୍କରଣ ସହ ମେଳ ଖାଉନାହିଁ। ଦୟାକରି ସମସ୍ୟା ଠିକ୍ କରନ୍ତୁ ବା ପ୍ରୋଗ୍ରାମର ଏକ ନୂଆ କପି ସଂଗ୍ରହ କରନ୍ତୁ।
InvalidParameter=କମାଣ୍ଡ ଲାଇନରେ ଏକ ଅବୈଧ ପାରାମିଟର ଦିଆଯାଇଛି:%n%n%1
SetupAlreadyRunning=ସେଟଅପ୍ ପୂର୍ବରୁ ଚାଲୁଛି।
WindowsVersionNotSupported=ଆପଣଙ୍କ କମ୍ପ୍ୟୁଟରରେ ଚାଲୁଥିବା Windows ସଂସ୍କରଣକୁ ଏହି ପ୍ରୋଗ୍ରାମ ସମର୍ଥନ କରେ ନାହିଁ।
WindowsServicePackRequired=ଏହି ପ୍ରୋଗ୍ରାମ ପାଇଁ %1 Service Pack %2 ବା ତାହାଠାରୁ ନୂଆ ସଂସ୍କରଣ ଦରକାର।
NotOnThisPlatform=ଏହି ପ୍ରୋଗ୍ରାମ %1 ରେ ଚାଲିବ ନାହିଁ।
OnlyOnThisPlatform=ଏହି ପ୍ରୋଗ୍ରାମକୁ %1 ରେ ଚଲାଇବାକୁ ହେବ।
OnlyOnTheseArchitectures=ଏହି ପ୍ରୋଗ୍ରାମ କେବଳ ନିମ୍ନ ପ୍ରୋସେସର ଆର୍କିଟେକ୍ଚର ପାଇଁ ତିଆରି Windows ସଂସ୍କରଣରେ ଇନଷ୍ଟଲ୍ ହୋଇପାରିବ:%n%n%1
WinVersionTooLowError=ଏହି ପ୍ରୋଗ୍ରାମ ପାଇଁ %1 ସଂସ୍କରଣ %2 ବା ତାହାଠାରୁ ନୂଆ ଦରକାର।
WinVersionTooHighError=ଏହି ପ୍ରୋଗ୍ରାମ %1 ସଂସ୍କରଣ %2 ବା ତାହାଠାରୁ ନୂଆ ସଂସ୍କରଣରେ ଇନଷ୍ଟଲ୍ ହୋଇପାରିବ ନାହିଁ।
AdminPrivilegesRequired=ଏହି ପ୍ରୋଗ୍ରାମ ଇନଷ୍ଟଲ୍ କରିବା ବେଳେ ଆପଣ ଆଡମିନିଷ୍ଟ୍ରେଟର ଭାବେ ସାଇନ୍ ଇନ୍ କରିଥିବା ଦରକାର।
PowerUserPrivilegesRequired=ଏହି ପ୍ରୋଗ୍ରାମ ଇନଷ୍ଟଲ୍ କରିବା ବେଳେ ଆପଣ ଆଡମିନିଷ୍ଟ୍ରେଟର ଭାବେ ବା Power Users ଗ୍ରୁପର ସଦସ୍ୟ ଭାବେ ସାଇନ୍ ଇନ୍ କରିଥିବା ଦରକାର।
SetupAppRunningError=ସେଟଅପ୍ ଦେଖିଲା ଯେ %1 ଏବେ ଚାଲୁଛି।%n%nଦୟାକରି ଏହାର ସବୁ ୱିଣ୍ଡୋ ବନ୍ଦ କରନ୍ତୁ, ତାପରେ ଜାରି ରଖିବାକୁ “ଠିକ୍ ଅଛି” ବା ବାହାରିବାକୁ “ବାତିଲ୍” ଦବାନ୍ତୁ।
UninstallAppRunningError=ଅନଇନଷ୍ଟଲ୍ ଦେଖିଲା ଯେ %1 ଏବେ ଚାଲୁଛି।%n%nଦୟାକରି ଏହାର ସବୁ ୱିଣ୍ଡୋ ବନ୍ଦ କରନ୍ତୁ, ତାପରେ ଜାରି ରଖିବାକୁ “ଠିକ୍ ଅଛି” ବା ବାହାରିବାକୁ “ବାତିଲ୍” ଦବାନ୍ତୁ।

; *** Startup questions
PrivilegesRequiredOverrideTitle=ସେଟଅପ୍ ଇନଷ୍ଟଲ୍ ମୋଡ୍ ବାଛନ୍ତୁ
PrivilegesRequiredOverrideInstruction=ଇନଷ୍ଟଲ୍ ମୋଡ୍ ବାଛନ୍ତୁ
PrivilegesRequiredOverrideText1=%1 ସମସ୍ତ ଉପଯୋଗକର୍ତ୍ତାଙ୍କ ପାଇଁ (ଆଡମିନିଷ୍ଟ୍ରେଟର ଅଧିକାର ଦରକାର), ବା କେବଳ ଆପଣଙ୍କ ପାଇଁ ଇନଷ୍ଟଲ୍ ହୋଇପାରିବ।
PrivilegesRequiredOverrideText2=%1 କେବଳ ଆପଣଙ୍କ ପାଇଁ, ବା ସମସ୍ତ ଉପଯୋଗକର୍ତ୍ତାଙ୍କ ପାଇଁ (ଆଡମିନିଷ୍ଟ୍ରେଟର ଅଧିକାର ଦରକାର) ଇନଷ୍ଟଲ୍ ହୋଇପାରିବ।
PrivilegesRequiredOverrideAllUsers=ସମସ୍ତ ଉପଯୋଗକର୍ତ୍ତାଙ୍କ ପାଇଁ ଇନଷ୍ଟଲ୍ କରନ୍ତୁ
PrivilegesRequiredOverrideAllUsersRecommended=ସମସ୍ତ ଉପଯୋଗକର୍ତ୍ତାଙ୍କ ପାଇଁ ଇନଷ୍ଟଲ୍ କରନ୍ତୁ (ପରାମର୍ଶିତ)
PrivilegesRequiredOverrideCurrentUser=କେବଳ ମୋ ପାଇଁ ଇନଷ୍ଟଲ୍ କରନ୍ତୁ
PrivilegesRequiredOverrideCurrentUserRecommended=କେବଳ ମୋ ପାଇଁ ଇନଷ୍ଟଲ୍ କରନ୍ତୁ (ପରାମର୍ଶିତ)

; *** Misc. errors
ErrorCreatingDir=ସେଟଅପ୍ "%1" ଡାଇରେକ୍ଟୋରୀ ତିଆରି କରିପାରିଲା ନାହିଁ
ErrorTooManyFilesInDir="%1" ଡାଇରେକ୍ଟୋରୀରେ ବହୁତ ଫାଇଲ ଥିବାରୁ ସେଥିରେ ଫାଇଲ ତିଆରି କରାଯାଇପାରିଲା ନାହିଁ

; *** Setup common messages
ExitSetupTitle=ସେଟଅପରୁ ବାହାରନ୍ତୁ
ExitSetupMessage=ସେଟଅପ୍ ସମ୍ପୂର୍ଣ୍ଣ ହୋଇନାହିଁ। ଏବେ ବାହାରିଗଲେ ପ୍ରୋଗ୍ରାମ ଇନଷ୍ଟଲ୍ ହେବ ନାହିଁ।%n%nଇନଷ୍ଟଲେସନ୍ ସମ୍ପୂର୍ଣ୍ଣ କରିବାକୁ ଆପଣ ଅନ୍ୟ ସମୟରେ ସେଟଅପ୍ ପୁଣି ଚଲାଇପାରିବେ।%n%nସେଟଅପରୁ ବାହାରିବେ କି?
AboutSetupMenuItem=ସେଟଅପ୍ ବିଷୟରେ...
AboutSetupTitle=ସେଟଅପ୍ ବିଷୟରେ
AboutSetupMessage=%1 ସଂସ୍କରଣ %2%n%3%n%n%1 ହୋମ୍ ପେଜ୍:%n%4
AboutSetupNote=
TranslatorNote=

; *** Buttons
ButtonBack=< ପଛକୁ
ButtonNext=ପରବର୍ତ୍ତୀ >
ButtonInstall=ଇନଷ୍ଟଲ୍
ButtonOK=ଠିକ୍ ଅଛି
ButtonCancel=ବାତିଲ୍
ButtonYes=ହଁ
ButtonYesToAll=ସବୁକୁ ହଁ
ButtonNo=ନା
ButtonNoToAll=ସବୁକୁ ନା
ButtonFinish=ସମାପ୍ତ
ButtonBrowse=ବ୍ରାଉଜ୍...
ButtonWizardBrowse=ବ୍ରାଉଜ୍...
ButtonNewFolder=ନୂଆ ଫୋଲ୍ଡର ତିଆରି କରନ୍ତୁ

; *** "Select Language" dialog messages
SelectLanguageTitle=ସେଟଅପ୍ ଭାଷା ବାଛନ୍ତୁ
SelectLanguageLabel=ଇନଷ୍ଟଲେସନ୍ ସମୟରେ ବ୍ୟବହାର କରିବାକୁ ଭାଷା ବାଛନ୍ତୁ।

; *** Common wizard text
ClickNext=ଜାରି ରଖିବାକୁ “ପରବର୍ତ୍ତୀ” ବା ସେଟଅପରୁ ବାହାରିବାକୁ “ବାତିଲ୍” ଦବାନ୍ତୁ।
BeveledLabel=
BrowseDialogTitle=ଫୋଲ୍ଡର ଖୋଜନ୍ତୁ
BrowseDialogLabel=ତଳ ତାଲିକାରୁ ଏକ ଫୋଲ୍ଡର ବାଛନ୍ତୁ, ତାପରେ “ଠିକ୍ ଅଛି” ଦବାନ୍ତୁ।
NewFolderName=ନୂଆ ଫୋଲ୍ଡର

; *** "Welcome" wizard page
WelcomeLabel1=[name] ସେଟଅପ୍ ୱିଜାର୍ଡକୁ ସ୍ୱାଗତ
WelcomeLabel2=ଏହା ଆପଣଙ୍କ କମ୍ପ୍ୟୁଟରରେ [name/ver] ଇନଷ୍ଟଲ୍ କରିବ।%n%nଜାରି ରଖିବା ପୂର୍ବରୁ ଅନ୍ୟ ସମସ୍ତ ପ୍ରୋଗ୍ରାମ ବନ୍ଦ କରିବାକୁ ପରାମର୍ଶ ଦିଆଯାଏ।

; *** "Password" wizard page
WizardPassword=ପାସୱାର୍ଡ
PasswordLabel1=ଏହି ଇନଷ୍ଟଲେସନ୍ ପାସୱାର୍ଡ ଦ୍ୱାରା ସୁରକ୍ଷିତ।
PasswordLabel3=ଦୟାକରି ପାସୱାର୍ଡ ଦିଅନ୍ତୁ, ତାପରେ ଜାରି ରଖିବାକୁ “ପରବର୍ତ୍ତୀ” ଦବାନ୍ତୁ। ପାସୱାର୍ଡରେ ବଡ଼ ଓ ସାନ ଅକ୍ଷର ଭିନ୍ନ ଗଣାଯାଏ।
PasswordEditLabel=ପାସୱାର୍ଡ:
IncorrectPassword=ଆପଣ ଦେଇଥିବା ପାସୱାର୍ଡ ଠିକ୍ ନୁହେଁ। ଦୟାକରି ପୁଣି ଚେଷ୍ଟା କରନ୍ତୁ।

; *** "License Agreement" wizard page
WizardLicense=ଲାଇସେନ୍ସ ଚୁକ୍ତିନାମା
LicenseLabel=ଜାରି ରଖିବା ପୂର୍ବରୁ ଦୟାକରି ନିମ୍ନ ଗୁରୁତ୍ୱପୂର୍ଣ୍ଣ ସୂଚନା ପଢ଼ନ୍ତୁ।
LicenseLabel3=ଦୟାକରି ନିମ୍ନ ଲାଇସେନ୍ସ ଚୁକ୍ତିନାମା ପଢ଼ନ୍ତୁ। ଇନଷ୍ଟଲେସନ୍ ଜାରି ରଖିବା ପୂର୍ବରୁ ଆପଣଙ୍କୁ ଏହି ଚୁକ୍ତିନାମାର ସର୍ତ୍ତ ମାନିବାକୁ ହେବ।
LicenseAccepted=ମୁଁ ଚୁକ୍ତିନାମା ମାନୁଛି
LicenseNotAccepted=ମୁଁ ଚୁକ୍ତିନାମା ମାନୁନାହିଁ

; *** "Information" wizard pages
WizardInfoBefore=ସୂଚନା
InfoBeforeLabel=ଜାରି ରଖିବା ପୂର୍ବରୁ ଦୟାକରି ନିମ୍ନ ଗୁରୁତ୍ୱପୂର୍ଣ୍ଣ ସୂଚନା ପଢ଼ନ୍ତୁ।
InfoBeforeClickLabel=ସେଟଅପ୍ ଜାରି ରଖିବାକୁ ପ୍ରସ୍ତୁତ ହେଲେ “ପରବର୍ତ୍ତୀ” ଦବାନ୍ତୁ।
WizardInfoAfter=ସୂଚନା
InfoAfterLabel=ଜାରି ରଖିବା ପୂର୍ବରୁ ଦୟାକରି ନିମ୍ନ ଗୁରୁତ୍ୱପୂର୍ଣ୍ଣ ସୂଚନା ପଢ଼ନ୍ତୁ।
InfoAfterClickLabel=ସେଟଅପ୍ ଜାରି ରଖିବାକୁ ପ୍ରସ୍ତୁତ ହେଲେ “ପରବର୍ତ୍ତୀ” ଦବାନ୍ତୁ।

; *** "User Information" wizard page
WizardUserInfo=ଉପଯୋଗକର୍ତ୍ତାଙ୍କ ସୂଚନା
UserInfoDesc=ଦୟାକରି ଆପଣଙ୍କ ସୂଚନା ଦିଅନ୍ତୁ।
UserInfoName=ଉପଯୋଗକର୍ତ୍ତାଙ୍କ ନାମ:
UserInfoOrg=ସଂସ୍ଥା:
UserInfoSerial=ସିରିଆଲ୍ ନମ୍ବର:
UserInfoNameRequired=ଆପଣଙ୍କୁ ଏକ ନାମ ଦେବାକୁ ହେବ।

; *** "Select Destination Location" wizard page
WizardSelectDir=ଗନ୍ତବ୍ୟ ସ୍ଥାନ ବାଛନ୍ତୁ
SelectDirDesc=[name] କେଉଁଠି ଇନଷ୍ଟଲ୍ ହେବ?
SelectDirLabel3=ସେଟଅପ୍ [name] କୁ ନିମ୍ନ ଫୋଲ୍ଡରରେ ଇନଷ୍ଟଲ୍ କରିବ।
SelectDirBrowseLabel=ଜାରି ରଖିବାକୁ “ପରବର୍ତ୍ତୀ” ଦବାନ୍ତୁ। ଅନ୍ୟ ଫୋଲ୍ଡର ବାଛିବାକୁ ଚାହିଁଲେ “ବ୍ରାଉଜ୍” ଦବାନ୍ତୁ।
DiskSpaceGBLabel=ଅତିକମରେ [gb] GB ଖାଲି ଡିସ୍କ ସ୍ଥାନ ଦରକାର।
DiskSpaceMBLabel=ଅତିକମରେ [mb] MB ଖାଲି ଡିସ୍କ ସ୍ଥାନ ଦରକାର।
CannotInstallToNetworkDrive=ସେଟଅପ୍ ନେଟୱର୍କ ଡ୍ରାଇଭରେ ଇନଷ୍ଟଲ୍ କରିପାରିବ ନାହିଁ।
CannotInstallToUNCPath=ସେଟଅପ୍ UNC ପାଥରେ ଇନଷ୍ଟଲ୍ କରିପାରିବ ନାହିଁ।
InvalidPath=ଆପଣଙ୍କୁ ଡ୍ରାଇଭ୍ ଅକ୍ଷର ସହ ସମ୍ପୂର୍ଣ୍ଣ ପାଥ୍ ଦେବାକୁ ହେବ; ଯେପରି:%n%nC:\APP%n%nବା ଏହି ରୂପରେ UNC ପାଥ୍:%n%n\\server\share
InvalidDrive=ଆପଣ ବାଛିଥିବା ଡ୍ରାଇଭ୍ ବା UNC ଶେୟାର୍ ନାହିଁ କିମ୍ବା ପହଞ୍ଚିହେଉନାହିଁ। ଦୟାକରି ଅନ୍ୟଟି ବାଛନ୍ତୁ।
DiskSpaceWarningTitle=ଡିସ୍କରେ ଯଥେଷ୍ଟ ସ୍ଥାନ ନାହିଁ
DiskSpaceWarning=ଇନଷ୍ଟଲ୍ କରିବାକୁ ସେଟଅପ୍ ପାଇଁ ଅତିକମରେ %1 KB ଖାଲି ସ୍ଥାନ ଦରକାର, କିନ୍ତୁ ବାଛିଥିବା ଡ୍ରାଇଭରେ କେବଳ %2 KB ଉପଲବ୍ଧ।%n%nତଥାପି ଜାରି ରଖିବେ କି?
DirNameTooLong=ଫୋଲ୍ଡରର ନାମ ବା ପାଥ୍ ବହୁତ ଲମ୍ବା।
InvalidDirName=ଫୋଲ୍ଡରର ନାମ ଠିକ୍ ନୁହେଁ।
BadDirName32=ଫୋଲ୍ଡରର ନାମରେ ନିମ୍ନ ଅକ୍ଷରଗୁଡ଼ିକ ରହିପାରିବ ନାହିଁ:%n%n%1
DirExistsTitle=ଫୋଲ୍ଡର ପୂର୍ବରୁ ଅଛି
DirExists=ଫୋଲ୍ଡର:%n%n%1%n%nପୂର୍ବରୁ ଅଛି। ତଥାପି ସେହି ଫୋଲ୍ଡରରେ ଇନଷ୍ଟଲ୍ କରିବେ କି?
DirDoesntExistTitle=ଫୋଲ୍ଡର ନାହିଁ
DirDoesntExist=ଫୋଲ୍ଡର:%n%n%1%n%nନାହିଁ। ଫୋଲ୍ଡର ତିଆରି ହେବ କି?

; *** "Select Components" wizard page
WizardSelectComponents=ଉପାଦାନ ବାଛନ୍ତୁ
SelectComponentsDesc=କେଉଁ ଉପାଦାନ ଇନଷ୍ଟଲ୍ ହେବ?
SelectComponentsLabel2=ଯେଉଁ ଉପାଦାନ ଇନଷ୍ଟଲ୍ କରିବାକୁ ଚାହାଁନ୍ତି ବାଛନ୍ତୁ; ଚାହାଁନ୍ତି ନାହିଁ ତାହାର ଚିହ୍ନ ହଟାନ୍ତୁ। ଜାରି ରଖିବାକୁ ପ୍ରସ୍ତୁତ ହେଲେ “ପରବର୍ତ୍ତୀ” ଦବାନ୍ତୁ।
FullInstallation=ପୂର୍ଣ୍ଣ ଇନଷ୍ଟଲେସନ୍
; if possible don't translate 'Compact' as 'Minimal' (I mean 'Minimal' in your language)
CompactInstallation=ସଂକ୍ଷିପ୍ତ ଇନଷ୍ଟଲେସନ୍
CustomInstallation=କଷ୍ଟମ୍ ଇନଷ୍ଟଲେସନ୍
NoUninstallWarningTitle=ଉପାଦାନ ପୂର୍ବରୁ ଅଛି
NoUninstallWarning=ସେଟଅପ୍ ଦେଖିଲା ଯେ ନିମ୍ନ ଉପାଦାନଗୁଡ଼ିକ ଆପଣଙ୍କ କମ୍ପ୍ୟୁଟରରେ ପୂର୍ବରୁ ଇନଷ୍ଟଲ୍ ହୋଇଛି:%n%n%1%n%nଏଗୁଡ଼ିକର ଚୟନ ହଟାଇଲେ ସେଗୁଡ଼ିକ ଅନଇନଷ୍ଟଲ୍ ହେବ ନାହିଁ।%n%nତଥାପି ଜାରି ରଖିବେ କି?
ComponentSize1=%1 KB
ComponentSize2=%1 MB
ComponentsDiskSpaceGBLabel=ବର୍ତ୍ତମାନର ଚୟନ ପାଇଁ ଅତିକମରେ [gb] GB ଡିସ୍କ ସ୍ଥାନ ଦରକାର।
ComponentsDiskSpaceMBLabel=ବର୍ତ୍ତମାନର ଚୟନ ପାଇଁ ଅତିକମରେ [mb] MB ଡିସ୍କ ସ୍ଥାନ ଦରକାର।

; *** "Select Additional Tasks" wizard page
WizardSelectTasks=ଅତିରିକ୍ତ କାର୍ଯ୍ୟ ବାଛନ୍ତୁ
SelectTasksDesc=କେଉଁ ଅତିରିକ୍ତ କାର୍ଯ୍ୟ କରାଯିବ?
SelectTasksLabel2=[name] ଇନଷ୍ଟଲ୍ କରିବା ସମୟରେ ସେଟଅପ୍ ଯେଉଁ ଅତିରିକ୍ତ କାର୍ଯ୍ୟ କରୁ ବୋଲି ଆପଣ ଚାହାଁନ୍ତି ତାହା ବାଛନ୍ତୁ, ତାପରେ “ପରବର୍ତ୍ତୀ” ଦବାନ୍ତୁ।

; *** "Select Start Menu Folder" wizard page
WizardSelectProgramGroup=ଷ୍ଟାର୍ଟ ମେନୁ ଫୋଲ୍ଡର ବାଛନ୍ତୁ
SelectStartMenuFolderDesc=ସେଟଅପ୍ ପ୍ରୋଗ୍ରାମର ସର୍ଟକଟ୍ କେଉଁଠି ରଖିବ?
SelectStartMenuFolderLabel3=ସେଟଅପ୍ ପ୍ରୋଗ୍ରାମର ସର୍ଟକଟ୍ ନିମ୍ନ ଷ୍ଟାର୍ଟ ମେନୁ ଫୋଲ୍ଡରରେ ତିଆରି କରିବ।
SelectStartMenuFolderBrowseLabel=ଜାରି ରଖିବାକୁ “ପରବର୍ତ୍ତୀ” ଦବାନ୍ତୁ। ଅନ୍ୟ ଫୋଲ୍ଡର ବାଛିବାକୁ ଚାହିଁଲେ “ବ୍ରାଉଜ୍” ଦବାନ୍ତୁ।
MustEnterGroupName=ଆପଣଙ୍କୁ ଏକ ଫୋଲ୍ଡର ନାମ ଦେବାକୁ ହେବ।
GroupNameTooLong=ଫୋଲ୍ଡରର ନାମ ବା ପାଥ୍ ବହୁତ ଲମ୍ବା।
InvalidGroupName=ଫୋଲ୍ଡରର ନାମ ଠିକ୍ ନୁହେଁ।
BadGroupName=ଫୋଲ୍ଡରର ନାମରେ ନିମ୍ନ ଅକ୍ଷରଗୁଡ଼ିକ ରହିପାରିବ ନାହିଁ:%n%n%1
NoProgramGroupCheck2=ଷ୍ଟାର୍ଟ ମେନୁ ଫୋଲ୍ଡର ତିଆରି କରନ୍ତୁ ନାହିଁ

; *** "Ready to Install" wizard page
WizardReady=ଇନଷ୍ଟଲ୍ କରିବାକୁ ପ୍ରସ୍ତୁତ
ReadyLabel1=ସେଟଅପ୍ ଏବେ ଆପଣଙ୍କ କମ୍ପ୍ୟୁଟରରେ [name] ଇନଷ୍ଟଲ୍ କରିବା ଆରମ୍ଭ କରିବାକୁ ପ୍ରସ୍ତୁତ।
ReadyLabel2a=ଇନଷ୍ଟଲେସନ୍ ଜାରି ରଖିବାକୁ “ଇନଷ୍ଟଲ୍” ଦବାନ୍ତୁ, ବା କୌଣସି ସେଟିଂସ୍ ଦେଖିବାକୁ ବା ବଦଳାଇବାକୁ ଚାହିଁଲେ “ପଛକୁ” ଦବାନ୍ତୁ।
ReadyLabel2b=ଇନଷ୍ଟଲେସନ୍ ଜାରି ରଖିବାକୁ “ଇନଷ୍ଟଲ୍” ଦବାନ୍ତୁ।
ReadyMemoUserInfo=ଉପଯୋଗକର୍ତ୍ତାଙ୍କ ସୂଚନା:
ReadyMemoDir=ଗନ୍ତବ୍ୟ ସ୍ଥାନ:
ReadyMemoType=ସେଟଅପ୍ ପ୍ରକାର:
ReadyMemoComponents=ବାଛିଥିବା ଉପାଦାନ:
ReadyMemoGroup=ଷ୍ଟାର୍ଟ ମେନୁ ଫୋଲ୍ଡର:
ReadyMemoTasks=ଅତିରିକ୍ତ କାର୍ଯ୍ୟ:

; *** TDownloadWizardPage wizard page and DownloadTemporaryFile
DownloadingLabel2=ଫାଇଲ ଡାଉନଲୋଡ୍ ହେଉଛି...
ButtonStopDownload=ଡାଉନଲୋଡ୍ ବନ୍ଦ କରନ୍ତୁ
StopDownload=ଆପଣ ନିଶ୍ଚିତ ଭାବେ ଡାଉନଲୋଡ୍ ବନ୍ଦ କରିବାକୁ ଚାହାଁନ୍ତି କି?
ErrorDownloadAborted=ଡାଉନଲୋଡ୍ ବାତିଲ୍ ହେଲା
ErrorDownloadFailed=ଡାଉନଲୋଡ୍ ବିଫଳ ହେଲା: %1 %2
ErrorDownloadSizeFailed=ଆକାର ଜାଣିବା ବିଫଳ ହେଲା: %1 %2
ErrorProgress=ଅବୈଧ ପ୍ରଗତି: %2 ମଧ୍ୟରୁ %1
ErrorFileSize=ଅବୈଧ ଫାଇଲ ଆକାର: ଆଶା %1, ମିଳିଲା %2

; *** TExtractionWizardPage wizard page and ExtractArchive
ExtractingLabel=ଫାଇଲ ବାହାର କରାଯାଉଛି...
ButtonStopExtraction=ବାହାର କରିବା ବନ୍ଦ କରନ୍ତୁ
StopExtraction=ଆପଣ ନିଶ୍ଚିତ ଭାବେ ବାହାର କରିବା ବନ୍ଦ କରିବାକୁ ଚାହାଁନ୍ତି କି?
ErrorExtractionAborted=ବାହାର କରିବା ବାତିଲ୍ ହେଲା
ErrorExtractionFailed=ବାହାର କରିବା ବିଫଳ ହେଲା: %1

; *** Archive extraction failure details
ArchiveIncorrectPassword=ପାସୱାର୍ଡ ଠିକ୍ ନୁହେଁ
ArchiveIsCorrupted=ଆର୍କାଇଭ୍ ନଷ୍ଟ ହୋଇଯାଇଛି
ArchiveUnsupportedFormat=ଆର୍କାଇଭ୍ ଫର୍ମାଟ୍ ସମର୍ଥିତ ନୁହେଁ

; *** "Preparing to Install" wizard page
WizardPreparing=ଇନଷ୍ଟଲ୍ କରିବାର ପ୍ରସ୍ତୁତି
PreparingDesc=ସେଟଅପ୍ ଆପଣଙ୍କ କମ୍ପ୍ୟୁଟରରେ [name] ଇନଷ୍ଟଲ୍ କରିବାର ପ୍ରସ୍ତୁତି କରୁଛି।
PreviousInstallNotCompleted=ପୂର୍ବ ପ୍ରୋଗ୍ରାମର ଇନଷ୍ଟଲେସନ୍ ବା ଅପସାରଣ ସମ୍ପୂର୍ଣ୍ଣ ହୋଇନାହିଁ। ସେହି ଇନଷ୍ଟଲେସନ୍ ସମ୍ପୂର୍ଣ୍ଣ କରିବାକୁ ଆପଣଙ୍କୁ କମ୍ପ୍ୟୁଟର ପୁନଃ ଆରମ୍ଭ କରିବାକୁ ହେବ।%n%nକମ୍ପ୍ୟୁଟର ପୁନଃ ଆରମ୍ଭ କରିବା ପରେ [name] ର ଇନଷ୍ଟଲେସନ୍ ସମ୍ପୂର୍ଣ୍ଣ କରିବାକୁ ସେଟଅପ୍ ପୁଣି ଚଲାନ୍ତୁ।
CannotContinue=ସେଟଅପ୍ ଜାରି ରହିପାରିବ ନାହିଁ। ଦୟାକରି ବାହାରିବାକୁ “ବାତିଲ୍” ଦବାନ୍ତୁ।
ApplicationsFound=ନିମ୍ନ ପ୍ରୋଗ୍ରାମଗୁଡ଼ିକ ସେଟଅପ୍ ଅପଡେଟ୍ କରିବାକୁ ଥିବା ଫାଇଲ ବ୍ୟବହାର କରୁଛନ୍ତି। ସେଟଅପ୍ ଏହି ପ୍ରୋଗ୍ରାମଗୁଡ଼ିକୁ ସ୍ୱୟଂ ବନ୍ଦ କରିବାକୁ ଦେବାକୁ ପରାମର୍ଶ ଦିଆଯାଏ।
ApplicationsFound2=ନିମ୍ନ ପ୍ରୋଗ୍ରାମଗୁଡ଼ିକ ସେଟଅପ୍ ଅପଡେଟ୍ କରିବାକୁ ଥିବା ଫାଇଲ ବ୍ୟବହାର କରୁଛନ୍ତି। ସେଟଅପ୍ ଏହି ପ୍ରୋଗ୍ରାମଗୁଡ଼ିକୁ ସ୍ୱୟଂ ବନ୍ଦ କରିବାକୁ ଦେବାକୁ ପରାମର୍ଶ ଦିଆଯାଏ। ଇନଷ୍ଟଲେସନ୍ ସମାପ୍ତ ହେଲେ ସେଟଅପ୍ ପ୍ରୋଗ୍ରାମଗୁଡ଼ିକୁ ପୁଣି ଆରମ୍ଭ କରିବାକୁ ଚେଷ୍ଟା କରିବ।
CloseApplications=ପ୍ରୋଗ୍ରାମଗୁଡ଼ିକୁ ସ୍ୱୟଂ ବନ୍ଦ କରନ୍ତୁ
DontCloseApplications=ପ୍ରୋଗ୍ରାମଗୁଡ଼ିକୁ ବନ୍ଦ କରନ୍ତୁ ନାହିଁ
ErrorCloseApplications=ସେଟଅପ୍ ସବୁ ପ୍ରୋଗ୍ରାମକୁ ସ୍ୱୟଂ ବନ୍ଦ କରିପାରିଲା ନାହିଁ। ଜାରି ରଖିବା ପୂର୍ବରୁ ସେଟଅପ୍ ଅପଡେଟ୍ କରିବାକୁ ଥିବା ଫାଇଲ ବ୍ୟବହାର କରୁଥିବା ସବୁ ପ୍ରୋଗ୍ରାମ ବନ୍ଦ କରିବାକୁ ପରାମର୍ଶ ଦିଆଯାଏ।
PrepareToInstallNeedsRestart=ସେଟଅପ୍ ଆପଣଙ୍କ କମ୍ପ୍ୟୁଟର ପୁନଃ ଆରମ୍ଭ କରିବା ଦରକାର। କମ୍ପ୍ୟୁଟର ପୁନଃ ଆରମ୍ଭ ହେଲା ପରେ [name] ର ଇନଷ୍ଟଲେସନ୍ ସମ୍ପୂର୍ଣ୍ଣ କରିବାକୁ ସେଟଅପ୍ ପୁଣି ଚଲାନ୍ତୁ।%n%nଏବେ ପୁନଃ ଆରମ୍ଭ କରିବେ କି?

; *** "Installing" wizard page
WizardInstalling=ଇନଷ୍ଟଲ୍ ହେଉଛି
InstallingLabel=ସେଟଅପ୍ ଆପଣଙ୍କ କମ୍ପ୍ୟୁଟରରେ [name] ଇନଷ୍ଟଲ୍ କରୁଥିବା ବେଳେ ଦୟାକରି ଅପେକ୍ଷା କରନ୍ତୁ।

; *** "Setup Completed" wizard page
FinishedHeadingLabel=[name] ସେଟଅପ୍ ୱିଜାର୍ଡ ସମାପ୍ତ ହେଉଛି
FinishedLabelNoIcons=ସେଟଅପ୍ ଆପଣଙ୍କ କମ୍ପ୍ୟୁଟରରେ [name] ଇନଷ୍ଟଲ୍ କରିସାରିଛି।
FinishedLabel=ସେଟଅପ୍ ଆପଣଙ୍କ କମ୍ପ୍ୟୁଟରରେ [name] ଇନଷ୍ଟଲ୍ କରିସାରିଛି। ଇନଷ୍ଟଲ୍ ହୋଇଥିବା ସର୍ଟକଟ୍ ବାଛି ପ୍ରୋଗ୍ରାମ ଚଲାଇପାରିବେ।
ClickFinish=ସେଟଅପରୁ ବାହାରିବାକୁ “ସମାପ୍ତ” ଦବାନ୍ତୁ।
FinishedRestartLabel=[name] ର ଇନଷ୍ଟଲେସନ୍ ସମ୍ପୂର୍ଣ୍ଣ କରିବାକୁ ସେଟଅପ୍ ଆପଣଙ୍କ କମ୍ପ୍ୟୁଟର ପୁନଃ ଆରମ୍ଭ କରିବା ଦରକାର। ଏବେ ପୁନଃ ଆରମ୍ଭ କରିବେ କି?
FinishedRestartMessage=[name] ର ଇନଷ୍ଟଲେସନ୍ ସମ୍ପୂର୍ଣ୍ଣ କରିବାକୁ ସେଟଅପ୍ ଆପଣଙ୍କ କମ୍ପ୍ୟୁଟର ପୁନଃ ଆରମ୍ଭ କରିବା ଦରକାର।%n%nଏବେ ପୁନଃ ଆରମ୍ଭ କରିବେ କି?
ShowReadmeCheck=ହଁ, ମୁଁ README ଫାଇଲ ଦେଖିବାକୁ ଚାହେଁ
YesRadio=ହଁ, କମ୍ପ୍ୟୁଟର ଏବେ ପୁନଃ ଆରମ୍ଭ କରନ୍ତୁ
NoRadio=ନା, ମୁଁ କମ୍ପ୍ୟୁଟର ପରେ ପୁନଃ ଆରମ୍ଭ କରିବି
; used for example as 'Run MyProg.exe'
RunEntryExec=%1 ଚଲାନ୍ତୁ
; used for example as 'View Readme.txt'
RunEntryShellExec=%1 ଦେଖନ୍ତୁ

; *** "Setup Needs the Next Disk" stuff
ChangeDiskTitle=ସେଟଅପ୍ ପାଇଁ ପରବର୍ତ୍ତୀ ଡିସ୍କ ଦରକାର
SelectDiskLabel2=ଦୟାକରି ଡିସ୍କ %1 ଲଗାନ୍ତୁ ଏବଂ “ଠିକ୍ ଅଛି” ଦବାନ୍ତୁ।%n%nଏହି ଡିସ୍କର ଫାଇଲ ତଳେ ଦେଖାଯାଇଥିବା ଫୋଲ୍ଡରଠାରୁ ଭିନ୍ନ ଫୋଲ୍ଡରରେ ମିଳୁଥିଲେ, ଠିକ୍ ପାଥ୍ ଦିଅନ୍ତୁ ବା “ବ୍ରାଉଜ୍” ଦବାନ୍ତୁ।
PathLabel=ପାଥ୍:
FileNotInDir2="%2" ରେ "%1" ଫାଇଲ ମିଳିଲା ନାହିଁ। ଦୟାକରି ଠିକ୍ ଡିସ୍କ ଲଗାନ୍ତୁ ବା ଅନ୍ୟ ଫୋଲ୍ଡର ବାଛନ୍ତୁ।
SelectDirectoryLabel=ଦୟାକରି ପରବର୍ତ୍ତୀ ଡିସ୍କର ସ୍ଥାନ ଦିଅନ୍ତୁ।

; *** Installation phase messages
SetupAborted=ସେଟଅପ୍ ସମ୍ପୂର୍ଣ୍ଣ ହେଲା ନାହିଁ।%n%nଦୟାକରି ସମସ୍ୟା ଠିକ୍ କରି ସେଟଅପ୍ ପୁଣି ଚଲାନ୍ତୁ।
AbortRetryIgnoreSelectAction=କାର୍ଯ୍ୟ ବାଛନ୍ତୁ
AbortRetryIgnoreRetry=ପୁଣି ଚେଷ୍ଟା କରନ୍ତୁ
AbortRetryIgnoreIgnore=ତ୍ରୁଟି ଅଣଦେଖା କରି ଜାରି ରଖନ୍ତୁ
AbortRetryIgnoreCancel=ଇନଷ୍ଟଲେସନ୍ ବାତିଲ୍ କରନ୍ତୁ
RetryCancelSelectAction=କାର୍ଯ୍ୟ ବାଛନ୍ତୁ
RetryCancelRetry=ପୁଣି ଚେଷ୍ଟା କରନ୍ତୁ
RetryCancelCancel=ବାତିଲ୍

; *** Installation status messages
StatusClosingApplications=ପ୍ରୋଗ୍ରାମ ବନ୍ଦ କରାଯାଉଛି...
StatusCreateDirs=ଡାଇରେକ୍ଟୋରୀ ତିଆରି ହେଉଛି...
StatusExtractFiles=ଫାଇଲ ବାହାର କରାଯାଉଛି...
StatusDownloadFiles=ଫାଇଲ ଡାଉନଲୋଡ୍ ହେଉଛି...
StatusCreateIcons=ସର୍ଟକଟ୍ ତିଆରି ହେଉଛି...
StatusCreateIniEntries=INI ଏଣ୍ଟ୍ରି ତିଆରି ହେଉଛି...
StatusCreateRegistryEntries=ରେଜିଷ୍ଟ୍ରି ଏଣ୍ଟ୍ରି ତିଆରି ହେଉଛି...
StatusRegisterFiles=ଫାଇଲ ରେଜିଷ୍ଟର୍ ହେଉଛି...
StatusSavingUninstall=ଅନଇନଷ୍ଟଲ୍ ସୂଚନା ସେଭ୍ ହେଉଛି...
StatusRunProgram=ଇନଷ୍ଟଲେସନ୍ ସମାପ୍ତ ହେଉଛି...
StatusRestartingApplications=ପ୍ରୋଗ୍ରାମ ପୁନଃ ଆରମ୍ଭ ହେଉଛି...
StatusRollback=ପରିବର୍ତ୍ତନ ଫେରାଇ ନିଆଯାଉଛି...

; *** Misc. errors
ErrorInternal2=ଆଭ୍ୟନ୍ତରୀଣ ତ୍ରୁଟି: %1
ErrorFunctionFailedNoCode=%1 ବିଫଳ ହେଲା
ErrorFunctionFailed=%1 ବିଫଳ ହେଲା; କୋଡ୍ %2
ErrorFunctionFailedWithMessage=%1 ବିଫଳ ହେଲା; କୋଡ୍ %2.%n%3
ErrorExecutingProgram=ଫାଇଲ ଚଲାଇପାରିଲା ନାହିଁ:%n%1

; *** Registry errors
ErrorRegOpenKey=ରେଜିଷ୍ଟ୍ରି କୀ ଖୋଲିବାରେ ତ୍ରୁଟି:%n%1\%2
ErrorRegCreateKey=ରେଜିଷ୍ଟ୍ରି କୀ ତିଆରି କରିବାରେ ତ୍ରୁଟି:%n%1\%2
ErrorRegWriteKey=ରେଜିଷ୍ଟ୍ରି କୀରେ ଲେଖିବାରେ ତ୍ରୁଟି:%n%1\%2

; *** INI errors
ErrorIniEntry="%1" ଫାଇଲରେ INI ଏଣ୍ଟ୍ରି ତିଆରି କରିବାରେ ତ୍ରୁଟି।

; *** File copying errors
FileAbortRetryIgnoreSkipNotRecommended=ଏହି ଫାଇଲ ଛାଡ଼ି ଦିଅନ୍ତୁ (ପରାମର୍ଶିତ ନୁହେଁ)
FileAbortRetryIgnoreIgnoreNotRecommended=ତ୍ରୁଟି ଅଣଦେଖା କରି ଜାରି ରଖନ୍ତୁ (ପରାମର୍ଶିତ ନୁହେଁ)
SourceIsCorrupted=ମୂଳ ଫାଇଲ ନଷ୍ଟ ହୋଇଯାଇଛି
SourceDoesntExist=ମୂଳ ଫାଇଲ "%1" ନାହିଁ
SourceVerificationFailed=ମୂଳ ଫାଇଲର ଯାଞ୍ଚ ବିଫଳ ହେଲା: %1
VerificationSignatureDoesntExist=ସ୍ୱାକ୍ଷର ଫାଇଲ "%1" ନାହିଁ
VerificationSignatureInvalid=ସ୍ୱାକ୍ଷର ଫାଇଲ "%1" ଅବୈଧ
VerificationKeyNotFound=ସ୍ୱାକ୍ଷର ଫାଇଲ "%1" ଏକ ଅଜ୍ଞାତ କୀ ବ୍ୟବହାର କରୁଛି
VerificationFileNameIncorrect=ଫାଇଲର ନାମ ଠିକ୍ ନୁହେଁ
VerificationFileTagIncorrect=ଫାଇଲର ଟ୍ୟାଗ୍ ଠିକ୍ ନୁହେଁ
VerificationFileSizeIncorrect=ଫାଇଲର ଆକାର ଠିକ୍ ନୁହେଁ
VerificationFileHashIncorrect=ଫାଇଲର ହାଶ୍ ଠିକ୍ ନୁହେଁ
ExistingFileReadOnly2=ପୂର୍ବରୁ ଥିବା ଫାଇଲ କେବଳ-ପଢ଼ିବା ଭାବେ ଚିହ୍ନିତ ଥିବାରୁ ତାହାକୁ ବଦଳାଯାଇପାରିଲା ନାହିଁ।
ExistingFileReadOnlyRetry=କେବଳ-ପଢ଼ିବା ଗୁଣ ହଟାଇ ପୁଣି ଚେଷ୍ଟା କରନ୍ତୁ
ExistingFileReadOnlyKeepExisting=ପୂର୍ବରୁ ଥିବା ଫାଇଲ ରଖନ୍ତୁ
ErrorReadingExistingDest=ପୂର୍ବରୁ ଥିବା ଫାଇଲ ପଢ଼ିବା ସମୟରେ ଏକ ତ୍ରୁଟି ଘଟିଲା:
FileExistsSelectAction=କାର୍ଯ୍ୟ ବାଛନ୍ତୁ
FileExists2=ଫାଇଲ ପୂର୍ବରୁ ଅଛି।
FileExistsOverwriteExisting=ପୂର୍ବରୁ ଥିବା ଫାଇଲ ଓଭରରାଇଟ୍ କରନ୍ତୁ
FileExistsKeepExisting=ପୂର୍ବରୁ ଥିବା ଫାଇଲ ରଖନ୍ତୁ
FileExistsOverwriteOrKeepAll=ପରବର୍ତ୍ତୀ ସମାନ ସ୍ଥିତିରେ ଏହା ହିଁ କରନ୍ତୁ
ExistingFileNewerSelectAction=କାର୍ଯ୍ୟ ବାଛନ୍ତୁ
ExistingFileNewer2=ପୂର୍ବରୁ ଥିବା ଫାଇଲ ସେଟଅପ୍ ଇନଷ୍ଟଲ୍ କରିବାକୁ ଚାହୁଁଥିବା ଫାଇଲଠାରୁ ନୂଆ।
ExistingFileNewerOverwriteExisting=ପୂର୍ବରୁ ଥିବା ଫାଇଲ ଓଭରରାଇଟ୍ କରନ୍ତୁ
ExistingFileNewerKeepExisting=ପୂର୍ବରୁ ଥିବା ଫାଇଲ ରଖନ୍ତୁ (ପରାମର୍ଶିତ)
ExistingFileNewerOverwriteOrKeepAll=ପରବର୍ତ୍ତୀ ସମାନ ସ୍ଥିତିରେ ଏହା ହିଁ କରନ୍ତୁ
ErrorChangingAttr=ପୂର୍ବରୁ ଥିବା ଫାଇଲର ଗୁଣ ବଦଳାଇବା ସମୟରେ ଏକ ତ୍ରୁଟି ଘଟିଲା:
ErrorCreatingTemp=ଗନ୍ତବ୍ୟ ଡାଇରେକ୍ଟୋରୀରେ ଫାଇଲ ତିଆରି କରିବା ସମୟରେ ଏକ ତ୍ରୁଟି ଘଟିଲା:
ErrorReadingSource=ମୂଳ ଫାଇଲ ପଢ଼ିବା ସମୟରେ ଏକ ତ୍ରୁଟି ଘଟିଲା:
ErrorCopying=ଫାଇଲ କପି କରିବା ସମୟରେ ଏକ ତ୍ରୁଟି ଘଟିଲା:
ErrorDownloading=ଫାଇଲ ଡାଉନଲୋଡ୍ କରିବା ସମୟରେ ଏକ ତ୍ରୁଟି ଘଟିଲା:
ErrorExtracting=ଆର୍କାଇଭ୍ ବାହାର କରିବା ସମୟରେ ଏକ ତ୍ରୁଟି ଘଟିଲା:
ErrorReplacingExistingFile=ପୂର୍ବରୁ ଥିବା ଫାଇଲ ବଦଳାଇବା ସମୟରେ ଏକ ତ୍ରୁଟି ଘଟିଲା:
ErrorRestartReplace=RestartReplace ବିଫଳ ହେଲା:
ErrorRenamingTemp=ଗନ୍ତବ୍ୟ ଡାଇରେକ୍ଟୋରୀରେ ଫାଇଲର ନାମ ବଦଳାଇବା ସମୟରେ ଏକ ତ୍ରୁଟି ଘଟିଲା:
ErrorRegisterServer=DLL/OCX ରେଜିଷ୍ଟର୍ କରିପାରିଲା ନାହିଁ: %1
ErrorRegSvr32Failed=RegSvr32 ଏକ୍ସିଟ୍ କୋଡ୍ %1 ସହ ବିଫଳ ହେଲା
ErrorRegisterTypeLib=ଟାଇପ୍ ଲାଇବ୍ରେରୀ ରେଜିଷ୍ଟର୍ କରିପାରିଲା ନାହିଁ: %1

; *** Uninstall display name markings
; used for example as 'My Program (32-bit)'
UninstallDisplayNameMark=%1 (%2)
; used for example as 'My Program (32-bit, All users)'
UninstallDisplayNameMarks=%1 (%2, %3)
UninstallDisplayNameMark32Bit=32-ବିଟ୍
UninstallDisplayNameMark64Bit=64-ବିଟ୍
UninstallDisplayNameMarkAllUsers=ସମସ୍ତ ଉପଯୋଗକର୍ତ୍ତା
UninstallDisplayNameMarkCurrentUser=ବର୍ତ୍ତମାନର ଉପଯୋଗକର୍ତ୍ତା

; *** Post-installation errors
ErrorOpeningReadme=README ଫାଇଲ ଖୋଲିବା ସମୟରେ ଏକ ତ୍ରୁଟି ଘଟିଲା।
ErrorRestartingComputer=ସେଟଅପ୍ କମ୍ପ୍ୟୁଟର ପୁନଃ ଆରମ୍ଭ କରିପାରିଲା ନାହିଁ। ଦୟାକରି ଏହାକୁ ହାତରେ କରନ୍ତୁ।

; *** Uninstaller messages
UninstallNotFound="%1" ଫାଇଲ ନାହିଁ। ଅନଇନଷ୍ଟଲ୍ କରିହେବ ନାହିଁ।
UninstallOpenError="%1" ଫାଇଲ ଖୋଲିପାରିଲା ନାହିଁ। ଅନଇନଷ୍ଟଲ୍ କରିହେବ ନାହିଁ
UninstallUnsupportedVer=ଅନଇନଷ୍ଟଲ୍ ଲଗ୍ ଫାଇଲ "%1" ଅନଇନଷ୍ଟଲରର ଏହି ସଂସ୍କରଣ ଚିହ୍ନି ନଥିବା ଫର୍ମାଟରେ ଅଛି। ଅନଇନଷ୍ଟଲ୍ କରିହେବ ନାହିଁ
UninstallUnknownEntry=ଅନଇନଷ୍ଟଲ୍ ଲଗରେ ଏକ ଅଜ୍ଞାତ ଏଣ୍ଟ୍ରି (%1) ମିଳିଲା
ConfirmUninstall=ଆପଣ ନିଶ୍ଚିତ ଭାବେ %1 ଓ ଏହାର ସମସ୍ତ ଉପାଦାନ ସମ୍ପୂର୍ଣ୍ଣ ଭାବେ ହଟାଇବାକୁ ଚାହାଁନ୍ତି କି?
UninstallOnlyOnWin64=ଏହି ଇନଷ୍ଟଲେସନ୍ କେବଳ 64-ବିଟ୍ Windows ରେ ଅନଇନଷ୍ଟଲ୍ ହୋଇପାରିବ।
OnlyAdminCanUninstall=ଏହି ଇନଷ୍ଟଲେସନ୍ କେବଳ ଆଡମିନିଷ୍ଟ୍ରେଟର ଅଧିକାର ଥିବା ଉପଯୋଗକର୍ତ୍ତା ଅନଇନଷ୍ଟଲ୍ କରିପାରିବେ।
UninstallStatusLabel=%1 ଆପଣଙ୍କ କମ୍ପ୍ୟୁଟରରୁ ହଟାଯାଉଥିବା ବେଳେ ଦୟାକରି ଅପେକ୍ଷା କରନ୍ତୁ।
UninstalledAll=%1 ଆପଣଙ୍କ କମ୍ପ୍ୟୁଟରରୁ ସଫଳତାର ସହ ହଟାଗଲା।
UninstalledMost=%1 ଅନଇନଷ୍ଟଲ୍ ସମ୍ପୂର୍ଣ୍ଣ।%n%nକିଛି ଅଂଶ ହଟାଯାଇପାରିଲା ନାହିଁ। ସେଗୁଡ଼ିକୁ ହାତରେ ହଟାଯାଇପାରିବ।
UninstalledAndNeedsRestart=%1 ର ଅନଇନଷ୍ଟଲ୍ ସମ୍ପୂର୍ଣ୍ଣ କରିବାକୁ ଆପଣଙ୍କ କମ୍ପ୍ୟୁଟର ପୁନଃ ଆରମ୍ଭ କରିବାକୁ ହେବ।%n%nଏବେ ପୁନଃ ଆରମ୍ଭ କରିବେ କି?
UninstallDataCorrupted="%1" ଫାଇଲ ନଷ୍ଟ ହୋଇଯାଇଛି। ଅନଇନଷ୍ଟଲ୍ କରିହେବ ନାହିଁ

; *** Uninstallation phase messages
ConfirmDeleteSharedFileTitle=ଶେୟାର୍ ହୋଇଥିବା ଫାଇଲ ହଟାଇବେ କି?
ConfirmDeleteSharedFile2=ସିଷ୍ଟମ୍ ଦର୍ଶାଉଛି ଯେ ନିମ୍ନ ଶେୟାର୍ ହୋଇଥିବା ଫାଇଲ ଆଉ କୌଣସି ପ୍ରୋଗ୍ରାମ ବ୍ୟବହାର କରୁନାହିଁ। ଅନଇନଷ୍ଟଲ୍ ଏହି ଶେୟାର୍ ହୋଇଥିବା ଫାଇଲ ହଟାଇବ କି?%n%nଯଦି କୌଣସି ପ୍ରୋଗ୍ରାମ ଏବେ ବି ଏହି ଫାଇଲ ବ୍ୟବହାର କରୁଛି ଓ ଏହା ହଟାଗଲା, ସେହି ପ୍ରୋଗ୍ରାମ ଠିକ୍ ଭାବେ କାମ ନକରିପାରେ। ନିଶ୍ଚିତ ନଥିଲେ “ନା” ବାଛନ୍ତୁ। ଫାଇଲକୁ ସିଷ୍ଟମରେ ରଖିଲେ କୌଣସି କ୍ଷତି ହେବ ନାହିଁ।
SharedFileNameLabel=ଫାଇଲ ନାମ:
SharedFileLocationLabel=ସ୍ଥାନ:
WizardUninstalling=ଅନଇନଷ୍ଟଲ୍ ସ୍ଥିତି
StatusUninstalling=%1 ଅନଇନଷ୍ଟଲ୍ ହେଉଛି...

; *** Shutdown block reasons
ShutdownBlockReasonInstallingApp=%1 ଇନଷ୍ଟଲ୍ ହେଉଛି।
ShutdownBlockReasonUninstallingApp=%1 ଅନଇନଷ୍ଟଲ୍ ହେଉଛି।

; The custom messages below aren't used by Setup itself, but if you make
; use of them in your scripts, you'll want to translate them.

[CustomMessages]

NameAndVersion=%1 ସଂସ୍କରଣ %2
AdditionalIcons=ଅତିରିକ୍ତ ସର୍ଟକଟ୍:
CreateDesktopIcon=ଏକ ଡେସ୍କଟପ୍ ସର୍ଟକଟ୍ ତିଆରି କରନ୍ତୁ
CreateQuickLaunchIcon=ଏକ Quick Launch ସର୍ଟକଟ୍ ତିଆରି କରନ୍ତୁ
ProgramOnTheWeb=ୱେବରେ %1
UninstallProgram=%1 ଅନଇନଷ୍ଟଲ୍ କରନ୍ତୁ
LaunchProgram=%1 ଚଲାନ୍ତୁ
AssocFileExtension=%1 କୁ %2 ଫାଇଲ ଏକ୍ସଟେନସନ୍ ସହ ଯୋଡ଼ନ୍ତୁ
AssocingFileExtension=%1 କୁ %2 ଫାଇଲ ଏକ୍ସଟେନସନ୍ ସହ ଯୋଡ଼ାଯାଉଛି...
AutoStartProgramGroupDescription=ଷ୍ଟାର୍ଟଅପ୍:
AutoStartProgram=%1 ସ୍ୱୟଂ ଆରମ୍ଭ କରନ୍ତୁ
AddonHostProgramNotFound=ଆପଣ ବାଛିଥିବା ଫୋଲ୍ଡରରେ %1 ମିଳିଲା ନାହିଁ।%n%nତଥାପି ଜାରି ରଖିବେ କି?

; The messages that are Lang Switcher's own (the sample is English.isl).
LocalCertReady=ସେଟଅପ୍ ଏହି କମ୍ପ୍ୟୁଟରରେ ଏକ ସର୍ଟିଫିକେଟ୍ ତିଆରି କରି ତାହା ଦ୍ୱାରା Lang Switcher କୁ ସାଇନ୍ କରିବ, ଯାହା ଫଳରେ ଏହା ଆଡମିନିଷ୍ଟ୍ରେଟର ଭାବେ ଚାଲୁଥିବା ପ୍ରୋଗ୍ରାମର ୱିଣ୍ଡୋରେ ମଧ୍ୟ କାମ କରିପାରିବ।
LocalCertFailed=Lang Switcher ଇନଷ୍ଟଲ୍ ହେଲା, କିନ୍ତୁ ଏହା ଆଡମିନିଷ୍ଟ୍ରେଟର ଭାବେ ଚାଲୁଥିବା ପ୍ରୋଗ୍ରାମର ୱିଣ୍ଡୋରେ କାମ କରିବ ନାହିଁ: Windows ଏହି କମ୍ପ୍ୟୁଟରରେ ତିଆରି ସର୍ଟିଫିକେଟ୍ ଗ୍ରହଣ କଲା ନାହିଁ। ବିବରଣୀ ପାଇଁ ସେଟଅପ୍ ଲଗ୍ ଦେଖନ୍ତୁ।
RemoveSettingsQuestion=Lang Switcher ର ସେଟିଂସ୍ ଫାଇଲ ମଧ୍ୟ ହଟାଇବେ କି?
RemoveSettingsKeep=ପରେ ପୁଣି ଇନଷ୍ଟଲ୍ କରିବା ପାଇଁ ଆପଣଙ୍କ ସେଟିଂସ୍ ରଖିବାକୁ “ନା” ବାଛନ୍ତୁ।
AuthorSlogan=ହୃଦୟରୁ ତିଆରି ପ୍ରୋଗ୍ରାମ
