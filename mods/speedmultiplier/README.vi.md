# SpeedMultiplier

[English](README.md)

Tăng hoặc giảm tốc độ của người chơi, Torrent, spirit và kẻ địch, riêng cho từng loại hành động.

## Tóm tắt

SpeedMultiplier đổi tốc độ animation của nhân vật theo hệ số bạn chọn, riêng cho từng loại hành động:
chạy nhanh hơn mà không đánh nhanh hơn, hay niệm phép nhanh hơn mà không lăn nhanh hơn. Mod còn cho đặt
tốc độ khác nhau khi người chơi đang có một hiệu ứng hoặc mang một mức tải trọng.

| Đối tượng | Chỉnh được |
|---|---|
| **Người chơi** | một hệ số cho mọi hành động, hoặc từng nhóm: đi bộ, chạy, đi lén, nhảy, lăn, leo thang, đòn thường, đòn chí mạng, kỹ năng vũ khí, niệm phép, dùng item, hành động khác |
| **Torrent** | một hệ số cho mọi thứ, hoặc đi bộ, chạy, nhảy, hành động khác |
| **Spirit đã gọi** | một hệ số chung |
| **Kẻ địch và boss** | một hệ số chung (không gồm động vật hiền, NPC, thương nhân) |

Hệ số tổng khác 1 thì ghi đè các key riêng của nhóm đó. Tốc độ luôn được giữ trong khoảng 0,1 đến 10.

**Ghi đè theo điều kiện:** các khối `[[Override]]` đặt tốc độ khác khi người chơi đang có một SpEffect,
đang ở một lớp tải trọng (Light, Medium, Heavy, Overweight), hoặc cả hai. Nhiều khối khớp cùng lúc thì
xếp chồng, khối nằm sau thắng ở key cả hai cùng đặt.

## Cài đặt

1. Cài một mod loader nạp được DLL (xem [README của repo](../../README.vi.md)).
2. Đặt `SpeedMultiplier.dll` vào thư mục mod của loader và khai báo nó.
3. Chạy game. `SpeedMultiplier.toml` được tạo cạnh DLL ở lần chạy đầu.

## Cấu hình

Cấu hình nằm trong `SpeedMultiplier.toml` cạnh DLL, đặt tên theo DLL. Tên key, giá trị mặc định và ý
nghĩa nằm ở chú thích trong [SpeedMultiplier.toml](SpeedMultiplier.toml), file mẫu nhúng sẵn trong DLL.
Bấm phím nạp lại (mặc định `F5`) để áp dụng, không cần khởi động lại game.

Khi nâng cấp mod, key mới được thêm vào file của bạn mà không đụng tới giá trị đã chỉnh; key không còn
dùng trở thành comment ở cuối file. Key gõ sai hoặc sai kiểu được báo kèm số dòng, và cấu hình đang
chạy được giữ nguyên thay vì bị thay bằng file lỗi.

### Menu trong game (tùy chọn)

Nếu [ERNativeUI](https://github.com/Flammrock/ERNativeUI) được nạp trước mod, mục **Speed Multiplier**
xuất hiện trong System > Game Options với một thanh trượt cho mỗi tốc độ (0,1 đến 10) và công tắc ghi
log. Thay đổi được ghi vào file cấu hình, giữ nguyên comment. Khối `[[Override]]` chỉ sửa trong file.
Không có ERNativeUI thì mod chạy bình thường từ file.

## Cách hoạt động

Mỗi frame mod ghi trường `animation_speed` của từng nhân vật theo nhóm của animation đang chạy, và chỉ
ghi khi giá trị đổi vì game không tự reset trường này. Chi tiết về trường này, các trường đã thử và
việc Seamless Co-op hook chỗ đọc: [docs/animation_speed.md](docs/animation_speed.md). Bảng nhóm và dải
animation: [docs/action_groups.md](docs/action_groups.md).

## Tương thích và giới hạn

- Game chỉ có **một** tốc độ animation cho cả nhân vật, nên không tăng riêng được nửa thân trên: uống
  bình khi đang chạy theo nhóm di chuyển, khi đứng yên theo nhóm item.
- Seamless Co-op hook chỗ game đọc tốc độ; mod phát hiện và vòng qua hook đó cho người chơi và Torrent.
  Tốc độ spirit và kẻ địch không áp dụng khi chạy Seamless.
- Trong co-op, hình ảnh không đồng bộ giữa hai máy: mỗi máy chỉ tăng tốc nhân vật của chính nó.
- Đòn chí mạng và đòn tóm đi cặp với animation của nạn nhân. Nạn nhân chạy theo tốc độ của người chơi
  ([docs/critical_victim.md](docs/critical_victim.md)); giá trị `Critical` cao vẫn có thể lệch nhịp.
- Tia của Placidusax's Ruin chạy theo đồng hồ riêng nên không theo tốc độ niệm phép.
- Animation đi bộ khi quá tải nằm ngoài dải đi bộ. Tải trọng được khớp theo lớp, chưa theo phần trăm.
- Hệ số tổng cũng ảnh hưởng lúc bị đánh, cưỡi ngựa và nghỉ ở grace.

## Tài liệu

- [CHANGELOG.md](CHANGELOG.md): ghi chú phát hành.
- [docs/animation_speed.md](docs/animation_speed.md): trường tốc độ, các trường đã thử, chỗ đọc bị hook.
- [docs/action_groups.md](docs/action_groups.md): animation nào thuộc nhóm nào.
- [docs/enemy_speed.md](docs/enemy_speed.md): cách phân biệt kẻ địch và boss.
- [docs/critical_victim.md](docs/critical_victim.md): nạn nhân của đòn chí mạng.
