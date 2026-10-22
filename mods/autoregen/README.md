<div align="center">

# AutoRegen

**Tự động hồi HP, FP và Stamina: theo thời gian hoặc khi đánh trúng.**

![Rust](https://img.shields.io/badge/Rust-cdylib-orange?logo=rust)
![Platform](https://img.shields.io/badge/platform-Windows_x64-blue)
![Game](https://img.shields.io/badge/game-Elden_Ring-8a6d3b)

</div>

---

## Giới thiệu

AutoRegen hồi HP, FP và Stamina cho người chơi theo hai cách độc lập, có thể bật cùng
lúc: **hồi theo thời gian** (mỗi chu kỳ, có điều kiện) và **hồi khi đánh trúng**.
Mọi con số đều chỉnh được, và đổi cấu hình xong bấm phím reload là áp dụng ngay.

## Tính năng

### Hồi theo thời gian (Per Tick)

Mỗi chu kỳ, mod hồi một lượng cho HP, FP và Stamina, khi điều kiện được chọn thoả:

| Điều kiện | Hồi khi |
|---|---|
| **Luôn luôn** | mọi lúc |
| **Ngoài giao tranh** | không đánh và không bị đánh trong 15 giây gần nhất |
| **Trong giao tranh** | vừa đánh hoặc bị đánh trong 15 giây gần nhất |
| **Đứng yên** | không di chuyển, không giữ hành động nào trong 5 giây |
| **Đang làm gesture** | đang ngồi (hoặc gesture khác trong danh sách tuỳ chỉnh) |

Lượng hồi tính theo ba cách: **điểm cố định**, **% máu tối đa**, hoặc **% phần đã mất**
(hồi nhanh khi thấp, chậm dần khi gần đầy). Có **trần hồi phục** để mỗi tick không hồi
quá một mức cho trước.

### Hồi khi đánh trúng (Per Hit)

Mỗi đòn người chơi đánh trúng hồi một lượng, tính theo **điểm cố định**, **% tối đa**
hoặc **% sát thương gây ra**. Có thể chọn loại đòn được tính (cận chiến, phép / tầm
xa, hoặc cả hai) và loại trừ Weapon Art / Ash of War.

## Cấu hình

Cấu hình nằm trong `AutoRegen.ini` cạnh DLL, tự tạo ở lần chạy đầu. Khi nâng cấp, key
mới được thêm vào file có sẵn mà không ghi đè giá trị bạn đã chỉnh; key đã đổi tên
được dồn xuống mục `[Legacy]`. Tên key, giá trị mặc định và ý nghĩa nằm ở chú thích
trong [AutoRegen.ini](AutoRegen.ini), file mẫu nhúng sẵn trong DLL. Bấm phím reload để
áp dụng, không cần khởi động lại game.

### Menu trong game (tùy chọn)

Nếu [ERNativeUI](https://github.com/Flammrock/ERNativeUI) được nạp trước mod, mục
**Auto Regen** xuất hiện trong System > Game Options, gồm trang **Per Tick**, trang
**Per Hit** và công tắc ghi log. Lượng hồi và chu kỳ là ô nhập số (gõ được số lẻ).
Danh sách gesture chỉ sửa trong file. Không có ERNativeUI thì mod chạy bằng file như
bình thường.

## Cách hoạt động

- **Hồi theo thời gian:** một task chạy mỗi frame trên nhóm `FrameBegin` của game,
  đọc và ghi HP/FP/Stamina qua `CSChrDataModule` của
  [fromsoftware-rs](https://github.com/vswarte/fromsoftware-rs).
- **Hồi khi đánh trúng:** một hook vào hàm hit-resolution của game chỉ quan sát (ai
  đánh ai, sát thương bao nhiêu); lượng hồi vẫn do mod ghi vào HP của người chơi.
- **Gesture:** chỉ tính là đang ngồi khi animation thật sự chuyển sang trạng thái đó
  và giữ ổn định, nên gesture bị chặn giữa lúc cast phép không làm mod hồi nhầm.

## Giới hạn

- "Đang giao tranh" chỉ là xấp xỉ: đứng yên né đòn boss không tính là giao tranh.
- Phép có động tác cận chiến (vd. Carian Greatsword) bị tính là đòn cận chiến.
- Loại trừ Ash of War dựa trên nút bấm gần nhất: Ash hai bước như Unsheathe (`L2` rồi
  `R1`/`R2`) có thể bị tính là đòn thường.
- Hook hit mới được xác minh trên một bản game (2.7.1.0). Nếu không tìm thấy thì chỉ
  tắt phần hồi khi đánh trúng, hồi theo thời gian vẫn chạy.

## Tài liệu

| Tài liệu | Nội dung |
|---|---|
| [docs/per_hit.md](docs/per_hit.md) | Hook hồi khi đánh trúng, phân loại đòn, loại trừ Ash of War, các lỗi đã gặp |
| [docs/gesture_trigger.md](docs/gesture_trigger.md) | Điều kiện đứng yên và gesture, các hướng đã thử |
| [docs/task_and_startup.md](docs/task_and_startup.md) | Khởi động, đăng ký tick bằng AOB, banner, log |
| [docs/cpp_origin.md](docs/cpp_origin.md) | Dịch ngược `AutoRecovery.dll` và bản C++ cũ (lịch sử) |
| [HISTORY.md](HISTORY.md) | Dòng thời gian thay đổi |
| [CHANGELOG.md](CHANGELOG.md) | Ghi chú phát hành cho người dùng |

Mã nguồn: [src/regen.rs](src/regen.rs) (vòng tick), [src/heal.rs](src/heal.rs) (tính
lượng hồi), [src/trigger.rs](src/trigger.rs) (giao tranh / đứng yên / gesture),
[src/hit_hook.rs](src/hit_hook.rs), [src/menu.rs](src/menu.rs).
