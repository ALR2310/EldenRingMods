<div align="center">

# FasterRevival

**Rút ngắn thời gian từ lúc hết máu đến khi hồi sinh.**

![Rust](https://img.shields.io/badge/Rust-cdylib-orange?logo=rust)
![Platform](https://img.shields.io/badge/platform-Windows_x64-blue)
![Game](https://img.shields.io/badge/game-Elden_Ring-8a6d3b)

</div>

---

## Giới thiệu

Khi chết, Elden Ring bắt bạn xem hết animation ngã, đợi màn "YOU DIED" hiện, rồi đợi
thêm một khoảng dừng trước khi hồi sinh. FasterRevival cắt hai khoảng chờ đó, không
sửa file animation (`.anibnd`, `.tae`) nên không xung đột với các mod animation khác.

| | Thời gian từ lúc hết máu đến khi hồi sinh |
|---|---|
| Vanilla | 12,2 - 14,4 giây |
| Có mod | khoảng 4,2 - 4,7 giây |

Mod **luôn bật**: cài vào là chạy, không có gì phải chỉnh.

## Cách hoạt động

Hai việc độc lập, cộng lại ra mức tiết kiệm trên.

| Việc | Cách làm | Mã |
|---|---|---|
| **Kéo mốc "chết" lên sớm** | Game chỉ "giết" nhân vật bằng một sự kiện gần cuối animation chết, nên "YOU DIED" hiện muộn. Mod gọi thẳng hàm giết của game ngay khi HP về 0 và animation chết bắt đầu; animation vẫn phát bình thường. | [src/death.rs](src/death.rs) |
| **Bỏ khoảng dừng sau "YOU DIED"** | Đặt thời gian chờ trước khi mờ màn hình (`soloPlayDeath_ToFadeOutTime`, mặc định 3,8 giây) về 0, một lần, khi nhân vật đã vào thế giới. Không cần patch code. | [src/fade.rs](src/fade.rs) |

Hàm giết của game được tìm bằng AOB, chạy được trên 2.7.1.0 và 2.6.2.0. Mod chỉ giết
khi animation đang phát là kiểu chết có sự kiện đó, nên không làm sai các kiểu chết khác.

## Cấu hình

`FasterRevival.ini` cạnh DLL (tự tạo ở lần chạy đầu) chỉ có hai việc: ghi log, và chế
độ dò animation chết để chẩn đoán (ghi từng bước của animation vào log, mặc định tắt). Tên
key và ý nghĩa nằm ở chú thích trong [FasterRevival.ini](FasterRevival.ini). Mod không có
phím reload: đổi file thì khởi động lại game.

Mỗi lần chết đều có dòng log ghi thời gian từ lúc hết máu đến khi game ghi nhận chết và
đến khi hồi sinh, nên so sánh thời gian không cần bật chế độ dò.

## Giới hạn

- Chỉ tác dụng với kiểu chết có animation chứa sự kiện đó (chết khi bị đánh, đòn tóm).
  Rơi vực không đổi vì game giết ngay từ frame đầu.
- Chưa kiểm tra khi đang cưỡi Torrent.
- Mod không kiểm tra một điều kiện nội bộ của game trước khi giết (nằm trong vùng bảo
  vệ chống can thiệp, chưa hiểu), nên có thể giết sai lúc trong trường hợp hiếm mà điều
  kiện đó từng chặn.
- Không dời sự kiện "Send Ghost Info" của mod gốc: dùng mod DLL phải tắt EAC nên không
  có bloodstain hay ghost để gửi.
- Nếu game cập nhật làm mất mẫu AOB của hàm giết, log ghi lỗi và phần giết sớm bị tắt;
  phần bỏ khoảng dừng vẫn chạy.

## Nguồn ý tưởng

- **FasterDeathAnimation** của 0-F ([Nexus 3367](https://www.nexusmods.com/eldenring/mods/3367),
  [GitHub](https://github.com/0-F/FasterDeathAnimation))
- **FasterRespawn** của ImAxel0 ([Nexus 501](https://www.nexusmods.com/eldenring/mods/501),
  [GitHub](https://github.com/ImAxel0/EldenRing-FasterRespawn-Mod))

## Tài liệu

| Tài liệu | Nội dung |
|---|---|
| [docs/death_flow.md](docs/death_flow.md) | Luồng chết của game, điều kiện kích hoạt, kết quả test |
| [docs/respawn_fade.md](docs/respawn_fade.md) | Khoảng dừng sau "YOU DIED" |
| [HISTORY.md](HISTORY.md) | Dòng thời gian thay đổi |
| [CHANGELOG.md](CHANGELOG.md) | Ghi chú phát hành cho người dùng |
