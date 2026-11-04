<div align="center">

# EldenRingMods

**Các mod Elden Ring viết bằng Rust, mỗi mod một DLL.**

[English](README.md)

![Rust](https://img.shields.io/badge/Rust-2024_edition-orange?logo=rust)
![Platform](https://img.shields.io/badge/platform-Windows_x64-blue)
![Game](https://img.shields.io/badge/game-Elden_Ring-8a6d3b)
[![License](https://img.shields.io/badge/license-MIT-lightgrey)](LICENSE)

</div>

---

## Giới thiệu

Các mod DLL của tôi cho Elden Ring. Mỗi mod là một DLL độc lập: bỏ vào thư mục của mod loader, không
thay file game và không đụng tới `regulation.bin`. Cài đặt nằm trong một file TOML cạnh DLL, và có phím
nạp lại để áp dụng thay đổi mà không cần khởi động lại game.

## Danh sách mod

| Mod | Chức năng | Nexus |
| --- | --- | --- |
| [**AnimCancel**](mods/animcancel) | Cắt animation đang chạy bằng guard, lăn, nhảy hoặc đổi vũ khí | [11328](https://www.nexusmods.com/eldenring/mods/11328) |
| [**AutoPickup**](mods/autopickup) | Tự nhặt nguyên liệu, vật phẩm, đồ trên xác và rune khi đứng trong tầm | [11470](https://www.nexusmods.com/eldenring/mods/11470) |
| [**AutoRegen**](mods/autoregen) | Tự hồi HP, FP và Stamina theo thời gian hoặc khi đánh trúng | [10548](https://www.nexusmods.com/eldenring/mods/10548) |
| [**DropMultiplier**](mods/dropmultiplier) | Chỉnh tỉ lệ rơi đồ của quái và số lượng nguyên liệu nhận được | [11075](https://www.nexusmods.com/eldenring/mods/11075) |
| [**SpeedMultiplier**](mods/speedmultiplier) | Tăng hoặc giảm tốc độ của người chơi, Torrent, spirit và kẻ địch theo từng loại hành động | [11173](https://www.nexusmods.com/eldenring/mods/11173) |

Trang của mỗi mod mô tả cách hoạt động và giới hạn, còn `CHANGELOG.md` có ghi chú phát hành.

## Cài đặt

1. Tải mod từ trang Nexus ở bảng trên.
2. Cài một loader nạp được DLL, ví dụ
   [Mod Engine 2](https://github.com/soulsmods/ModEngine2/releases) (khuyến nghị),
   [Mod Engine 3](https://github.com/garyttierney/me3/releases) hoặc
   [Elden Mod Loader](https://www.nexusmods.com/eldenring/mods/117).
3. Đặt `<Mod>.dll` vào thư mục mod của loader và khai báo theo hướng dẫn của loader.
4. Chạy game. `<Mod>.toml` được tạo cạnh DLL ở lần chạy đầu; sửa file đó để đổi hành vi, rồi bấm `F5`
   (mặc định) để áp dụng.

Mỗi mod còn thêm một trang cài đặt trong phần tùy chọn của game khi có
[ERNativeUI](https://github.com/Flammrock/ERNativeUI). Đây là tùy chọn.

## Build và đóng góp

Cần Windows x64, Rust stable với target MSVC và Windows SDK:

```powershell
cargo build --release --workspace                 # mọi mod
pwsh scripts/build-mod.ps1 -Mod AutoRegen         # một mod, chép DLL và config vào builds/autoregen/
```

[CONTRIBUTING.vi.md](CONTRIBUTING.vi.md) nói về cấu trúc repo, crate dùng chung, quy ước viết mã và cách
viết ghi chú nghiên cứu.

## Giấy phép

[MIT](LICENSE)
