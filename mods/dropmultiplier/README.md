<div align="center">

# DropMultiplier

**Chỉnh tỉ lệ rơi đồ của quái và số lượng nguyên liệu nhận được.**

![Rust](https://img.shields.io/badge/Rust-cdylib-orange?logo=rust)
![Platform](https://img.shields.io/badge/platform-Windows_x64-blue)
![Game](https://img.shields.io/badge/game-Elden_Ring-8a6d3b)

</div>

---

## Giới thiệu

DropMultiplier chỉnh hai thứ độc lập: **tỉ lệ rơi đồ** của quái và **số lượng nguyên
liệu** nhận được mỗi lần nhặt. Mod sửa thẳng bảng dữ liệu của game khi đang chạy, không
ghi đè `regulation.bin`, nên dùng được cả với các bản mod lớn như Convergence. Đổi cấu
hình xong bấm phím reload là áp dụng ngay.

## Tính năng

### Tỉ lệ rơi đồ

Chọn một trong hai cách (không dùng cả hai cùng lúc):

| Cách | Hiệu ứng | Ví dụ |
|---|---|---|
| **Nhân hệ số** | nhân tỉ lệ rơi của mọi món | món rơi 1% với hệ số x3 thành 3% |
| **Tỉ lệ cố định** | ép tỉ lệ "có rơi đồ" của mỗi con quái về một giá trị | đặt 50: món 1% và món 100% đều thành 50% |

Đặt tỉ lệ cố định là 100 để quái luôn rơi đồ. Khi ép tỉ lệ cố định, tỉ lệ tương đối giữa
các món của cùng một con quái được giữ nguyên.

### Số lượng nguyên liệu

Nhân số lượng nhận được, tách theo ba nhóm với hệ số riêng:

| Nhóm | Gồm | Ví dụ |
|---|---|---|
| **Crafting** | nguyên liệu chế tạo có thể cày | Rowa Fruit, Herba, bướm, xương, nấm |
| **Upgrade** | nguyên liệu nâng cấp có thể cày | Smithing Stone, Gloveworts |
| **Unique** | nguyên liệu chỉ nhặt được một lần | Smithing Stone trên xác, Sacred Tear, Scadutree Fragment |

Cả đồ rơi từ quái lẫn đồ hái ngoài thế giới mở đều được tính. Vũ khí, vật phẩm khoá và
đồ tiêu thụ không bị đụng tới.

## Cấu hình

Cấu hình nằm trong `DropMultiplier.ini` cạnh DLL, tự tạo ở lần chạy đầu. Khi nâng cấp,
key mới được thêm vào file có sẵn mà không ghi đè giá trị bạn đã chỉnh. Tên key, giá trị
mặc định và ý nghĩa nằm ở chú thích trong [DropMultiplier.ini](DropMultiplier.ini), file
mẫu nhúng sẵn trong DLL. Bấm phím reload để áp dụng, không cần khởi động lại game.

### Menu trong game (tùy chọn)

Nếu [ERNativeUI](https://github.com/Flammrock/ERNativeUI) được nạp trước mod, mục
**Drop Multiplier** xuất hiện trong System > Game Options, gồm trang **Drop Chance**,
trang **Materials** và công tắc ghi log. Không có ERNativeUI thì mod chạy bằng file như
bình thường.

## Cách hoạt động

Mỗi con quái có một bảng "item lot" gồm tối đa 8 ô; mỗi ô có một trọng số và xác suất
của ô là trọng số của nó chia cho tổng trọng số của bảng. Mod đổi các trọng số này, và
với nguyên liệu thì đổi số lượng của ô. Trọng số và số lượng gốc được chụp một lần trước
mọi chỉnh sửa, nên mỗi lần reload tính lại từ bản gốc và không bao giờ cộng dồn.

Mod duyệt các bảng theo thứ tự dòng thay vì tra theo ID, vì cách tra của thư viện sẽ
panic trên regulation có header lệch như Convergence. Công thức và các trường hợp biên:
[docs/drop_rate.md](docs/drop_rate.md).

## Giới hạn

- Chỉ chỉnh **tỉ lệ cơ bản**; chỉ số Discovery của người chơi vẫn cộng thêm như vanilla.
- Tỉ lệ cố định có thể biến một món vốn chắc chắn rơi (100%) thành rơi theo xác suất.
- Chỉ ảnh hưởng bảng rơi đồ của quái (tỉ lệ và số lượng) và bảng đồ hái ngoài thế giới
  (số lượng nguyên liệu); các bảng khác giữ nguyên.

## Tài liệu

| Tài liệu | Nội dung |
|---|---|
| [docs/drop_rate.md](docs/drop_rate.md) | Mô hình bốc thăm của game, hai chế độ, công thức tỉ lệ cố định |
| [docs/materials.md](docs/materials.md) | Thiết kế và phân loại nguyên liệu |
| [docs/convergence_panic.md](docs/convergence_panic.md) | Panic với Convergence và cách sửa |
| [HISTORY.md](HISTORY.md) | Dòng thời gian thay đổi |
| [CHANGELOG.md](CHANGELOG.md) | Ghi chú phát hành cho người dùng |

Mã nguồn: [src/drop_rate.rs](src/drop_rate.rs) (tỉ lệ rơi), [src/materials.rs](src/materials.rs)
(nguyên liệu), [src/lot.rs](src/lot.rs) (đọc/ghi từng ô), [src/menu.rs](src/menu.rs).
