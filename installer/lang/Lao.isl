; Lang Switcher -- the messages of the installer in Lao (stage E108, group A).
;
; Inno Setup has no official Lao file, so this file is whole: [LangOptions], all the messages of
; compiler:Default.isl (version 6.5.0 sample) translated, then the stock [CustomMessages] of that
; file and the messages of the installer that are its own (the sample is English.isl). Read as the
; only MessagesFile of the "lao" entry of [Languages] in ..\LangSwitcher.iss; Default.isl may
; stand before it: what is missing here falls back to English.
; No accelerators (&): a file without an official translation has none, so that one window never
; shows two equal letters (decision of the E108 trial, RULES 16). Messages that end without a
; full stop in Default.isl end without one here: Setup adds a Latin full stop itself. The names of
; the buttons inside a message stand in the quotes of the language. The name of the program is
; not translated.
; AuthorSlogan (stage E99, decision 167.1): the slogan of the signature of the author at the
; bottom of every page of the wizard -- after "Panda Koder" and a middle dot; light, without
; pathos. The name of the author and the address of the site stay as they are in every language.

[LangOptions]
LanguageName=ລາວ
LanguageID=$0454
LanguageCodePage=0
;DialogFontName=
;DialogFontSize=9
;DialogFontBaseScaleWidth=7
;DialogFontBaseScaleHeight=15
;WelcomeFontName=Segoe UI
;WelcomeFontSize=14

[Messages]

; *** Application titles
SetupAppTitle=ຕິດຕັ້ງ
SetupWindowTitle=ຕິດຕັ້ງ - %1
UninstallAppTitle=ຖອນການຕິດຕັ້ງ
UninstallAppFullTitle=ຖອນການຕິດຕັ້ງ %1

; *** Misc. common
InformationTitle=ຂໍ້ມູນ
ConfirmTitle=ຢືນຢັນ
ErrorTitle=ຂໍ້ຜິດພາດ

; *** SetupLdr messages
SetupLdrStartupMessage=ນີ້ຈະຕິດຕັ້ງ %1 ທ່ານຕ້ອງການດຳເນີນການຕໍ່ບໍ?
LdrCannotCreateTemp=ບໍ່ສາມາດສ້າງໄຟລ໌ຊົ່ວຄາວໄດ້ ການຕິດຕັ້ງຖືກຢຸດ
LdrCannotExecTemp=ບໍ່ສາມາດເປີດໃຊ້ໄຟລ໌ໃນໂຟນເດີຊົ່ວຄາວໄດ້ ການຕິດຕັ້ງຖືກຢຸດ
HelpTextNote=

; *** Startup error messages
LastErrorMessage=%1.%n%nຂໍ້ຜິດພາດ %2: %3
SetupFileMissing=ບໍ່ພົບໄຟລ໌ %1 ໃນໂຟນເດີຕິດຕັ້ງ ກະລຸນາແກ້ໄຂບັນຫາ ຫຼືຮັບສຳເນົາໂປຣແກຣມໃໝ່
SetupFileCorrupt=ໄຟລ໌ຕິດຕັ້ງເສຍຫາຍ ກະລຸນາຮັບສຳເນົາໂປຣແກຣມໃໝ່
SetupFileCorruptOrWrongVer=ໄຟລ໌ຕິດຕັ້ງເສຍຫາຍ ຫຼືບໍ່ເຂົ້າກັນກັບຕົວຕິດຕັ້ງເວີຊັນນີ້ ກະລຸນາແກ້ໄຂບັນຫາ ຫຼືຮັບສຳເນົາໂປຣແກຣມໃໝ່
InvalidParameter=ມີພາຣາມິເຕີທີ່ບໍ່ຖືກຕ້ອງໃນບັນທັດຄຳສັ່ງ:%n%n%1
SetupAlreadyRunning=ຕົວຕິດຕັ້ງກຳລັງເຮັດວຽກຢູ່ແລ້ວ
WindowsVersionNotSupported=ໂປຣແກຣມນີ້ບໍ່ຮອງຮັບ Windows ເວີຊັນທີ່ຄອມພິວເຕີຂອງທ່ານໃຊ້ຢູ່
WindowsServicePackRequired=ໂປຣແກຣມນີ້ຕ້ອງການ %1 Service Pack %2 ຫຼືໃໝ່ກວ່າ
NotOnThisPlatform=ໂປຣແກຣມນີ້ຈະບໍ່ເຮັດວຽກຢູ່ໃນ %1
OnlyOnThisPlatform=ໂປຣແກຣມນີ້ຕ້ອງເປີດໃຊ້ໃນ %1
OnlyOnTheseArchitectures=ໂປຣແກຣມນີ້ຕິດຕັ້ງໄດ້ສະເພາະໃນ Windows ທີ່ອອກແບບສຳລັບສະຖາປັດຕະຍະກຳໂປຣເຊດເຊີຕໍ່ໄປນີ້:%n%n%1
WinVersionTooLowError=ໂປຣແກຣມນີ້ຕ້ອງການ %1 ເວີຊັນ %2 ຫຼືໃໝ່ກວ່າ
WinVersionTooHighError=ໂປຣແກຣມນີ້ບໍ່ສາມາດຕິດຕັ້ງໃນ %1 ເວີຊັນ %2 ຫຼືໃໝ່ກວ່າໄດ້
AdminPrivilegesRequired=ທ່ານຕ້ອງເຂົ້າສູ່ລະບົບໃນຖານະຜູ້ດູແລລະບົບ ເມື່ອຕິດຕັ້ງໂປຣແກຣມນີ້
PowerUserPrivilegesRequired=ທ່ານຕ້ອງເຂົ້າສູ່ລະບົບໃນຖານະຜູ້ດູແລລະບົບ ຫຼືສະມາຊິກກຸ່ມ Power Users ເມື່ອຕິດຕັ້ງໂປຣແກຣມນີ້
SetupAppRunningError=ຕົວຕິດຕັ້ງກວດພົບວ່າ %1 ກຳລັງເຮັດວຽກຢູ່%n%nກະລຸນາປິດທຸກໜ້າຕ່າງຂອງມັນດຽວນີ້ ແລ້ວກົດ “ຕົກລົງ” ເພື່ອດຳເນີນການຕໍ່ ຫຼື “ຍົກເລີກ” ເພື່ອອອກ
UninstallAppRunningError=ຕົວຖອນການຕິດຕັ້ງກວດພົບວ່າ %1 ກຳລັງເຮັດວຽກຢູ່%n%nກະລຸນາປິດທຸກໜ້າຕ່າງຂອງມັນດຽວນີ້ ແລ້ວກົດ “ຕົກລົງ” ເພື່ອດຳເນີນການຕໍ່ ຫຼື “ຍົກເລີກ” ເພື່ອອອກ

; *** Startup questions
PrivilegesRequiredOverrideTitle=ເລືອກໂໝດການຕິດຕັ້ງ
PrivilegesRequiredOverrideInstruction=ເລືອກໂໝດການຕິດຕັ້ງ
PrivilegesRequiredOverrideText1=%1 ຕິດຕັ້ງໄດ້ສຳລັບຜູ້ໃຊ້ທຸກຄົນ (ຕ້ອງມີສິດຜູ້ດູແລລະບົບ) ຫຼືສຳລັບທ່ານຄົນດຽວ
PrivilegesRequiredOverrideText2=%1 ຕິດຕັ້ງໄດ້ສຳລັບທ່ານຄົນດຽວ ຫຼືສຳລັບຜູ້ໃຊ້ທຸກຄົນ (ຕ້ອງມີສິດຜູ້ດູແລລະບົບ)
PrivilegesRequiredOverrideAllUsers=ຕິດຕັ້ງສຳລັບຜູ້ໃຊ້ທຸກຄົນ
PrivilegesRequiredOverrideAllUsersRecommended=ຕິດຕັ້ງສຳລັບຜູ້ໃຊ້ທຸກຄົນ (ແນະນຳ)
PrivilegesRequiredOverrideCurrentUser=ຕິດຕັ້ງສຳລັບຂ້ອຍຄົນດຽວ
PrivilegesRequiredOverrideCurrentUserRecommended=ຕິດຕັ້ງສຳລັບຂ້ອຍຄົນດຽວ (ແນະນຳ)

; *** Misc. errors
ErrorCreatingDir=ຕົວຕິດຕັ້ງບໍ່ສາມາດສ້າງໂຟນເດີ "%1" ໄດ້
ErrorTooManyFilesInDir=ບໍ່ສາມາດສ້າງໄຟລ໌ໃນໂຟນເດີ "%1" ໄດ້ ເພາະມີໄຟລ໌ຫຼາຍເກີນໄປ

; *** Setup common messages
ExitSetupTitle=ອອກຈາກການຕິດຕັ້ງ
ExitSetupMessage=ການຕິດຕັ້ງຍັງບໍ່ສຳເລັດ ຖ້າອອກດຽວນີ້ ໂປຣແກຣມຈະບໍ່ຖືກຕິດຕັ້ງ%n%nທ່ານສາມາດເປີດຕົວຕິດຕັ້ງອີກຄັ້ງເພື່ອຕິດຕັ້ງໃຫ້ສຳເລັດໃນພາຍຫຼັງ%n%nອອກຈາກການຕິດຕັ້ງບໍ?
AboutSetupMenuItem=ກ່ຽວກັບການຕິດຕັ້ງ...
AboutSetupTitle=ກ່ຽວກັບການຕິດຕັ້ງ
AboutSetupMessage=%1 ເວີຊັນ %2%n%3%n%nໜ້າເວັບຂອງ %1:%n%4
AboutSetupNote=
TranslatorNote=

; *** Buttons
ButtonBack=< ກັບຄືນ
ButtonNext=ຖັດໄປ >
ButtonInstall=ຕິດຕັ້ງ
ButtonOK=ຕົກລົງ
ButtonCancel=ຍົກເລີກ
ButtonYes=ແມ່ນ
ButtonYesToAll=ແມ່ນທັງໝົດ
ButtonNo=ບໍ່
ButtonNoToAll=ບໍ່ທັງໝົດ
ButtonFinish=ສຳເລັດ
ButtonBrowse=ຄົ້ນຫາ...
ButtonWizardBrowse=ຄົ້ນຫາ...
ButtonNewFolder=ສ້າງໂຟນເດີໃໝ່

; *** "Select Language" dialog messages
SelectLanguageTitle=ເລືອກພາສາຂອງການຕິດຕັ້ງ
SelectLanguageLabel=ເລືອກພາສາທີ່ຈະໃຊ້ໃນລະຫວ່າງການຕິດຕັ້ງ

; *** Common wizard text
ClickNext=ກົດ “ຖັດໄປ” ເພື່ອດຳເນີນການຕໍ່ ຫຼື “ຍົກເລີກ” ເພື່ອອອກຈາກການຕິດຕັ້ງ
BeveledLabel=
BrowseDialogTitle=ເລືອກໂຟນເດີ
BrowseDialogLabel=ເລືອກໂຟນເດີໃນລາຍການຂ້າງລຸ່ມນີ້ ແລ້ວກົດ “ຕົກລົງ”
NewFolderName=ໂຟນເດີໃໝ່

; *** "Welcome" wizard page
WelcomeLabel1=ຍິນດີຕ້ອນຮັບສູ່ຕົວຊ່ວຍຕິດຕັ້ງ [name]
WelcomeLabel2=ນີ້ຈະຕິດຕັ້ງ [name/ver] ໃນຄອມພິວເຕີຂອງທ່ານ%n%nແນະນຳໃຫ້ປິດໂປຣແກຣມອື່ນທັງໝົດກ່ອນດຳເນີນການຕໍ່

; *** "Password" wizard page
WizardPassword=ລະຫັດຜ່ານ
PasswordLabel1=ການຕິດຕັ້ງນີ້ມີລະຫັດຜ່ານປ້ອງກັນ
PasswordLabel3=ກະລຸນາປ້ອນລະຫັດຜ່ານ ແລ້ວກົດ “ຖັດໄປ” ເພື່ອດຳເນີນການຕໍ່ ລະຫັດຜ່ານແຍກຕົວພິມນ້ອຍພິມໃຫຍ່
PasswordEditLabel=ລະຫັດຜ່ານ:
IncorrectPassword=ລະຫັດຜ່ານທີ່ທ່ານປ້ອນບໍ່ຖືກຕ້ອງ ກະລຸນາລອງໃໝ່

; *** "License Agreement" wizard page
WizardLicense=ຂໍ້ຕົກລົງໃບອະນຸຍາດ
LicenseLabel=ກະລຸນາອ່ານຂໍ້ມູນສຳຄັນຕໍ່ໄປນີ້ກ່ອນດຳເນີນການຕໍ່
LicenseLabel3=ກະລຸນາອ່ານຂໍ້ຕົກລົງໃບອະນຸຍາດຕໍ່ໄປນີ້ ທ່ານຕ້ອງຍອມຮັບເງື່ອນໄຂຂອງຂໍ້ຕົກລົງນີ້ກ່ອນຈຶ່ງຈະຕິດຕັ້ງຕໍ່ໄດ້
LicenseAccepted=ຂ້ອຍຍອມຮັບຂໍ້ຕົກລົງ
LicenseNotAccepted=ຂ້ອຍບໍ່ຍອມຮັບຂໍ້ຕົກລົງ

; *** "Information" wizard pages
WizardInfoBefore=ຂໍ້ມູນ
InfoBeforeLabel=ກະລຸນາອ່ານຂໍ້ມູນສຳຄັນຕໍ່ໄປນີ້ກ່ອນດຳເນີນການຕໍ່
InfoBeforeClickLabel=ເມື່ອພ້ອມທີ່ຈະຕິດຕັ້ງຕໍ່ ກົດ “ຖັດໄປ”
WizardInfoAfter=ຂໍ້ມູນ
InfoAfterLabel=ກະລຸນາອ່ານຂໍ້ມູນສຳຄັນຕໍ່ໄປນີ້ກ່ອນດຳເນີນການຕໍ່
InfoAfterClickLabel=ເມື່ອພ້ອມທີ່ຈະຕິດຕັ້ງຕໍ່ ກົດ “ຖັດໄປ”

; *** "User Information" wizard page
WizardUserInfo=ຂໍ້ມູນຜູ້ໃຊ້
UserInfoDesc=ກະລຸນາປ້ອນຂໍ້ມູນຂອງທ່ານ
UserInfoName=ຊື່ຜູ້ໃຊ້:
UserInfoOrg=ອົງກອນ:
UserInfoSerial=ໝາຍເລກລຳດັບ:
UserInfoNameRequired=ທ່ານຕ້ອງປ້ອນຊື່

; *** "Select Destination Location" wizard page
WizardSelectDir=ເລືອກບ່ອນຕິດຕັ້ງ
SelectDirDesc=ຈະຕິດຕັ້ງ [name] ໄວ້ບ່ອນໃດ?
SelectDirLabel3=ຕົວຕິດຕັ້ງຈະຕິດຕັ້ງ [name] ລົງໃນໂຟນເດີຕໍ່ໄປນີ້
SelectDirBrowseLabel=ເພື່ອດຳເນີນການຕໍ່ ກົດ “ຖັດໄປ” ຖ້າຕ້ອງການເລືອກໂຟນເດີອື່ນ ກົດ “ຄົ້ນຫາ”
DiskSpaceGBLabel=ຕ້ອງການພື້ນທີ່ວ່າງໃນດິດຢ່າງໜ້ອຍ [gb] GB
DiskSpaceMBLabel=ຕ້ອງການພື້ນທີ່ວ່າງໃນດິດຢ່າງໜ້ອຍ [mb] MB
CannotInstallToNetworkDrive=ຕົວຕິດຕັ້ງບໍ່ສາມາດຕິດຕັ້ງລົງໃນໄດຣຟ໌ເຄືອຂ່າຍໄດ້
CannotInstallToUNCPath=ຕົວຕິດຕັ້ງບໍ່ສາມາດຕິດຕັ້ງລົງໃນເສັ້ນທາງ UNC ໄດ້
InvalidPath=ທ່ານຕ້ອງປ້ອນເສັ້ນທາງເຕັມພ້ອມຕົວອັກສອນໄດຣຟ໌ ຕົວຢ່າງ:%n%nC:\APP%n%nຫຼືເສັ້ນທາງ UNC ໃນຮູບແບບ:%n%n\\server\share
InvalidDrive=ໄດຣຟ໌ ຫຼື UNC share ທີ່ທ່ານເລືອກບໍ່ມີ ຫຼືເຂົ້າເຖິງບໍ່ໄດ້ ກະລຸນາເລືອກອັນອື່ນ
DiskSpaceWarningTitle=ພື້ນທີ່ໃນດິດບໍ່ພໍ
DiskSpaceWarning=ການຕິດຕັ້ງຕ້ອງການພື້ນທີ່ວ່າງຢ່າງໜ້ອຍ %1 KB ແຕ່ໄດຣຟ໌ທີ່ເລືອກມີພຽງ %2 KB%n%nທ່ານຕ້ອງການດຳເນີນການຕໍ່ຢູ່ບໍ?
DirNameTooLong=ຊື່ໂຟນເດີ ຫຼືເສັ້ນທາງຍາວເກີນໄປ
InvalidDirName=ຊື່ໂຟນເດີບໍ່ຖືກຕ້ອງ
BadDirName32=ຊື່ໂຟນເດີຫ້າມມີຕົວອັກສອນຕໍ່ໄປນີ້:%n%n%1
DirExistsTitle=ມີໂຟນເດີແລ້ວ
DirExists=ໂຟນເດີ:%n%n%1%n%nມີຢູ່ແລ້ວ ທ່ານຕ້ອງການຕິດຕັ້ງລົງໃນໂຟນເດີນັ້ນຢູ່ບໍ?
DirDoesntExistTitle=ບໍ່ມີໂຟນເດີ
DirDoesntExist=ໂຟນເດີ:%n%n%1%n%nບໍ່ມີ ທ່ານຕ້ອງການໃຫ້ສ້າງໂຟນເດີນີ້ບໍ?

; *** "Select Components" wizard page
WizardSelectComponents=ເລືອກອົງປະກອບ
SelectComponentsDesc=ຈະຕິດຕັ້ງອົງປະກອບໃດແດ່?
SelectComponentsLabel2=ເລືອກອົງປະກອບທີ່ຕ້ອງການຕິດຕັ້ງ ແລະເອົາເຄື່ອງໝາຍອອກຈາກອົງປະກອບທີ່ບໍ່ຕ້ອງການ ກົດ “ຖັດໄປ” ເມື່ອພ້ອມດຳເນີນການຕໍ່
FullInstallation=ຕິດຕັ້ງຄົບຖ້ວນ
; if possible don't translate 'Compact' as 'Minimal' (I mean 'Minimal' in your language)
CompactInstallation=ຕິດຕັ້ງແບບກະທັດຮັດ
CustomInstallation=ຕິດຕັ້ງແບບກຳນົດເອງ
NoUninstallWarningTitle=ມີອົງປະກອບຢູ່ແລ້ວ
NoUninstallWarning=ຕົວຕິດຕັ້ງກວດພົບວ່າອົງປະກອບຕໍ່ໄປນີ້ຖືກຕິດຕັ້ງໃນຄອມພິວເຕີຂອງທ່ານແລ້ວ:%n%n%1%n%nການເອົາເຄື່ອງໝາຍອອກຈາກອົງປະກອບເຫຼົ່ານີ້ຈະບໍ່ຖອນການຕິດຕັ້ງມັນ%n%nທ່ານຕ້ອງການດຳເນີນການຕໍ່ຢູ່ບໍ?
ComponentSize1=%1 KB
ComponentSize2=%1 MB
ComponentsDiskSpaceGBLabel=ການເລືອກປັດຈຸບັນຕ້ອງການພື້ນທີ່ໃນດິດຢ່າງໜ້ອຍ [gb] GB
ComponentsDiskSpaceMBLabel=ການເລືອກປັດຈຸບັນຕ້ອງການພື້ນທີ່ໃນດິດຢ່າງໜ້ອຍ [mb] MB

; *** "Select Additional Tasks" wizard page
WizardSelectTasks=ເລືອກວຽກເພີ່ມເຕີມ
SelectTasksDesc=ຈະເຮັດວຽກເພີ່ມເຕີມໃດແດ່?
SelectTasksLabel2=ເລືອກວຽກເພີ່ມເຕີມທີ່ທ່ານຕ້ອງການໃຫ້ຕົວຕິດຕັ້ງເຮັດໃນຂະນະຕິດຕັ້ງ [name] ແລ້ວກົດ “ຖັດໄປ”

; *** "Select Start Menu Folder" wizard page
WizardSelectProgramGroup=ເລືອກໂຟນເດີໃນເມນູ Start
SelectStartMenuFolderDesc=ຈະໃຫ້ຕົວຕິດຕັ້ງວາງທາງລັດຂອງໂປຣແກຣມໄວ້ບ່ອນໃດ?
SelectStartMenuFolderLabel3=ຕົວຕິດຕັ້ງຈະສ້າງທາງລັດຂອງໂປຣແກຣມໃນໂຟນເດີເມນູ Start ຕໍ່ໄປນີ້
SelectStartMenuFolderBrowseLabel=ເພື່ອດຳເນີນການຕໍ່ ກົດ “ຖັດໄປ” ຖ້າຕ້ອງການເລືອກໂຟນເດີອື່ນ ກົດ “ຄົ້ນຫາ”
MustEnterGroupName=ທ່ານຕ້ອງປ້ອນຊື່ໂຟນເດີ
GroupNameTooLong=ຊື່ໂຟນເດີ ຫຼືເສັ້ນທາງຍາວເກີນໄປ
InvalidGroupName=ຊື່ໂຟນເດີບໍ່ຖືກຕ້ອງ
BadGroupName=ຊື່ໂຟນເດີຫ້າມມີຕົວອັກສອນຕໍ່ໄປນີ້:%n%n%1
NoProgramGroupCheck2=ບໍ່ສ້າງໂຟນເດີໃນເມນູ Start

; *** "Ready to Install" wizard page
WizardReady=ພ້ອມຕິດຕັ້ງ
ReadyLabel1=ຕົວຕິດຕັ້ງພ້ອມທີ່ຈະເລີ່ມຕິດຕັ້ງ [name] ໃນຄອມພິວເຕີຂອງທ່ານແລ້ວ
ReadyLabel2a=ກົດ “ຕິດຕັ້ງ” ເພື່ອດຳເນີນການຕິດຕັ້ງຕໍ່ ຫຼື “ກັບຄືນ” ຖ້າຕ້ອງການກວດ ຫຼືປ່ຽນການຕັ້ງຄ່າ
ReadyLabel2b=ກົດ “ຕິດຕັ້ງ” ເພື່ອດຳເນີນການຕິດຕັ້ງຕໍ່
ReadyMemoUserInfo=ຂໍ້ມູນຜູ້ໃຊ້:
ReadyMemoDir=ບ່ອນຕິດຕັ້ງ:
ReadyMemoType=ປະເພດການຕິດຕັ້ງ:
ReadyMemoComponents=ອົງປະກອບທີ່ເລືອກ:
ReadyMemoGroup=ໂຟນເດີໃນເມນູ Start:
ReadyMemoTasks=ວຽກເພີ່ມເຕີມ:

; *** TDownloadWizardPage wizard page and DownloadTemporaryFile
DownloadingLabel2=ກຳລັງດາວໂຫຼດໄຟລ໌...
ButtonStopDownload=ຢຸດການດາວໂຫຼດ
StopDownload=ທ່ານແນ່ໃຈບໍວ່າຕ້ອງການຢຸດການດາວໂຫຼດ?
ErrorDownloadAborted=ການດາວໂຫຼດຖືກຢຸດ
ErrorDownloadFailed=ດາວໂຫຼດບໍ່ສຳເລັດ: %1 %2
ErrorDownloadSizeFailed=ບໍ່ສາມາດຮັບຂະໜາດໄດ້: %1 %2
ErrorProgress=ຄວາມຄືບໜ້າບໍ່ຖືກຕ້ອງ: %1 ຈາກ %2
ErrorFileSize=ຂະໜາດໄຟລ໌ບໍ່ຖືກຕ້ອງ: ຄາດໄວ້ %1 ພົບ %2

; *** TExtractionWizardPage wizard page and ExtractArchive
ExtractingLabel=ກຳລັງແຍກໄຟລ໌...
ButtonStopExtraction=ຢຸດການແຍກ
StopExtraction=ທ່ານແນ່ໃຈບໍວ່າຕ້ອງການຢຸດການແຍກໄຟລ໌?
ErrorExtractionAborted=ການແຍກໄຟລ໌ຖືກຢຸດ
ErrorExtractionFailed=ແຍກໄຟລ໌ບໍ່ສຳເລັດ: %1

; *** Archive extraction failure details
ArchiveIncorrectPassword=ລະຫັດຜ່ານບໍ່ຖືກຕ້ອງ
ArchiveIsCorrupted=ໄຟລ໌ບີບອັດເສຍຫາຍ
ArchiveUnsupportedFormat=ບໍ່ຮອງຮັບຮູບແບບຂອງໄຟລ໌ບີບອັດ

; *** "Preparing to Install" wizard page
WizardPreparing=ກຳລັງກຽມຕິດຕັ້ງ
PreparingDesc=ຕົວຕິດຕັ້ງກຳລັງກຽມຕິດຕັ້ງ [name] ໃນຄອມພິວເຕີຂອງທ່ານ
PreviousInstallNotCompleted=ການຕິດຕັ້ງ/ຖອນການຕິດຕັ້ງໂປຣແກຣມກ່ອນໜ້າຍັງບໍ່ສຳເລັດ ທ່ານຕ້ອງເລີ່ມຄອມພິວເຕີໃໝ່ເພື່ອໃຫ້ການຕິດຕັ້ງນັ້ນສຳເລັດ%n%nຫຼັງເລີ່ມຄອມພິວເຕີໃໝ່ ໃຫ້ເປີດຕົວຕິດຕັ້ງອີກຄັ້ງເພື່ອຕິດຕັ້ງ [name] ໃຫ້ສຳເລັດ
CannotContinue=ຕົວຕິດຕັ້ງດຳເນີນການຕໍ່ບໍ່ໄດ້ ກະລຸນາກົດ “ຍົກເລີກ” ເພື່ອອອກ
ApplicationsFound=ໂປຣແກຣມຕໍ່ໄປນີ້ກຳລັງໃຊ້ໄຟລ໌ທີ່ຕົວຕິດຕັ້ງຕ້ອງອັບເດດ ແນະນຳໃຫ້ອະນຸຍາດໃຫ້ຕົວຕິດຕັ້ງປິດໂປຣແກຣມເຫຼົ່ານີ້ອັດຕະໂນມັດ
ApplicationsFound2=ໂປຣແກຣມຕໍ່ໄປນີ້ກຳລັງໃຊ້ໄຟລ໌ທີ່ຕົວຕິດຕັ້ງຕ້ອງອັບເດດ ແນະນຳໃຫ້ອະນຸຍາດໃຫ້ຕົວຕິດຕັ້ງປິດໂປຣແກຣມເຫຼົ່ານີ້ອັດຕະໂນມັດ ເມື່ອຕິດຕັ້ງສຳເລັດ ຕົວຕິດຕັ້ງຈະພະຍາຍາມເປີດໂປຣແກຣມເຫຼົ່ານີ້ໃໝ່
CloseApplications=ປິດໂປຣແກຣມອັດຕະໂນມັດ
DontCloseApplications=ບໍ່ປິດໂປຣແກຣມ
ErrorCloseApplications=ຕົວຕິດຕັ້ງບໍ່ສາມາດປິດທຸກໂປຣແກຣມອັດຕະໂນມັດໄດ້ ແນະນຳໃຫ້ປິດທຸກໂປຣແກຣມທີ່ໃຊ້ໄຟລ໌ທີ່ຕົວຕິດຕັ້ງຕ້ອງອັບເດດ ກ່ອນດຳເນີນການຕໍ່
PrepareToInstallNeedsRestart=ຕົວຕິດຕັ້ງຕ້ອງເລີ່ມຄອມພິວເຕີຂອງທ່ານໃໝ່ ຫຼັງເລີ່ມຄອມພິວເຕີໃໝ່ ໃຫ້ເປີດຕົວຕິດຕັ້ງອີກຄັ້ງເພື່ອຕິດຕັ້ງ [name] ໃຫ້ສຳເລັດ%n%nທ່ານຕ້ອງການເລີ່ມໃໝ່ດຽວນີ້ບໍ?

; *** "Installing" wizard page
WizardInstalling=ກຳລັງຕິດຕັ້ງ
InstallingLabel=ກະລຸນາລໍຖ້າໃນຂະນະທີ່ຕົວຕິດຕັ້ງກຳລັງຕິດຕັ້ງ [name] ໃນຄອມພິວເຕີຂອງທ່ານ

; *** "Setup Completed" wizard page
FinishedHeadingLabel=ການຕິດຕັ້ງ [name] ສຳເລັດແລ້ວ
FinishedLabelNoIcons=ຕົວຕິດຕັ້ງຕິດຕັ້ງ [name] ໃນຄອມພິວເຕີຂອງທ່ານສຳເລັດແລ້ວ
FinishedLabel=ຕົວຕິດຕັ້ງຕິດຕັ້ງ [name] ໃນຄອມພິວເຕີຂອງທ່ານສຳເລັດແລ້ວ ທ່ານສາມາດເປີດໂປຣແກຣມໄດ້ຈາກທາງລັດທີ່ຕິດຕັ້ງໄວ້
ClickFinish=ກົດ “ສຳເລັດ” ເພື່ອອອກຈາກການຕິດຕັ້ງ
FinishedRestartLabel=ເພື່ອຕິດຕັ້ງ [name] ໃຫ້ສຳເລັດ ຕົວຕິດຕັ້ງຕ້ອງເລີ່ມຄອມພິວເຕີຂອງທ່ານໃໝ່ ທ່ານຕ້ອງການເລີ່ມໃໝ່ດຽວນີ້ບໍ?
FinishedRestartMessage=ເພື່ອຕິດຕັ້ງ [name] ໃຫ້ສຳເລັດ ຕົວຕິດຕັ້ງຕ້ອງເລີ່ມຄອມພິວເຕີຂອງທ່ານໃໝ່%n%nທ່ານຕ້ອງການເລີ່ມໃໝ່ດຽວນີ້ບໍ?
ShowReadmeCheck=ແມ່ນ, ຂ້ອຍຢາກເບິ່ງໄຟລ໌ README
YesRadio=ແມ່ນ, ເລີ່ມຄອມພິວເຕີໃໝ່ດຽວນີ້
NoRadio=ບໍ່, ຂ້ອຍຈະເລີ່ມຄອມພິວເຕີໃໝ່ພາຍຫຼັງ
; used for example as 'Run MyProg.exe'
RunEntryExec=ເປີດ %1
; used for example as 'View Readme.txt'
RunEntryShellExec=ເບິ່ງ %1

; *** "Setup Needs the Next Disk" stuff
ChangeDiskTitle=ຕົວຕິດຕັ້ງຕ້ອງການດິດແຜ່ນຕໍ່ໄປ
SelectDiskLabel2=ກະລຸນາໃສ່ດິດ %1 ແລ້ວກົດ “ຕົກລົງ”%n%nຖ້າໄຟລ໌ໃນດິດນີ້ຢູ່ໃນໂຟນເດີອື່ນທີ່ບໍ່ແມ່ນອັນທີ່ສະແດງຢູ່ຂ້າງລຸ່ມ ໃຫ້ປ້ອນເສັ້ນທາງທີ່ຖືກຕ້ອງ ຫຼືກົດ “ຄົ້ນຫາ”
PathLabel=ເສັ້ນທາງ:
FileNotInDir2=ບໍ່ພົບໄຟລ໌ "%1" ໃນ "%2" ກະລຸນາໃສ່ດິດທີ່ຖືກຕ້ອງ ຫຼືເລືອກໂຟນເດີອື່ນ
SelectDirectoryLabel=ກະລຸນາລະບຸບ່ອນທີ່ຢູ່ຂອງດິດແຜ່ນຕໍ່ໄປ

; *** Installation phase messages
SetupAborted=ການຕິດຕັ້ງບໍ່ສຳເລັດ%n%nກະລຸນາແກ້ໄຂບັນຫາ ແລ້ວເປີດຕົວຕິດຕັ້ງອີກຄັ້ງ
AbortRetryIgnoreSelectAction=ເລືອກການດຳເນີນການ
AbortRetryIgnoreRetry=ລອງໃໝ່
AbortRetryIgnoreIgnore=ຂ້າມຂໍ້ຜິດພາດແລ້ວດຳເນີນການຕໍ່
AbortRetryIgnoreCancel=ຍົກເລີກການຕິດຕັ້ງ
RetryCancelSelectAction=ເລືອກການດຳເນີນການ
RetryCancelRetry=ລອງໃໝ່
RetryCancelCancel=ຍົກເລີກ

; *** Installation status messages
StatusClosingApplications=ກຳລັງປິດໂປຣແກຣມ...
StatusCreateDirs=ກຳລັງສ້າງໂຟນເດີ...
StatusExtractFiles=ກຳລັງແຍກໄຟລ໌...
StatusDownloadFiles=ກຳລັງດາວໂຫຼດໄຟລ໌...
StatusCreateIcons=ກຳລັງສ້າງທາງລັດ...
StatusCreateIniEntries=ກຳລັງສ້າງລາຍການ INI...
StatusCreateRegistryEntries=ກຳລັງສ້າງລາຍການຣີຈິສຕຣີ...
StatusRegisterFiles=ກຳລັງລົງທະບຽນໄຟລ໌...
StatusSavingUninstall=ກຳລັງບັນທຶກຂໍ້ມູນການຖອນການຕິດຕັ້ງ...
StatusRunProgram=ກຳລັງສຳເລັດການຕິດຕັ້ງ...
StatusRestartingApplications=ກຳລັງເປີດໂປຣແກຣມໃໝ່...
StatusRollback=ກຳລັງຍົກເລີກການປ່ຽນແປງ...

; *** Misc. errors
ErrorInternal2=ຂໍ້ຜິດພາດພາຍໃນ: %1
ErrorFunctionFailedNoCode=%1 ບໍ່ສຳເລັດ
ErrorFunctionFailed=%1 ບໍ່ສຳເລັດ; ລະຫັດ %2
ErrorFunctionFailedWithMessage=%1 ບໍ່ສຳເລັດ; ລະຫັດ %2.%n%3
ErrorExecutingProgram=ບໍ່ສາມາດເປີດໃຊ້ໄຟລ໌ໄດ້:%n%1

; *** Registry errors
ErrorRegOpenKey=ຂໍ້ຜິດພາດໃນການເປີດຄີຣີຈິສຕຣີ:%n%1\%2
ErrorRegCreateKey=ຂໍ້ຜິດພາດໃນການສ້າງຄີຣີຈິສຕຣີ:%n%1\%2
ErrorRegWriteKey=ຂໍ້ຜິດພາດໃນການຂຽນຄີຣີຈິສຕຣີ:%n%1\%2

; *** INI errors
ErrorIniEntry=ຂໍ້ຜິດພາດໃນການສ້າງລາຍການ INI ໃນໄຟລ໌ "%1"

; *** File copying errors
FileAbortRetryIgnoreSkipNotRecommended=ຂ້າມໄຟລ໌ນີ້ (ບໍ່ແນະນຳ)
FileAbortRetryIgnoreIgnoreNotRecommended=ຂ້າມຂໍ້ຜິດພາດແລ້ວດຳເນີນການຕໍ່ (ບໍ່ແນະນຳ)
SourceIsCorrupted=ໄຟລ໌ຕົ້ນທາງເສຍຫາຍ
SourceDoesntExist=ບໍ່ມີໄຟລ໌ຕົ້ນທາງ "%1"
SourceVerificationFailed=ການກວດສອບໄຟລ໌ຕົ້ນທາງບໍ່ສຳເລັດ: %1
VerificationSignatureDoesntExist=ບໍ່ມີໄຟລ໌ລາຍເຊັນ "%1"
VerificationSignatureInvalid=ໄຟລ໌ລາຍເຊັນ "%1" ບໍ່ຖືກຕ້ອງ
VerificationKeyNotFound=ໄຟລ໌ລາຍເຊັນ "%1" ໃຊ້ຄີທີ່ບໍ່ຮູ້ຈັກ
VerificationFileNameIncorrect=ຊື່ໄຟລ໌ບໍ່ຖືກຕ້ອງ
VerificationFileTagIncorrect=ແທັກຂອງໄຟລ໌ບໍ່ຖືກຕ້ອງ
VerificationFileSizeIncorrect=ຂະໜາດຂອງໄຟລ໌ບໍ່ຖືກຕ້ອງ
VerificationFileHashIncorrect=ຄ່າແຮດຂອງໄຟລ໌ບໍ່ຖືກຕ້ອງ
ExistingFileReadOnly2=ບໍ່ສາມາດປ່ຽນແທນໄຟລ໌ທີ່ມີຢູ່ໄດ້ ເພາະຖືກໝາຍເປັນອ່ານໄດ້ຢ່າງດຽວ
ExistingFileReadOnlyRetry=ເອົາຄຸນສົມບັດອ່ານໄດ້ຢ່າງດຽວອອກແລ້ວລອງໃໝ່
ExistingFileReadOnlyKeepExisting=ເກັບໄຟລ໌ທີ່ມີຢູ່ໄວ້
ErrorReadingExistingDest=ເກີດຂໍ້ຜິດພາດໃນຂະນະອ່ານໄຟລ໌ທີ່ມີຢູ່:
FileExistsSelectAction=ເລືອກການດຳເນີນການ
FileExists2=ມີໄຟລ໌ນີ້ຢູ່ແລ້ວ
FileExistsOverwriteExisting=ຂຽນທັບໄຟລ໌ທີ່ມີຢູ່
FileExistsKeepExisting=ເກັບໄຟລ໌ທີ່ມີຢູ່ໄວ້
FileExistsOverwriteOrKeepAll=ເຮັດແບບນີ້ກັບຄວາມຂັດແຍ້ງຕໍ່ໄປ
ExistingFileNewerSelectAction=ເລືອກການດຳເນີນການ
ExistingFileNewer2=ໄຟລ໌ທີ່ມີຢູ່ໃໝ່ກວ່າໄຟລ໌ທີ່ຕົວຕິດຕັ້ງກຳລັງຈະຕິດຕັ້ງ
ExistingFileNewerOverwriteExisting=ຂຽນທັບໄຟລ໌ທີ່ມີຢູ່
ExistingFileNewerKeepExisting=ເກັບໄຟລ໌ທີ່ມີຢູ່ໄວ້ (ແນະນຳ)
ExistingFileNewerOverwriteOrKeepAll=ເຮັດແບບນີ້ກັບຄວາມຂັດແຍ້ງຕໍ່ໄປ
ErrorChangingAttr=ເກີດຂໍ້ຜິດພາດໃນຂະນະປ່ຽນຄຸນສົມບັດຂອງໄຟລ໌ທີ່ມີຢູ່:
ErrorCreatingTemp=ເກີດຂໍ້ຜິດພາດໃນຂະນະສ້າງໄຟລ໌ໃນໂຟນເດີປາຍທາງ:
ErrorReadingSource=ເກີດຂໍ້ຜິດພາດໃນຂະນະອ່ານໄຟລ໌ຕົ້ນທາງ:
ErrorCopying=ເກີດຂໍ້ຜິດພາດໃນຂະນະສຳເນົາໄຟລ໌:
ErrorDownloading=ເກີດຂໍ້ຜິດພາດໃນຂະນະດາວໂຫຼດໄຟລ໌:
ErrorExtracting=ເກີດຂໍ້ຜິດພາດໃນຂະນະແຍກໄຟລ໌ບີບອັດ:
ErrorReplacingExistingFile=ເກີດຂໍ້ຜິດພາດໃນຂະນະປ່ຽນແທນໄຟລ໌ທີ່ມີຢູ່:
ErrorRestartReplace=RestartReplace ບໍ່ສຳເລັດ:
ErrorRenamingTemp=ເກີດຂໍ້ຜິດພາດໃນຂະນະປ່ຽນຊື່ໄຟລ໌ໃນໂຟນເດີປາຍທາງ:
ErrorRegisterServer=ບໍ່ສາມາດລົງທະບຽນ DLL/OCX ໄດ້: %1
ErrorRegSvr32Failed=RegSvr32 ບໍ່ສຳເລັດດ້ວຍລະຫັດອອກ %1
ErrorRegisterTypeLib=ບໍ່ສາມາດລົງທະບຽນ type library ໄດ້: %1

; *** Uninstall display name markings
; used for example as 'My Program (32-bit)'
UninstallDisplayNameMark=%1 (%2)
; used for example as 'My Program (32-bit, All users)'
UninstallDisplayNameMarks=%1 (%2, %3)
UninstallDisplayNameMark32Bit=32-bit
UninstallDisplayNameMark64Bit=64-bit
UninstallDisplayNameMarkAllUsers=ຜູ້ໃຊ້ທຸກຄົນ
UninstallDisplayNameMarkCurrentUser=ຜູ້ໃຊ້ປັດຈຸບັນ

; *** Post-installation errors
ErrorOpeningReadme=ເກີດຂໍ້ຜິດພາດໃນຂະນະເປີດໄຟລ໌ README
ErrorRestartingComputer=ຕົວຕິດຕັ້ງບໍ່ສາມາດເລີ່ມຄອມພິວເຕີໃໝ່ໄດ້ ກະລຸນາເລີ່ມໃໝ່ດ້ວຍຕົນເອງ

; *** Uninstaller messages
UninstallNotFound=ບໍ່ມີໄຟລ໌ "%1" ຖອນການຕິດຕັ້ງບໍ່ໄດ້
UninstallOpenError=ເປີດໄຟລ໌ "%1" ບໍ່ໄດ້ ຖອນການຕິດຕັ້ງບໍ່ໄດ້
UninstallUnsupportedVer=ໄຟລ໌ບັນທຶກການຖອນການຕິດຕັ້ງ "%1" ຢູ່ໃນຮູບແບບທີ່ຕົວຖອນການຕິດຕັ້ງເວີຊັນນີ້ບໍ່ຮູ້ຈັກ ຖອນການຕິດຕັ້ງບໍ່ໄດ້
UninstallUnknownEntry=ພົບລາຍການທີ່ບໍ່ຮູ້ຈັກ (%1) ໃນບັນທຶກການຖອນການຕິດຕັ້ງ
ConfirmUninstall=ທ່ານແນ່ໃຈບໍວ່າຕ້ອງການລຶບ %1 ແລະທຸກອົງປະກອບຂອງມັນອອກທັງໝົດ?
UninstallOnlyOnWin64=ການຕິດຕັ້ງນີ້ຖອນໄດ້ສະເພາະໃນ Windows 64-bit
OnlyAdminCanUninstall=ການຕິດຕັ້ງນີ້ຖອນໄດ້ສະເພາະຜູ້ໃຊ້ທີ່ມີສິດຜູ້ດູແລລະບົບ
UninstallStatusLabel=ກະລຸນາລໍຖ້າໃນຂະນະທີ່ %1 ກຳລັງຖືກລຶບອອກຈາກຄອມພິວເຕີຂອງທ່ານ
UninstalledAll=ລຶບ %1 ອອກຈາກຄອມພິວເຕີຂອງທ່ານສຳເລັດແລ້ວ
UninstalledMost=ຖອນການຕິດຕັ້ງ %1 ສຳເລັດ%n%nບາງລາຍການລຶບບໍ່ໄດ້ ທ່ານສາມາດລຶບມັນດ້ວຍຕົນເອງ
UninstalledAndNeedsRestart=ເພື່ອຖອນການຕິດຕັ້ງ %1 ໃຫ້ສຳເລັດ ຕ້ອງເລີ່ມຄອມພິວເຕີຂອງທ່ານໃໝ່%n%nທ່ານຕ້ອງການເລີ່ມໃໝ່ດຽວນີ້ບໍ?
UninstallDataCorrupted=ໄຟລ໌ "%1" ເສຍຫາຍ ຖອນການຕິດຕັ້ງບໍ່ໄດ້

; *** Uninstallation phase messages
ConfirmDeleteSharedFileTitle=ລຶບໄຟລ໌ທີ່ໃຊ້ຮ່ວມກັນບໍ?
ConfirmDeleteSharedFile2=ລະບົບສະແດງວ່າໄຟລ໌ທີ່ໃຊ້ຮ່ວມກັນຕໍ່ໄປນີ້ບໍ່ມີໂປຣແກຣມໃດໃຊ້ແລ້ວ ທ່ານຕ້ອງການໃຫ້ຕົວຖອນການຕິດຕັ້ງລຶບໄຟລ໌ນີ້ບໍ?%n%nຖ້າຍັງມີໂປຣແກຣມໃຊ້ໄຟລ໌ນີ້ຢູ່ແລ້ວມັນຖືກລຶບ ໂປຣແກຣມເຫຼົ່ານັ້ນອາດເຮັດວຽກຜິດປົກກະຕິ ຖ້າບໍ່ແນ່ໃຈ ໃຫ້ເລືອກ “ບໍ່” ການເກັບໄຟລ໌ໄວ້ໃນລະບົບບໍ່ກໍ່ໃຫ້ເກີດອັນຕະລາຍ
SharedFileNameLabel=ຊື່ໄຟລ໌:
SharedFileLocationLabel=ບ່ອນຕັ້ງ:
WizardUninstalling=ສະຖານະການຖອນການຕິດຕັ້ງ
StatusUninstalling=ກຳລັງຖອນການຕິດຕັ້ງ %1...

; *** Shutdown block reasons
ShutdownBlockReasonInstallingApp=ກຳລັງຕິດຕັ້ງ %1
ShutdownBlockReasonUninstallingApp=ກຳລັງຖອນການຕິດຕັ້ງ %1

; The custom messages below aren't used by Setup itself, but if you make
; use of them in your scripts, you'll want to translate them.

[CustomMessages]

NameAndVersion=%1 ເວີຊັນ %2
AdditionalIcons=ທາງລັດເພີ່ມເຕີມ:
CreateDesktopIcon=ສ້າງທາງລັດເທິງເດັສທັອບ
CreateQuickLaunchIcon=ສ້າງທາງລັດ Quick Launch
ProgramOnTheWeb=%1 ເທິງເວັບ
UninstallProgram=ຖອນການຕິດຕັ້ງ %1
LaunchProgram=ເປີດ %1
AssocFileExtension=ເຊື່ອມ %1 ກັບນາມສະກຸນໄຟລ໌ %2
AssocingFileExtension=ກຳລັງເຊື່ອມ %1 ກັບນາມສະກຸນໄຟລ໌ %2...
AutoStartProgramGroupDescription=ເລີ່ມອັດຕະໂນມັດ:
AutoStartProgram=ເລີ່ມ %1 ອັດຕະໂນມັດ
AddonHostProgramNotFound=ບໍ່ພົບ %1 ໃນໂຟນເດີທີ່ທ່ານເລືອກ%n%nທ່ານຕ້ອງການດຳເນີນການຕໍ່ຢູ່ບໍ?

; The messages that are Lang Switcher's own (the sample is English.isl).
LocalCertReady=ຕົວຕິດຕັ້ງຈະສ້າງໃບຮັບຮອງໃນຄອມພິວເຕີເຄື່ອງນີ້ ແລະລົງລາຍເຊັນ Lang Switcher ດ້ວຍໃບຮັບຮອງນັ້ນ ເພື່ອໃຫ້ເຮັດວຽກໄດ້ໃນໜ້າຕ່າງຂອງໂປຣແກຣມທີ່ເປີດໃຊ້ໃນຖານະຜູ້ດູແລລະບົບ
LocalCertFailed=ຕິດຕັ້ງ Lang Switcher ແລ້ວ ແຕ່ຈະບໍ່ເຮັດວຽກໃນໜ້າຕ່າງຂອງໂປຣແກຣມທີ່ເປີດໃຊ້ໃນຖານະຜູ້ດູແລລະບົບ: Windows ບໍ່ຍອມຮັບໃບຮັບຮອງທີ່ສ້າງໃນຄອມພິວເຕີເຄື່ອງນີ້ ເບິ່ງລາຍລະອຽດໃນບັນທຶກການຕິດຕັ້ງ
RemoveSettingsQuestion=ລຶບໄຟລ໌ການຕັ້ງຄ່າຂອງ Lang Switcher ນຳບໍ?
RemoveSettingsKeep=ເລືອກ “ບໍ່” ເພື່ອເກັບການຕັ້ງຄ່າໄວ້ສຳລັບການຕິດຕັ້ງໃໝ່ໃນພາຍຫຼັງ
AuthorSlogan=ໂປຣແກຣມທີ່ສ້າງດ້ວຍຫົວໃຈ
