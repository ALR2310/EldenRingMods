# conflict: phát hiện hook của mod khác và báo người dùng

**Status 2026-10-22: chỉ `weightmultiplier` dùng. Đã thử trong game với `ZibaPeso.dll` (cùng hook vòng lặp trọng lượng): phát hiện đúng, hộp thoại hiện, hook của mod được bỏ qua; chưa nêu được tên DLL (stub của ZibaPeso không có con trỏ về DLL của nó trong 128 byte đầu, hoặc hook cách khác), chấp nhận vì mục tiêu chính là phát hiện để không làm hỏng tính năng. Code ở `shared/src/conflict.rs` (có test cho việc nhận dạng lệnh nhảy).**

## Vì sao có

Hai mod ghi đè cùng một chỗ trong code game thì mod nạp sau làm hỏng mod nạp
trước, thường chỉ thấy game crash hoặc tính năng không chạy. Trước đây
`weightmultiplier` có `report_foreign_patch` riêng, chỉ ghi log. Người dùng
không đọc log, và người dùng báo lỗi lên Nexus thường không biết mod nào gây
ra. Giờ mod tự nói: log nêu tên DLL kia, và một hộp thoại bảo người dùng chọn
một trong ba hướng (tắt mod kia, tắt mod của mình, báo lên trang Nexus kèm log).
Từ báo cáo đó thử nối chuỗi với mod kia, được thì thêm hỗ trợ, không thì ghi
"không tương thích" vào `docs/` của mod.

## Cách hoạt động

- `foreign_hook_at(site)`: đọc 13 byte tại chỗ sắp patch, nhận ra `E9 rel32`,
  `FF 25 rel32` (nhảy qua con trỏ) và `mov r64, imm64; jmp r64` (kể cả
  `41 FF E0+r`). Đích nhảy nằm trong exe game thì là code của game, không tính
  là hook.
- `foreign_hook(site, dest)`: cho nơi đã tự tìm ra lệnh nhảy bằng pattern riêng
  (`weightmultiplier` quét `FOREIGN_PATCH_PATTERN` khi anchor không khớp).
- `owner` lấy từ `diag::module_containing(dest)`, chỉ tên file, không đường dẫn
  (log đăng công khai, xem `diag.md`).
- Thường đích nhảy là stub mod kia tự cấp phát bằng `VirtualAlloc` gần exe (thử
  với ZibaPeso 2026-10-22: đích `exe base - 0x10000`), không thuộc module nào.
  Khi đó `owner_from_stub` quét 128 byte đầu của stub, lấy module đầu tiên không
  phải exe game, không phải DLL hệ thống mà một con trỏ 8 byte hoặc `E8`/`E9
  rel32` trong stub trỏ vào (stub gọi hoặc nhảy về DLL của mod đó). Đây là suy
  đoán, log ghi `from its stub`. Stub chỉ nhảy ngược vào game thì không ra tên,
  hộp thoại ghi "another mod".
- `report(...)`: ghi 1 dòng ERROR (có địa chỉ site, đích nhảy, owner và 12 byte
  đầu của site, để thấy hook của mod kia trông ra sao khi người dùng báo lỗi)
  rồi xếp conflict vào hộp thoại nếu `show_box`.

## Hộp thoại

- Một hộp **mỗi mod**, tiêu đề `<tên mod> conflict`. Conflict đầu tiên mở cửa
  sổ gom 2 giây (`COLLECT_WINDOW`), mọi conflict báo trong lúc đó vào cùng hộp.
  Học từ Solid Uncapper, vốn gộp danh sách tính năng không áp dụng được vào một
  hộp cuối giai đoạn khởi động. Hai mod khác nhau cùng conflict thì ra hai hộp,
  tiêu đề khác nhau (mỗi DLL link bản `common` riêng nên danh sách chờ là static
  riêng của từng mod, không cần mutex đặt tên).
- Một conflict: một câu (`box_text`): "<tính năng> of <mod> conflicts with <DLL
  hoặc another mod> and has been turned off. See <file log> for details."
  Nhiều conflict: "N features of <mod> conflict with other mods and have been
  turned off:" rồi mỗi dòng `- <tính năng>: <DLL hoặc another mod>`. Không có
  phần đề xuất xử lý; chi tiết nằm trong log.
- Chạy trên thread riêng: hộp modal trên thread gọi có thể làm game treo hoặc
  kẹt loader lock. `MB_TOPMOST | MB_SETFOREGROUND` để không nằm sau cửa sổ game.
- Mỗi mod có khoá `ConflictMessageBox` (mặc định `true`) trong file config để
  tắt hộp.

## Giới hạn

- Chỉ phát hiện hook có **trước** khi mod patch. Mod khác hook sau (ghi đè byte
  của mình hoặc nối vào jmp của mình) thì phía mình không biết.
- Chưa có `expected bytes` cho từng chỗ patch: chỉ kiểm tra "bắt đầu bằng lệnh
  nhảy". Byte bị đổi theo cách khác (ví dụ `overwrite_bytes` của mod khác) vẫn
  qua. Các mod dùng `redirect_rel32` đã tự nối chuỗi (xem `codepatch.md`), không
  qua kiểm tra này.
- Chưa phân loại lý do chi tiết như Solid Uncapper (đầu hàm còn nguyên, bị jmp
  gần, jmp xa, đổi không phải jmp, đọc không được): log chỉ ghi các byte tìm
  thấy. Chưa có tìm hàm bằng chữ ký đuôi để vẫn chạy khi đầu hàm bị hook, chưa
  kiểm tra pattern duy nhất, chưa phát hiện hiệu ứng đã có sẵn ở tầng param.
- Hộp thoại dùng `MessageBoxW`: nếu antivirus gắn cờ sau thay đổi này (xem `mods/animcancel/docs/antivirus.md`), cân nhắc quét lại bằng
  `scripts/vt-scan.py`.
