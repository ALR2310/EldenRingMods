# Đóng góp

[English](CONTRIBUTING.md)

Trang này mô tả cách tổ chức repo và các quy ước chung của mọi mod, để một thay đổi ở mod này
đọc giống một thay đổi ở mod khác.

Nếu dùng trợ lý AI, cho nó đọc [AGENTS.md](AGENTS.md) và chạy prompt trong
[docs/review-prompt.md](docs/review-prompt.md) lên kết quả trước khi commit.

## Cấu trúc repo

```
shared/            mã dùng chung cho mọi mod (config, log, quét bộ nhớ, hook, ...)
mods/<tên>/        một mod = một crate = một DLL
docs/              tài liệu chung của dự án (quy ước nghiên cứu, bảng tra dùng chung)
scripts/           script build, phát hành và IDA
vendor/            crate bên thứ ba đã vá cho dự án (hudhook)
.github/workflows/ CI
```

Một mod gồm:

```
mods/<tên>/
├── Cargo.toml          phải có `[lints] workspace = true`
├── build.rs            nhúng version resource của Windows
├── <Tên>.toml          config mặc định, nhúng vào DLL lúc build
├── locales/            văn bản menu (en.json, vi.json), cho mod có menu trong game
├── README.md           tiếng Anh: chức năng, cách cài, link tới tài liệu
├── README.vi.md        bản tiếng Việt
├── CHANGELOG.md        ghi chú phát hành cho người chơi (tiếng Anh)
├── nexus_page.bbcode   phần mô tả cho trang Nexus của mod (BBCode), nếu mod có trang
├── THIRD-PARTY-NOTICES.txt  tuỳ chọn; mọi file *license*.txt / *notice*.txt ở đây hoặc trong assets/
│                       được scripts/build-mod.ps1 đóng gói cùng DLL
└── src/
    ├── lib.rs          chỉ DllMain và nối các phần với nhau
    └── ...             mỗi tính năng một module
```

Mod nội bộ không phát hành (Toolbox) không có README, CHANGELOG hay trang Nexus.

Nội dung dài hơn README (ghi chú nghiên cứu, offset, cách một hệ thống của game vận hành)
đặt ở `mods/<tên>/docs/<chủ đề>.md`. Viết theo [docs/research/README.md](docs/research/README.md):
nhãn bằng chứng, bản build đã thử cho từng phát hiện, và giữ lại các hướng đã bị loại.

### Crate `shared`

Các mod dùng chung mã qua `shared` thay vì sao chép, nên một bản sửa có hiệu lực cho cả bộ. Mỗi mod
link bản riêng của nó, nên các mod không chia sẻ trạng thái lúc chạy. Hãy tìm hàm có sẵn ở đây trước
khi viết mới; cách mỗi module hoạt động và lý do thiết kế nằm trong `shared/docs/`.

| Nhóm | Module | Vai trò |
| --- | --- | --- |
| Cấu hình và log | `toml_config`, `logger`, `diag`, `crash`, `i18n` | TOML có migration, file log, ghi môi trường, vết crash, văn bản menu |
| Bộ nhớ | `memscan`, `codepatch`, `conflict` | Quét AOB, hook mã máy, phát hiện hook của mod khác |
| Game engine | `task`, `alloc_hook`, `player`, `params`, `anim` | Task chạy theo frame và tra cứu engine không vỡ khi game cập nhật, cổng "người chơi đã vào thế giới", đọc param an toàn, nhóm animation |
| Tương tác | `input`, `reload`, `announce`, `menu` | Phím nóng, nạp lại config, banner đầu màn hình, menu trong game |
| Overlay và HUD | `gfx`, `overlay` (feature) | Chỉnh movie Scaleform trong bộ nhớ; cửa sổ Dear ImGui trên DX12 (font, giao diện, bố cục đã lưu) cùng hook chuột và con trỏ |

## Build

Yêu cầu: Windows x64, Rust với target MSVC ở bản `rust-version` của `Cargo.toml` gốc hoặc mới
hơn, Windows SDK (cần `rc.exe` cho version resource).

```powershell
cargo build --release -p <crate>
cargo test --workspace
cargo fmt --all
cargo clippy --workspace --all-targets
```

Build và đóng gói một mod: `pwsh scripts/build-mod.ps1 -Mod <Mod> [-Zip]` (kết quả ở
`builds/<crate>/`). Bản phát hành build qua script này: nó ánh xạ lại đường dẫn nguồn để DLL không
mang đường dẫn người dùng hay toolchain cục bộ. File zip gồm DLL và `files` trong `[package.metadata.archive]` của `Cargo.toml` của
mod: đường dẫn tương đối theo thư mục mod, cho phép `*`, thư mục nghĩa là cả cây bên dưới, mỗi file
giữ đường dẫn tương đối của nó. Mọi mod đều phải khai báo, ví dụ `files = ["<Mod>.toml", "THIRD-PARTY-NOTICES.txt"]`;
`files = []` chỉ có DLL (mod tự tạo config ở lần chạy đầu).

Mỗi mod có một task build trong `.vscode/tasks.json` (`Build <Mod>`, `<Mod>` là `[lib] name`) và
nằm trong `dependsOn` của `Cargo: Build All`; thêm, đổi tên hoặc xoá task cùng lúc với mod.

Script trong `scripts/` dùng PowerShell cho các bước build và đóng gói đơn giản. Dùng Python
(3.10+, ưu tiên chỉ thư viện chuẩn) khi script cần logic thật, như đọc JSON hoặc gọi API.

CI chạy `fmt --check`, `clippy -D warnings` và `test`; chạy cục bộ trước khi mở PR.

## Quy ước code

**Comment nói "vì sao", không nói "cái gì".** Comment chỉ lặp lại dòng code bên dưới thì xoá.
Comment tốt giải thích một ràng buộc, một đặc thù của game, hoặc một quyết định không hiển
nhiên.

- Không dòng kẻ phân mục, không kể lể "Bước 1 / Bước 2", không emoji.
- Không ghi ngày, số issue, tên người dùng hay lịch sử "đã sửa ở ..." trong comment. Những
  thứ đó thuộc về commit message hoặc changelog.
- Doc comment (`///`) chỉ khi tên và chữ ký hàm chưa đủ nói lên ý nghĩa.
- Mỗi khối `unsafe` có comment `// SAFETY:` nêu lý do nó an toàn.

**Cấu trúc.**

- `lib.rs` giữ mỏng: nạp config, khởi tạo log, đăng ký tính năng.
- Mỗi tính năng một module. File vượt khoảng 400 dòng thì tìm chỗ để tách.
- Ưu tiên hàm nhỏ và return sớm thay vì lồng nhiều tầng.
- Logic dùng chung nằm ở `shared`. Không sao chép hàm giữa các mod; cũng không đưa code
  lên `shared` cho tới khi có mod thứ hai thật sự cần.
- Mod link tĩnh với `shared` và không chia sẻ trạng thái lúc chạy với mod khác.

**Lỗi và panic.** Mod chạy trong tiến trình game nên không được làm game sập. Dùng `Result`
và ghi log lỗi; chỉ dùng `unwrap`/`expect` cho điều kiện bất biến không thể sai. Không có gì
có thể panic bên trong hook.

**An toàn theo phiên bản game.**

- Tìm hàm của game bằng AOB (`shared::memscan`), không hard-code RVA, để bản cập nhật game
  không âm thầm làm hỏng mod. Kiểm pattern mới là duy nhất trên mọi bản exe được hỗ trợ.
- Không gọi `eldenring::rva::get()`: bảng chỉ phủ một số bản và sẽ panic ở bản khác.
- Không đặt hardware breakpoint (debug register) từ DLL: game sẽ crash.
- Inline hook không được cấp phát bộ nhớ hay lấy khoá.

**Phần mềm diệt virus.** Mod sửa bộ nhớ game nên vài cờ là không tránh được, nhưng một số thói
quen làm tăng cờ rất nhiều:

- Không chạy mã từ một `static` đã được cấp quyền thực thi; dùng
  `shared::codepatch::alloc_executable`. DLL nhảy vào vùng dữ liệu của chính nó bị nhiều engine
  gắn cờ.
- Không xuất gì ngoài `DllMain`. Dùng `sym` trong `global_asm!` thay vì `#[no_mangle]`.
- Không đóng gói, làm rối hay mã hoá DLL.
- Không viết thẳng tên file của mod (`<Mod>.toml`, `<Mod>.log`) trong code; ghép từ
  `shared::dll_stem`.
- Giữ dòng `/PDBALTPATH:mod.pdb` trong `build.rs`, và không để tên mod trong `FileDescription` (`Elden Ring mod`).
  Mỗi chỗ này đã từng bị engine gắn cờ.

**Config.** TOML, một file cạnh DLL, tạo từ bản mặc định nhúng sẵn ở lần chạy đầu. Khoá còn
thiếu được thêm từ bản mặc định mà không đụng vào comment hay giá trị của người dùng. Mỗi khoá
trong file mặc định có comment ngắn: nó làm gì và khoảng giá trị. Mod từng đọc `.ini` nhập nó
một lần (`import_ini`) và giữ giá trị cũ.

**Log.** Mỗi mod một file log cạnh DLL. Thông điệp ngắn và kỹ thuật: `Thành phần: trạng thái`,
ví dụ `Menu: registered`. Lỗi kèm lý do. Không in danh sách dài ở mức log bình thường. Log viết
cho dev, không cho người chơi; ngoại lệ là lời nhắn cố ý như
`game may need a mod update, check Nexus`.

**Đặt tên.** Theo chuẩn Rust: hàm và module `snake_case`, kiểu `UpperCamelCase`, hằng
`SCREAMING_SNAKE_CASE`. Ngoại lệ duy nhất là `[lib] name` của crate viết PascalCase để DLL có
đúng tên file (`#![allow(non_snake_case)]` trong `lib.rs`).

## Quy ước tài liệu

- Văn bản cho người dùng (README, CHANGELOG) viết cho người chơi: nói mod làm gì, không nói
  code tổ chức ra sao.
- README của mọi mod ngắn và có cùng ba mục: Giới thiệu (mod làm gì và các giới hạn đã biết, bằng
  ngôn ngữ thường), Cài đặt, và Tài liệu (changelog, trang Nexus và các trang `docs/`). Cách hoạt
  động đặt ở `docs/`, không đặt trong README.
- Ghi chú kỹ thuật (offset, cách một hệ thống của game vận hành) đặt ở `mods/<tên>/docs/`; cách một
  module chung hoạt động đặt ở `shared/docs/`; bảng tra nhiều mod cùng dùng đặt ở `docs/`.
- Chỉ nêu tên mod khác khi mod này cố ý hỗ trợ hoặc phát hiện nó (ví dụ Seamless Co-op); còn lại
  viết "a third-party mod".
- README và CHANGELOG bằng tiếng Anh. Mỗi mod có thêm `README.vi.md` và giữ đồng bộ.
- Mục Features trên `nexus_page.bbcode`: mỗi bullet một ý, một câu ngắn; không nhồi số liệu, mặc định
  hay "(adjustable)" vào một dòng.
- CHANGELOG theo [Keep a Changelog](https://keepachangelog.com): mỗi thay đổi một dòng ngắn,
  ví dụ `Added X option.` hoặc `Fixed crash when Y.` Chỉ ghi tính năng và bản sửa người dùng
  thấy được: không ghi thay đổi về log, debug, build hay phát triển. Phiên bản theo
  [SemVer](https://semver.org). Tăng `version` trong `Cargo.toml` của mod trong cùng commit biến
  `[Unreleased]` thành một bản phát hành.

## Git

- `main` luôn build được. Làm việc trên nhánh `<loại>/<chủ đề-ngắn>`, ví dụ
  `feat/autopickup-radius`, `fix/speed-seamless`.
- Commit theo [Conventional Commits](https://www.conventionalcommits.org), scope là tên mod:
  `fix(autoregen): stop healing while dead`. Tiêu đề dạng mệnh lệnh, dưới 72 ký tự, mỗi commit
  một thay đổi logic.
- Không commit binary, file game, `regulation.bin`, crash dump, API key hay đường dẫn máy cá
  nhân.
- `.local/`, `TODO.md` (ghi chú cá nhân, file nháp) và `dumps/` được git bỏ qua. Để ở đó
  mọi thứ không dành cho repo.
- Thử trong game trước khi mở PR và nói rõ đã thử với bản game nào, cấu hình mod nào (vanilla,
  Seamless Co-op, ...).
