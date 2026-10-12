# AnimCancel

> **nội bộ, đang nghiên cứu** (chưa đăng Nexus, chưa có mod ID) · probe đã chứng minh guard, lăn và đổi vũ khí huỷ được đòn đánh

Mod DLL cho Elden Ring nhằm cho phép **cắt animation** (guard cancel, dodge
cancel, đổi vũ khí giữa đòn đánh). Yêu cầu từ người dùng Nexus (DAM127,
2026-10-12). Hiện là **probe nghiên cứu**, chưa phải tính năng cho người chơi:
đã chứng minh làm được bằng DLL (test trong game 2026-10-12), còn phải tách
thành mod thật.

## Hiện làm gì

- **Log cửa sổ cancel**: mỗi khi animation hoặc các mask "hành động nào được
  phép cắt" của người chơi đổi, ghi một dòng vào log (anim, vị trí trong anim,
  các mask). Cho thấy đòn nào của game vốn đã có cửa sổ cancel nào.
- **Ép cửa sổ cancel**: một phím bật/tắt, khi bật thì các hành động đã chọn
  (guard, lăn, đổi vũ khí...) luôn được phép cắt animation hiện tại. Dùng để thử
  xem HKS của game có chuyển state giữa chừng hay không.

Tên key, giá trị mặc định và ý nghĩa nằm ở chú thích trong
[`AnimCancel.ini`](AnimCancel.ini) (nhúng vào DLL, nguồn sự thật duy nhất).
Đổi ini cần khởi động lại game (chưa có hot reload).

## Cách hoạt động

Ghi vào `CSChrActionRequestModule` của người chơi (các mask cancel, hàng đợi
theo từng state `cancel_entries`, cờ `cancel_disable`) qua task của game, ở task group
chọn được. Không dùng AOB hay RVA, chỉ struct trong `fromsoftware-rs`. Cơ chế
của game, các điều chưa biết và kế hoạch thử:
[docs/animation_cancel.md](../../docs/animation_cancel.md).

## Giới hạn

- Đang ép trong mọi anim, chưa lọc theo loại anim nguồn.
- Chưa thử với cast, item, khiên hay vũ khí hai tay đặc biệt.
- Chỉ người chơi local; chưa xét tương thích Seamless Co-op.

Code: [src/probe.rs](src/probe.rs) (toàn bộ logic), [src/lib.rs](src/lib.rs).
Lịch sử: [HISTORY.md](HISTORY.md).
