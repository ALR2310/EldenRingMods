# RiseArcher: dòng thời gian

Mỗi dòng một thay đổi, mới nhất ở dưới cùng. Chuyện nào có phân tích dài thì
link sang `docs/`. Bản nhật ký chi tiết cũ (trước 2026-10-07) nằm trong lịch
sử git của `README.md`.

| Ngày | Thay đổi | Chi tiết |
|---|---|---|
| trước 2026-08-18 | 1.16.1: sửa tĩnh `regulation.bin` bằng Smithbox Mass Edit (`csv/RiseArcher.MASSEDIT`), overhaul bow/crossbow/ballista/arrow/bolt | [docs](docs/param_patching.md) |
| 2026-08-18 | 2.0.0: viết lại thành DLL Rust (patch sống `SoloParamRepository`, cấu hình bằng ini), giữ song song cách `.MASSEDIT`; `weapon.rs` (lọc theo category) và `bullet.rs` (danh sách ID ghép bảng offset) | [docs](docs/param_patching.md) |
| 2026-09-08 | Thêm hot reload (`ReloadKey`) với baseline giá trị gốc để không cộng dồn | [docs](docs/param_patching.md) |
| 2026-09-08 | Sửa crash lặng khi khởi động: chờ `WorldChrMan::main_player` thay vì chỉ chờ `SoloParamRepository`; thêm `apply_with_retry` | [docs](docs/startup_and_ini_bugs.md) |
| 2026-09-08 | Sửa `RiseArcher.ini` không có tác dụng (key không tiền tố); tổ chức lại section theo loại tuỳ chỉnh, tách key trọng lượng/giá bán từng loại, `Bow.UnlockAOW` | [docs](docs/startup_and_ini_bugs.md) |
| 2026-09-17 | Dùng `common::task`/`common::player`/`common::reload` thay ba bản chép riêng; chờ `SoloParamRepository` vô hạn thay vì bỏ cuộc sau 5 phút | [docs](docs/startup_and_ini_bugs.md) |
| 2026-09-23 | `LogFile=false` không còn tạo file log (sửa `common::logger`; mod này vốn đã đúng hành vi đó) | |
| 2026-09-24 | Sửa đường dẫn DLL có ký tự không phải ASCII làm mod bỏ qua ini (`common::dll_dir`) | |
| 2026-09-26 | Log: `[INFO ]` thành `[INFO]`; thêm phiên bản game và danh sách DLL đã nạp (`common::diag`) | |
| 2026-10-05 | Version thật trong thuộc tính file DLL (`build.rs` + `winresource`); thêm key `ReloadBanner` | |
| 2026-10-07 | Đổi thư mục `crates/` thành `mods/`; tách README thành README + HISTORY + docs/ | |
| 2026-10-13 | Mặc định `ReloadBanner` đổi thành `false`: nhiều mod cùng bật thì mỗi mod hiện một banner vài giây khi bấm reload | |
| 2026-10-13 | Thêm menu trong game tùy chọn (ERNativeUI, `src/menu.rs`): hàng nhập số (gõ số lẻ) cho các hệ số, slider cho giới hạn đạn và số mũi tên Rain; đổi trong menu đi qua `RELOAD_GENERATION` nên vẫn tính lại từ baseline, không cộng dồn | [docs](../../docs/ingame_menu.md) |
| 2026-10-15 | Chữ của menu trong game chuyển sang `common::i18n` (`locales/en.json`, `vi.json`); host để tiếng Việt thì menu hiện tiếng Việt, ngôn ngữ khác lùi về tiếng Anh | [docs](../../shared/docs/i18n.md) |
| 2026-10-15 | Menu trong game: các hàng Damage, Scaling, Weight, Sell Value, Projectile Speed, Rain Range/Spread (0..10 hoặc 0..5) thành slider native khi host có API 1.2 (ERNativeUI >= 1.2), phạm vi slider hẹp hơn phạm vi mod chấp nhận (ô gõ và tệp cấu hình vẫn nhận số lớn); chưa thử trong game | [docs](../../docs/ingame_menu.md) |
| 2026-10-15 | Sửa param qua `common::params::for_each_row_mut` / `row_ids` thay cho `rows_mut()` / `get_mut()` (panic trên regulation bị mod); `project.json` (có đường dẫn máy) chuyển thành `project.json.example` và bị git-ignore | [docs](../../shared/docs/params.md) |
| 2026-10-16 | Log gọn lại theo kiểu ngắn, kỹ thuật cho dev: bỏ dòng lặp ý, địa chỉ patch, câu dài; chỉ ghi 1 lần khi chờ thế giới | |
| 2026-10-19 | Sửa lỗi phím reload lọt ra ngoài game: `common::reload` đọc bằng `common::input` (`GetAsyncKeyState`, chỉ nhận phím khi cửa sổ game có focus) thay cho `GetKeyState` của `eldenring`, và antivirus gắn ít cờ hơn (RuneMultiplier 6 xuống 2 cờ trên VirusTotal); chưa test phím reload trong game | [docs](../../shared/docs/reload.md) |
