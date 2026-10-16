# DropMultiplier: dòng thời gian

Mỗi dòng một thay đổi, mới nhất ở dưới cùng. Chuyện nào có phân tích dài thì
link sang file trong `docs/` của mod. Bản nhật ký chi tiết cũ (trước 2026-10-07) nằm trong
lịch sử git của `README.md`.

| Ngày | Thay đổi | Chi tiết |
|---|---|---|
| 2026-09-14 | Tách `Drop Rate` khỏi `sometweaks` thành mod độc lập (1.0.0): cùng thuật toán, bỏ tiền tố `DropRate.` trong key ini, dùng chung `common` (task/player/reload), không mang theo rủi ro `rva::get()` khóa theo phiên bản của bản gốc | |
| 2026-09-17 | Bỏ tiền tố `DropMultiplier:` trong log; sửa `common::player::wait_for_solo_param_repository` chờ vô hạn thay vì bỏ cuộc sau 5 phút (lỗi thật: vào world muộn thì mod không tự áp) | |
| 2026-09-17 | Banner "Config reloaded" khi bấm `ReloadKey` (`common::announce`) | |
| 2026-09-23 | `LogFile=false` không còn tạo file log (sửa `common::logger`) | |
| 2026-09-24 | Sửa đường dẫn DLL có ký tự không phải ASCII làm mod bỏ qua ini (`common::dll_dir` dùng `GetModuleFileNameW`) | |
| 2026-09-26 | Log: `[INFO ]` thành `[INFO]`; thêm phiên bản game và danh sách DLL đã nạp (`common::diag`) | |
| 2026-09-27 | 1.0.2: sửa panic với Convergence, duyệt param theo index qua `common::params` | [docs](docs/convergence_panic.md) |
| 2026-09-28 | 1.1.0: thêm `[Materials]` (Crafting/Upgrade/Unique); đổi `[Settings]` thành `[Drop]` và `ChancePercent` thành `Percentage` (tự chuyển giá trị cũ) | [docs](docs/materials.md) |
| 2026-10-05 | Version thật trong thuộc tính file DLL (`build.rs` + `winresource`); thêm key `ReloadBanner` | |
| 2026-10-07 | Đổi thư mục `crates/` thành `mods/`; tách README thành README + HISTORY + docs/ | |
| 2026-10-07 | Thêm `CHANGELOG.md` (gộp từ khối changelog của trang Nexus) và gỡ khối đó khỏi `nexus_page.bbcode` (đổi tên từ `DESCRIPTION.bbcode`) | [CHANGELOG](CHANGELOG.md) |
| 2026-10-13 | Mặc định `ReloadBanner` đổi thành `false`: nhiều mod cùng bật thì mỗi mod hiện một banner vài giây khi bấm reload | |
| 2026-10-13 | Menu trong game tùy chọn qua ERNativeUI (`src/menu.rs`): chế độ (lựa chọn), hệ số và 3 hệ số nguyên liệu (hàng nhập số), tỉ lệ cố định (thanh trượt 0-100), log; đổi trong menu ghi vào ini rồi kích hoạt lại đường reload (`RELOAD_GENERATION`) một lần sau khi ngừng thao tác. Reload yêu cầu khi chưa vào thế giới (vd. đổi ở menu màn hình chính) giờ được giữ lại thay vì bị bỏ | [docs](../../docs/ingame_menu.md) |
| 2026-10-15 | Chữ của menu trong game chuyển sang hệ đa ngôn ngữ (`locales/en.json`, `vi.json`, `common::i18n`): hiện tiếng Việt khi game chạy tiếng Việt, ngôn ngữ khác dùng tiếng Anh; thêm test khoá khớp giữa `menu.rs` và 2 file ngôn ngữ | [docs](../../shared/docs/i18n.md) |

| 2026-10-15 | Menu trong game: các hệ số Multiplier/Crafting/Upgrade/Unique (0..10, bước 0.5) thành slider native khi host có API 1.2 (ERNativeUI >= 1.2), phạm vi slider hẹp hơn phạm vi mod chấp nhận (ô gõ và tệp cấu hình vẫn nhận số lớn); chưa thử trong game | [docs](../../docs/ingame_menu.md) |
| 2026-10-15 | Bỏ việc nhân đồ hái Erdleaf Flower, Trina's Lily (LordSoulOfNito, 2026-10-02): không có thêm phản hồi từ người báo; `[Materials]` vẫn chỉ nhận goodsType 2 và 14 | [docs](docs/materials.md) |
| 2026-10-16 | Log gọn lại theo kiểu ngắn, kỹ thuật cho dev: bỏ dòng lặp ý, địa chỉ patch, câu dài; chỉ ghi 1 lần khi chờ thế giới | |
