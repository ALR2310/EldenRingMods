<div align="center">

# RiseArcher

**Buff toàn diện cho Bow, Crossbow, Ballista, mũi tên và tên nỏ.**

![Rust](https://img.shields.io/badge/Rust-cdylib-orange?logo=rust)
![Platform](https://img.shields.io/badge/platform-Windows_x64-blue)
![Game](https://img.shields.io/badge/game-Elden_Ring-8a6d3b)

</div>

---

## Giới thiệu

RiseArcher làm cho các vũ khí bắn tầm xa đáng dùng hơn: tăng sát thương và scaling, mở
khoá Ash of War cho cung, giảm trọng lượng, tăng giá bán, tăng số đạn mang theo, và làm
Rain of Arrows nhanh hơn, xa hơn, nhiều mũi tên hơn. Mọi hệ số đều chỉnh được.

## Tính năng

Hệ số của DLL, chỉnh riêng cho từng loại vũ khí khi hợp lý (giá trị mặc định):

| | Bow | Crossbow | Ballista |
|---|---|---|---|
| **Sát thương** | x1,5 | x3 | x4 |
| **Scaling** | x2 | | |
| **Mở khoá Ash of War** | có | | |
| **Trọng lượng** | x0,5 | x0,5 | x0,5 |
| **Giá bán** | x2 | x2 | x2 |

| Đạn | Mặc định |
|---|---|
| **Số mũi tên / tên nỏ mang tối đa** | 255 |
| **Tốc độ đạn** | x3,5 |
| **Tầm Rain of Arrows** | x2 |
| **Độ loe Rain of Arrows** | x2 |
| **Số mũi tên Rain of Arrows** | 6 |

## Hai cách dùng, cùng tồn tại

| Cách | Mô tả |
|---|---|
| **DLL** (`src/`, khuyến nghị) | Sửa sống bảng dữ liệu trong bộ nhớ lúc chạy, cấu hình bằng `RiseArcher.ini`. Không đụng `regulation.bin` trên đĩa nên không xung đột với mod `regulation.bin` khác. |
| **Sửa tĩnh `regulation.bin` qua Smithbox** (`csv/`) | Cách làm gốc, giữ nguyên: `csv/RiseArcher.MASSEDIT` và các file CSV dán vào Smithbox Mass Edit. Hữu ích khi cần merge tay với một mod `regulation.bin` cụ thể hoặc không muốn cài DLL. Dùng `project.json.example` (chép thành `project.json` rồi sửa `GameRoot`). |

Hai cách này độc lập và không tự đồng bộ: hệ số mặc định của DLL được lấy từ `.MASSEDIT`
nên hiện khớp nhau, nhưng sửa một bên không cập nhật bên kia.

## Cấu hình

Cấu hình của DLL nằm trong `RiseArcher.ini` cạnh DLL, tự tạo ở lần chạy đầu. Khi nâng cấp,
key mới được thêm vào file có sẵn mà không ghi đè giá trị bạn đã chỉnh. Tên key và ý nghĩa
nằm ở chú thích trong [RiseArcher.ini](RiseArcher.ini), file mẫu nhúng sẵn trong DLL. Bấm
phím reload để áp dụng, không cần khởi động lại game.

### Menu trong game (tùy chọn)

Nếu [ERNativeUI](https://github.com/Flammrock/ERNativeUI) được nạp trước mod, mục
**Rise Archer** xuất hiện trong System > Game Options, gồm ba trang: **Damage** (sát thương,
scaling, mở khoá Ash of War), **Weight & Value** (trọng lượng, giá bán) và **Ammo & Shots**
(số đạn mang tối đa, tốc độ, tầm, độ loe, số mũi tên Rain of Arrows), cùng công tắc ghi log.
Không có ERNativeUI thì mod chạy bằng file như bình thường.

## Cách hoạt động

- **Vũ khí:** lọc bảng `EquipParamWeapon` theo `weaponCategory` và `wepType` (bỏ qua các dòng
  chỉ dành cho NPC), rồi nhân hoặc gán các trường tương ứng.
- **Đạn:** bảng `Bullet` không có trường để lọc, nên mod dùng danh sách ID gốc ghép với bảng
  offset cố định cho các biến thể Ash of War của từng họ mũi tên.
- **Không cộng dồn:** giá trị gốc của mỗi dòng được lưu ngay lần áp đầu, mỗi lần reload tính
  lại từ đó.
- **Chờ vào thế giới:** mod đợi người chơi thật sự vào thế giới mới đụng dữ liệu, vì sớm hơn thì
  bảng chưa nạp xong và đọc sẽ panic.

Chi tiết: [docs/param_patching.md](docs/param_patching.md).

## Giới hạn

- Danh sách ID đạn gắn với regulation vanilla: game cập nhật hoặc bản mod lớn đổi bố cục ID
  thì mod ghi cảnh báo "không tìm thấy ID" (không crash) và cần dò lại danh sách.
- Phép scale số nguyên (sát thương, số lượng tối đa) làm tròn khác Smithbox Mass Edit có thể
  lệch ±1.

## Tài liệu

| Tài liệu | Nội dung |
|---|---|
| [docs/param_patching.md](docs/param_patching.md) | Bộ lọc vũ khí, họ đạn, giá trị gốc, rủi ro |
| [docs/startup_and_ini_bugs.md](docs/startup_and_ini_bugs.md) | Crash lặng khi khởi động, ini không tác dụng, bỏ cuộc sau 5 phút |
| [HISTORY.md](HISTORY.md) | Dòng thời gian thay đổi |
| [CHANGELOG.md](CHANGELOG.md) | Ghi chú phát hành cho người dùng |

Mã nguồn: [src/weapon.rs](src/weapon.rs) (vũ khí), [src/bullet.rs](src/bullet.rs) (đạn),
[src/lib.rs](src/lib.rs), [src/menu.rs](src/menu.rs).
