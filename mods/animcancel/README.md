# AnimCancel

> **Nexus mod ID 11328** (chưa phát hành bản đầu) · đã test trong game: đòn đánh, weapon art và phép

Mod DLL cho Elden Ring cho phép **cắt animation đang chạy** bằng một hành động
khác mà không phải đợi anim chạy hết: guard cancel, dodge cancel, đổi vũ khí
giữa đòn đánh. Yêu cầu từ người dùng Nexus (DAM127, 2026-10-12).

## Cấu hình

Cấu hình trong `AnimCancel.toml` cạnh DLL (tự tạo nếu thiếu). Có thể chỉnh:

- hành động nào được cắt animation: guard, lăn/lùi, đổi vũ khí;
- animation nào bị cắt: đòn đánh thường, weapon art, phép, dùng item (uống flask...);
- phím reload config trong game và banner báo reload;
- ghi log để chẩn đoán, và hai probe nghiên cứu (log cửa sổ cancel, log cờ anim; mặc định tắt).

Tên key, giá trị mặc định và ý nghĩa nằm ở chú thích trong
[`AnimCancel.toml`](AnimCancel.toml) (nhúng vào DLL, nguồn sự thật duy nhất).
Đổi file rồi bấm phím reload, không cần khởi động lại game.

## Menu trong game (tùy chọn)

Nếu `ERNativeUI.dll` (host menu dùng chung, https://github.com/Flammrock/ERNativeUI) đã được nạp trước mod, System >
Game Options có thêm hàng **Anim Cancel**: mở ra trang gồm nhóm **Cancel With** (guard, lăn/lùi, đổi vũ khí), nhóm
**Cancel What** (đòn đánh, weapon art, phép, item) và bật/tắt log. Đổi trong menu được ghi vào `AnimCancel.toml` (giữ
nguyên comment) sau khi ngừng thao tác 1 giây; mod đọc cấu hình mỗi frame nên có hiệu lực ngay. Hai probe chỉ chỉnh trong
file. Không có host thì mod chạy bằng file như trước. Host chỉ hỗ trợ exe 2.7.0.0 và 2.7.1.0. Client: `common::erui`
([shared/src/erui.rs](../../shared/src/erui.rs)), mod: [src/menu.rs](src/menu.rs), ghi chú dùng chung:
[docs/ingame_menu.md](../../docs/ingame_menu.md).

## Cách hoạt động

Game chỉ cho cắt anim trong "cửa sổ cancel" do event của anim bật, thường là
nửa sau của đòn (lăn được sau ~30% đòn kiếm). Mod mở cửa sổ đó sớm hơn cho người
chơi: mỗi frame, khi anim đang chạy thuộc nhóm được bật, nó ghi quyền cancel vào
hàng đợi theo từng anim của `CSChrActionRequestModule`, đúng chỗ HKS đọc. Không
dùng AOB hay RVA, chỉ struct trong `fromsoftware-rs` và `common`. Cơ chế của game,
vì sao phải ghi đúng chỗ đó và kết quả các lần test:
[docs/animation_cancel.md](docs/animation_cancel.md).

## Giới hạn

- Chỉ người chơi local; chưa xét tương thích Seamless Co-op.
- Đã test: kiếm, một sorcery (Loretta's Greatbow) và một incantation (Placidusax's Ruin). Chưa thử khiên hay vũ khí hai tay đặc biệt.
- Cắt giữa chừng có thể bỏ lại i-frame hay hitbox của anim cũ (chưa kiểm tra kỹ).

Code: [src/cancel.rs](src/cancel.rs) (logic), [src/config.rs](src/config.rs),
[src/lib.rs](src/lib.rs). Phân loại anim: `common::anim`. Lịch sử:
[HISTORY.md](HISTORY.md). Ghi chú phát hành: [CHANGELOG.md](CHANGELOG.md); trang Nexus:
[nexus_page.bbcode](nexus_page.bbcode).
