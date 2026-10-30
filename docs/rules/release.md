# Quy tắc: phát hành và cấu hình dự án

Tách từ `AGENTS.md` (2026-10-30). Đọc khi phát hành mod, chốt version, thêm/xoá/đổi tên mod, hoặc cần Nexus ID.

## Thông tin phiên bản trong DLL (Properties > Details)

Mỗi mod khai báo ở `[package.metadata.winresource]` của `Cargo.toml` (`build.rs`
chỉ gọi `winresource`): `ProductName`, `FileDescription`, `InternalName`,
`OriginalFilename`, `LegalCopyright = "Copyright (c) 2026 ALR2310"` (tên theo
`LICENSE`). File version lấy từ `version`. Mod mới làm đủ năm trường theo
`mods/dropmultiplier/Cargo.toml`.

## Cập nhật `.vscode/tasks.json` khi thêm / xoá / đổi tên mod

Mỗi mod trong `mods/` có 1 task build riêng trong `.vscode/tasks.json`
và nằm trong `dependsOn` của task `Cargo: Build All (Release)`. Khi
**thêm, xoá hoặc đổi tên** 1 mod, **luôn cập nhật file này cùng lúc**:

- Thêm: 1 task mới theo đúng mẫu các task sẵn có - `label` =
  `Cargo: Build <Mod> (Release)`, `-Mod <Mod>` với `<Mod>` lấy từ
  `[lib] name` trong `mods/<mod>/Cargo.toml` (PascalCase, không tự
  viết hoa tên thư mục) - và thêm label đó vào `dependsOn` của Build All.
- Xoá: bỏ task của mod đó và dòng tương ứng trong `dependsOn`.
- Đổi tên: sửa `label`, `detail`, `-Mod` và dòng trong `dependsOn`.

Sau khi sửa, kiểm tra file vẫn là JSON hợp lệ và số task build riêng
bằng đúng số thư mục trong `mods/`. Không cần hỏi lại người dùng.
Người dùng yêu cầu (2026-09-30) sau khi phát hiện thiếu task của
FasterRevival, SpeedMultiplier, SpiritMultiplier, WindowResize.

## Đối chiếu changelog thật trên Nexus Mods trước khi chốt version trong `CHANGELOG.md`

Trước khi cắt `[Unreleased]` thành 1 mục version (hoặc sửa mục version cũ)
trong `mods/<mod>/CHANGELOG.md`, **luôn gọi Nexus Mods API để lấy changelog
thật hiện có trên trang mod** trước, rồi mới chốt version nối tiếp đúng theo đó - không tự đặt số phiên
bản/nội dung dựa trên suy đoán hay dựa vào lịch sử trong README/HISTORY (lịch sử
này ghi theo ngày phát triển nội bộ, không phải số phiên bản đã publish
trên Nexus, 2 thứ có thể lệch nhau).

Cách gọi:

```bash
set -a; . <(tr -d '\r' < .env); set +a   # nạp NEXUS_API_KEY
curl -sS -H "apikey: $NEXUS_API_KEY" \
  "https://api.nexusmods.com/v1/games/eldenring/mods/<mod_id>/changelogs.json"
```

Mod ID theo từng mod trong workspace này (game domain luôn là `eldenring`):

- `autoregen` → 10548
- `dropmultiplier` → 11075
- `infiniteailment` → 11087
- `runemultiplier` → 10630
- `weightmultiplier` → 10549 (Weight Multiplier)
- `passiverunes` → 10528
- `risearcher` → 5807
- `sometweaks` → **chưa có, chưa đăng lên Nexus**
- `soulsteleport` → 11119 (Souls Teleport)
- `fasterrevival` → 11160 (Faster Revival)
- `spiritmultiplier` → 11168 (Spirit Multiplier)
- `speedmultiplier` → 11173 (Speed Multiplier)
- `animcancel` → 11328 (Anim Cancel)
- `autopickup` → 11470 (Auto Pickup)
- `windowresize` → **không đăng lên Nexus, dùng nội bộ (quyết định 2026-10-07)**

Quét một file bằng VirusTotal (dùng thủ công, không nằm trong quy trình
phát hành): `python scripts/vt-scan.py <zip>` (khoá `VT_API_KEY` trong `.env`,
miễn phí; tra hash trước, chưa có thì tải lên và chờ kết quả).

Key API cá nhân của người dùng lưu ở biến `NEXUS_API_KEY` trong `.env` ở
gốc repo (mẫu: `.env.example`; chỉ dùng nội bộ cho quy trình phát hành,
không mod nào đọc nó lúc chạy/build; đã
gitignore, không commit). Nếu API trả về thiếu 1 vài version cũ so với thực
tế trên trang web (đã xảy ra 1 lần, 2026-09-03) - đó là do giới hạn của
API, không phải version đó không tồn tại - hỏi lại người dùng xác nhận
qua trang web thật trước khi tự xoá bất kỳ mục changelog cũ nào.

## Bump `version` trong `Cargo.toml` cùng lúc với version mới trong `CHANGELOG.md`

Khi cắt `[Unreleased]` thành 1 mục version mới trong
`mods/<mod>/CHANGELOG.md` (thường do skill `deploy-nexus-mod` làm lúc phát
hành), **luôn đổi luôn `version` trong
`mods/<mod>/Cargo.toml` thành đúng số version đó** (vd. changelog
`1.1.0` → `version = "1.1.0"`), rồi build lại để `Cargo.lock` cập nhật
theo. Version crate phải luôn khớp với version mới nhất trên Nexus.

Không cần hỏi lại người dùng - coi đây là 1 phần của việc chốt version.
Người dùng yêu cầu (2026-09-29, SpiritMultiplier 1.1.0) vì trước đó version
crate không được bump (vd. `autoregen` vẫn `2.0.0` trong khi Nexus đã lên
2.6.x).
