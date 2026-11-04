# EldenRingMods

[English](README.md)

Bộ mod Elden Ring viết bằng Rust. Mỗi mod là một DLL độc lập: bỏ vào thư mục của mod loader,
không thay file game và không đụng tới `regulation.bin`. Mod đọc và thay đổi trạng thái game
lúc đang chạy, cấu hình bằng một file TOML đặt cạnh DLL.

> Các mod đang được chuyển vào repo này từng cái một. Danh sách dưới đây sẽ dài thêm theo.

## Danh sách mod

| Mod | Chức năng |
| --- | --- |
| [AutoPickup](mods/autopickup) | Tự nhặt nguyên liệu, vật phẩm, đồ trên xác và rune khi đứng trong tầm |

## Cài đặt

1. Cài một loader nạp được DLL, ví dụ
   [Mod Engine 2](https://github.com/soulsmods/ModEngine2/releases),
   [Mod Engine 3](https://github.com/garyttierney/me3/releases) hoặc
   [Elden Mod Loader](https://www.nexusmods.com/eldenring/mods/117).
2. Đặt `<Mod>.dll` vào thư mục mod của loader và khai báo theo hướng dẫn của loader.
3. Chạy game. `<Mod>.toml` được tạo cạnh DLL ở lần chạy đầu; sửa file đó để đổi hành vi.

## Build từ mã nguồn

Yêu cầu: Windows x64, Rust stable với target MSVC, Windows SDK.

```powershell
cargo build --release -p <crate>
```

Xem [CONTRIBUTING.vi.md](CONTRIBUTING.vi.md) để biết cấu trúc repo và quy ước code.

## Giấy phép

[MIT](LICENSE)
