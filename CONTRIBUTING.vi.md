# Đóng góp

[English](CONTRIBUTING.md)

Trang này mô tả cách tổ chức repo và các quy ước chung của mọi mod, để một thay đổi ở mod này
đọc giống một thay đổi ở mod khác.

Nếu dùng trợ lý AI, cho nó đọc [AGENTS.md](AGENTS.md) và chạy prompt trong
[docs/review-prompt.md](docs/review-prompt.md) lên kết quả trước khi commit.

## Cấu trúc repo

```
common/            crate dùng chung cho mọi mod (config, log, quét bộ nhớ, hook, ...)
mods/<tên>/        một mod = một crate = một DLL
docs/              tài liệu chung của dự án
scripts/           script build và phát hành
```

Một mod gồm:

```
mods/<tên>/
├── Cargo.toml          phải có `[lints] workspace = true`
├── build.rs            nhúng version resource của Windows
├── <Tên>.toml          config mặc định, nhúng vào DLL lúc build
├── README.md           tiếng Anh: chức năng, cách hoạt động, giới hạn
├── README.vi.md        bản tiếng Việt
├── CHANGELOG.md        ghi chú phát hành cho người chơi (tiếng Anh)
├── license-notice.txt  tuỳ chọn; mọi file *license*.txt / *notice*.txt ở đây hoặc trong assets/
│                       được scripts/build-mod.ps1 đóng gói cùng DLL
└── src/
    ├── lib.rs          chỉ DllMain và nối các phần với nhau
    └── ...             mỗi tính năng một module
```

Nội dung dài hơn README (ghi chú nghiên cứu, offset, cách một hệ thống của game vận hành)
đặt ở `mods/<tên>/docs/<chủ đề>.md`.

## Build

Yêu cầu: Windows x64, Rust stable với target MSVC, Windows SDK (cần `rc.exe` cho version
resource).

```powershell
cargo build --release -p <crate>
cargo test --workspace
cargo fmt --all
cargo clippy --workspace --all-targets
```

Build và đóng gói một mod: `pwsh scripts/build-mod.ps1 -Mod <Mod> [-Zip]` (kết quả ở
`builds/<crate>/`).

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
- `common` lớn dần theo các mod: chỉ chuyển một module sang khi mod đang thêm vào cần nó.
- Logic dùng chung nằm ở `common`. Không sao chép hàm giữa các mod; cũng không đưa code
  lên `common` cho tới khi có mod thứ hai thật sự cần.
- Mod link tĩnh với `common` và không chia sẻ trạng thái lúc chạy với mod khác.

**Lỗi và panic.** Mod chạy trong tiến trình game nên không được làm game sập. Dùng `Result`
và ghi log lỗi; chỉ dùng `unwrap`/`expect` cho điều kiện bất biến không thể sai. Không có gì
có thể panic bên trong hook.

**An toàn theo phiên bản game.**

- Tìm hàm của game bằng AOB (`common::memscan`), không hard-code RVA, để bản cập nhật game
  không âm thầm làm hỏng mod. Kiểm pattern mới là duy nhất trên mọi bản exe được hỗ trợ.
- Không gọi `eldenring::rva::get()`: bảng chỉ phủ một số bản và sẽ panic ở bản khác.
- Không đặt hardware breakpoint (debug register) từ DLL: game sẽ crash.
- Inline hook không được cấp phát bộ nhớ hay lấy khoá.

**Config.** TOML, một file cạnh DLL, tạo từ bản mặc định nhúng sẵn ở lần chạy đầu. Khoá còn
thiếu được thêm từ bản mặc định mà không đụng vào comment hay giá trị của người dùng. Mỗi khoá
trong file mặc định có một dòng comment.

**Log.** Mỗi mod một file log cạnh DLL. Thông điệp ngắn và kỹ thuật: `Thành phần: trạng thái`,
ví dụ `Menu: registered`. Lỗi kèm lý do. Không in danh sách dài ở mức log bình thường.

**Đặt tên.** Theo chuẩn Rust: hàm và module `snake_case`, kiểu `UpperCamelCase`, hằng
`SCREAMING_SNAKE_CASE`. Ngoại lệ duy nhất là `[lib] name` của crate viết PascalCase để DLL có
đúng tên file (`#![allow(non_snake_case)]` trong `lib.rs`).

## Quy ước tài liệu

- Văn bản cho người dùng (README, CHANGELOG) viết cho người chơi: nói mod làm gì, không nói
  code tổ chức ra sao.
- README của mọi mod có cùng các mục: tóm tắt, cài đặt, cấu hình, cách hoạt động, tương thích
  và giới hạn.
- Ghi chú kỹ thuật (offset, cách một hệ thống của game vận hành) đặt ở `mods/<tên>/docs/`.
- README và CHANGELOG bằng tiếng Anh. Mỗi mod có thêm `README.vi.md` và giữ đồng bộ.
- CHANGELOG theo [Keep a Changelog](https://keepachangelog.com): mỗi thay đổi một dòng ngắn,
  ví dụ `Added X option.` hoặc `Fixed crash when Y.` Phiên bản theo [SemVer](https://semver.org). Tăng `version` trong `Cargo.toml` của mod
  trong cùng commit biến `[Unreleased]` thành một bản phát hành.

## Git

- `main` luôn build được. Làm việc trên nhánh `<loại>/<chủ đề-ngắn>`, ví dụ
  `feat/autopickup-radius`, `fix/speed-seamless`.
- Commit theo [Conventional Commits](https://www.conventionalcommits.org), scope là tên mod:
  `fix(autoregen): stop healing while dead`. Tiêu đề dạng mệnh lệnh, dưới 72 ký tự, mỗi commit
  một thay đổi logic.
- Không commit binary, file game, `regulation.bin`, crash dump, API key hay đường dẫn máy cá
  nhân.
- Thử trong game trước khi mở PR và nói rõ đã thử với bản game nào, cấu hình mod nào (vanilla,
  Seamless Co-op, ...).
