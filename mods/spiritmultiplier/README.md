<div align="center">

# SpiritMultiplier

**Triệu hồi nhiều spirit hơn, nhiều Spirit Ash cùng lúc, gọi ở bất cứ đâu.**

![Rust](https://img.shields.io/badge/Rust-cdylib-orange?logo=rust)
![Platform](https://img.shields.io/badge/platform-Windows_x64-blue)
![Game](https://img.shields.io/badge/game-Elden_Ring-8a6d3b)

</div>

---

## Giới thiệu

SpiritMultiplier nâng cấp hệ thống triệu hồi Spirit Ash: **nhiều spirit hơn mỗi Ash** vượt trần 10 con
của game, **nhiều Ash khác nhau cùng lúc**, gọi lại không cần nghỉ, gọi ở mọi nơi, bỏ màu ma và cho
spirit tự hồi máu. Chạy được ở chế độ offline và Seamless Co-op (tắt EAC).

## Tính năng

| Tính năng | Mô tả |
|---|---|
| **Nhiều spirit mỗi Ash** | nhân hệ số hoặc đặt số cố định, vượt trần 10 con của game |
| **Nhiều Ash cùng lúc** | gọi Ash khác khi spirit cũ còn đó; bấm lại Ash đang gọi chỉ cho Ash đó về |
| **Gọi lại không cần nghỉ** | bỏ luật một lần mỗi lần nghỉ |
| **Gọi ở mọi nơi** | không còn giới hạn vùng gọi |
| **Hồi máu** | spirit và Torrent tự hồi: số HP cố định, % HP tối đa hoặc % HP đã mất |
| **Bỏ màu ma** | spirit trông như nhân vật thường |
| **Cải thiện AI** | giác quan, hung hăng, bám theo người chơi |
| **Mimic Tear tức thì** | bỏ giai đoạn Blob |
| **Kẻ địch không biến mất** | giữ suất activate của kẻ địch khi có nhiều spirit |
| **Dịch chuyển spirit** *(thử nghiệm)* | spirit tụt lại thì dịch chuyển về chỗ người chơi |

## Cấu hình

Cấu hình nằm trong `SpiritMultiplier.ini` cạnh DLL, tự tạo ở lần chạy đầu; key mới được thêm vào file
có sẵn mà không ghi đè giá trị đã chỉnh. Phím nạp lại (`ReloadKey`) áp dụng thay đổi ngay, trừ
`MaxSpirits` (cần khởi động lại game). Tên key, giá trị mặc định và ý nghĩa nằm ở chú thích trong
[SpiritMultiplier.ini](SpiritMultiplier.ini), file mẫu nhúng sẵn trong DLL.

Nếu `ERNativeUI.dll` ([ERNativeUI](https://github.com/Flammrock/ERNativeUI)) được nạp trước mod, System >
Game Options có thêm hàng **Spirit Multiplier** để chỉnh các giá trị ngay trong game (ghi vào ini, có
tác dụng ngay; riêng `MaxSpirits` cần khởi động lại). Không có host thì mod chạy bằng ini như bình
thường. Mod: [src/menu.rs](src/menu.rs), client: [shared/src/erui.rs](../../shared/src/erui.rs).

## Cách hoạt động

| Phần | Cách làm |
|---|---|
| **Vượt trần 10 spirit** | Spirit nằm trong mảng slot `summon_buddy_chr_set`; band của người chơi dài đúng 10. Mod patch vòng tìm slot và con trỏ vòng để band dài `MaxSpirits`, và kéo dài chain SpEffect → BuddyParam theo hệ số. Seamless Co-op mỗi người một band riêng. ([docs/slots_and_band.md](docs/slots_and_band.md)) |
| **Kẻ địch không biến mất** | Game chỉ activate tối đa 60 nhân vật; mod nâng giới hạn đúng bằng số spirit đang có. ([docs/activate_limit.md](docs/activate_limit.md)) |
| **MultiSpirit** | Bỏ luật "một nhóm spirit" của `SummonBuddyManager`; Ash gọi thêm vẫn tốn FP/HP. ([docs/multi_spirit.md](docs/multi_spirit.md)) |
| **NoRestResummon / SummonAnywhere** | Sửa `BuddyStoneParam` và vài patch gọi hàm. ([docs/summon_anywhere.md](docs/summon_anywhere.md)) |
| **GhostColor** | Tắt ghi đè PhantomParam trên các `SpEffectVfx` chỉ spirit dùng. ([docs/ghost_color.md](docs/ghost_color.md)) |
| **Cải thiện AI** | Nạp sẵn các field `NpcThinkParam`, kèm hai file ini tuỳ chọn để tự chỉnh. ([docs/think_override.md](docs/think_override.md)) |
| **InstantMimic** | Viết lại hàng Blob của Mimic Tear, bỏ qua giai đoạn Blob. ([docs/mimic_tear.md](docs/mimic_tear.md)) |
| **HubHpBars** | HUD chỉ có 5 thanh HP spirit (dữ liệu, vòng lặp hiển thị và danh sách hàng của view đều cứng 5). Mod thêm hàng vào movie lúc game mở tệp, dựng thêm view trên cùng clip và đặt vị trí panel theo số thanh đang dùng. ([docs/buddy_hud.md](docs/buddy_hud.md)) |
| **Dịch chuyển spirit** | Hạ ngưỡng dịch chuyển của game, tuỳ chọn yêu cầu dịch chuyển khi spirit ở xa. ([docs/warp_research.md](docs/warp_research.md)) |
| **Hồi máu** | Mỗi giây cộng HP cho mọi nhân vật trong `summon_buddy_chr_set`. ([src/regen.rs](src/regen.rs)) |

Giá trị param gốc được chụp một lần và mọi lần áp lại đều tính từ bản chụp nên reload không cộng dồn.
Mọi patch code chỉ đổi đích `call` / `jmp` sẵn có, vì Seamless Co-op huỷ game khi chữ ký byte lệch.
([docs/seamless_conflict.md](docs/seamless_conflict.md))

## Giới hạn

- Vượt 60 spirit (`MaxSpirits > 60`) là tự chịu rủi ro: FPS tụt, có thể crash do hết heap nhân vật.
- Seamless Co-op: người chơi không cài mod vẫn dùng band vanilla; đôi khi chỉ đồng bộ một phần spirit
  sau khi một bên load lại khu vực; spirit mất màu ma.
- Không dùng chung với mod khác sửa việc triệu hồi spirit (vd. phần spirit của Solid Uncapper).
- ELDEN RING Reforged: spirit mặc định ở trạng thái đỏ (Fury) và `MultiSpirit` chỉ cho một nhóm vào
  Fury; `NoRestResummon` không gỡ được luật của Reforged sau Spirit-Severing Blade.
- Dịch chuyển spirit là thử nghiệm, chỉ chạy trên bản exe mod nhận ra được.

## Tài liệu

| Tài liệu | Nội dung |
|---|---|
| [docs/slots_and_band.md](docs/slots_and_band.md) | Slot, band, `MaxSpirits`, chain, Seamless |
| [docs/activate_limit.md](docs/activate_limit.md) | Giới hạn 60 nhân vật active, `EnemyProbe` |
| [docs/multi_spirit.md](docs/multi_spirit.md) | Gọi nhiều Ash cùng lúc |
| [docs/summon_anywhere.md](docs/summon_anywhere.md) | NoRestResummon, SummonAnywhere |
| [docs/mimic_tear.md](docs/mimic_tear.md) | Mimic Tear, Blob, `InstantMimic` |
| [docs/buddy_hud.md](docs/buddy_hud.md) | Thanh HP spirit trên HUD, `HubHpBars` |
| [docs/ghost_color.md](docs/ghost_color.md) | Màu ma, cờ hành vi spirit |
| [docs/think_override.md](docs/think_override.md) | AI spirit, file ini tuỳ chọn |
| [docs/warp_research.md](docs/warp_research.md) | Cơ chế dịch chuyển spirit |
| [docs/seamless_conflict.md](docs/seamless_conflict.md) | Xung đột Seamless, quy tắc đổi đích `call` |
| [docs/open_issues.md](docs/open_issues.md) | Vấn đề còn mở (Reforged, Seamless) |
| [HISTORY.md](HISTORY.md) | Dòng thời gian thay đổi |
| [CHANGELOG.md](CHANGELOG.md) | Ghi chú phát hành cho người dùng |
