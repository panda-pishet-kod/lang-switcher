; Lang Switcher -- the messages of the installer in Maltese (stage E109, group B).
;
; Inno Setup has no official Maltese translation (inno_official = false in res\lang\languages.toml),
; so this file carries the whole of it: [LangOptions], every message of the compiler:Default.isl
; (6.5.0+), and the messages of the installer that are Lang Switcher's own. It is read after
; compiler:Default.isl as the second MessagesFile of the "maltese" entry of [Languages] in
; ..\LangSwitcher.iss, and so replaces the English text name for name.
; English.isl is the sample the own messages translate; the glossary of the language is
; l10n\glossary\mt.tsv of the control contour. The name of the program is not translated.
; No accelerator "&" is set (as in Tamil.isl): the window must not show two equal letters.
; The names of the buttons inside a message stand in the quotes of the language.
; As in Default.isl: a message that has no full stop there has none here (Setup adds its own).

[LangOptions]
LanguageName=Malti
LanguageID=$043A
LanguageCodePage=0

[Messages]

; *** Application titles
SetupAppTitle=Installazzjoni
SetupWindowTitle=Installazzjoni - %1
UninstallAppTitle=Diżinstallazzjoni
UninstallAppFullTitle=Diżinstallazzjoni ta' %1

; *** Misc. common
InformationTitle=Informazzjoni
ConfirmTitle=Konferma
ErrorTitle=Żball

; *** SetupLdr messages
SetupLdrStartupMessage=Dan se jinstalla %1. Tixtieq tkompli?
LdrCannotCreateTemp=Ma setax jinħoloq fajl temporanju. L-installazzjoni twaqqfet
LdrCannotExecTemp=Ma setax jitħaddem fajl fil-folder temporanju. L-installazzjoni twaqqfet
HelpTextNote=

; *** Startup error messages
LastErrorMessage=%1.%n%nŻball %2: %3
SetupFileMissing=Il-fajl %1 nieqes mill-folder tal-installazzjoni. Jekk jogħġbok sewwi l-problema jew ikseb kopja ġdida tal-programm.
SetupFileCorrupt=Il-fajls tal-installazzjoni huma bil-ħsara. Jekk jogħġbok ikseb kopja ġdida tal-programm.
SetupFileCorruptOrWrongVer=Il-fajls tal-installazzjoni huma bil-ħsara jew mhumiex kompatibbli ma' din il-verżjoni tal-installatur. Jekk jogħġbok sewwi l-problema jew ikseb kopja ġdida tal-programm.
InvalidParameter=Ġie mgħoddi parametru invalidu fil-linja tal-kmand:%n%n%1
SetupAlreadyRunning=L-installatur diġà qed jaħdem.
WindowsVersionNotSupported=Dan il-programm ma jappoġġjax il-verżjoni tal-Windows li qed taħdem fuq il-kompjuter tiegħek.
WindowsServicePackRequired=Dan il-programm jeħtieġ %1 Service Pack %2 jew aktar ġdid.
NotOnThisPlatform=Dan il-programm mhux se jaħdem fuq %1.
OnlyOnThisPlatform=Dan il-programm irid jitħaddem fuq %1.
OnlyOnTheseArchitectures=Dan il-programm jista' jiġi installat biss fuq verżjonijiet tal-Windows magħmula għall-arkitetturi tal-proċessur li ġejjin:%n%n%1
WinVersionTooLowError=Dan il-programm jeħtieġ %1 verżjoni %2 jew aktar ġdida.
WinVersionTooHighError=Dan il-programm ma jistax jiġi installat fuq %1 verżjoni %2 jew aktar ġdida.
AdminPrivilegesRequired=Meta tinstalla dan il-programm trid tkun dħalt bħala amministratur.
PowerUserPrivilegesRequired=Meta tinstalla dan il-programm trid tkun dħalt bħala amministratur jew bħala membru tal-grupp Power Users.
SetupAppRunningError=L-installatur induna li %1 qed jaħdem bħalissa.%n%nAgħlaq l-istanzi kollha tiegħu issa, imbagħad agħfas “OK” biex tkompli jew “Ikkanċella” biex toħroġ.
UninstallAppRunningError=Id-diżinstallatur induna li %1 qed jaħdem bħalissa.%n%nAgħlaq l-istanzi kollha tiegħu issa, imbagħad agħfas “OK” biex tkompli jew “Ikkanċella” biex toħroġ.

; *** Startup questions
PrivilegesRequiredOverrideTitle=Agħżel il-modalità tal-installazzjoni
PrivilegesRequiredOverrideInstruction=Agħżel il-modalità tal-installazzjoni
PrivilegesRequiredOverrideText1=%1 jista' jiġi installat għall-utenti kollha (jeħtieġ drittijiet ta' amministratur) jew għalik biss.
PrivilegesRequiredOverrideText2=%1 jista' jiġi installat għalik biss jew għall-utenti kollha (jeħtieġ drittijiet ta' amministratur).
PrivilegesRequiredOverrideAllUsers=Installa għall-utenti kollha
PrivilegesRequiredOverrideAllUsersRecommended=Installa għall-utenti kollha (rakkomandat)
PrivilegesRequiredOverrideCurrentUser=Installa għalija biss
PrivilegesRequiredOverrideCurrentUserRecommended=Installa għalija biss (rakkomandat)

; *** Misc. errors
ErrorCreatingDir=L-installatur ma setax joħloq il-folder "%1"
ErrorTooManyFilesInDir=Ma setax jinħoloq fajl fil-folder "%1" għax fih wisq fajls

; *** Setup common messages
ExitSetupTitle=Oħroġ mill-installazzjoni
ExitSetupMessage=L-installazzjoni ma tlestitx. Jekk toħroġ issa, il-programm mhux se jiġi installat.%n%nTista' terġa' tħaddem l-installatur f'ħin ieħor biex tlesti l-installazzjoni.%n%nToħroġ mill-installazzjoni?
AboutSetupMenuItem=Dwar l-installatur...
AboutSetupTitle=Dwar l-installatur
AboutSetupMessage=%1 verżjoni %2%n%3%n%nIl-paġna ewlenija ta' %1:%n%4
AboutSetupNote=
TranslatorNote=

; *** Buttons
ButtonBack=< Lura
ButtonNext=Li jmiss >
ButtonInstall=Installa
ButtonOK=OK
ButtonCancel=Ikkanċella
ButtonYes=Iva
ButtonYesToAll=Iva għal kollox
ButtonNo=Le
ButtonNoToAll=Le għal kollox
ButtonFinish=Spiċċa
ButtonBrowse=Fittex...
ButtonWizardBrowse=Fittex...
ButtonNewFolder=Oħloq folder ġdid

; *** "Select Language" dialog messages
SelectLanguageTitle=Agħżel il-lingwa tal-installazzjoni
SelectLanguageLabel=Agħżel il-lingwa li se tintuża waqt l-installazzjoni.

; *** Common wizard text
ClickNext=Agħfas “Li jmiss” biex tkompli jew “Ikkanċella” biex toħroġ mill-installazzjoni.
BeveledLabel=
BrowseDialogTitle=Fittex folder
BrowseDialogLabel=Agħżel folder mil-lista ta' hawn taħt, imbagħad agħfas “OK”.
NewFolderName=Folder ġdid

; *** "Welcome" wizard page
WelcomeLabel1=Merħba fil-wizard tal-installazzjoni ta' [name]
WelcomeLabel2=Dan se jinstalla [name/ver] fuq il-kompjuter tiegħek.%n%nHuwa rakkomandat li tagħlaq l-applikazzjonijiet l-oħra kollha qabel ma tkompli.

; *** "Password" wizard page
WizardPassword=Password
PasswordLabel1=Din l-installazzjoni hija protetta bil-password.
PasswordLabel3=Ipprovdi l-password, imbagħad agħfas “Li jmiss” biex tkompli. Il-passwords huma sensittivi għall-ittri kbar u żgħar.
PasswordEditLabel=Password:
IncorrectPassword=Il-password li daħħalt mhix korretta. Erġa' pprova.

; *** "License Agreement" wizard page
WizardLicense=Ftehim tal-liċenzja
LicenseLabel=Jekk jogħġbok aqra l-informazzjoni importanti li ġejja qabel ma tkompli.
LicenseLabel3=Jekk jogħġbok aqra l-Ftehim tal-Liċenzja li ġej. Trid taċċetta t-termini ta' dan il-ftehim qabel ma tkompli bl-installazzjoni.
LicenseAccepted=Naċċetta l-ftehim
LicenseNotAccepted=Ma naċċettax il-ftehim

; *** "Information" wizard pages
WizardInfoBefore=Informazzjoni
InfoBeforeLabel=Jekk jogħġbok aqra l-informazzjoni importanti li ġejja qabel ma tkompli.
InfoBeforeClickLabel=Biex tkompli bl-installazzjoni, agħfas “Li jmiss”.
WizardInfoAfter=Informazzjoni
InfoAfterLabel=Jekk jogħġbok aqra l-informazzjoni importanti li ġejja qabel ma tkompli.
InfoAfterClickLabel=Biex tkompli bl-installazzjoni, agħfas “Li jmiss”.

; *** "User Information" wizard page
WizardUserInfo=Informazzjoni dwar l-utent
UserInfoDesc=Daħħal l-informazzjoni tiegħek.
UserInfoName=Isem l-utent:
UserInfoOrg=Organizzazzjoni:
UserInfoSerial=Numru tas-serje:
UserInfoNameRequired=Trid iddaħħal isem.

; *** "Select Destination Location" wizard page
WizardSelectDir=Agħżel il-post tad-destinazzjoni
SelectDirDesc=Fejn għandu jiġi installat [name]?
SelectDirLabel3=L-installatur se jinstalla [name] fil-folder li ġej.
SelectDirBrowseLabel=Biex tkompli, agħfas “Li jmiss”. Jekk tixtieq tagħżel folder differenti, agħfas “Fittex”.
DiskSpaceGBLabel=Huma meħtieġa mill-inqas [gb] GB ta' spazju ħieles fuq id-disk.
DiskSpaceMBLabel=Huma meħtieġa mill-inqas [mb] MB ta' spazju ħieles fuq id-disk.
CannotInstallToNetworkDrive=L-installatur ma jistax jinstalla fuq drive tan-netwerk.
CannotInstallToUNCPath=L-installatur ma jistax jinstalla fuq path UNC.
InvalidPath=Trid iddaħħal path sħiħ bl-ittra tad-drive; per eżempju:%n%nC:\APP%n%njew path UNC fil-forma:%n%n\\server\share
InvalidDrive=Id-drive jew is-share UNC li għażilt ma jeżistix jew mhuwiex aċċessibbli. Agħżel ieħor.
DiskSpaceWarningTitle=Spazju fuq id-disk mhuwiex biżżejjed
DiskSpaceWarning=L-installatur jeħtieġ mill-inqas %1 KB ta' spazju ħieles biex jinstalla, imma d-drive magħżul għandu biss %2 KB disponibbli.%n%nTixtieq tkompli xorta waħda?
DirNameTooLong=L-isem jew il-path tal-folder huwa twil wisq.
InvalidDirName=L-isem tal-folder mhuwiex validu.
BadDirName32=L-ismijiet tal-folders ma jistgħux jinkludu ebda wieħed minn dawn il-karattri:%n%n%1
DirExistsTitle=Il-folder jeżisti
DirExists=Il-folder:%n%n%1%n%ndiġà jeżisti. Tixtieq tinstalla f'dak il-folder xorta waħda?
DirDoesntExistTitle=Il-folder ma jeżistix
DirDoesntExist=Il-folder:%n%n%1%n%nma jeżistix. Tixtieq li jinħoloq il-folder?

; *** "Select Components" wizard page
WizardSelectComponents=Agħżel il-komponenti
SelectComponentsDesc=Liema komponenti għandhom jiġu installati?
SelectComponentsLabel2=Agħżel il-komponenti li tixtieq tinstalla; neħħi l-marka mill-komponenti li ma tridx tinstalla. Biex tkompli, agħfas “Li jmiss”.
FullInstallation=Installazzjoni sħiħa
; if possible don't translate 'Compact' as 'Minimal' (I mean 'Minimal' in your language)
CompactInstallation=Installazzjoni kompatta
CustomInstallation=Installazzjoni personalizzata
NoUninstallWarningTitle=Il-komponenti jeżistu
NoUninstallWarning=L-installatur induna li l-komponenti li ġejjin diġà huma installati fuq il-kompjuter tiegħek:%n%n%1%n%nIt-tneħħija tal-marka minn dawn il-komponenti mhix se tiddiżinstallahom.%n%nTixtieq tkompli xorta waħda?
ComponentSize1=%1 KB
ComponentSize2=%1 MB
ComponentsDiskSpaceGBLabel=Is-selezzjoni attwali teħtieġ mill-inqas [gb] GB ta' spazju fuq id-disk.
ComponentsDiskSpaceMBLabel=Is-selezzjoni attwali teħtieġ mill-inqas [mb] MB ta' spazju fuq id-disk.

; *** "Select Additional Tasks" wizard page
WizardSelectTasks=Agħżel il-kompiti addizzjonali
SelectTasksDesc=Liema kompiti addizzjonali għandhom jitwettqu?
SelectTasksLabel2=Agħżel il-kompiti addizzjonali li tixtieq li l-installatur iwettaq waqt l-installazzjoni ta' [name], imbagħad agħfas “Li jmiss”.

; *** "Select Start Menu Folder" wizard page
WizardSelectProgramGroup=Agħżel il-folder tal-menu Start
SelectStartMenuFolderDesc=Fejn għandu l-installatur iqiegħed ix-shortcuts tal-programm?
SelectStartMenuFolderLabel3=L-installatur se joħloq ix-shortcuts tal-programm fil-folder li ġej tal-menu Start.
SelectStartMenuFolderBrowseLabel=Biex tkompli, agħfas “Li jmiss”. Jekk tixtieq tagħżel folder differenti, agħfas “Fittex”.
MustEnterGroupName=Trid iddaħħal isem ta' folder.
GroupNameTooLong=L-isem jew il-path tal-folder huwa twil wisq.
InvalidGroupName=L-isem tal-folder mhuwiex validu.
BadGroupName=L-isem tal-folder ma jistax jinkludi ebda wieħed minn dawn il-karattri:%n%n%1
NoProgramGroupCheck2=Toħloqx folder fil-menu Start

; *** "Ready to Install" wizard page
WizardReady=Lest għall-installazzjoni
ReadyLabel1=L-installatur issa lest biex jibda jinstalla [name] fuq il-kompjuter tiegħek.
ReadyLabel2a=Agħfas “Installa” biex tkompli bl-installazzjoni, jew “Lura” jekk tixtieq tirrevedi jew tibdel l-issettjar.
ReadyLabel2b=Agħfas “Installa” biex tkompli bl-installazzjoni.
ReadyMemoUserInfo=Informazzjoni dwar l-utent:
ReadyMemoDir=Post tad-destinazzjoni:
ReadyMemoType=Tip ta' installazzjoni:
ReadyMemoComponents=Komponenti magħżula:
ReadyMemoGroup=Folder tal-menu Start:
ReadyMemoTasks=Kompiti addizzjonali:

; *** TDownloadWizardPage wizard page and DownloadTemporaryFile
DownloadingLabel2=Qed jitniżżlu l-fajls...
ButtonStopDownload=Waqqaf it-tniżżil
StopDownload=Żgur li trid twaqqaf it-tniżżil?
ErrorDownloadAborted=It-tniżżil twaqqaf
ErrorDownloadFailed=It-tniżżil falla: %1 %2
ErrorDownloadSizeFailed=Ma setax jinkiseb id-daqs: %1 %2
ErrorProgress=Progress invalidu: %1 minn %2
ErrorFileSize=Daqs tal-fajl invalidu: mistenni %1, misjub %2

; *** TExtractionWizardPage wizard page and ExtractArchive
ExtractingLabel=Qed jiġu estratti l-fajls...
ButtonStopExtraction=Waqqaf l-estrazzjoni
StopExtraction=Żgur li trid twaqqaf l-estrazzjoni?
ErrorExtractionAborted=L-estrazzjoni twaqqfet
ErrorExtractionFailed=L-estrazzjoni falliet: %1

; *** Archive extraction failure details
ArchiveIncorrectPassword=Il-password mhix korretta
ArchiveIsCorrupted=L-arkivju huwa bil-ħsara
ArchiveUnsupportedFormat=Il-format tal-arkivju mhux appoġġjat

; *** "Preparing to Install" wizard page
WizardPreparing=Qed jitħejja għall-installazzjoni
PreparingDesc=L-installatur qed jitħejja biex jinstalla [name] fuq il-kompjuter tiegħek.
PreviousInstallNotCompleted=L-installazzjoni jew it-tneħħija ta' programm preċedenti ma tlestitx. Ikollok bżonn terġa' tibda l-kompjuter biex tlesti dik l-installazzjoni.%n%nWara li terġa' tibda l-kompjuter, erġa' ħaddem l-installatur biex tlesti l-installazzjoni ta' [name].
CannotContinue=L-installatur ma jistax ikompli. Agħfas “Ikkanċella” biex toħroġ.
ApplicationsFound=L-applikazzjonijiet li ġejjin qed jużaw fajls li għandhom jiġu aġġornati mill-installatur. Huwa rakkomandat li tħalli lill-installatur jagħlaq dawn l-applikazzjonijiet awtomatikament.
ApplicationsFound2=L-applikazzjonijiet li ġejjin qed jużaw fajls li għandhom jiġu aġġornati mill-installatur. Huwa rakkomandat li tħalli lill-installatur jagħlaq dawn l-applikazzjonijiet awtomatikament. Wara li titlesta l-installazzjoni, l-installatur se jipprova jerġa' jibda l-applikazzjonijiet.
CloseApplications=Agħlaq l-applikazzjonijiet awtomatikament
DontCloseApplications=Tagħlaqx l-applikazzjonijiet
ErrorCloseApplications=L-installatur ma setax jagħlaq l-applikazzjonijiet kollha awtomatikament. Huwa rakkomandat li tagħlaq l-applikazzjonijiet kollha li qed jużaw fajls li għandhom jiġu aġġornati mill-installatur qabel ma tkompli.
PrepareToInstallNeedsRestart=L-installatur irid jerġa' jibda l-kompjuter tiegħek. Wara li terġa' tibda l-kompjuter, erġa' ħaddem l-installatur biex tlesti l-installazzjoni ta' [name].%n%nTixtieq terġa' tibda issa?

; *** "Installing" wizard page
WizardInstalling=Qed jinstalla
InstallingLabel=Jekk jogħġbok stenna waqt li l-installatur jinstalla [name] fuq il-kompjuter tiegħek.

; *** "Setup Completed" wizard page
FinishedHeadingLabel=Qed titlesta l-installazzjoni ta' [name]
FinishedLabelNoIcons=L-installatur spiċċa jinstalla [name] fuq il-kompjuter tiegħek.
FinishedLabel=L-installatur spiċċa jinstalla [name] fuq il-kompjuter tiegħek. L-applikazzjoni tista' titnieda billi tagħżel ix-shortcuts installati.
ClickFinish=Agħfas “Spiċċa” biex toħroġ mill-installazzjoni.
FinishedRestartLabel=Biex titlesta l-installazzjoni ta' [name], l-installatur irid jerġa' jibda l-kompjuter tiegħek. Tixtieq terġa' tibda issa?
FinishedRestartMessage=Biex titlesta l-installazzjoni ta' [name], l-installatur irid jerġa' jibda l-kompjuter tiegħek.%n%nTixtieq terġa' tibda issa?
ShowReadmeCheck=Iva, nixtieq nara l-fajl README
YesRadio=Iva, erġa' ibda l-kompjuter issa
NoRadio=Le, se nerġa' nibda l-kompjuter aktar tard
; used for example as 'Run MyProg.exe'
RunEntryExec=Ħaddem %1
; used for example as 'View Readme.txt'
RunEntryShellExec=Ara %1

; *** "Setup Needs the Next Disk" stuff
ChangeDiskTitle=L-installatur jeħtieġ id-disk li jmiss
SelectDiskLabel2=Daħħal id-Disk %1 u agħfas “OK”.%n%nJekk il-fajls ta' dan id-disk jinsabu f'folder differenti minn dak muri hawn taħt, daħħal il-path korrett jew agħfas “Fittex”.
PathLabel=Path:
FileNotInDir2=Il-fajl "%1" ma nstabx f'"%2". Daħħal id-disk korrett jew agħżel folder ieħor.
SelectDirectoryLabel=Speċifika l-post tad-disk li jmiss.

; *** Installation phase messages
SetupAborted=L-installazzjoni ma tlestitx.%n%nSewwi l-problema u erġa' ħaddem l-installatur.
AbortRetryIgnoreSelectAction=Agħżel azzjoni
AbortRetryIgnoreRetry=Erġa' pprova
AbortRetryIgnoreIgnore=Injora l-iżball u kompli
AbortRetryIgnoreCancel=Ikkanċella l-installazzjoni
RetryCancelSelectAction=Agħżel azzjoni
RetryCancelRetry=Erġa' pprova
RetryCancelCancel=Ikkanċella

; *** Installation status messages
StatusClosingApplications=Qed jingħalqu l-applikazzjonijiet...
StatusCreateDirs=Qed jinħolqu l-folders...
StatusExtractFiles=Qed jiġu estratti l-fajls...
StatusDownloadFiles=Qed jitniżżlu l-fajls...
StatusCreateIcons=Qed jinħolqu x-shortcuts...
StatusCreateIniEntries=Qed jinħolqu d-daħliet INI...
StatusCreateRegistryEntries=Qed jinħolqu d-daħliet fir-reġistru...
StatusRegisterFiles=Qed jiġu rreġistrati l-fajls...
StatusSavingUninstall=Qed tiġi ssejvjata l-informazzjoni tad-diżinstallazzjoni...
StatusRunProgram=Qed titlesta l-installazzjoni...
StatusRestartingApplications=Qed jerġgħu jinbdew l-applikazzjonijiet...
StatusRollback=Qed jitreġġgħu lura l-bidliet...

; *** Misc. errors
ErrorInternal2=Żball intern: %1
ErrorFunctionFailedNoCode=%1 falla
ErrorFunctionFailed=%1 falla; kodiċi %2
ErrorFunctionFailedWithMessage=%1 falla; kodiċi %2.%n%3
ErrorExecutingProgram=Ma setax jitħaddem il-fajl:%n%1

; *** Registry errors
ErrorRegOpenKey=Żball waqt il-ftuħ taċ-ċavetta tar-reġistru:%n%1\%2
ErrorRegCreateKey=Żball waqt il-ħolqien taċ-ċavetta tar-reġistru:%n%1\%2
ErrorRegWriteKey=Żball waqt il-kitba fiċ-ċavetta tar-reġistru:%n%1\%2

; *** INI errors
ErrorIniEntry=Żball waqt il-ħolqien tad-daħla INI fil-fajl "%1".

; *** File copying errors
FileAbortRetryIgnoreSkipNotRecommended=Aqbeż dan il-fajl (mhux rakkomandat)
FileAbortRetryIgnoreIgnoreNotRecommended=Injora l-iżball u kompli (mhux rakkomandat)
SourceIsCorrupted=Il-fajl tas-sors huwa bil-ħsara
SourceDoesntExist=Il-fajl tas-sors "%1" ma jeżistix
SourceVerificationFailed=Il-verifika tal-fajl tas-sors falliet: %1
VerificationSignatureDoesntExist=Il-fajl tal-firma "%1" ma jeżistix
VerificationSignatureInvalid=Il-fajl tal-firma "%1" mhuwiex validu
VerificationKeyNotFound=Il-fajl tal-firma "%1" juża ċavetta mhux magħrufa
VerificationFileNameIncorrect=L-isem tal-fajl mhuwiex korrett
VerificationFileTagIncorrect=It-tag tal-fajl mhuwiex korrett
VerificationFileSizeIncorrect=Id-daqs tal-fajl mhuwiex korrett
VerificationFileHashIncorrect=L-hash tal-fajl mhuwiex korrett
ExistingFileReadOnly2=Il-fajl eżistenti ma setax jiġi sostitwit għax huwa mmarkat bħala read-only.
ExistingFileReadOnlyRetry=Neħħi l-attribut read-only u erġa' pprova
ExistingFileReadOnlyKeepExisting=Żomm il-fajl eżistenti
ErrorReadingExistingDest=Seħħ żball waqt li kien qed jinqara l-fajl eżistenti:
FileExistsSelectAction=Agħżel azzjoni
FileExists2=Il-fajl diġà jeżisti.
FileExistsOverwriteExisting=Ikteb fuq il-fajl eżistenti
FileExistsKeepExisting=Żomm il-fajl eżistenti
FileExistsOverwriteOrKeepAll=Agħmel dan għall-konflitti li jmiss
ExistingFileNewerSelectAction=Agħżel azzjoni
ExistingFileNewer2=Il-fajl eżistenti huwa aktar ġdid minn dak li l-installatur qed jipprova jinstalla.
ExistingFileNewerOverwriteExisting=Ikteb fuq il-fajl eżistenti
ExistingFileNewerKeepExisting=Żomm il-fajl eżistenti (rakkomandat)
ExistingFileNewerOverwriteOrKeepAll=Agħmel dan għall-konflitti li jmiss
ErrorChangingAttr=Seħħ żball waqt li kien qed jinbidel l-attribut tal-fajl eżistenti:
ErrorCreatingTemp=Seħħ żball waqt li kien qed jinħoloq fajl fil-folder tad-destinazzjoni:
ErrorReadingSource=Seħħ żball waqt li kien qed jinqara l-fajl tas-sors:
ErrorCopying=Seħħ żball waqt li kien qed jiġi kkupjat fajl:
ErrorDownloading=Seħħ żball waqt li kien qed jitniżżel fajl:
ErrorExtracting=Seħħ żball waqt li kien qed jiġi estratt arkivju:
ErrorReplacingExistingFile=Seħħ żball waqt li kien qed jiġi sostitwit il-fajl eżistenti:
ErrorRestartReplace=RestartReplace falla:
ErrorRenamingTemp=Seħħ żball waqt li kien qed jiġi rinominat fajl fil-folder tad-destinazzjoni:
ErrorRegisterServer=Ma setax jiġi rreġistrat id-DLL/OCX: %1
ErrorRegSvr32Failed=RegSvr32 falla bil-kodiċi ta' ħruġ %1
ErrorRegisterTypeLib=Ma setgħetx tiġi rreġistrata l-librerija tat-tipi: %1

; *** Uninstall display name markings
; used for example as 'My Program (32-bit)'
UninstallDisplayNameMark=%1 (%2)
; used for example as 'My Program (32-bit, All users)'
UninstallDisplayNameMarks=%1 (%2, %3)
UninstallDisplayNameMark32Bit=32-bit
UninstallDisplayNameMark64Bit=64-bit
UninstallDisplayNameMarkAllUsers=L-utenti kollha
UninstallDisplayNameMarkCurrentUser=L-utent attwali

; *** Post-installation errors
ErrorOpeningReadme=Seħħ żball waqt li kien qed jinfetaħ il-fajl README.
ErrorRestartingComputer=L-installatur ma setax jerġa' jibda l-kompjuter. Agħmel dan manwalment.

; *** Uninstaller messages
UninstallNotFound=Il-fajl "%1" ma jeżistix. Ma jistax jiġi diżinstallat.
UninstallOpenError=Il-fajl "%1" ma setax jinfetaħ. Ma jistax jiġi diżinstallat
UninstallUnsupportedVer=Il-fajl tal-ġurnal tad-diżinstallazzjoni "%1" huwa f'format li din il-verżjoni tad-diżinstallatur ma tagħrafx. Ma jistax jiġi diżinstallat
UninstallUnknownEntry=Instabet daħla mhux magħrufa (%1) fil-ġurnal tad-diżinstallazzjoni
ConfirmUninstall=Żgur li trid tneħħi għal kollox %1 u l-komponenti kollha tiegħu?
UninstallOnlyOnWin64=Din l-installazzjoni tista' tiġi diżinstallata biss fuq Windows 64-bit.
OnlyAdminCanUninstall=Din l-installazzjoni tista' tiġi diżinstallata biss minn utent b'drittijiet ta' amministratur.
UninstallStatusLabel=Jekk jogħġbok stenna waqt li %1 jitneħħa mill-kompjuter tiegħek.
UninstalledAll=%1 tneħħa b'suċċess mill-kompjuter tiegħek.
UninstalledMost=Id-diżinstallazzjoni ta' %1 tlestiet.%n%nXi elementi ma setgħux jitneħħew. Dawn jistgħu jitneħħew manwalment.
UninstalledAndNeedsRestart=Biex titlesta d-diżinstallazzjoni ta' %1, il-kompjuter tiegħek irid jerġa' jinbeda.%n%nTixtieq terġa' tibda issa?
UninstallDataCorrupted=Il-fajl "%1" huwa bil-ħsara. Ma jistax jiġi diżinstallat

; *** Uninstallation phase messages
ConfirmDeleteSharedFileTitle=Tneħħi l-fajl kondiviż?
ConfirmDeleteSharedFile2=Is-sistema tindika li l-fajl kondiviż li ġej m'għadux jintuża minn ebda programm. Tixtieq li d-diżinstallatur ineħħi dan il-fajl kondiviż?%n%nJekk xi programmi għadhom jużaw dan il-fajl u jitneħħa, dawk il-programmi jistgħu ma jaħdmux sew. Jekk għandek xi dubju, agħżel “Le”. Jekk iżżomm il-fajl fuq is-sistema tiegħek, ma jagħmel ebda ħsara.
SharedFileNameLabel=Isem il-fajl:
SharedFileLocationLabel=Post:
WizardUninstalling=Status tad-diżinstallazzjoni
StatusUninstalling=Qed jiġi diżinstallat %1...

; *** Shutdown block reasons
ShutdownBlockReasonInstallingApp=Qed jiġi installat %1.
ShutdownBlockReasonUninstallingApp=Qed jiġi diżinstallat %1.

; The custom messages below aren't used by Setup itself, but if you make
; use of them in your scripts, you'll want to translate them.

[CustomMessages]

NameAndVersion=%1 verżjoni %2
AdditionalIcons=Shortcuts addizzjonali:
CreateDesktopIcon=Oħloq shortcut fuq id-desktop
CreateQuickLaunchIcon=Oħloq shortcut tal-“Quick Launch”
ProgramOnTheWeb=%1 fuq il-Web
UninstallProgram=Diżinstalla %1
LaunchProgram=Nieda %1
AssocFileExtension=Assoċja %1 mal-estensjoni tal-fajls %2
AssocingFileExtension=Qed jiġi assoċjat %1 mal-estensjoni tal-fajls %2...
AutoStartProgramGroupDescription=Startup:
AutoStartProgram=Ibda %1 awtomatikament
AddonHostProgramNotFound=%1 ma nstabx fil-folder li għażilt.%n%nTixtieq tkompli xorta waħda?

; The messages that are Lang Switcher's own (the sample is English.isl).
LocalCertReady=L-installatur se joħloq ċertifikat fuq dan il-kompjuter u jiffirma Lang Switcher miegħu, biex il-programm jaħdem ukoll fit-twieqi tal-programmi mħaddma bħala amministratur.
LocalCertFailed=Lang Switcher ġie installat, imma mhux se jaħdem fit-twieqi tal-programmi mħaddma bħala amministratur: Windows ma aċċettax iċ-ċertifikat maħluq fuq dan il-kompjuter. Id-dettalji jinsabu fil-ġurnal tal-installazzjoni.
RemoveSettingsQuestion=Tneħħi wkoll il-fajl tal-issettjar ta' Lang Switcher?
RemoveSettingsKeep=Agħżel “Le” biex iżżomm l-issettjar tiegħek għal installazzjoni mill-ġdid aktar tard.
AuthorSlogan=programmi magħmula bil-qalb
