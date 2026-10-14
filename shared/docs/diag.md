# diag: dump môi trường vào log báo lỗi

**Status 2026-10-13: hoạt động, mọi mod gọi `log_environment_when_game_ready()` (hoặc `log_environment()`) lúc khởi động. Code ở `shared/src/diag.rs`, có test cho riêng việc rút gọn đường dẫn.**

## Vì sao có

Thêm 2026-09-26 sau một báo cáo WeightMultiplier mà log chỉ nói "anchor pattern
not found": không có cách nào biết người dùng đang ở bản game khác hay có mod
trọng tải khác cùng patch một chỗ. Dạng dump mô phỏng phần đầu log của
MapForGoblins. Giờ log tự trả lời cả hai câu hỏi:

Danh sách `Loaded modules` ghi tên, version, base, size của exe game (dòng đầu
tiên) và của từng DLL không thuộc hệ thống. Version file của exe (vd.
`v2.7.1.0`) cho biết bản game.

Lịch sử: ban đầu exe được liệt kê trong danh sách, rồi tách ra thành một dòng
`Game:` riêng (kèm `TimeDateStamp` để nhận đúng bản build kể cả khi version không
đổi), rồi 2026-10-13 đưa về lại danh sách cho gọn mắt. `TimeDateStamp` không còn
được log; trường `ModuleInfo::timestamp` vẫn còn nếu cần.

## Danh sách bỏ gì

Bỏ DLL nằm dưới thư mục Windows và `HIDDEN_MODULES`: DLL đi kèm game và DLL
Steam client chèn vào. Phần còn lại là
proxy loader (dinput8/winmm/dxgi), mod loader và mod, đúng thứ cần xem khi hai
mod xung đột.

## Quyền riêng tư (2026-09-26)

Người dùng được yêu cầu đăng log công khai (bình luận Nexus), nên log không
được lộ nhiều hơn mức cần:

- Module trong thư mục game in theo đường dẫn tương đối (`modengine2\bin\lua.dll`,
  cho biết luôn DLL thuộc loader nào).
- Module ngoài thư mục game chỉ in tên file, vì đường dẫn đầy đủ có thể chứa
  tên tài khoản Windows (`C:\Users\<tên>\...`).
- `steam_api64.dll` và `OnlineFix64.dll` bị ẩn dù chúng gợi ý bản crack: người
  dùng không nên phải tiết lộ điều đó để được hỗ trợ về một mod.
- So sánh thư mục game theo độ dài byte, không `to_lowercase()` hai vế: đổi chữ
  thường với đường dẫn không phải ASCII có thể đổi độ dài, và cắt chuỗi theo độ
  dài sai sẽ panic đúng ở những đường dẫn từng làm hỏng `dll_dir`.

## Khi nào gọi

Gọi sau khi các mod khác đã nạp, không gọi thẳng từ `DllMain`.
`log_environment_when_game_ready` chờ `CSTaskImp` (lúc đó mọi mod loader đã nạp
xong DLL) rồi mới ghi, chạy ở thread khởi động của mod ngay trước khi tính năng
bắt đầu. Nhiều tính năng chỉ tới `wait_for_cs_task` của riêng chúng khi người
chơi đã vào thế giới, quá muộn cho báo cáo kiểu "mod không chạy". Việc chờ được
cache, nên lần gọi sau của tính năng trả về ngay.

## Hàm tra cứu

- `module_containing(addr)`: module chứa một địa chỉ, để gọi tên DLL mà một
  `jmp` lạ trong code game đang trỏ tới. Chỉ đọc PE header để tìm module, rồi mới
  đọc version của đúng module đó.
- `is_module_loaded(name)`: kiểm tra một DLL theo tên (vd. `ersc.dll`) bằng
  `GetModuleHandleW`.
- `loaded_modules()`: đọc cả tài nguyên version của mọi module, chậm hơn; chỉ
  dùng khi thật sự cần cả danh sách.
