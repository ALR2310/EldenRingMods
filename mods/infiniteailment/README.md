<div align="center">

# InfiniteAilment

**Chỉnh thời lượng và sát thương của Poison, Scarlet Rot, Black Flame và Destined Death.**

![Rust](https://img.shields.io/badge/Rust-cdylib-orange?logo=rust)
![Platform](https://img.shields.io/badge/platform-Windows_x64-blue)
![Game](https://img.shields.io/badge/game-Elden_Ring-8a6d3b)

</div>

---

## Giới thiệu

Poison và Scarlet Rot trong Elden Ring chỉ kéo dài 90 giây, Black Flame và Destined Death
chỉ cháy vài giây. InfiniteAilment cho bạn chỉnh **thời lượng** (kể cả vô hạn) và **sát
thương mỗi nhịp** của bốn hiệu ứng đó khi chính **người chơi gây ra** cho kẻ địch. Mod sửa
thẳng bảng dữ liệu của game khi đang chạy, không ghi đè `regulation.bin`, và chạy được trên
các bản mod lớn như Convergence.

## Tính năng

Bốn hiệu ứng, mỗi hiệu ứng chỉnh riêng:

| Hiệu ứng | Thời lượng mặc định của mod | Thời lượng vanilla |
|---|---|---|
| **Scarlet Rot** | vô hạn | 90 giây |
| **Poison** | vô hạn | 90 giây |
| **Black Flame** | 10 giây | 2 giây |
| **Destined Death** (cháy) | 10 giây | 2 giây |

Với mỗi hiệu ứng bạn chỉnh được ba thứ:

| Thiết lập | Ý nghĩa |
|---|---|
| **Thời lượng** | số giây, hoặc vô hạn |
| **% máu tối đa mỗi nhịp** | phần trăm máu tối đa của kẻ địch mất mỗi nhịp |
| **Sát thương cố định mỗi nhịp** | cộng thêm vào mỗi nhịp; số âm thì hồi máu thay vì gây sát thương |

Destined Death còn có thiết lập riêng cho **thời gian giảm máu tối đa** (vanilla 15 và 45
giây, mặc định vô hạn).

## Cấu hình

Cấu hình nằm trong `InfiniteAilment.ini` cạnh DLL, tự tạo ở lần chạy đầu. Tên key, giá
trị mặc định và giá trị vanilla nằm ở chú thích trong
[InfiniteAilment.ini](InfiniteAilment.ini), file mẫu nhúng sẵn trong DLL. Bấm phím reload
để áp dụng, không cần khởi động lại game.

### Menu trong game (tùy chọn)

Nếu [ERNativeUI](https://github.com/Flammrock/ERNativeUI) được nạp trước mod, mục
**Infinite Ailment** xuất hiện trong System > Game Options: một trang cho mỗi hiệu ứng (ba
thiết lập ở trên, gõ số vào ô, kể cả số thập phân như 0.18) và công tắc ghi log. Nếu đang ở
màn hình chính thì thay đổi được chờ đến khi vào thế giới. Không có ERNativeUI thì mod chạy
bằng file như bình thường.

## Cách hoạt động

Mỗi hiệu ứng được lưu thành nhiều dòng trong bảng `SpEffectParam` của game. Mod chọn đúng
những dòng do người chơi gây ra:

- **Poison và Scarlet Rot:** lọc theo chữ ký các trường (`SpCategory`, `StateInfo`, có hút
  máu, thời lượng vanilla 90). Chữ ký này loại các dòng của quái, hồ độc, hồ rot và boss
  (Malenia).
- **Black Flame và Destined Death:** không có chữ ký riêng nên mod chỉ xét một danh sách ID
  cố định (vũ khí và phép lửa đen của người chơi).

Mod ghi ba trường mỗi dòng (thời lượng, % máu, sát thương cố định). Giá trị là tuyệt đối chứ
không phải hệ số nhân, nên reload không cộng dồn. Danh sách dòng được tìm một lần rồi lưu
lại, vì chính điều kiện thời lượng 90 sẽ không còn khớp sau khi đã ghi đè. Cách tìm ra tập
dòng này: [docs/row_discovery.md](docs/row_discovery.md).

## Giới hạn

- Chỉ sửa các dòng "người chơi tự gây ra"; Poison, Rot của quái, hồ độc, hồ rot và Malenia
  giữ nguyên.
- Chữ ký lọc dựa trên giá trị vanilla (thời lượng 90): regulation đổi giá trị gốc này có
  thể khiến mod không khớp dòng nào.

## Tài liệu

| Tài liệu | Nội dung |
|---|---|
| [docs/row_discovery.md](docs/row_discovery.md) | Tìm đúng các dòng `SpEffectParam`, chữ ký từng hiệu ứng, hướng đã bỏ |
| [docs/black_flame.md](docs/black_flame.md) | Các dòng Black Flame và Destined Death |
| [HISTORY.md](HISTORY.md) | Dòng thời gian thay đổi |
| [CHANGELOG.md](CHANGELOG.md) | Ghi chú phát hành cho người dùng |

Mã nguồn: [src/status_effect.rs](src/status_effect.rs) (chọn và ghi dòng),
[src/menu.rs](src/menu.rs).
