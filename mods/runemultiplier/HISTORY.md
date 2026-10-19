# RuneMultiplier: dòng thời gian

Mỗi dòng một thay đổi, mới nhất ở dưới cùng. Chuyện nào có phân tích dài thì
link sang `docs/`. Bản nhật ký chi tiết cũ (trước 2026-10-07) nằm trong lịch
sử git của `README.md`.

| Ngày | Thay đổi | Chi tiết |
|---|---|---|
| 2026-08-14 | Bản C++: ban đầu có 2 hook, giết quái bị nhân 2 lần (64 ra 256); bỏ hook ở bước nhân thưởng, giữ 1 hook `AddSoul_Call`; test trong game ổn | [docs](docs/addsoul_hook.md) |
| 2026-08-18 | 1.0.0: viết lại bằng Rust; jump tuyệt đối thay `E9 rel32` (bỏ `CodePatch.cpp`); ini/log/hotkey dùng `common`; reload bằng task `FrameBegin` | [docs](docs/addsoul_hook.md) |
| 2026-08-19 | 1.0.1: sửa việc nhân luôn cả rune chi tiêu (stub bỏ qua `amount <= 0`); đổi key ini `RuneMultiplier` thành `Multiplier`, `HotReloadKey` thành `ReloadKey` (mặc định `F9` thành `F5`) | [docs](docs/addsoul_hook.md) |
| 2026-08-26 | Pin `fromsoftware-rs` 0.14.0; chờ `CSTaskImp` có retry khi gặp `InvalidRva` (trước đó lỗi này tắt reload vĩnh viễn); tick reload bọc `catch_unwind` | |
| 2026-08-31 | 1.0.2: cập nhật cho ER 1.17 | |
| 2026-09-09 | 1.0.3: cập nhật cho ER 1.17.1 | |
| 2026-09-14 | Đổi `[Debug]`/`DebugLog` thành `[Logging]`/`LogFile` cho thống nhất với các mod khác | |
| 2026-09-23 | `LogFile` do `common::logger` tự gate (mod này vốn đã đúng hành vi đó, bỏ điều kiện riêng) | |
| 2026-09-24 | 1.0.4: sửa đường dẫn DLL có ký tự không phải ASCII làm mod bỏ qua ini (`common::dll_dir`) | |
| 2026-09-26 | Dòng lỗi `[INFO ] ERROR:` chuyển sang `logger::error`; `[INFO ]` thành `[INFO]`; thêm phiên bản game và danh sách DLL vào log (`common::diag`) | |
| 2026-10-05 | Version thật trong thuộc tính file DLL (`build.rs` + `winresource`) | |
| 2026-10-07 | Dùng `common::task::wait_for_cs_task` và `run_recurring_safe` thay hai bản chép riêng trong `hook.rs` (-65 dòng); task reload giờ đăng ký bằng AOB của `common::task_hook` thay vì `rva::get()` khoá theo phiên bản game; bỏ dependency `fromsoftware-shared` không còn dùng | |
| 2026-10-07 | Dùng `common::reload`: thêm banner "Config reloaded" và key `ReloadBanner` (mặc định `true`) như các mod khác; mod chỉ còn theo dõi `RELOAD_GENERATION` để cập nhật hệ số | |
| 2026-10-07 | Đổi thư mục `crates/` thành `mods/`; tách README thành README + HISTORY + docs/; kiểm tra bằng IDA: mẫu AOB neo duy nhất trên 2.6.2.0, 2.7.0.0, 2.7.1.0 | [docs](docs/addsoul_hook.md) |
| 2026-10-13 | Menu trong game tùy chọn qua ERNativeUI (`common::erui`, `src/menu.rs`): hàng "Rune Multiplier" ở System > Game Options mở trang với hàng nhập số cho hệ số (-1000..1000) và công tắc ghi log; đổi trong menu áp ngay và ghi lại ini; thiếu host thì mod chạy như cũ | [docs](../../docs/ingame_menu.md) |
| 2026-10-13 | Mặc định `ReloadBanner` đổi thành `false`: nhiều mod cùng bật thì mỗi mod hiện một banner vài giây khi bấm reload | |
| 2026-10-15 | Chữ của menu trong game chuyển sang `common::i18n` (`locales/en.json`, `vi.json`): menu hiện tiếng Việt khi game chạy ngôn ngữ Việt, còn lại lùi về tiếng Anh | [docs](../../shared/docs/i18n.md) |
| 2026-10-15 | Menu trong game: hệ số Multiplier (-10..10, bước 0.5) thành slider native khi host có API 1.2 (ERNativeUI >= 1.2), phạm vi slider hẹp hơn phạm vi mod chấp nhận (ô gõ và tệp cấu hình vẫn nhận số lớn); chưa thử trong game | [docs](../../docs/ingame_menu.md) |
| 2026-10-15 | Reload qua `common::reload` + banner coi như đã kiểm: các mod khác dùng cùng cơ chế và chạy ổn | |
| 2026-10-19 | Sửa lỗi phím reload lọt ra ngoài game: `common::reload` đọc bằng `common::input` (`GetAsyncKeyState`, chỉ nhận phím khi cửa sổ game có focus) thay cho `GetKeyState` của `eldenring`, và antivirus gắn ít cờ hơn (RuneMultiplier 6 xuống 2 cờ trên VirusTotal); chưa test phím reload trong game | [docs](../../shared/docs/reload.md) |
