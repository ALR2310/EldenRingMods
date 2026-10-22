<div align="center">

# WindowResize

**Kéo viền để đổi kích thước cửa sổ Elden Ring ở chế độ Windowed.**

![Rust](https://img.shields.io/badge/Rust-cdylib-orange?logo=rust)
![Platform](https://img.shields.io/badge/platform-Windows_x64-blue)
![Game](https://img.shields.io/badge/game-Elden_Ring-8a6d3b)

</div>

---

## Giới thiệu

Game tạo cửa sổ không có viền kéo, nên mức nhỏ nhất bạn chọn được trong menu độ phân giải là
800x450. WindowResize thêm viền kéo vào cửa sổ, để bạn thu nó nhỏ hơn mức đó (hoặc đổi kích
thước tuỳ ý). Mod làm cho nhu cầu cá nhân.

## Tính năng

| Tính năng | Mô tả |
|---|---|
| **Viền kéo** | kéo cạnh hoặc góc cửa sổ để đổi kích thước, có mũi tên kéo ở viền |
| **Khoá tỉ lệ** | giữ vùng hình theo một tỉ lệ (mặc định 16:9) khi kéo, hoặc kéo tự do |
| **Kích thước nhỏ nhất** | vùng hình không thể kéo nhỏ hơn mức đặt (mặc định 160x90) |
| **Kích thước khởi động** | đặt vùng hình theo chiều rộng và chiều cao khi game mở; chỉ đặt một chiều thì chiều còn lại theo tỉ lệ |

Chỉ hoạt động ở chế độ **Windowed**: Fullscreen và Borderless được bỏ qua.

## Cấu hình

Cấu hình nằm trong `WindowResize.ini` cạnh DLL, tự tạo ở lần chạy đầu. Tên key, giá trị mặc
định và ý nghĩa nằm ở chú thích trong [WindowResize.ini](WindowResize.ini), file mẫu nhúng sẵn
trong DLL. Mod không có phím reload: đổi file thì khởi động lại game.

## Cách hoạt động

Mod chỉ dùng Win32, không AOB hay RVA, không đụng dữ liệu hay hệ thống task của game, nên
không vỡ khi game cập nhật.

1. Tìm cửa sổ của game và thêm lại kiểu cửa sổ `WS_THICKFRAME` (game tạo cửa sổ mà không có
   nó), nên Windows tự lo việc kéo.
2. Gắn vào hàm xử lý thông điệp của cửa sổ để: hiện mũi tên kéo ở viền, áp kích thước nhỏ nhất
   và khoá tỉ lệ. Các phép tính kích thước nằm riêng ở [src/geometry.rs](src/geometry.rs) và có
   test.
3. Một vòng kiểm tra mỗi giây thêm lại viền khi game ghi đè kiểu cửa sổ (đổi chế độ màn hình
   hoặc độ phân giải) và gắn lại nếu cửa sổ bị tạo lại.

Kích thước khởi động được áp lại mỗi giây trong 20 giây đầu và dừng ngay khi bạn bắt đầu kéo
viền. Khảo sát và lý do từng quyết định: [docs/resize_approach.md](docs/resize_approach.md).

## Giới hạn

- Game vẫn render ở độ phân giải trong cài đặt rồi co lại, nên FPS không tăng khi cửa sổ nhỏ.
- Chưa kiểm chứng: khi game đang focus, nó giới hạn con trỏ trong vùng hình, nên có thể phải
  click ra ngoài cửa sổ trước khi kéo viền (xem [docs/resize_approach.md](docs/resize_approach.md)).

## Tài liệu

| Tài liệu | Nội dung |
|---|---|
| [docs/resize_approach.md](docs/resize_approach.md) | Khảo sát và thiết kế |
| [HISTORY.md](HISTORY.md) | Dòng thời gian thay đổi |

Mã nguồn: [src/window.rs](src/window.rs) (Win32: tìm cửa sổ, viền, gắn hàm xử lý),
[src/geometry.rs](src/geometry.rs) (tính kích thước).
