; Lang Switcher -- the messages of the installer in Gujarati (stage E108, group A).
;
; Inno Setup has no official Gujarati translation (inno_official = false in res\lang\languages.toml),
; so this file carries the whole of it: [LangOptions], every message of the compiler:Default.isl
; (6.5.0+), and the messages of the installer that are Lang Switcher's own. It is read after
; compiler:Default.isl as the second MessagesFile of the "gujarati" entry of [Languages] in
; ..\LangSwitcher.iss, and so replaces the English text name for name.
; English.isl is the sample the own messages translate; the glossary of the language is
; l10n\glossary\gu.tsv of the control contour. The name of the program is not translated.
; As in Default.isl: a message that has no full stop there has none here (Setup adds its own).
; No accelerator letters (&) -- a language without an official file has none (RULES 16); the names
; of buttons inside a message are in the quotation marks of the language.

[LangOptions]
LanguageName=ગુજરાતી
LanguageID=$0447
LanguageCodePage=0

[Messages]

; *** Application titles
SetupAppTitle=સેટઅપ
SetupWindowTitle=સેટઅપ - %1
UninstallAppTitle=અનઇન્સ્ટોલ
UninstallAppFullTitle=%1 અનઇન્સ્ટોલ

; *** Misc. common
InformationTitle=માહિતી
ConfirmTitle=પુષ્ટિ
ErrorTitle=ભૂલ

; *** SetupLdr messages
SetupLdrStartupMessage=આ %1 ઇન્સ્ટોલ કરશે. શું ચાલુ રાખવું છે?
LdrCannotCreateTemp=કામચલાઉ ફાઇલ બનાવી શકાઈ નથી. સેટઅપ રદ કરવામાં આવ્યું
LdrCannotExecTemp=કામચલાઉ ફોલ્ડરમાં ફાઇલ ચલાવી શકાઈ નથી. સેટઅપ રદ કરવામાં આવ્યું
HelpTextNote=

; *** Startup error messages
LastErrorMessage=%1.%n%nભૂલ %2: %3
SetupFileMissing=ઇન્સ્ટોલેશન ફોલ્ડરમાં ફાઇલ %1 નથી. કૃપા કરીને સમસ્યા ઠીક કરો અથવા પ્રોગ્રામની નવી નકલ મેળવો.
SetupFileCorrupt=સેટઅપ ફાઇલો બગડેલી છે. કૃપા કરીને પ્રોગ્રામની નવી નકલ મેળવો.
SetupFileCorruptOrWrongVer=સેટઅપ ફાઇલો બગડેલી છે, અથવા સેટઅપના આ વર્ઝન સાથે સુસંગત નથી. કૃપા કરીને સમસ્યા ઠીક કરો અથવા પ્રોગ્રામની નવી નકલ મેળવો.
InvalidParameter=કમાન્ડ લાઇનમાં ખોટો પેરામીટર આપવામાં આવ્યો:%n%n%1
SetupAlreadyRunning=સેટઅપ પહેલેથી ચાલી રહ્યું છે.
WindowsVersionNotSupported=આ પ્રોગ્રામ તમારા કમ્પ્યુટર પર ચાલતા Windows ના વર્ઝનને સપોર્ટ કરતો નથી.
WindowsServicePackRequired=આ પ્રોગ્રામને %1 સર્વિસ પૅક %2 અથવા નવો જોઈએ.
NotOnThisPlatform=આ પ્રોગ્રામ %1 પર ચાલશે નહીં.
OnlyOnThisPlatform=આ પ્રોગ્રામ %1 પર જ ચલાવવો જોઈએ.
OnlyOnTheseArchitectures=આ પ્રોગ્રામ ફક્ત નીચેના પ્રોસેસર આર્કિટેક્ચર માટે બનેલા Windows ના વર્ઝન પર જ ઇન્સ્ટોલ થઈ શકે છે:%n%n%1
WinVersionTooLowError=આ પ્રોગ્રામને %1 વર્ઝન %2 અથવા નવું જોઈએ.
WinVersionTooHighError=આ પ્રોગ્રામ %1 વર્ઝન %2 અથવા તેનાથી નવા પર ઇન્સ્ટોલ થઈ શકતો નથી.
AdminPrivilegesRequired=આ પ્રોગ્રામ ઇન્સ્ટોલ કરતી વખતે એડમિનિસ્ટ્રેટર તરીકે સાઇન ઇન હોવું જરૂરી છે.
PowerUserPrivilegesRequired=આ પ્રોગ્રામ ઇન્સ્ટોલ કરતી વખતે એડમિનિસ્ટ્રેટર તરીકે અથવા Power Users જૂથના સભ્ય તરીકે સાઇન ઇન હોવું જરૂરી છે.
SetupAppRunningError=સેટઅપને જાણવા મળ્યું છે કે %1 હાલમાં ચાલી રહ્યું છે.%n%nકૃપા કરીને તેની બધી ચાલુ વિન્ડો હમણાં બંધ કરો, પછી ચાલુ રાખવા માટે “ઓકે” અથવા બહાર નીકળવા માટે “રદ કરો” દબાવો.
UninstallAppRunningError=અનઇન્સ્ટોલને જાણવા મળ્યું છે કે %1 હાલમાં ચાલી રહ્યું છે.%n%nકૃપા કરીને તેની બધી ચાલુ વિન્ડો હમણાં બંધ કરો, પછી ચાલુ રાખવા માટે “ઓકે” અથવા બહાર નીકળવા માટે “રદ કરો” દબાવો.

; *** Startup questions
PrivilegesRequiredOverrideTitle=સેટઅપનો ઇન્સ્ટોલ મોડ પસંદ કરો
PrivilegesRequiredOverrideInstruction=ઇન્સ્ટોલ મોડ પસંદ કરો
PrivilegesRequiredOverrideText1=%1 બધા વપરાશકર્તાઓ માટે (એડમિનિસ્ટ્રેટર અધિકારો જોઈએ) અથવા ફક્ત તમારા માટે ઇન્સ્ટોલ કરી શકાય છે.
PrivilegesRequiredOverrideText2=%1 ફક્ત તમારા માટે અથવા બધા વપરાશકર્તાઓ માટે (એડમિનિસ્ટ્રેટર અધિકારો જોઈએ) ઇન્સ્ટોલ કરી શકાય છે.
PrivilegesRequiredOverrideAllUsers=બધા વપરાશકર્તાઓ માટે ઇન્સ્ટોલ કરો
PrivilegesRequiredOverrideAllUsersRecommended=બધા વપરાશકર્તાઓ માટે ઇન્સ્ટોલ કરો (ભલામણ કરેલ)
PrivilegesRequiredOverrideCurrentUser=ફક્ત મારા માટે ઇન્સ્ટોલ કરો
PrivilegesRequiredOverrideCurrentUserRecommended=ફક્ત મારા માટે ઇન્સ્ટોલ કરો (ભલામણ કરેલ)

; *** Misc. errors
ErrorCreatingDir=સેટઅપ ડિરેક્ટરી "%1" બનાવી શક્યું નહીં
ErrorTooManyFilesInDir=ડિરેક્ટરી "%1" માં ફાઇલ બનાવી શકાઈ નથી, કારણ કે તેમાં ઘણી વધારે ફાઇલો છે

; *** Setup common messages
ExitSetupTitle=સેટઅપમાંથી બહાર નીકળો
ExitSetupMessage=સેટઅપ પૂર્ણ થયું નથી. હમણાં બહાર નીકળવાથી પ્રોગ્રામ ઇન્સ્ટોલ થશે નહીં.%n%nઇન્સ્ટોલેશન પૂર્ણ કરવા માટે સેટઅપ બીજા કોઈ સમયે ફરીથી ચલાવી શકાય છે.%n%nશું સેટઅપમાંથી બહાર નીકળવું છે?
AboutSetupMenuItem=સેટઅપ વિશે...
AboutSetupTitle=સેટઅપ વિશે
AboutSetupMessage=%1 વર્ઝન %2%n%3%n%n%1 નું હોમ પેજ:%n%4
AboutSetupNote=
TranslatorNote=

; *** Buttons
ButtonBack=< પાછળ
ButtonNext=આગળ >
ButtonInstall=ઇન્સ્ટોલ કરો
ButtonOK=ઓકે
ButtonCancel=રદ કરો
ButtonYes=હા
ButtonYesToAll=બધા માટે હા
ButtonNo=ના
ButtonNoToAll=બધા માટે ના
ButtonFinish=સમાપ્ત કરો
ButtonBrowse=બ્રાઉઝ કરો...
ButtonWizardBrowse=બ્રાઉઝ કરો...
ButtonNewFolder=નવું ફોલ્ડર બનાવો

; *** "Select Language" dialog messages
SelectLanguageTitle=સેટઅપની ભાષા પસંદ કરો
SelectLanguageLabel=ઇન્સ્ટોલેશન દરમિયાન વાપરવા માટે ભાષા પસંદ કરો.

; *** Common wizard text
ClickNext=ચાલુ રાખવા માટે “આગળ” દબાવો, અથવા સેટઅપમાંથી બહાર નીકળવા માટે “રદ કરો” દબાવો.
BeveledLabel=
BrowseDialogTitle=ફોલ્ડર માટે બ્રાઉઝ કરો
BrowseDialogLabel=નીચેની યાદીમાંથી એક ફોલ્ડર પસંદ કરો, પછી “ઓકે” દબાવો.
NewFolderName=નવું ફોલ્ડર

; *** "Welcome" wizard page
WelcomeLabel1=[name] સેટઅપ વિઝાર્ડમાં આપનું સ્વાગત છે
WelcomeLabel2=આ તમારા કમ્પ્યુટર પર [name/ver] ઇન્સ્ટોલ કરશે.%n%nચાલુ રાખતાં પહેલાં બીજી બધી એપ્લિકેશનો બંધ કરવાની ભલામણ કરવામાં આવે છે.

; *** "Password" wizard page
WizardPassword=પાસવર્ડ
PasswordLabel1=આ ઇન્સ્ટોલેશન પાસવર્ડથી સુરક્ષિત છે.
PasswordLabel3=કૃપા કરીને પાસવર્ડ આપો, પછી ચાલુ રાખવા માટે “આગળ” દબાવો. પાસવર્ડમાં મોટા-નાના અક્ષરો મહત્ત્વના છે.
PasswordEditLabel=પાસવર્ડ:
IncorrectPassword=તમે આપેલો પાસવર્ડ સાચો નથી. કૃપા કરીને ફરીથી પ્રયાસ કરો.

; *** "License Agreement" wizard page
WizardLicense=લાઇસન્સ કરાર
LicenseLabel=ચાલુ રાખતાં પહેલાં કૃપા કરીને આ મહત્ત્વની માહિતી વાંચો.
LicenseLabel3=કૃપા કરીને નીચેનો લાઇસન્સ કરાર વાંચો. ઇન્સ્ટોલેશન ચાલુ રાખતાં પહેલાં આ કરારની શરતો સ્વીકારવી જરૂરી છે.
LicenseAccepted=મને કરાર સ્વીકાર્ય છે
LicenseNotAccepted=મને કરાર સ્વીકાર્ય નથી

; *** "Information" wizard pages
WizardInfoBefore=માહિતી
InfoBeforeLabel=ચાલુ રાખતાં પહેલાં કૃપા કરીને આ મહત્ત્વની માહિતી વાંચો.
InfoBeforeClickLabel=જ્યારે તમે સેટઅપ ચાલુ રાખવા માટે તૈયાર હો, ત્યારે “આગળ” દબાવો.
WizardInfoAfter=માહિતી
InfoAfterLabel=ચાલુ રાખતાં પહેલાં કૃપા કરીને આ મહત્ત્વની માહિતી વાંચો.
InfoAfterClickLabel=જ્યારે તમે સેટઅપ ચાલુ રાખવા માટે તૈયાર હો, ત્યારે “આગળ” દબાવો.

; *** "User Information" wizard page
WizardUserInfo=વપરાશકર્તાની માહિતી
UserInfoDesc=કૃપા કરીને તમારી માહિતી આપો.
UserInfoName=વપરાશકર્તાનું નામ:
UserInfoOrg=સંસ્થા:
UserInfoSerial=સિરિયલ નંબર:
UserInfoNameRequired=નામ આપવું જરૂરી છે.

; *** "Select Destination Location" wizard page
WizardSelectDir=ગંતવ્ય સ્થાન પસંદ કરો
SelectDirDesc=[name] ક્યાં ઇન્સ્ટોલ કરવું?
SelectDirLabel3=સેટઅપ [name] ને નીચેના ફોલ્ડરમાં ઇન્સ્ટોલ કરશે.
SelectDirBrowseLabel=ચાલુ રાખવા માટે “આગળ” દબાવો. જો બીજું ફોલ્ડર પસંદ કરવું હોય, તો “બ્રાઉઝ કરો” દબાવો.
DiskSpaceGBLabel=ઓછામાં ઓછી [gb] GB ખાલી ડિસ્ક જગ્યા જોઈએ.
DiskSpaceMBLabel=ઓછામાં ઓછી [mb] MB ખાલી ડિસ્ક જગ્યા જોઈએ.
CannotInstallToNetworkDrive=સેટઅપ નેટવર્ક ડ્રાઇવ પર ઇન્સ્ટોલ કરી શકતું નથી.
CannotInstallToUNCPath=સેટઅપ UNC પાથ પર ઇન્સ્ટોલ કરી શકતું નથી.
InvalidPath=ડ્રાઇવ અક્ષર સાથેનો પૂરો પાથ આપવો જરૂરી છે; ઉદાહરણ તરીકે:%n%nC:\APP%n%nઅથવા આ સ્વરૂપમાં UNC પાથ:%n%n\\server\share
InvalidDrive=તમે પસંદ કરેલી ડ્રાઇવ અથવા UNC શેર નથી અથવા તેના સુધી પહોંચી શકાતું નથી. કૃપા કરીને બીજી પસંદ કરો.
DiskSpaceWarningTitle=ડિસ્ક જગ્યા પૂરતી નથી
DiskSpaceWarning=ઇન્સ્ટોલ કરવા માટે સેટઅપને ઓછામાં ઓછી %1 KB ખાલી જગ્યા જોઈએ, પરંતુ પસંદ કરેલી ડ્રાઇવ પર ફક્ત %2 KB ઉપલબ્ધ છે.%n%nશું તેમ છતાં ચાલુ રાખવું છે?
DirNameTooLong=ફોલ્ડરનું નામ અથવા પાથ ખૂબ લાંબો છે.
InvalidDirName=ફોલ્ડરનું નામ માન્ય નથી.
BadDirName32=ફોલ્ડરના નામમાં નીચેના અક્ષરો હોઈ શકતા નથી:%n%n%1
DirExistsTitle=ફોલ્ડર પહેલેથી છે
DirExists=ફોલ્ડર:%n%n%1%n%nપહેલેથી છે. શું તેમ છતાં તે ફોલ્ડરમાં ઇન્સ્ટોલ કરવું છે?
DirDoesntExistTitle=ફોલ્ડર નથી
DirDoesntExist=ફોલ્ડર:%n%n%1%n%nનથી. શું તે ફોલ્ડર બનાવવું છે?

; *** "Select Components" wizard page
WizardSelectComponents=ઘટકો પસંદ કરો
SelectComponentsDesc=કયા ઘટકો ઇન્સ્ટોલ કરવા?
SelectComponentsLabel2=જે ઘટકો ઇન્સ્ટોલ કરવા હોય તે પસંદ કરો; જે ન કરવા હોય તેમની પસંદગી દૂર કરો. તૈયાર હો ત્યારે “આગળ” દબાવો.
FullInstallation=સંપૂર્ણ ઇન્સ્ટોલેશન
; if possible don't translate 'Compact' as 'Minimal' (I mean 'Minimal' in your language)
CompactInstallation=સંક્ષિપ્ત ઇન્સ્ટોલેશન
CustomInstallation=કસ્ટમ ઇન્સ્ટોલેશન
NoUninstallWarningTitle=ઘટકો પહેલેથી છે
NoUninstallWarning=સેટઅપને જાણવા મળ્યું છે કે નીચેના ઘટકો તમારા કમ્પ્યુટર પર પહેલેથી ઇન્સ્ટોલ થયેલા છે:%n%n%1%n%nઆ ઘટકોની પસંદગી દૂર કરવાથી તે અનઇન્સ્ટોલ થશે નહીં.%n%nશું તેમ છતાં ચાલુ રાખવું છે?
ComponentSize1=%1 KB
ComponentSize2=%1 MB
ComponentsDiskSpaceGBLabel=હાલની પસંદગી માટે ઓછામાં ઓછી [gb] GB ડિસ્ક જગ્યા જોઈએ.
ComponentsDiskSpaceMBLabel=હાલની પસંદગી માટે ઓછામાં ઓછી [mb] MB ડિસ્ક જગ્યા જોઈએ.

; *** "Select Additional Tasks" wizard page
WizardSelectTasks=વધારાનાં કાર્યો પસંદ કરો
SelectTasksDesc=કયાં વધારાનાં કાર્યો કરવાં?
SelectTasksLabel2=[name] ઇન્સ્ટોલ કરતી વખતે સેટઅપ જે વધારાનાં કાર્યો કરે તે પસંદ કરો, પછી “આગળ” દબાવો.

; *** "Select Start Menu Folder" wizard page
WizardSelectProgramGroup=સ્ટાર્ટ મેનૂ ફોલ્ડર પસંદ કરો
SelectStartMenuFolderDesc=સેટઅપ પ્રોગ્રામના શૉર્ટકટ ક્યાં મૂકે?
SelectStartMenuFolderLabel3=સેટઅપ પ્રોગ્રામના શૉર્ટકટ નીચેના સ્ટાર્ટ મેનૂ ફોલ્ડરમાં બનાવશે.
SelectStartMenuFolderBrowseLabel=ચાલુ રાખવા માટે “આગળ” દબાવો. જો બીજું ફોલ્ડર પસંદ કરવું હોય, તો “બ્રાઉઝ કરો” દબાવો.
MustEnterGroupName=ફોલ્ડરનું નામ આપવું જરૂરી છે.
GroupNameTooLong=ફોલ્ડરનું નામ અથવા પાથ ખૂબ લાંબો છે.
InvalidGroupName=ફોલ્ડરનું નામ માન્ય નથી.
BadGroupName=ફોલ્ડરના નામમાં નીચેના અક્ષરો હોઈ શકતા નથી:%n%n%1
NoProgramGroupCheck2=સ્ટાર્ટ મેનૂ ફોલ્ડર ન બનાવો

; *** "Ready to Install" wizard page
WizardReady=ઇન્સ્ટોલ કરવા માટે તૈયાર
ReadyLabel1=સેટઅપ હવે તમારા કમ્પ્યુટર પર [name] ઇન્સ્ટોલ કરવા માટે તૈયાર છે.
ReadyLabel2a=ઇન્સ્ટોલેશન ચાલુ રાખવા માટે “ઇન્સ્ટોલ કરો” દબાવો, અથવા કોઈ સેટિંગ જોવી કે બદલવી હોય તો “પાછળ” દબાવો.
ReadyLabel2b=ઇન્સ્ટોલેશન ચાલુ રાખવા માટે “ઇન્સ્ટોલ કરો” દબાવો.
ReadyMemoUserInfo=વપરાશકર્તાની માહિતી:
ReadyMemoDir=ગંતવ્ય સ્થાન:
ReadyMemoType=સેટઅપનો પ્રકાર:
ReadyMemoComponents=પસંદ કરેલા ઘટકો:
ReadyMemoGroup=સ્ટાર્ટ મેનૂ ફોલ્ડર:
ReadyMemoTasks=વધારાનાં કાર્યો:

; *** TDownloadWizardPage wizard page and DownloadTemporaryFile
DownloadingLabel2=ફાઇલો ડાઉનલોડ થઈ રહી છે...
ButtonStopDownload=ડાઉનલોડ અટકાવો
StopDownload=શું ડાઉનલોડ અટકાવવું છે?
ErrorDownloadAborted=ડાઉનલોડ રદ કરવામાં આવ્યું
ErrorDownloadFailed=ડાઉનલોડ નિષ્ફળ ગયું: %1 %2
ErrorDownloadSizeFailed=કદ જાણવું નિષ્ફળ ગયું: %1 %2
ErrorProgress=ખોટી પ્રગતિ: %1 / %2
ErrorFileSize=ફાઇલનું કદ ખોટું છે: અપેક્ષા %1 હતી, મળ્યું %2

; *** TExtractionWizardPage wizard page and ExtractArchive
ExtractingLabel=ફાઇલો કાઢવામાં આવી રહી છે...
ButtonStopExtraction=કાઢવાનું અટકાવો
StopExtraction=શું ફાઇલો કાઢવાનું અટકાવવું છે?
ErrorExtractionAborted=ફાઇલો કાઢવાનું રદ કરવામાં આવ્યું
ErrorExtractionFailed=ફાઇલો કાઢવાનું નિષ્ફળ ગયું: %1

; *** Archive extraction failure details
ArchiveIncorrectPassword=પાસવર્ડ સાચો નથી
ArchiveIsCorrupted=આર્કાઇવ બગડેલું છે
ArchiveUnsupportedFormat=આર્કાઇવનું ફૉર્મેટ સપોર્ટેડ નથી

; *** "Preparing to Install" wizard page
WizardPreparing=ઇન્સ્ટોલેશનની તૈયારી
PreparingDesc=સેટઅપ તમારા કમ્પ્યુટર પર [name] ઇન્સ્ટોલ કરવાની તૈયારી કરી રહ્યું છે.
PreviousInstallNotCompleted=અગાઉના કોઈ પ્રોગ્રામનું ઇન્સ્ટોલેશન અથવા દૂર કરવાનું કાર્ય પૂર્ણ થયું નથી. તેને પૂર્ણ કરવા માટે કમ્પ્યુટર રીસ્ટાર્ટ કરવું પડશે.%n%nકમ્પ્યુટર રીસ્ટાર્ટ કર્યા પછી [name] નું ઇન્સ્ટોલેશન પૂર્ણ કરવા માટે સેટઅપ ફરીથી ચલાવો.
CannotContinue=સેટઅપ ચાલુ રહી શકતું નથી. બહાર નીકળવા માટે કૃપા કરીને “રદ કરો” દબાવો.
ApplicationsFound=નીચેની એપ્લિકેશનો એવી ફાઇલોનો ઉપયોગ કરી રહી છે જેને સેટઅપે અપડેટ કરવી છે. ભલામણ કરવામાં આવે છે કે સેટઅપને આ એપ્લિકેશનો આપોઆપ બંધ કરવા દેવી.
ApplicationsFound2=નીચેની એપ્લિકેશનો એવી ફાઇલોનો ઉપયોગ કરી રહી છે જેને સેટઅપે અપડેટ કરવી છે. ભલામણ કરવામાં આવે છે કે સેટઅપને આ એપ્લિકેશનો આપોઆપ બંધ કરવા દેવી. ઇન્સ્ટોલેશન પૂર્ણ થયા પછી સેટઅપ આ એપ્લિકેશનો ફરી ચલાવવાનો પ્રયાસ કરશે.
CloseApplications=એપ્લિકેશનો આપોઆપ બંધ કરો
DontCloseApplications=એપ્લિકેશનો બંધ ન કરો
ErrorCloseApplications=સેટઅપ બધી એપ્લિકેશનો આપોઆપ બંધ કરી શક્યું નહીં. ચાલુ રાખતાં પહેલાં સેટઅપ દ્વારા અપડેટ થનારી ફાઇલોનો ઉપયોગ કરતી બધી એપ્લિકેશનો બંધ કરવાની ભલામણ કરવામાં આવે છે.
PrepareToInstallNeedsRestart=સેટઅપે તમારું કમ્પ્યુટર રીસ્ટાર્ટ કરવું પડશે. કમ્પ્યુટર રીસ્ટાર્ટ કર્યા પછી [name] નું ઇન્સ્ટોલેશન પૂર્ણ કરવા માટે સેટઅપ ફરીથી ચલાવો.%n%nશું હમણાં રીસ્ટાર્ટ કરવું છે?

; *** "Installing" wizard page
WizardInstalling=ઇન્સ્ટોલ થઈ રહ્યું છે
InstallingLabel=કૃપા કરીને રાહ જુઓ, સેટઅપ તમારા કમ્પ્યુટર પર [name] ઇન્સ્ટોલ કરી રહ્યું છે.

; *** "Setup Completed" wizard page
FinishedHeadingLabel=[name] સેટઅપ વિઝાર્ડ પૂર્ણ થઈ રહ્યો છે
FinishedLabelNoIcons=સેટઅપે તમારા કમ્પ્યુટર પર [name] ઇન્સ્ટોલ કરવાનું પૂર્ણ કર્યું છે.
FinishedLabel=સેટઅપે તમારા કમ્પ્યુટર પર [name] ઇન્સ્ટોલ કરવાનું પૂર્ણ કર્યું છે. ઇન્સ્ટોલ કરેલા શૉર્ટકટ પસંદ કરીને એપ્લિકેશન ચલાવી શકાય છે.
ClickFinish=સેટઅપમાંથી બહાર નીકળવા માટે “સમાપ્ત કરો” દબાવો.
FinishedRestartLabel=[name] નું ઇન્સ્ટોલેશન પૂર્ણ કરવા માટે સેટઅપે તમારું કમ્પ્યુટર રીસ્ટાર્ટ કરવું પડશે. શું હમણાં રીસ્ટાર્ટ કરવું છે?
FinishedRestartMessage=[name] નું ઇન્સ્ટોલેશન પૂર્ણ કરવા માટે સેટઅપે તમારું કમ્પ્યુટર રીસ્ટાર્ટ કરવું પડશે.%n%nશું હમણાં રીસ્ટાર્ટ કરવું છે?
ShowReadmeCheck=હા, README ફાઇલ જોવી છે
YesRadio=હા, કમ્પ્યુટર હમણાં રીસ્ટાર્ટ કરો
NoRadio=ના, રીસ્ટાર્ટ પછીથી કરવામાં આવશે
; used for example as 'Run MyProg.exe'
RunEntryExec=%1 ચલાવો
; used for example as 'View Readme.txt'
RunEntryShellExec=%1 જુઓ

; *** "Setup Needs the Next Disk" stuff
ChangeDiskTitle=સેટઅપને આગલી ડિસ્ક જોઈએ
SelectDiskLabel2=કૃપા કરીને ડિસ્ક %1 દાખલ કરો અને “ઓકે” દબાવો.%n%nજો આ ડિસ્કની ફાઇલો નીચે બતાવેલા ફોલ્ડર સિવાયના કોઈ બીજા ફોલ્ડરમાં મળી શકે તેમ હોય, તો સાચો પાથ આપો અથવા “બ્રાઉઝ કરો” દબાવો.
PathLabel=પાથ:
FileNotInDir2=ફાઇલ "%1" "%2" માં મળી નથી. કૃપા કરીને સાચી ડિસ્ક દાખલ કરો અથવા બીજું ફોલ્ડર પસંદ કરો.
SelectDirectoryLabel=કૃપા કરીને આગલી ડિસ્કનું સ્થાન જણાવો.

; *** Installation phase messages
SetupAborted=સેટઅપ પૂર્ણ થયું નથી.%n%nકૃપા કરીને સમસ્યા ઠીક કરો અને સેટઅપ ફરીથી ચલાવો.
AbortRetryIgnoreSelectAction=ક્રિયા પસંદ કરો
AbortRetryIgnoreRetry=ફરી પ્રયાસ કરો
AbortRetryIgnoreIgnore=ભૂલને અવગણો અને ચાલુ રાખો
AbortRetryIgnoreCancel=ઇન્સ્ટોલેશન રદ કરો
RetryCancelSelectAction=ક્રિયા પસંદ કરો
RetryCancelRetry=ફરી પ્રયાસ કરો
RetryCancelCancel=રદ કરો

; *** Installation status messages
StatusClosingApplications=એપ્લિકેશનો બંધ થઈ રહી છે...
StatusCreateDirs=ડિરેક્ટરીઓ બની રહી છે...
StatusExtractFiles=ફાઇલો કાઢવામાં આવી રહી છે...
StatusDownloadFiles=ફાઇલો ડાઉનલોડ થઈ રહી છે...
StatusCreateIcons=શૉર્ટકટ બની રહ્યા છે...
StatusCreateIniEntries=INI એન્ટ્રીઓ બની રહી છે...
StatusCreateRegistryEntries=રજિસ્ટ્રી એન્ટ્રીઓ બની રહી છે...
StatusRegisterFiles=ફાઇલો રજિસ્ટર થઈ રહી છે...
StatusSavingUninstall=અનઇન્સ્ટોલની માહિતી સાચવાઈ રહી છે...
StatusRunProgram=ઇન્સ્ટોલેશન પૂર્ણ થઈ રહ્યું છે...
StatusRestartingApplications=એપ્લિકેશનો ફરી ચાલુ થઈ રહી છે...
StatusRollback=ફેરફારો પાછા લેવાઈ રહ્યા છે...

; *** Misc. errors
ErrorInternal2=આંતરિક ભૂલ: %1
ErrorFunctionFailedNoCode=%1 નિષ્ફળ ગયું
ErrorFunctionFailed=%1 નિષ્ફળ ગયું; કોડ %2
ErrorFunctionFailedWithMessage=%1 નિષ્ફળ ગયું; કોડ %2.%n%3
ErrorExecutingProgram=ફાઇલ ચલાવી શકાઈ નથી:%n%1

; *** Registry errors
ErrorRegOpenKey=રજિસ્ટ્રી કી ખોલવામાં ભૂલ:%n%1\%2
ErrorRegCreateKey=રજિસ્ટ્રી કી બનાવવામાં ભૂલ:%n%1\%2
ErrorRegWriteKey=રજિસ્ટ્રી કીમાં લખવામાં ભૂલ:%n%1\%2

; *** INI errors
ErrorIniEntry=ફાઇલ "%1" માં INI એન્ટ્રી બનાવવામાં ભૂલ.

; *** File copying errors
FileAbortRetryIgnoreSkipNotRecommended=આ ફાઇલ છોડી દો (ભલામણ કરેલ નથી)
FileAbortRetryIgnoreIgnoreNotRecommended=ભૂલને અવગણો અને ચાલુ રાખો (ભલામણ કરેલ નથી)
SourceIsCorrupted=સ્રોત ફાઇલ બગડેલી છે
SourceDoesntExist=સ્રોત ફાઇલ "%1" નથી
SourceVerificationFailed=સ્રોત ફાઇલની ચકાસણી નિષ્ફળ ગઈ: %1
VerificationSignatureDoesntExist=હસ્તાક્ષર ફાઇલ "%1" નથી
VerificationSignatureInvalid=હસ્તાક્ષર ફાઇલ "%1" માન્ય નથી
VerificationKeyNotFound=હસ્તાક્ષર ફાઇલ "%1" અજાણી કીનો ઉપયોગ કરે છે
VerificationFileNameIncorrect=ફાઇલનું નામ ખોટું છે
VerificationFileTagIncorrect=ફાઇલનો ટૅગ ખોટો છે
VerificationFileSizeIncorrect=ફાઇલનું કદ ખોટું છે
VerificationFileHashIncorrect=ફાઇલનો હૅશ ખોટો છે
ExistingFileReadOnly2=હાલની ફાઇલ બદલી શકાઈ નથી, કારણ કે તે ફક્ત-વાંચવા માટે ચિહ્નિત છે.
ExistingFileReadOnlyRetry=ફક્ત-વાંચવાનો ગુણધર્મ દૂર કરો અને ફરી પ્રયાસ કરો
ExistingFileReadOnlyKeepExisting=હાલની ફાઇલ રાખો
ErrorReadingExistingDest=હાલની ફાઇલ વાંચતી વખતે ભૂલ થઈ:
FileExistsSelectAction=ક્રિયા પસંદ કરો
FileExists2=ફાઇલ પહેલેથી છે.
FileExistsOverwriteExisting=હાલની ફાઇલની જગ્યાએ નવી લખો
FileExistsKeepExisting=હાલની ફાઇલ રાખો
FileExistsOverwriteOrKeepAll=આગળના ટકરાવો માટે પણ આમ જ કરો
ExistingFileNewerSelectAction=ક્રિયા પસંદ કરો
ExistingFileNewer2=હાલની ફાઇલ, સેટઅપ જે ફાઇલ ઇન્સ્ટોલ કરવાનો પ્રયાસ કરી રહ્યું છે તેના કરતાં નવી છે.
ExistingFileNewerOverwriteExisting=હાલની ફાઇલની જગ્યાએ નવી લખો
ExistingFileNewerKeepExisting=હાલની ફાઇલ રાખો (ભલામણ કરેલ)
ExistingFileNewerOverwriteOrKeepAll=આગળના ટકરાવો માટે પણ આમ જ કરો
ErrorChangingAttr=હાલની ફાઇલના ગુણધર્મ બદલતી વખતે ભૂલ થઈ:
ErrorCreatingTemp=ગંતવ્ય ડિરેક્ટરીમાં ફાઇલ બનાવતી વખતે ભૂલ થઈ:
ErrorReadingSource=સ્રોત ફાઇલ વાંચતી વખતે ભૂલ થઈ:
ErrorCopying=ફાઇલ કૉપિ કરતી વખતે ભૂલ થઈ:
ErrorDownloading=ફાઇલ ડાઉનલોડ કરતી વખતે ભૂલ થઈ:
ErrorExtracting=આર્કાઇવ ખોલતી વખતે ભૂલ થઈ:
ErrorReplacingExistingFile=હાલની ફાઇલ બદલતી વખતે ભૂલ થઈ:
ErrorRestartReplace=RestartReplace નિષ્ફળ ગયું:
ErrorRenamingTemp=ગંતવ્ય ડિરેક્ટરીમાં ફાઇલનું નામ બદલતી વખતે ભૂલ થઈ:
ErrorRegisterServer=DLL/OCX રજિસ્ટર થઈ શક્યું નહીં: %1
ErrorRegSvr32Failed=RegSvr32 એક્ઝિટ કોડ %1 સાથે નિષ્ફળ ગયું
ErrorRegisterTypeLib=ટાઇપ લાઇબ્રેરી રજિસ્ટર થઈ શકી નહીં: %1

; *** Uninstall display name markings
; used for example as 'My Program (32-bit)'
UninstallDisplayNameMark=%1 (%2)
; used for example as 'My Program (32-bit, All users)'
UninstallDisplayNameMarks=%1 (%2, %3)
UninstallDisplayNameMark32Bit=32-બિટ
UninstallDisplayNameMark64Bit=64-બિટ
UninstallDisplayNameMarkAllUsers=બધા વપરાશકર્તાઓ
UninstallDisplayNameMarkCurrentUser=હાલના વપરાશકર્તા

; *** Post-installation errors
ErrorOpeningReadme=README ફાઇલ ખોલતી વખતે ભૂલ થઈ.
ErrorRestartingComputer=સેટઅપ કમ્પ્યુટર રીસ્ટાર્ટ કરી શક્યું નહીં. કૃપા કરીને તે જાતે કરો.

; *** Uninstaller messages
UninstallNotFound=ફાઇલ "%1" નથી. અનઇન્સ્ટોલ કરી શકાતું નથી.
UninstallOpenError=ફાઇલ "%1" ખોલી શકાઈ નથી. અનઇન્સ્ટોલ કરી શકાતું નથી
UninstallUnsupportedVer=અનઇન્સ્ટોલ લૉગ ફાઇલ "%1" એવા ફૉર્મેટમાં છે જેને અનઇન્સ્ટોલરનું આ વર્ઝન ઓળખતું નથી. અનઇન્સ્ટોલ કરી શકાતું નથી
UninstallUnknownEntry=અનઇન્સ્ટોલ લૉગમાં એક અજાણી એન્ટ્રી (%1) મળી
ConfirmUninstall=શું %1 અને તેના બધા ઘટકો સંપૂર્ણપણે દૂર કરવા છે?
UninstallOnlyOnWin64=આ ઇન્સ્ટોલેશન ફક્ત 64-બિટ Windows પર જ અનઇન્સ્ટોલ કરી શકાય છે.
OnlyAdminCanUninstall=આ ઇન્સ્ટોલેશન ફક્ત એડમિનિસ્ટ્રેટર અધિકારો ધરાવતો વપરાશકર્તા જ અનઇન્સ્ટોલ કરી શકે છે.
UninstallStatusLabel=કૃપા કરીને રાહ જુઓ, %1 તમારા કમ્પ્યુટરમાંથી દૂર થઈ રહ્યું છે.
UninstalledAll=%1 તમારા કમ્પ્યુટરમાંથી સફળતાપૂર્વક દૂર કરવામાં આવ્યું.
UninstalledMost=%1 નું અનઇન્સ્ટોલ પૂર્ણ થયું.%n%nકેટલાક ભાગો દૂર કરી શકાયા નથી. તેમને જાતે દૂર કરી શકાય છે.
UninstalledAndNeedsRestart=%1 નું અનઇન્સ્ટોલ પૂર્ણ કરવા માટે તમારું કમ્પ્યુટર રીસ્ટાર્ટ કરવું પડશે.%n%nશું હમણાં રીસ્ટાર્ટ કરવું છે?
UninstallDataCorrupted="%1" ફાઇલ બગડેલી છે. અનઇન્સ્ટોલ કરી શકાતું નથી

; *** Uninstallation phase messages
ConfirmDeleteSharedFileTitle=શેર કરેલી ફાઇલ દૂર કરવી છે?
ConfirmDeleteSharedFile2=સિસ્ટમ જણાવે છે કે નીચેની શેર કરેલી ફાઇલ હવે કોઈ પ્રોગ્રામ વાપરતો નથી. શું અનઇન્સ્ટોલ આ શેર કરેલી ફાઇલ દૂર કરે?%n%nજો કોઈ પ્રોગ્રામ હજી પણ આ ફાઇલ વાપરતો હોય અને તે દૂર થઈ જાય, તો તે પ્રોગ્રામ કદાચ બરાબર કામ ન કરે. જો ખાતરી ન હોય, તો “ના” પસંદ કરો. ફાઇલને સિસ્ટમમાં રહેવા દેવાથી કોઈ નુકસાન થશે નહીં.
SharedFileNameLabel=ફાઇલનું નામ:
SharedFileLocationLabel=સ્થાન:
WizardUninstalling=અનઇન્સ્ટોલની સ્થિતિ
StatusUninstalling=%1 અનઇન્સ્ટોલ થઈ રહ્યું છે...

; *** Shutdown block reasons
ShutdownBlockReasonInstallingApp=%1 ઇન્સ્ટોલ થઈ રહ્યું છે.
ShutdownBlockReasonUninstallingApp=%1 અનઇન્સ્ટોલ થઈ રહ્યું છે.

; The custom messages below aren't used by Setup itself, but if you make
; use of them in your scripts, you'll want to translate them.

[CustomMessages]

NameAndVersion=%1 વર્ઝન %2
AdditionalIcons=વધારાના શૉર્ટકટ:
CreateDesktopIcon=ડેસ્કટૉપ શૉર્ટકટ બનાવો
CreateQuickLaunchIcon=ક્વિક લૉન્ચ શૉર્ટકટ બનાવો
ProgramOnTheWeb=વેબ પર %1
UninstallProgram=%1 અનઇન્સ્ટોલ કરો
LaunchProgram=%1 ચલાવો
AssocFileExtension=%1 ને %2 ફાઇલ એક્સ્ટેન્શન સાથે જોડો
AssocingFileExtension=%1 ને %2 ફાઇલ એક્સ્ટેન્શન સાથે જોડવામાં આવી રહ્યું છે...
AutoStartProgramGroupDescription=સ્ટાર્ટઅપ:
AutoStartProgram=%1 આપોઆપ શરૂ કરો
AddonHostProgramNotFound=તમે પસંદ કરેલા ફોલ્ડરમાં %1 મળ્યું નહીં.%n%nશું તેમ છતાં ચાલુ રાખવું છે?

; The messages that are Lang Switcher's own (the sample is English.isl).
LocalCertReady=સેટઅપ આ કમ્પ્યુટર પર એક પ્રમાણપત્ર બનાવશે અને તેનાથી Lang Switcher પર હસ્તાક્ષર કરશે, જેથી તે એડમિનિસ્ટ્રેટર તરીકે ચલાવેલા પ્રોગ્રામોની વિન્ડોમાં પણ કામ કરી શકે.
LocalCertFailed=Lang Switcher ઇન્સ્ટોલ થઈ ગયું, પરંતુ તે એડમિનિસ્ટ્રેટર તરીકે ચલાવેલા પ્રોગ્રામોની વિન્ડોમાં કામ કરશે નહીં: Windows એ આ કમ્પ્યુટર પર બનાવેલું પ્રમાણપત્ર સ્વીકાર્યું નહીં. વિગતો માટે સેટઅપ લૉગ જુઓ.
RemoveSettingsQuestion=શું Lang Switcher ની સેટિંગ્સ ફાઇલ પણ દૂર કરવી છે?
RemoveSettingsKeep=ફરી ઇન્સ્ટોલ કરવા માટે તમારી સેટિંગ્સ રાખવી હોય, તો “ના” પસંદ કરો.
AuthorSlogan=દિલથી બનેલા પ્રોગ્રામ
