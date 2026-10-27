<div align="center">

# EldenRingMod

**Bộ mod DLL viết bằng Rust của tôi cho Elden Ring.**

![Rust](https://img.shields.io/badge/Rust-2024_edition-orange?logo=rust)
![Platform](https://img.shields.io/badge/platform-Windows_x64-blue)
![Game](https://img.shields.io/badge/game-Elden_Ring-8a6d3b)
![Mods](https://img.shields.io/badge/mods-17-green)

</div>

---

## Giới thiệu

EldenRingMod là một monorepo gồm các mod cho Elden Ring. Mỗi mod là một DLL độc lập:
bỏ vào thư mục mod loader, không cần sửa file game và không phải ghi đè `regulation.bin`.
Các mod đọc và ghi trạng thái game ngay trong lúc chạy (qua binding
[fromsoftware-rs](https://github.com/vswarte/fromsoftware-rs) hoặc hook mã máy), và cấu
hình bằng một file `.ini` hoặc `.toml` đặt cạnh DLL.

Phần lớn mod đã được đăng trên Nexus Mods và kiểm thử trong game, gồm cả bản vanilla,
Seamless Co-op và các bản mod lớn như Convergence và Reforged.

## Danh sách mod

| Mod                                           | Chức năng                                                               | Nexus                                                   |
| --------------------------------------------- | ----------------------------------------------------------------------- | ------------------------------------------------------- |
| [**AnimCancel**](mods/animcancel)             | Cắt animation đang chạy bằng guard, lăn, nhảy hoặc đổi vũ khí                 | [11328](https://www.nexusmods.com/eldenring/mods/11328) |
| [**AutoPickup**](mods/autopickup)             | Tự nhặt nguyên liệu, vật phẩm, đồ trên xác và rune rơi khi đứng trong tầm   | [11470](https://www.nexusmods.com/eldenring/mods/11470) |
| [**AutoRegen**](mods/autoregen)               | Hồi HP, FP, Stamina theo thời gian, khi đánh trúng và khi bị đánh trúng | [10548](https://www.nexusmods.com/eldenring/mods/10548) |
| [**DropMultiplier**](mods/dropmultiplier)     | Chỉnh tỉ lệ rớt đồ của quái                                             | [11075](https://www.nexusmods.com/eldenring/mods/11075) |
| [**FasterRevival**](mods/fasterrevival)       | Rút ngắn thời gian từ lúc chết đến lúc hồi sinh                         | [11160](https://www.nexusmods.com/eldenring/mods/11160) |
| [**InfiniteAilment**](mods/infiniteailment)   | Chỉnh thời lượng và sát thương Poison, Scarlet Rot mà người chơi gây ra | [11087](https://www.nexusmods.com/eldenring/mods/11087) |
| [**JourneyHud**](mods/journeyhud)             | HUD góc phải: máu, poise của kẻ địch đang khoá và minimap             | nội bộ                                                  |
| [**PassiveRunes**](mods/passiverunes)         | Cộng rune theo thời gian, có bonus theo mốc                             | [10528](https://www.nexusmods.com/eldenring/mods/10528) |
| [**RiseArcher**](mods/risearcher)             | Buff Bow, Crossbow, Ballista, Arrow, Bolt                               | [5807](https://www.nexusmods.com/eldenring/mods/5807)   |
| [**RuneMultiplier**](mods/runemultiplier)     | Nhân lượng rune nhận được từ mọi nguồn                                  | [10630](https://www.nexusmods.com/eldenring/mods/10630) |
| [**SoulsTeleport**](mods/soulsteleport)       | Dịch chuyển tới người chơi khác trong Seamless Co-op                    | [11119](https://www.nexusmods.com/eldenring/mods/11119) |
| [**SpeedMultiplier**](mods/speedmultiplier)   | Tăng tốc di chuyển, tấn công và thi triển phép                          | [11173](https://www.nexusmods.com/eldenring/mods/11173) |
| [**SpiritMultiplier**](mods/spiritmultiplier) | Nhân số linh hồn (spirit ash) triệu hồi, vượt trần mặc định             | [11168](https://www.nexusmods.com/eldenring/mods/11168) |
| [**WeightMultiplier**](mods/weightmultiplier) | Nhân hệ số Trọng Tải                                                    | [10549](https://www.nexusmods.com/eldenring/mods/10549) |
| [**WindowResize**](mods/windowresize)         | Kéo viền để đổi kích thước cửa sổ Windowed                              | nội bộ                                                  |
| [**InputBridge**](mods/inputbridge)           | Gửi phím vào game đang ở nền, để agent kiểm tra mod tự động            | nội bộ                                                  |
| [**SomeTweaks**](mods/sometweaks)             | Gói QoL thử nghiệm, đã ngừng phát triển                                 | không đăng                                              |

Phiên bản và ghi chú phát hành của từng mod nằm ở `CHANGELOG.md` trong thư mục mod đó.

## Cài đặt mod

1. Tải bản phát hành của mod từ trang Nexus tương ứng ở bảng trên.
2. Cần một mod loader nạp được DLL, ví dụ
   [Mod Engine 2](https://github.com/soulsmods/ModEngine2/releases) (khuyến nghị),
   [Mod Engine 3](https://github.com/garyttierney/me3/releases) hoặc
   [Elden Mod Loader](https://www.nexusmods.com/eldenring/mods/117).
3. Đặt `<Mod>.dll` vào thư mục mod của loader và khai báo nó theo hướng dẫn của loader.
4. Chạy game. File cấu hình (`<Mod>.ini` hoặc `<Mod>.toml`) được tạo cạnh DLL nếu chưa có;
   sửa file đó để đổi hành vi. Nhiều mod hỗ trợ phím nóng để nạp lại cấu hình khi đang chơi.

Hướng dẫn chi tiết hơn có trên trang Nexus của từng mod.

## Kiến trúc

```
EldenRingMod/
├── mods/       # mỗi thư mục là một mod, một crate cdylib, một DLL
├── shared/     # crate `common`: hạ tầng dùng chung (kèm docs/ giải thích từng module)
├── scripts/    # build-mod.ps1, công cụ dịch ngược (IDA, Ghidra), tiện ích trang Nexus
├── docs/       # tư liệu dùng chung: trang Nexus, prompt logo, OpenAPI của Nexus
└── builds/     # đầu ra build theo từng mod (không commit)
```

Một mod điển hình gồm:

```
mods/<mod>/
├── Cargo.toml        # [lib] crate-type = ["cdylib"]
├── <Mod>.ini|toml    # config mặc định, nhúng vào DLL lúc build
├── src/lib.rs        # DllMain mỏng: nạp config, khởi tạo log, đăng ký tính năng
├── README.md         # cách hoạt động và giới hạn
├── CHANGELOG.md      # ghi chú phát hành
├── HISTORY.md        # dòng thời gian thay đổi
└── docs/             # phân tích kỹ thuật theo chủ đề
```

### Crate `common`

Mọi mod chia sẻ mã nguồn qua `common` thay vì sao chép, nên một bản sửa lỗi có hiệu lực
cho cả bộ.

| Nhóm          | Module                                    | Vai trò                                                                                 |
| ------------- | ----------------------------------------- | --------------------------------------------------------------------------------------- |
| Cấu hình, log | `config`, `toml_config`, `logger`, `diag` | Đọc và ghi config, log ra file, ghi lại môi trường để phục vụ báo lỗi                   |
| Bộ nhớ        | `memscan`, `codepatch`                    | Quét AOB, JMP hook                                                                      |
| Game engine   | `task`, `task_hook`, `alloc_hook`         | Đăng ký tác vụ chạy theo frame, tìm hàm của game bằng AOB để không vỡ khi game cập nhật |
|               | `player`, `params`, `anim`                | Phát hiện lúc người chơi đã vào thế giới, duyệt param an toàn, phân loại animation      |
| Tương tác     | `input`, `reload`, `announce`             | Phím nóng, nạp lại config, banner thông báo trong game                                  |

Mỗi mod được link tĩnh với `common` và không chia sẻ trạng thái lúc chạy với mod khác.
Mô tả đầy đủ từng module ở [shared/src/lib.rs](shared/src/lib.rs).

## Build từ mã nguồn

**Yêu cầu:** Windows x64, [Rust](https://rustup.rs) (edition 2024), target MSVC.

```powershell
cargo build --release                 # build toàn bộ mod
cargo build --release -p autoregen    # build một mod
cargo test --workspace                # chạy unit test

# Build một mod, chép DLL và config vào builds/<mod>/
pwsh scripts/build-mod.ps1 -Mod AutoRegen
pwsh scripts/build-mod.ps1 -Mod AutoRegen -Zip   # kèm file .zip để phát hành
```

`-Mod` là tên PascalCase của mod, lấy từ `[lib] name` trong `Cargo.toml`. Trong VS Code,
`Ctrl+Shift+B` mở danh sách task build cho từng mod và task `Cargo: Build All (Release)`.

## Tài liệu

| Tài liệu                                       | Nội dung                                                                   |
| ---------------------------------------------- | -------------------------------------------------------------------------- |
| `mods/<mod>/README.md`                         | Cách hoạt động, giới hạn và cấu hình ở mức khái niệm của từng mod          |
| `mods/<mod>/docs/`                             | Phân tích kỹ thuật: hook, RVA, nguyên nhân gốc của lỗi, hướng đã thử và bỏ |
| `shared/docs/`                                 | Cách các module của `common` hoạt động: AOB, hook, lý do thiết kế          |
| [scripts/ida/README.md](scripts/ida/README.md) | Bộ script IDAPython cho dịch ngược                                         |
| [CLAUDE.md](CLAUDE.md)                         | Quy ước làm việc và viết tài liệu trong repo                               |
| [TODO.md](TODO.md)                             | Việc còn mở theo từng mod                                                  |

## Công nghệ

- [Rust](https://www.rust-lang.org) với `cdylib`, LTO và `codegen-units = 1` cho bản release
- [fromsoftware-rs](https://github.com/vswarte/fromsoftware-rs) cho binding tới struct của engine
- [vtable-rs](https://crates.io/crates/vtable-rs) để đăng ký closure Rust làm task của game
- `serde`, `toml`, `toml_edit` cho cấu hình TOML
- IDA Pro và Ghidra cho phần dịch ngược

## Lưu ý

Các mod chỉnh sửa bộ nhớ của game đang chạy. Dùng ở chế độ offline hoặc Seamless Co-op
với bạn bè và tự chịu rủi ro: không dùng khi chơi online có Easy Anti-Cheat, vì có thể
bị khoá tài khoản. Dự án không liên kết với FromSoftware hay Bandai Namco.
