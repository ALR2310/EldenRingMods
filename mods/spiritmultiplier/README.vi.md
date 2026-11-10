<div align="center">

# SpiritMultiplier

**Nhiều spirit hơn mỗi Ash, nhiều Spirit Ash cùng lúc, gọi ở bất cứ đâu.**

[English](README.md)

![Game](https://img.shields.io/badge/game-Elden_Ring-8a6d3b)
![Platform](https://img.shields.io/badge/platform-Windows_x64-blue)
[![Nexus Mods](https://img.shields.io/badge/Nexus_Mods-11168-orange)](https://www.nexusmods.com/eldenring/mods/11168)

</div>

---

## Giới thiệu

SpiritMultiplier cải thiện việc triệu hồi bằng Spirit Ash: nhiều spirit hơn mỗi Ash, nhiều Ash khác
nhau cùng lúc, gọi lại không cần nghỉ, gọi ở mọi nơi, spirit tự hồi máu và không mất khi bạn nghỉ, và
thêm thanh HP spirit trên HUD. Chạy được ở chế độ offline và Seamless Co-op.

## Cài đặt

1. Tải mod từ [trang Nexus](https://www.nexusmods.com/eldenring/mods/11168).
2. Cài một mod loader nạp được DLL (xem [README của repo](../../README.vi.md)).
3. Đặt `SpiritMultiplier.dll` vào thư mục mod của loader và đăng ký nó.
4. Chạy game. `SpiritMultiplier.toml` được tạo cạnh DLL ở lần chạy đầu. Ý nghĩa từng key nằm ở chú thích trong [SpiritMultiplier.toml](SpiritMultiplier.toml); sửa rồi nhấn `F5` (mặc định) để áp dụng mà không cần khởi động lại game. `MaxSpirits` và `HubHpBars` chỉ đọc một lần và cần khởi động lại.

Nếu [ERNativeUI](https://github.com/Flammrock/ERNativeUI) được nạp trước mod, mục **Spirit Multiplier**
cũng xuất hiện trong System > Game Options. Không bắt buộc.

Các bản trước dùng `SpiritMultiplier.ini`. Nếu chưa có `SpiritMultiplier.toml`, cài đặt trong file ini
được mang sang ở lần chạy đầu tiên; file ini không bị đụng tới.

## Tài liệu

- [CHANGELOG.md](CHANGELOG.md): ghi chú phát hành.
- [Trang Nexus](https://www.nexusmods.com/eldenring/mods/11168): tải về và bình luận.
- [docs/slots_and_band.md](docs/slots_and_band.md): trần 10 spirit và cách nâng.
- [docs/activate_limit.md](docs/activate_limit.md): kẻ địch biến mất khi có nhiều spirit.
- [docs/multi_spirit.md](docs/multi_spirit.md): nhiều Ash cùng lúc và `CloneSpirit`.
- [docs/summon_anywhere.md](docs/summon_anywhere.md): `NoRestResummon` và `SummonAnywhere`.
- [docs/persist_spirits.md](docs/persist_spirits.md): giữ spirit khi nghỉ ở grace.
- [docs/ghost_color.md](docs/ghost_color.md): bỏ màu ma.
- [docs/mimic_tear.md](docs/mimic_tear.md): giai đoạn Blob và số lượng Mimic Tear.
- [docs/think_override.md](docs/think_override.md): tuỳ chọn AI của spirit.
- [docs/buddy_hud.md](docs/buddy_hud.md): thêm thanh HP trên HUD.
- [docs/warp_research.md](docs/warp_research.md): dịch chuyển spirit về chỗ người chơi.
- [docs/seamless_conflict.md](docs/seamless_conflict.md): quy tắc patch khi dùng cùng Seamless Co-op.
- [docs/seamless_ersc.md](docs/seamless_ersc.md): Seamless Co-op làm gì với màu ma của spirit.
- [docs/open_issues.md](docs/open_issues.md): vấn đề còn mở với Reforged, Seamless, Convergence.
