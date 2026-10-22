<div align="center">

# RuneMultiplier

**Nhân hệ số lên số rune nhận được từ mọi nguồn.**

![Rust](https://img.shields.io/badge/Rust-cdylib-orange?logo=rust)
![Platform](https://img.shields.io/badge/platform-Windows_x64-blue)
![Game](https://img.shields.io/badge/game-Elden_Ring-8a6d3b)

</div>

---

## Giới thiệu

RuneMultiplier nhân số rune bạn **nhận được** lên một hệ số tuỳ chọn: giết quái, nhặt hoặc
dùng vật phẩm, bán đồ đều tính. Rune **chi tiêu** (lên cấp, mua đồ) không bị ảnh hưởng. Chỉ có
một thiết lập chính, nên cài vào là dùng được ngay.

## Tính năng

| Hệ số | Hiệu ứng |
|---|---|
| lớn hơn 1 | nhận nhiều rune hơn (mặc định x2) |
| nhỏ hơn 1 | nhận ít rune hơn |
| số âm | rune nhận được bị **trừ** thay vì cộng |

Sai số tối đa ±1 rune mỗi lần nhận. Đổi hệ số xong bấm phím reload là áp dụng ngay, không cần
khởi động lại game.

## Cấu hình

Cấu hình nằm trong `RuneMultiplier.ini` cạnh DLL, tự tạo ở lần chạy đầu. Tên key, giá trị mặc
định và ý nghĩa nằm ở chú thích trong [RuneMultiplier.ini](RuneMultiplier.ini), file mẫu nhúng
sẵn trong DLL. Phím reload cũng đổi được và có hiệu lực ngay.

### Menu trong game (tùy chọn)

Nếu [ERNativeUI](https://github.com/Flammrock/ERNativeUI) được nạp trước mod, mục **Rune
Multiplier** xuất hiện trong System > Game Options, gồm ô nhập hệ số (gõ được số lẻ, 0 và số âm,
trong khoảng -1000 đến 1000) và công tắc ghi log. Đổi trong menu áp ngay và được ghi vào file.
Không có ERNativeUI thì mod chạy bằng file như bình thường.

## Cách hoạt động

Mod chỉ có **một hook**, vào hàm cộng rune thấp nhất của game (`AddSoul_Call`, dùng chung cho
mọi nguồn):

1. Tìm hàm bằng một mẫu AOB neo (duy nhất trên 2.6.2.0, 2.7.0.0 và 2.7.1.0).
2. Thay 12 byte đầu của hàm bằng một lệnh nhảy tới một đoạn mã tự sinh.
3. Đoạn mã nhân số rune cộng thêm bằng số học cố định Q20, chạy lại đúng 3 lệnh gốc rồi nhảy
   về hàm.

Đoạn mã chỉ nhân khi số cộng thêm **dương**, nên các khoản trừ (lên cấp, mua đồ) không bị nhân.
Hệ số được đọc qua con trỏ lúc chạy, nên reload chỉ cập nhật một biến mà không patch lại. Chi
tiết, các hướng đã bỏ và cách kiểm tra tương thích: [docs/addsoul_hook.md](docs/addsoul_hook.md).

## Giới hạn

- Phụ thuộc mẫu AOB neo: nếu game vá làm đổi bố cục thì hook không cài được, log ghi lỗi và mod
  tắt cho phiên đó (không patch gì).
- Mọi nguồn rune nhận được đều bị nhân, không tách riêng được từng nguồn.
- Menu trong game chỉ nhập hệ số trong khoảng -1000 đến 1000; ngoài khoảng đó thì sửa file.

## Tài liệu

| Tài liệu | Nội dung |
|---|---|
| [docs/addsoul_hook.md](docs/addsoul_hook.md) | Hook `AddSoul_Call`, các hướng đã bỏ, tương thích |
| [HISTORY.md](HISTORY.md) | Dòng thời gian thay đổi |
| [CHANGELOG.md](CHANGELOG.md) | Ghi chú phát hành cho người dùng |

Mã nguồn: [src/hook.rs](src/hook.rs) (hook), [src/menu.rs](src/menu.rs) (menu).
