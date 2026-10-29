; Lang Switcher -- the messages of the installer in Vietnamese (stage E109, group B).
;
; Inno Setup has no official Vietnamese file, so this file is whole: [LangOptions], all the messages of
; compiler:Default.isl (version 6.5.0 sample) translated, then the stock [CustomMessages] of that
; file and the messages of the installer that are its own (the sample is English.isl). Read as the
; only MessagesFile of the "vietnamese" entry of [Languages] in ..\LangSwitcher.iss; Default.isl may
; stand before it: what is missing here falls back to English.
; No accelerators (&): a file without an official translation has none, so that one window never
; shows two equal letters (decision of the E108 trial, RULES 16). Messages that end without a
; full stop in Default.isl end without one here: Setup adds a Latin full stop itself. The names of
; the buttons inside a message stand in the quotes of the language. The name of the program is
; not translated. Tones are precomposed (NFC) letters.
; AuthorSlogan (stage E99, decision 167.1): the slogan of the signature of the author at the
; bottom of every page of the wizard -- after "Panda Koder" and a middle dot; light, without
; pathos. The name of the author and the address of the site stay as they are in every language.

[LangOptions]
LanguageName=Tiếng Việt
LanguageID=$042A
LanguageCodePage=0
;DialogFontName=
;DialogFontSize=9
;DialogFontBaseScaleWidth=7
;DialogFontBaseScaleHeight=15
;WelcomeFontName=Segoe UI
;WelcomeFontSize=14

[Messages]

; *** Application titles
SetupAppTitle=Cài đặt
SetupWindowTitle=Cài đặt - %1
UninstallAppTitle=Gỡ cài đặt
UninstallAppFullTitle=Gỡ cài đặt %1

; *** Misc. common
InformationTitle=Thông tin
ConfirmTitle=Xác nhận
ErrorTitle=Lỗi

; *** SetupLdr messages
SetupLdrStartupMessage=Thao tác này sẽ cài đặt %1. Bạn có muốn tiếp tục không?
LdrCannotCreateTemp=Không thể tạo tệp tạm thời. Quá trình cài đặt đã bị hủy
LdrCannotExecTemp=Không thể chạy tệp trong thư mục tạm thời. Quá trình cài đặt đã bị hủy
HelpTextNote=

; *** Startup error messages
LastErrorMessage=%1.%n%nLỗi %2: %3
SetupFileMissing=Tệp %1 bị thiếu trong thư mục cài đặt. Hãy khắc phục sự cố hoặc lấy một bản sao mới của chương trình.
SetupFileCorrupt=Các tệp cài đặt bị hỏng. Hãy lấy một bản sao mới của chương trình.
SetupFileCorruptOrWrongVer=Các tệp cài đặt bị hỏng hoặc không tương thích với phiên bản trình cài đặt này. Hãy khắc phục sự cố hoặc lấy một bản sao mới của chương trình.
InvalidParameter=Một tham số không hợp lệ đã được truyền trên dòng lệnh:%n%n%1
SetupAlreadyRunning=Trình cài đặt đang chạy.
WindowsVersionNotSupported=Chương trình này không hỗ trợ phiên bản Windows đang chạy trên máy tính của bạn.
WindowsServicePackRequired=Chương trình này yêu cầu %1 Service Pack %2 trở lên.
NotOnThisPlatform=Chương trình này sẽ không chạy trên %1.
OnlyOnThisPlatform=Chương trình này phải được chạy trên %1.
OnlyOnTheseArchitectures=Chương trình này chỉ có thể được cài đặt trên các phiên bản Windows dành cho các kiến trúc bộ xử lý sau:%n%n%1
WinVersionTooLowError=Chương trình này yêu cầu %1 phiên bản %2 trở lên.
WinVersionTooHighError=Không thể cài đặt chương trình này trên %1 phiên bản %2 trở lên.
AdminPrivilegesRequired=Bạn phải đăng nhập với tư cách quản trị viên khi cài đặt chương trình này.
PowerUserPrivilegesRequired=Bạn phải đăng nhập với tư cách quản trị viên hoặc thành viên của nhóm Power Users khi cài đặt chương trình này.
SetupAppRunningError=Trình cài đặt phát hiện %1 đang chạy.%n%nHãy đóng tất cả các cửa sổ của nó ngay bây giờ, rồi nhấn “OK” để tiếp tục hoặc “Hủy” để thoát.
UninstallAppRunningError=Trình gỡ cài đặt phát hiện %1 đang chạy.%n%nHãy đóng tất cả các cửa sổ của nó ngay bây giờ, rồi nhấn “OK” để tiếp tục hoặc “Hủy” để thoát.

; *** Startup questions
PrivilegesRequiredOverrideTitle=Chọn chế độ cài đặt
PrivilegesRequiredOverrideInstruction=Chọn chế độ cài đặt
PrivilegesRequiredOverrideText1=%1 có thể được cài đặt cho tất cả người dùng (cần quyền quản trị) hoặc chỉ cho bạn.
PrivilegesRequiredOverrideText2=%1 có thể được cài đặt chỉ cho bạn hoặc cho tất cả người dùng (cần quyền quản trị).
PrivilegesRequiredOverrideAllUsers=Cài đặt cho tất cả người dùng
PrivilegesRequiredOverrideAllUsersRecommended=Cài đặt cho tất cả người dùng (khuyến nghị)
PrivilegesRequiredOverrideCurrentUser=Chỉ cài đặt cho tôi
PrivilegesRequiredOverrideCurrentUserRecommended=Chỉ cài đặt cho tôi (khuyến nghị)

; *** Misc. errors
ErrorCreatingDir=Trình cài đặt không thể tạo thư mục "%1"
ErrorTooManyFilesInDir=Không thể tạo tệp trong thư mục "%1" vì thư mục này chứa quá nhiều tệp

; *** Setup common messages
ExitSetupTitle=Thoát trình cài đặt
ExitSetupMessage=Quá trình cài đặt chưa hoàn tất. Nếu thoát bây giờ, chương trình sẽ không được cài đặt.%n%nBạn có thể chạy lại trình cài đặt vào lúc khác để hoàn tất việc cài đặt.%n%nThoát trình cài đặt?
AboutSetupMenuItem=Giới thiệu về trình cài đặt...
AboutSetupTitle=Giới thiệu về trình cài đặt
AboutSetupMessage=%1 phiên bản %2%n%3%n%nTrang chủ của %1:%n%4
AboutSetupNote=
TranslatorNote=

; *** Buttons
ButtonBack=< Quay lại
ButtonNext=Tiếp >
ButtonInstall=Cài đặt
ButtonOK=OK
ButtonCancel=Hủy
ButtonYes=Có
ButtonYesToAll=Có cho tất cả
ButtonNo=Không
ButtonNoToAll=Không cho tất cả
ButtonFinish=Hoàn tất
ButtonBrowse=Duyệt...
ButtonWizardBrowse=Duyệt...
ButtonNewFolder=Tạo thư mục mới

; *** "Select Language" dialog messages
SelectLanguageTitle=Chọn ngôn ngữ của trình cài đặt
SelectLanguageLabel=Chọn ngôn ngữ sẽ dùng trong quá trình cài đặt.

; *** Common wizard text
ClickNext=Nhấn “Tiếp” để tiếp tục hoặc “Hủy” để thoát khỏi trình cài đặt.
BeveledLabel=
BrowseDialogTitle=Chọn thư mục
BrowseDialogLabel=Chọn một thư mục trong danh sách bên dưới, rồi nhấn “OK”.
NewFolderName=Thư mục mới

; *** "Welcome" wizard page
WelcomeLabel1=Chào mừng bạn đến với trình hướng dẫn cài đặt [name]
WelcomeLabel2=Trình này sẽ cài đặt [name/ver] trên máy tính của bạn.%n%nNên đóng tất cả các ứng dụng khác trước khi tiếp tục.

; *** "Password" wizard page
WizardPassword=Mật khẩu
PasswordLabel1=Bản cài đặt này được bảo vệ bằng mật khẩu.
PasswordLabel3=Hãy nhập mật khẩu, rồi nhấn “Tiếp” để tiếp tục. Mật khẩu phân biệt chữ hoa và chữ thường.
PasswordEditLabel=Mật khẩu:
IncorrectPassword=Mật khẩu bạn nhập không đúng. Hãy thử lại.

; *** "License Agreement" wizard page
WizardLicense=Thỏa thuận cấp phép
LicenseLabel=Hãy đọc thông tin quan trọng sau đây trước khi tiếp tục.
LicenseLabel3=Hãy đọc Thỏa thuận cấp phép sau đây. Bạn phải chấp nhận các điều khoản của thỏa thuận này trước khi tiếp tục cài đặt.
LicenseAccepted=Tôi chấp nhận thỏa thuận
LicenseNotAccepted=Tôi không chấp nhận thỏa thuận

; *** "Information" wizard pages
WizardInfoBefore=Thông tin
InfoBeforeLabel=Hãy đọc thông tin quan trọng sau đây trước khi tiếp tục.
InfoBeforeClickLabel=Khi bạn sẵn sàng tiếp tục cài đặt, hãy nhấn “Tiếp”.
WizardInfoAfter=Thông tin
InfoAfterLabel=Hãy đọc thông tin quan trọng sau đây trước khi tiếp tục.
InfoAfterClickLabel=Khi bạn sẵn sàng tiếp tục cài đặt, hãy nhấn “Tiếp”.

; *** "User Information" wizard page
WizardUserInfo=Thông tin người dùng
UserInfoDesc=Hãy nhập thông tin của bạn.
UserInfoName=Tên người dùng:
UserInfoOrg=Tổ chức:
UserInfoSerial=Số sê-ri:
UserInfoNameRequired=Bạn phải nhập tên.

; *** "Select Destination Location" wizard page
WizardSelectDir=Chọn vị trí cài đặt
SelectDirDesc=Cài đặt [name] ở đâu?
SelectDirLabel3=Trình cài đặt sẽ cài đặt [name] vào thư mục sau.
SelectDirBrowseLabel=Để tiếp tục, hãy nhấn “Tiếp”. Nếu muốn chọn thư mục khác, hãy nhấn “Duyệt”.
DiskSpaceGBLabel=Cần ít nhất [gb] GB dung lượng đĩa trống.
DiskSpaceMBLabel=Cần ít nhất [mb] MB dung lượng đĩa trống.
CannotInstallToNetworkDrive=Trình cài đặt không thể cài đặt vào ổ đĩa mạng.
CannotInstallToUNCPath=Trình cài đặt không thể cài đặt vào đường dẫn UNC.
InvalidPath=Bạn phải nhập đường dẫn đầy đủ có ký tự ổ đĩa; ví dụ:%n%nC:\APP%n%nhoặc đường dẫn UNC có dạng:%n%n\\server\share
InvalidDrive=Ổ đĩa hoặc thư mục chia sẻ UNC bạn chọn không tồn tại hoặc không truy cập được. Hãy chọn nơi khác.
DiskSpaceWarningTitle=Không đủ dung lượng đĩa
DiskSpaceWarning=Trình cài đặt cần ít nhất %1 KB dung lượng trống để cài đặt, nhưng ổ đĩa đã chọn chỉ còn %2 KB.%n%nBạn vẫn muốn tiếp tục chứ?
DirNameTooLong=Tên thư mục hoặc đường dẫn quá dài.
InvalidDirName=Tên thư mục không hợp lệ.
BadDirName32=Tên thư mục không được chứa các ký tự sau:%n%n%1
DirExistsTitle=Thư mục đã tồn tại
DirExists=Thư mục:%n%n%1%n%nđã tồn tại. Bạn vẫn muốn cài đặt vào thư mục đó chứ?
DirDoesntExistTitle=Thư mục không tồn tại
DirDoesntExist=Thư mục:%n%n%1%n%nkhông tồn tại. Bạn có muốn tạo thư mục này không?

; *** "Select Components" wizard page
WizardSelectComponents=Chọn thành phần
SelectComponentsDesc=Cài đặt những thành phần nào?
SelectComponentsLabel2=Chọn các thành phần bạn muốn cài đặt; bỏ chọn các thành phần bạn không muốn cài đặt. Nhấn “Tiếp” khi bạn sẵn sàng tiếp tục.
FullInstallation=Cài đặt đầy đủ
; if possible don't translate 'Compact' as 'Minimal' (I mean 'Minimal' in your language)
CompactInstallation=Cài đặt gọn
CustomInstallation=Cài đặt tùy chỉnh
NoUninstallWarningTitle=Các thành phần đã tồn tại
NoUninstallWarning=Trình cài đặt phát hiện các thành phần sau đã được cài đặt trên máy tính của bạn:%n%n%1%n%nBỏ chọn các thành phần này sẽ không gỡ cài đặt chúng.%n%nBạn vẫn muốn tiếp tục chứ?
ComponentSize1=%1 KB
ComponentSize2=%1 MB
ComponentsDiskSpaceGBLabel=Lựa chọn hiện tại cần ít nhất [gb] GB dung lượng đĩa.
ComponentsDiskSpaceMBLabel=Lựa chọn hiện tại cần ít nhất [mb] MB dung lượng đĩa.

; *** "Select Additional Tasks" wizard page
WizardSelectTasks=Chọn tác vụ bổ sung
SelectTasksDesc=Thực hiện những tác vụ bổ sung nào?
SelectTasksLabel2=Chọn các tác vụ bổ sung mà bạn muốn trình cài đặt thực hiện khi cài đặt [name], rồi nhấn “Tiếp”.

; *** "Select Start Menu Folder" wizard page
WizardSelectProgramGroup=Chọn thư mục menu Bắt đầu
SelectStartMenuFolderDesc=Đặt lối tắt của chương trình ở đâu?
SelectStartMenuFolderLabel3=Trình cài đặt sẽ tạo lối tắt của chương trình trong thư mục menu Bắt đầu sau.
SelectStartMenuFolderBrowseLabel=Để tiếp tục, hãy nhấn “Tiếp”. Nếu muốn chọn thư mục khác, hãy nhấn “Duyệt”.
MustEnterGroupName=Bạn phải nhập tên thư mục.
GroupNameTooLong=Tên thư mục hoặc đường dẫn quá dài.
InvalidGroupName=Tên thư mục không hợp lệ.
BadGroupName=Tên thư mục không được chứa các ký tự sau:%n%n%1
NoProgramGroupCheck2=Không tạo thư mục menu Bắt đầu

; *** "Ready to Install" wizard page
WizardReady=Sẵn sàng cài đặt
ReadyLabel1=Trình cài đặt đã sẵn sàng cài đặt [name] trên máy tính của bạn.
ReadyLabel2a=Nhấn “Cài đặt” để tiếp tục cài đặt, hoặc nhấn “Quay lại” nếu bạn muốn xem lại hoặc thay đổi các thiết lập.
ReadyLabel2b=Nhấn “Cài đặt” để tiếp tục cài đặt.
ReadyMemoUserInfo=Thông tin người dùng:
ReadyMemoDir=Vị trí cài đặt:
ReadyMemoType=Kiểu cài đặt:
ReadyMemoComponents=Các thành phần đã chọn:
ReadyMemoGroup=Thư mục menu Bắt đầu:
ReadyMemoTasks=Các tác vụ bổ sung:

; *** TDownloadWizardPage wizard page and DownloadTemporaryFile
DownloadingLabel2=Đang tải các tệp xuống...
ButtonStopDownload=Dừng tải xuống
StopDownload=Bạn có chắc muốn dừng tải xuống không?
ErrorDownloadAborted=Tải xuống đã bị hủy
ErrorDownloadFailed=Tải xuống thất bại: %1 %2
ErrorDownloadSizeFailed=Không lấy được kích thước: %1 %2
ErrorProgress=Tiến độ không hợp lệ: %1 trên %2
ErrorFileSize=Kích thước tệp không hợp lệ: cần %1, thực tế %2

; *** TExtractionWizardPage wizard page and ExtractArchive
ExtractingLabel=Đang giải nén các tệp...
ButtonStopExtraction=Dừng giải nén
StopExtraction=Bạn có chắc muốn dừng giải nén không?
ErrorExtractionAborted=Giải nén đã bị hủy
ErrorExtractionFailed=Giải nén thất bại: %1

; *** Archive extraction failure details
ArchiveIncorrectPassword=Mật khẩu không đúng
ArchiveIsCorrupted=Kho lưu trữ bị hỏng
ArchiveUnsupportedFormat=Định dạng kho lưu trữ không được hỗ trợ

; *** "Preparing to Install" wizard page
WizardPreparing=Đang chuẩn bị cài đặt
PreparingDesc=Trình cài đặt đang chuẩn bị cài đặt [name] trên máy tính của bạn.
PreviousInstallNotCompleted=Việc cài đặt/gỡ bỏ chương trình trước đó chưa hoàn tất. Bạn cần khởi động lại máy tính để hoàn tất việc đó.%n%nSau khi khởi động lại máy tính, hãy chạy lại trình cài đặt để hoàn tất việc cài đặt [name].
CannotContinue=Trình cài đặt không thể tiếp tục. Hãy nhấn “Hủy” để thoát.
ApplicationsFound=Các ứng dụng sau đang dùng những tệp cần được trình cài đặt cập nhật. Nên cho phép trình cài đặt tự động đóng các ứng dụng này.
ApplicationsFound2=Các ứng dụng sau đang dùng những tệp cần được trình cài đặt cập nhật. Nên cho phép trình cài đặt tự động đóng các ứng dụng này. Sau khi cài đặt xong, trình cài đặt sẽ cố gắng khởi động lại các ứng dụng đó.
CloseApplications=Tự động đóng các ứng dụng
DontCloseApplications=Không đóng các ứng dụng
ErrorCloseApplications=Trình cài đặt không thể tự động đóng tất cả các ứng dụng. Nên đóng tất cả các ứng dụng đang dùng những tệp cần được trình cài đặt cập nhật trước khi tiếp tục.
PrepareToInstallNeedsRestart=Trình cài đặt phải khởi động lại máy tính của bạn. Sau khi khởi động lại, hãy chạy lại trình cài đặt để hoàn tất việc cài đặt [name].%n%nBạn có muốn khởi động lại ngay bây giờ không?

; *** "Installing" wizard page
WizardInstalling=Đang cài đặt
InstallingLabel=Vui lòng đợi trong khi trình cài đặt cài đặt [name] trên máy tính của bạn.

; *** "Setup Completed" wizard page
FinishedHeadingLabel=Hoàn tất trình hướng dẫn cài đặt [name]
FinishedLabelNoIcons=Trình cài đặt đã cài đặt xong [name] trên máy tính của bạn.
FinishedLabel=Trình cài đặt đã cài đặt xong [name] trên máy tính của bạn. Bạn có thể khởi chạy ứng dụng bằng các lối tắt đã cài đặt.
ClickFinish=Nhấn “Hoàn tất” để thoát khỏi trình cài đặt.
FinishedRestartLabel=Để hoàn tất việc cài đặt [name], trình cài đặt phải khởi động lại máy tính của bạn. Bạn có muốn khởi động lại ngay bây giờ không?
FinishedRestartMessage=Để hoàn tất việc cài đặt [name], trình cài đặt phải khởi động lại máy tính của bạn.%n%nBạn có muốn khởi động lại ngay bây giờ không?
ShowReadmeCheck=Có, tôi muốn xem tệp README
YesRadio=Có, khởi động lại máy tính ngay bây giờ
NoRadio=Không, tôi sẽ khởi động lại máy tính sau
; used for example as 'Run MyProg.exe'
RunEntryExec=Chạy %1
; used for example as 'View Readme.txt'
RunEntryShellExec=Xem %1

; *** "Setup Needs the Next Disk" stuff
ChangeDiskTitle=Trình cài đặt cần đĩa tiếp theo
SelectDiskLabel2=Hãy lắp đĩa %1 rồi nhấn “OK”.%n%nNếu các tệp trên đĩa này nằm trong thư mục khác với thư mục hiển thị bên dưới, hãy nhập đường dẫn đúng hoặc nhấn “Duyệt”.
PathLabel=Đường dẫn:
FileNotInDir2=Không tìm thấy tệp "%1" trong "%2". Hãy lắp đúng đĩa hoặc chọn thư mục khác.
SelectDirectoryLabel=Hãy chỉ định vị trí của đĩa tiếp theo.

; *** Installation phase messages
SetupAborted=Quá trình cài đặt chưa hoàn tất.%n%nHãy khắc phục sự cố và chạy lại trình cài đặt.
AbortRetryIgnoreSelectAction=Chọn thao tác
AbortRetryIgnoreRetry=Thử lại
AbortRetryIgnoreIgnore=Bỏ qua lỗi và tiếp tục
AbortRetryIgnoreCancel=Hủy cài đặt
RetryCancelSelectAction=Chọn thao tác
RetryCancelRetry=Thử lại
RetryCancelCancel=Hủy

; *** Installation status messages
StatusClosingApplications=Đang đóng các ứng dụng...
StatusCreateDirs=Đang tạo các thư mục...
StatusExtractFiles=Đang giải nén các tệp...
StatusDownloadFiles=Đang tải các tệp xuống...
StatusCreateIcons=Đang tạo lối tắt...
StatusCreateIniEntries=Đang tạo các mục INI...
StatusCreateRegistryEntries=Đang tạo các mục trong sổ đăng ký...
StatusRegisterFiles=Đang đăng ký các tệp...
StatusSavingUninstall=Đang lưu thông tin gỡ cài đặt...
StatusRunProgram=Đang hoàn tất cài đặt...
StatusRestartingApplications=Đang khởi động lại các ứng dụng...
StatusRollback=Đang hoàn tác các thay đổi...

; *** Misc. errors
ErrorInternal2=Lỗi nội bộ: %1
ErrorFunctionFailedNoCode=%1 thất bại
ErrorFunctionFailed=%1 thất bại; mã %2
ErrorFunctionFailedWithMessage=%1 thất bại; mã %2.%n%3
ErrorExecutingProgram=Không thể chạy tệp:%n%1

; *** Registry errors
ErrorRegOpenKey=Lỗi khi mở khóa sổ đăng ký:%n%1\%2
ErrorRegCreateKey=Lỗi khi tạo khóa sổ đăng ký:%n%1\%2
ErrorRegWriteKey=Lỗi khi ghi vào khóa sổ đăng ký:%n%1\%2

; *** INI errors
ErrorIniEntry=Lỗi khi tạo mục INI trong tệp "%1".

; *** File copying errors
FileAbortRetryIgnoreSkipNotRecommended=Bỏ qua tệp này (không khuyến nghị)
FileAbortRetryIgnoreIgnoreNotRecommended=Bỏ qua lỗi và tiếp tục (không khuyến nghị)
SourceIsCorrupted=Tệp nguồn bị hỏng
SourceDoesntExist=Tệp nguồn "%1" không tồn tại
SourceVerificationFailed=Xác minh tệp nguồn thất bại: %1
VerificationSignatureDoesntExist=Tệp chữ ký "%1" không tồn tại
VerificationSignatureInvalid=Tệp chữ ký "%1" không hợp lệ
VerificationKeyNotFound=Tệp chữ ký "%1" dùng một khóa không xác định
VerificationFileNameIncorrect=Tên tệp không đúng
VerificationFileTagIncorrect=Thẻ của tệp không đúng
VerificationFileSizeIncorrect=Kích thước của tệp không đúng
VerificationFileHashIncorrect=Mã băm của tệp không đúng
ExistingFileReadOnly2=Không thể thay thế tệp hiện có vì nó được đánh dấu chỉ đọc.
ExistingFileReadOnlyRetry=Gỡ thuộc tính chỉ đọc và thử lại
ExistingFileReadOnlyKeepExisting=Giữ tệp hiện có
ErrorReadingExistingDest=Đã xảy ra lỗi khi đọc tệp hiện có:
FileExistsSelectAction=Chọn thao tác
FileExists2=Tệp đã tồn tại.
FileExistsOverwriteExisting=Ghi đè tệp hiện có
FileExistsKeepExisting=Giữ tệp hiện có
FileExistsOverwriteOrKeepAll=Áp dụng cho các xung đột tiếp theo
ExistingFileNewerSelectAction=Chọn thao tác
ExistingFileNewer2=Tệp hiện có mới hơn tệp mà trình cài đặt đang cố cài đặt.
ExistingFileNewerOverwriteExisting=Ghi đè tệp hiện có
ExistingFileNewerKeepExisting=Giữ tệp hiện có (khuyến nghị)
ExistingFileNewerOverwriteOrKeepAll=Áp dụng cho các xung đột tiếp theo
ErrorChangingAttr=Đã xảy ra lỗi khi đổi thuộc tính của tệp hiện có:
ErrorCreatingTemp=Đã xảy ra lỗi khi tạo tệp trong thư mục đích:
ErrorReadingSource=Đã xảy ra lỗi khi đọc tệp nguồn:
ErrorCopying=Đã xảy ra lỗi khi sao chép tệp:
ErrorDownloading=Đã xảy ra lỗi khi tải tệp xuống:
ErrorExtracting=Đã xảy ra lỗi khi giải nén kho lưu trữ:
ErrorReplacingExistingFile=Đã xảy ra lỗi khi thay thế tệp hiện có:
ErrorRestartReplace=RestartReplace thất bại:
ErrorRenamingTemp=Đã xảy ra lỗi khi đổi tên tệp trong thư mục đích:
ErrorRegisterServer=Không thể đăng ký DLL/OCX: %1
ErrorRegSvr32Failed=RegSvr32 thất bại với mã thoát %1
ErrorRegisterTypeLib=Không thể đăng ký thư viện kiểu: %1

; *** Uninstall display name markings
; used for example as 'My Program (32-bit)'
UninstallDisplayNameMark=%1 (%2)
; used for example as 'My Program (32-bit, All users)'
UninstallDisplayNameMarks=%1 (%2, %3)
UninstallDisplayNameMark32Bit=32-bit
UninstallDisplayNameMark64Bit=64-bit
UninstallDisplayNameMarkAllUsers=Tất cả người dùng
UninstallDisplayNameMarkCurrentUser=Người dùng hiện tại

; *** Post-installation errors
ErrorOpeningReadme=Đã xảy ra lỗi khi mở tệp README.
ErrorRestartingComputer=Trình cài đặt không thể khởi động lại máy tính. Hãy tự khởi động lại.

; *** Uninstaller messages
UninstallNotFound=Tệp "%1" không tồn tại. Không thể gỡ cài đặt.
UninstallOpenError=Không thể mở tệp "%1". Không thể gỡ cài đặt
UninstallUnsupportedVer=Tệp nhật ký gỡ cài đặt "%1" có định dạng mà phiên bản trình gỡ cài đặt này không nhận ra. Không thể gỡ cài đặt
UninstallUnknownEntry=Gặp một mục không xác định (%1) trong nhật ký gỡ cài đặt
ConfirmUninstall=Bạn có chắc muốn gỡ bỏ hoàn toàn %1 và tất cả các thành phần của nó không?
UninstallOnlyOnWin64=Bản cài đặt này chỉ có thể được gỡ trên Windows 64-bit.
OnlyAdminCanUninstall=Bản cài đặt này chỉ có thể được gỡ bởi người dùng có quyền quản trị.
UninstallStatusLabel=Vui lòng đợi trong khi %1 được gỡ khỏi máy tính của bạn.
UninstalledAll=%1 đã được gỡ khỏi máy tính của bạn thành công.
UninstalledMost=Đã gỡ cài đặt xong %1.%n%nMột số thành phần không thể gỡ bỏ. Bạn có thể gỡ chúng thủ công.
UninstalledAndNeedsRestart=Để hoàn tất việc gỡ cài đặt %1, cần khởi động lại máy tính của bạn.%n%nBạn có muốn khởi động lại ngay bây giờ không?
UninstallDataCorrupted=Tệp "%1" bị hỏng. Không thể gỡ cài đặt

; *** Uninstallation phase messages
ConfirmDeleteSharedFileTitle=Xóa tệp dùng chung?
ConfirmDeleteSharedFile2=Hệ thống cho biết tệp dùng chung sau không còn được chương trình nào sử dụng. Bạn có muốn trình gỡ cài đặt xóa tệp dùng chung này không?%n%nNếu vẫn có chương trình đang dùng tệp này mà nó bị xóa, các chương trình đó có thể không hoạt động đúng. Nếu bạn không chắc, hãy chọn “Không”. Giữ tệp lại trên hệ thống sẽ không gây hại.
SharedFileNameLabel=Tên tệp:
SharedFileLocationLabel=Vị trí:
WizardUninstalling=Trạng thái gỡ cài đặt
StatusUninstalling=Đang gỡ cài đặt %1...

; *** Shutdown block reasons
ShutdownBlockReasonInstallingApp=Đang cài đặt %1.
ShutdownBlockReasonUninstallingApp=Đang gỡ cài đặt %1.

; The custom messages below aren't used by Setup itself, but if you make
; use of them in your scripts, you'll want to translate them.

[CustomMessages]

NameAndVersion=%1 phiên bản %2
AdditionalIcons=Lối tắt bổ sung:
CreateDesktopIcon=Tạo lối tắt trên màn hình nền
CreateQuickLaunchIcon=Tạo lối tắt Quick Launch
ProgramOnTheWeb=%1 trên web
UninstallProgram=Gỡ cài đặt %1
LaunchProgram=Chạy %1
AssocFileExtension=Liên kết %1 với phần mở rộng tệp %2
AssocingFileExtension=Đang liên kết %1 với phần mở rộng tệp %2...
AutoStartProgramGroupDescription=Khởi động:
AutoStartProgram=Tự động khởi động %1
AddonHostProgramNotFound=Không tìm thấy %1 trong thư mục bạn đã chọn.%n%nBạn vẫn muốn tiếp tục chứ?

; The messages that are Lang Switcher's own (the sample is English.isl).
LocalCertReady=Trình cài đặt sẽ tạo một chứng chỉ trên máy tính này và ký Lang Switcher bằng chứng chỉ đó, để chương trình hoạt động được cả trong cửa sổ của các chương trình chạy với quyền quản trị.
LocalCertFailed=Lang Switcher đã được cài đặt, nhưng sẽ không hoạt động trong cửa sổ của các chương trình chạy với quyền quản trị: Windows không chấp nhận chứng chỉ được tạo trên máy tính này. Xem chi tiết trong nhật ký cài đặt.
RemoveSettingsQuestion=Xóa cả tệp cấu hình của Lang Switcher?
RemoveSettingsKeep=Chọn “Không” để giữ các cài đặt của bạn cho lần cài lại sau.
AuthorSlogan=chương trình làm bằng cả tấm lòng
