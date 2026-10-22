<div align="center">

# WeightMultiplier

**Nhân Trọng Tải (equip load) với một hệ số, hoặc đặt cứng nó bằng một số.**

![Rust](https://img.shields.io/badge/Rust-cdylib-orange?logo=rust)
![Platform](https://img.shields.io/badge/platform-Windows_x64-blue)
![Game](https://img.shields.io/badge/game-Elden_Ring-8a6d3b)

</div>

---

## Giới thiệu

WeightMultiplier đổi tổng Trọng Tải của người chơi theo hệ số bạn chọn, thay vì ghi đè thành
một con số tuỳ ý như các mod "NoWeight" thông thường. Một hook duy nhất, áp cho cả số hiển
thị trên giao diện lẫn hành vi thật (roll, chạy). Mod đổi tên từ **ReductionWeight** khi
chuyển sang Rust, vì hệ số nhỏ hơn 1 giảm tải trọng còn lớn hơn 1 thì tăng.

## Tính năng

Hai chế độ, chọn một:

| Chế độ | Hiệu ứng | Ví dụ |
|---|---|---|
| **Multiplier** (mặc định) | nhân tổng tải với một hệ số (cho phép số âm) | hệ số 0,5 với tải thật 100 hiện 50 |
| **FixedValue** | đặt tổng tải bằng đúng một số, bất kể bạn mặc gì | đặt 0 để luôn nhẹ |

Đổi chế độ, hệ số hoặc giá trị cố định rồi bấm phím reload là áp dụng ngay, không cần khởi
động lại game.

## Cấu hình

Cấu hình nằm trong `WeightMultiplier.ini` cạnh DLL, tự tạo ở lần chạy đầu. Tên key, giá trị
mặc định và ý nghĩa nằm ở chú thích trong [WeightMultiplier.ini](WeightMultiplier.ini), file
mẫu nhúng sẵn trong DLL. Mod còn có một độ trễ sau khi game khởi động trước khi áp dụng, và
phím reload chỉ nhận khi cửa sổ game đang được focus.

### Menu trong game (tùy chọn)

Nếu [ERNativeUI](https://github.com/Flammrock/ERNativeUI) được nạp trước mod, mục **Weight
Multiplier** xuất hiện trong System > Game Options, gồm chế độ, hệ số, giá trị cố định (gõ
số vào ô, hệ số có thể là số thập phân) và công tắc ghi log. Đổi trong menu được ghi vào file
rồi áp dụng giống bấm phím reload. Không có ERNativeUI thì mod chạy bằng file như bình thường.

## Cách hoạt động

Game cộng trọng lượng 5 ô trang bị trong một vòng lặp rồi sao chép tổng ra bằng lệnh
`movaps xmm0,xmm6`. Mod vá đúng lệnh này, chạy một lần sau vòng lặp, nên nhân hay ghi đè ở
đó cho một con số sạch, không bị dồn qua từng ô.

| Bước | Cách làm |
|---|---|
| **Tìm điểm vá** | Quét AOB `FF C3 83 FB ?? 7C ?? 4C 8D 5C 24 ??` rồi đi 12 byte. Mẫu duy nhất trên 2.6.2.0, 2.7.0.0 và 2.7.1.0, cùng một RVA ở cả ba. |
| **Vá** | Lệnh chỉ có 7 byte nên dùng lệnh nhảy tương đối 5 byte tới một đoạn mã cấp phát gần đó (`common::codepatch`). Đoạn mã nhân hoặc ghi đè `xmm6` rồi chạy lại hai lệnh gốc. |
| **Reload** | Đổi giá trị chỉ cần ghi một biến mà đoạn mã đọc mỗi lần chạy. Đổi chế độ thì vá lại 4 byte opcode trong đoạn mã (`mulss` thành `movss` hoặc ngược lại). |

Nếu không tìm thấy neo, mod kiểm tra xem có phải một mod trọng lượng khác đã hook cùng chỗ
không, và ghi tên DLL đó vào log. Lịch sử tìm điểm hook (hai điểm thử trước đó đều sai) và
bảng tương thích: [docs/hook_point.md](docs/hook_point.md).

## Giới hạn

- Đây là patch code sống: nếu game đổi bố cục vùng này, mod không tìm thấy neo và tự tắt
  cho phiên đó (log ghi lỗi).
- Đổi chế độ lúc đang chạy ghi đè code có thể đang thực thi. Nếu vá thất bại, mod giữ chế độ
  cũ.
- Nếu một mod trọng lượng khác đã hook cùng chỗ, mod này không cài được hook (có ghi log).

## Tài liệu

| Tài liệu | Nội dung |
|---|---|
| [docs/hook_point.md](docs/hook_point.md) | Lịch sử tìm điểm hook, đoạn mã, tương thích |
| [HISTORY.md](HISTORY.md) | Dòng thời gian thay đổi |
| [CHANGELOG.md](CHANGELOG.md) | Ghi chú phát hành cho người dùng |

Mã nguồn: [src/hook.rs](src/hook.rs) (hook), [src/menu.rs](src/menu.rs) (menu).
