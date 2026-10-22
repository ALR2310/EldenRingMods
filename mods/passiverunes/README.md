<div align="center">

# PassiveRunes

**Tự động cộng (hoặc trừ) rune theo thời gian chơi.**

![Rust](https://img.shields.io/badge/Rust-cdylib-orange?logo=rust)
![Platform](https://img.shields.io/badge/platform-Windows_x64-blue)
![Game](https://img.shields.io/badge/game-Elden_Ring-8a6d3b)

</div>

---

## Giới thiệu

PassiveRunes cộng rune cho bạn đều đặn khi đang chơi, kèm **bonus ở các mốc thời gian
chơi**. Mọi giá trị đều có thể âm để **trừ** rune theo thời gian. Mod đọc và ghi số rune
trực tiếp, không cần patch code, và đổi cấu hình xong bấm phím reload là áp dụng ngay.

## Tính năng

Mỗi chu kỳ, mod cộng một tổng gồm ba thành phần độc lập, cộng dồn với nhau:

| Thành phần | Cộng mỗi chu kỳ |
|---|---|
| **Cố định** | một số rune không đổi |
| **Theo cấp** | một phần trăm chi phí lên cấp kế tiếp |
| **Lãi kép** | một phần trăm số rune đang giữ (tăng rất nhanh khi giá trị lớn) |

Mỗi thành phần tắt khi đặt về 0, và lấy rune đi khi âm.

**Mốc thời gian:** thưởng một số rune cố định khi thời gian chơi chạm các mốc cho trước
(mặc định 30 phút, 1 giờ, 2 giờ, 3 giờ, 6 giờ, 12 giờ, 24 giờ). Danh sách mốc tuỳ chỉnh được
hoặc để trống để tắt.

Đồng hồ chỉ chạy khi bạn đã vào thế giới: ở màn hình tiêu đề hay màn hình tải thì nó tạm
dừng (không reset), nên thời gian ở ngoài không tính vào mốc. Số rune đang giữ luôn nằm
trong khoảng từ 0 đến trần của game.

## Cấu hình

Cấu hình nằm trong `PassiveRunes.ini` cạnh DLL, tự tạo ở lần chạy đầu. Khi nâng cấp, key
mới được thêm vào file có sẵn mà không ghi đè giá trị bạn đã chỉnh. Tên key, khoảng giá trị
và giá trị mặc định nằm ở chú thích trong [PassiveRunes.ini](PassiveRunes.ini), file mẫu
nhúng sẵn trong DLL. Bấm phím reload để áp dụng, không cần khởi động lại game.

### Menu trong game (tùy chọn)

Nếu [ERNativeUI](https://github.com/Flammrock/ERNativeUI) được nạp trước mod, mục
**Passive Runes** xuất hiện trong System > Game Options, gồm công tắc bật tắt, chu kỳ, ba
thành phần ở trên và công tắc ghi log. Các giá trị số gõ vào ô (chu kỳ tính bằng mili giây,
số lượng có thể âm, hai phần trăm có thể là số thập phân). Danh sách mốc thời gian chỉ sửa
trong file. Không có ERNativeUI thì mod chạy bằng file như bình thường.

## Cách hoạt động

Mod đọc và ghi `rune_count` qua `WorldChrMan.main_player.player_game_data` của
[fromsoftware-rs](https://github.com/vswarte/fromsoftware-rs), trong một task chạy mỗi frame
trên nhóm `FrameBegin` của game. Mod không dùng bảng RVA theo từng phiên bản game, nên một
bản vá game không làm mod ngừng chạy.

Mốc thời gian được cộng khi thời gian chơi **vượt qua** mốc (không đòi khớp đúng giây),
nên một frame giật hoặc chu kỳ không chia hết mốc cũng không làm mất bonus. Công thức từng
thành phần, chi phí lên cấp, lãi kép và giá trị âm:
[docs/rune_math.md](docs/rune_math.md).

## Giới hạn

- Số rune không bao giờ xuống dưới 0 dù giá trị âm lớn đến đâu.
- Lãi kép tăng theo hàm mũ nên chạm trần rất nhanh với giá trị lớn.
- Mốc thời gian là số rune cố định, chưa hỗ trợ theo phần trăm.
- Chi phí lên cấp là công thức cứng trong exe do cộng đồng giải ra, đã đối chiếu khớp với
  màn hình lên cấp.
- Thoát ra màn hình tiêu đề rồi nạp save khác vẫn đếm tiếp từ thời gian cũ (chỉ không đếm
  phần ở ngoài game).

## Tài liệu

| Tài liệu | Nội dung |
|---|---|
| [docs/rune_math.md](docs/rune_math.md) | Cách tính rune mỗi chu kỳ và mốc thời gian |
| [docs/startup_and_game_updates.md](docs/startup_and_game_updates.md) | Khởi động, các lỗi đã gặp, bản vá game |
| [HISTORY.md](HISTORY.md) | Dòng thời gian thay đổi |
| [CHANGELOG.md](CHANGELOG.md) | Ghi chú phát hành cho người dùng |

Mã nguồn: [src/rune.rs](src/rune.rs) (vòng tick và ghi rune), [src/math.rs](src/math.rs)
(công thức, đồng hồ, mốc; có test), [src/menu.rs](src/menu.rs).
