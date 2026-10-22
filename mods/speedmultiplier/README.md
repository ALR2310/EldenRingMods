<div align="center">

# SpeedMultiplier

**Tăng hoặc giảm tốc độ của người chơi, Torrent, spirit và kẻ địch theo từng nhóm hành động.**

![Rust](https://img.shields.io/badge/Rust-cdylib-orange?logo=rust)
![Platform](https://img.shields.io/badge/platform-Windows_x64-blue)
![Game](https://img.shields.io/badge/game-Elden_Ring-8a6d3b)

</div>

---

## Giới thiệu

SpeedMultiplier đổi tốc độ animation của nhân vật theo hệ số bạn chọn, riêng cho từng loại
hành động, nên có thể chạy nhanh hơn mà không đánh nhanh hơn, hay niệm phép nhanh hơn mà
không lăn nhanh hơn. Mod còn cho đặt tốc độ khác nhau tuỳ theo hiệu ứng đang có trên người
hoặc lớp tải trọng.

## Tính năng

| Đối tượng | Chỉnh được |
|---|---|
| **Người chơi** | một hệ số tổng cho mọi hành động, hoặc từng nhóm: đi bộ, chạy, đi lén, nhảy, lăn, leo thang, đòn thường, đòn chí mạng, kỹ năng vũ khí, niệm phép, dùng item, hành động khác |
| **Torrent** | một hệ số tổng, hoặc đi bộ, chạy, nhảy, hành động khác |
| **Spirit đã gọi** | một hệ số chung |
| **Kẻ địch và boss** | một hệ số chung (không gồm động vật hiền, NPC, thương nhân) |

Hệ số tổng khác 1 thì ghi đè các hệ số riêng của đối tượng đó. Tốc độ luôn được giữ trong
khoảng 0,1 đến 10.

**Ghi đè theo điều kiện:** các khối `[[Override]]` đặt tốc độ khác khi người chơi đang có một
hiệu ứng (SpEffect) nào đó, hoặc đang ở một lớp tải trọng (Light, Medium, Heavy,
Overweight), hoặc cả hai. Nhiều khối khớp cùng lúc thì xếp chồng, khối nằm sau thắng.

## Cấu hình

Mod dùng **TOML** (`SpeedMultiplier.toml` cạnh DLL, tự tạo ở lần chạy đầu), không phải ini
như phần lớn mod khác. Khi nâng cấp, key mới được thêm vào file có sẵn mà không ghi đè giá
trị bạn đã chỉnh; key không còn dùng được chuyển thành comment ở cuối file. Key gõ sai hoặc
sai kiểu được báo kèm số dòng và **cấu hình đang chạy được giữ nguyên**, không bị thay bằng
file lỗi. Tên key và ý nghĩa nằm ở chú thích trong
[SpeedMultiplier.toml](SpeedMultiplier.toml), file mẫu nhúng sẵn trong DLL. Bấm phím reload
để áp dụng, không cần khởi động lại game.

### Menu trong game (tùy chọn)

Nếu [ERNativeUI](https://github.com/Flammrock/ERNativeUI) được nạp trước mod, mục **Speed
Multiplier** xuất hiện trong System > Game Options, gồm hàng **All Player Actions** và các
trang **Player Movement**, **Player Actions**, **Torrent**, **Spirit & Enemy**, cùng công tắc
ghi log. Các hàng tốc độ là ô gõ số (như `1.5` hoặc `1,5`, giới hạn 0,1 đến 10). Khối
`[[Override]]` chỉ sửa trong file. Không có ERNativeUI thì mod chạy bằng file như bình thường.

## Cách hoạt động

Mỗi frame mod đặt trường `animation_speed` của nhân vật theo nhóm của animation đang chạy.
Chỉ ghi khi giá trị khác, vì game không tự reset trường này.

- **Phân nhóm:** phép và kỹ năng vũ khí theo tiền tố của tên TAE, mọi thứ còn lại theo phần đuôi
  của mã animation. Bảng phân nhóm và các trường hợp đặc biệt:
  [docs/action_groups.md](docs/action_groups.md).
- **Ghi đè theo điều kiện:** mỗi frame mod đọc SpEffect đang có trên người chơi và lớp tải
  trọng, rồi tính tốc độ cuối từ cấu hình gốc cộng các khối ghi đè khớp.
- **Tương thích Seamless Co-op:** Seamless hook hàm đọc tốc độ animation của game nên giá trị
  mod ghi không được đọc; mod phát hiện điều đó và chuyển hướng hook về phía mình
  ([docs/seamless.md](docs/seamless.md)).

## Giới hạn

- Game chỉ có **một** tốc độ animation cho cả nhân vật, nên không tăng riêng được nửa thân
  trên: uống bình khi đang chạy theo nhóm di chuyển, khi đứng yên theo nhóm item.
- Đòn chí mạng và đòn tóm đi cặp với animation của nạn nhân; mod cho nạn nhân chạy theo tốc
  độ của người chơi, nhưng nhóm đòn chí mạng mặc định giữ tốc độ gốc và tăng nhiều sẽ lệch nhịp.
- Tia của Placidusax's Ruin chạy theo đồng hồ riêng của game nên không theo tốc độ niệm phép.
- Trong co-op, hình ảnh không đồng bộ giữa hai máy: mỗi máy chỉ tăng tốc nhân vật của chính nó.
  Tốc độ spirit và kẻ địch chưa hỗ trợ Seamless Co-op.
- Animation đi bộ khi quá tải nằm ngoài dải của nhóm đi bộ nên không được áp tốc độ đó; tải
  trọng theo phần trăm liên tục chưa hỗ trợ (chỉ theo lớp).
- Hệ số tổng cũng ảnh hưởng lúc bị đánh, cưỡi ngựa, nghỉ ở grace.

## Tài liệu

| Tài liệu | Nội dung |
|---|---|
| [docs/animation_speed_research.md](docs/animation_speed_research.md) | Các "núm" tốc độ, SpeedProbe, giới hạn của game |
| [docs/action_groups.md](docs/action_groups.md) | Phân nhóm hành động |
| [docs/enemy_speed.md](docs/enemy_speed.md) | Tốc độ kẻ địch và boss, cách lọc |
| [docs/critical_victim.md](docs/critical_victim.md) | Nạn nhân của đòn chí mạng chạy theo tốc độ người chơi |
| [docs/config_toml.md](docs/config_toml.md) | Cấu hình TOML, ghi đè theo điều kiện, migration |
| [docs/seamless.md](docs/seamless.md) | Sự cố và cách sửa với Seamless Co-op |
| [docs/tae-ids.md](docs/tae-ids.md) | Bảng ID TAE của Elden Ring |
| [HISTORY.md](HISTORY.md) | Dòng thời gian thay đổi |
| [CHANGELOG.md](CHANGELOG.md) | Ghi chú phát hành cho người dùng |

Mã nguồn: [src/speed.rs](src/speed.rs) (phân nhóm và ghi tốc độ), [src/config.rs](src/config.rs),
[src/enemy.rs](src/enemy.rs), [src/seamless.rs](src/seamless.rs), [src/probe.rs](src/probe.rs)
(công cụ dò, tắt mặc định).
