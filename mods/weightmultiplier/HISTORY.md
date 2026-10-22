# WeightMultiplier: dòng thời gian

Mỗi dòng một thay đổi, mới nhất ở dưới cùng. Chuyện nào có phân tích dài thì
link sang `docs/`. Bản nhật ký chi tiết cũ (trước 2026-10-07) nằm trong lịch
sử git của `README.md`.

| Ngày | Thay đổi | Chi tiết |
|---|---|---|
| trước 2026-08-18 | Bản C++ **ReductionWeight** (1.0.0, 1.1.0 cho phép giá trị âm): tìm điểm hook qua 3 lần thử, chốt `movaps xmm0,xmm6` sau vòng lặp cộng trọng lượng | [docs](docs/hook_point.md) |
| 2026-08-18 | Đổi tên thành WeightMultiplier, viết lại bằng Rust; giữ JMP tương đối 5 byte qua `common::codepatch` vì chỉ có 7 byte để ghi đè; không dùng `fromsoftware-rs`, không có hotkey reload | [docs](docs/hook_point.md) |
| 2026-09-14 | Đổi hẳn bộ ini (`Mode`/`Multiplier`/`FixedValue`, thêm chế độ đặt cứng, không tương thích với `WeightReductionPercent`); thêm `ReloadKey` (thăm dò phím từ thread riêng) và `[Logging] LogFile`; `InitialDelaySeconds` thành `LoadDelay` (mili giây) | [docs](docs/hook_point.md) |
| 2026-09-21 | 2.0.0 phát hành: bản Rust, chế độ giá trị cố định, hotkey reload | |
| 2026-09-23 | `LogFile=false` không còn tạo file log (sửa `common::logger`) | |
| 2026-09-24 | 2.0.1: sửa đường dẫn DLL có ký tự không phải ASCII (`common::dll_dir`); `ReloadKey` chỉ nhận khi cửa sổ game đang focus (sửa `common::input::is_key_pressed`) | |
| 2026-09-26 | Dòng lỗi `[INFO ] ERROR:` chuyển sang `logger::error`/`warn`; `[INFO ]` thành `[INFO]`; log phiên bản game và danh sách DLL, nhận ra mod khác hook đè (sau báo lỗi "anchor not found" trên Nexus) | [docs](docs/hook_point.md) |
| 2026-10-05 | Version thật trong thuộc tính file DLL (`build.rs` + `winresource`) | |
| 2026-10-07 | Đổi thư mục `crates/` thành `mods/`; tách README thành README + HISTORY + docs/; kiểm tra bằng IDA: mẫu neo duy nhất và cùng RVA `0x247C9E` trên 2.6.2.0, 2.7.0.0, 2.7.1.0 | [docs](docs/hook_point.md) |
| 2026-10-13 | Thêm menu tùy chọn trong game qua ERNativeUI (`common::erui`): chế độ, hệ số và giá trị cố định (gõ số vào ô văn bản), log. Menu ghi ini rồi gọi đúng hàm reload sẵn có (`reload_weight_value`) từ thread của bộ lưu; thiết kế không dùng task game giữ nguyên. | [docs](../../docs/ingame_menu.md) |
| 2026-10-15 | Chữ của menu trong game chuyển sang `common::i18n` (`locales/en.json`, `vi.json`): menu hiện tiếng Việt khi game chạy tiếng Việt, ngôn ngữ khác lùi về tiếng Anh | [docs](../../shared/docs/i18n.md) |
| 2026-10-15 | Menu trong game: hệ số Multiplier (0..5, bước 0.05) thành slider native khi host có API 1.2 (ERNativeUI >= 1.2), phạm vi slider hẹp hơn phạm vi mod chấp nhận (ô gõ và tệp cấu hình vẫn nhận số lớn); chưa thử trong game | [docs](../../docs/ingame_menu.md) |
| 2026-10-20 | Phím reload chuyển sang `common::reload` (task của game) thay cho thread thăm dò riêng; thêm tuỳ chọn `ReloadBanner` như các mod khác | [docs](../../shared/docs/reload.md) |
| 2026-10-22 | Phát hiện hook của mod khác chuyển sang `common::conflict`: log nêu tên DLL kia, thêm hộp thoại cảnh báo (luôn bật) và kiểm tra thêm chính chỗ patch; thay `report_foreign_patch` chỉ ghi log; thử trong game với ZibaPeso: hộp thoại hiện, chưa ra được tên DLL vì đích nhảy là stub ngoài module | [docs](../../shared/docs/conflict.md) |
| 2026-10-22 | Log gọn lại theo kiểu ngắn, kỹ thuật cho dev: bỏ dòng lặp ý, địa chỉ patch, câu dài; chỉ ghi 1 lần khi chờ thế giới | |
| 2026-10-22 | Dọn mã: doc đầu `hook.rs` và các comment dài về điểm hook chuyển sang `docs/hook_point.md`, bỏ các guard `LogFile` thừa, `reload_from_config` gọn lại, thêm test cho opcode theo chế độ và hình dạng của stub, log gọn lại | |
