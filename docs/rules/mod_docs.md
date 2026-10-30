# Quy tắc: tài liệu của mỗi mod

Tách từ `AGENTS.md` (2026-10-30). Đọc khi xong một thay đổi ở 1 mod, hoặc khi thêm/xoá/đổi tên mod.

## Tài liệu của mỗi mod: README, CHANGELOG, HISTORY, docs

Mỗi mod có 4 loại tài liệu, mỗi loại 1 vai trò, không trộn lẫn:

- `mods/<mod>/README.md` = **mô tả chung, ít đổi** (không phải nhật ký): mod làm
  gì, cấu hình ở mức khái niệm, cách hoạt động, giới hạn; link tới CHANGELOG,
  HISTORY và docs. **Không** ghi Nexus ID, trạng thái phát hành hay "đã test"
  (Nexus ID nằm ở `docs/rules/release.md` và README gốc). Kiểu trình bày: header căn
  giữa + badge, Giới thiệu, Tính năng, Cấu hình, Cách hoạt động, Giới hạn, Tài liệu.
  **Không** ghi bảng key, giá trị mặc định hay số phiên bản: tên key và mặc
  định nằm ở chú thích của file config `.ini`/`.toml` (nguồn sự thật duy nhất), phiên bản ở
  `CHANGELOG.md` và `Cargo.toml`. Số liệu hay đổi (số dòng param, RVA, bảng
  tương thích) để ở `docs/`. Khuôn mẫu: `mods/spiritmultiplier/README.md`.
- `mods/<mod>/CHANGELOG.md` = ghi chú phát hành cho **người dùng cuối**, theo
  từng version (mới nhất ở trên); mục `## [Unreleased]` gom các thay đổi chưa
  phát hành. Mỗi bullet 1 dòng ngắn, văn bản thuần không định dạng, nói điều
  người dùng thấy, không ghi tên key ini hay chi tiết nội bộ (văn phong: xem
  `docs/rules/style.md`). Skill `deploy-nexus-mod` gửi nội dung này
  lên tab Changelog của Nexus; `nexus_page.bbcode` (trang mod) **không** chứa
  changelog. **Mod chưa phát hành** (chưa lên Nexus lần nào, chưa có Nexus ID
  ở `docs/rules/release.md`): `[Unreleased]` chỉ giữ đúng một dòng `- Initial
  release`, **không thêm bullet nào** vì chưa có bản phát hành nào để so các
  thay đổi (người dùng, 2026-10-27); mọi thay đổi trong lúc phát triển chỉ ghi ở
  HISTORY và docs. Khi mod lên Nexus lần đầu thì mục này thành version đầu tiên
  với đúng dòng đó.
- `mods/<mod>/HISTORY.md` = dòng thời gian, bảng `Ngày | Thay đổi | Chi
  tiết`, mỗi thay đổi **1 dòng**, mới nhất ở dưới cùng; chuyện nào có phân
  tích dài thì cột "Chi tiết" link sang file trong `docs/` của mod.
- `mods/<mod>/docs/<chủ đề>.md` = phân tích theo chủ đề (vd.
  `mods/dropmultiplier/docs/materials.md`), viết theo **trạng thái hiện
  tại** (không viết kiểu nhật ký theo ngày): mở đầu bằng `**Status <ngày>:
  ...**`, rồi kết luận, bằng chứng, hướng đã thử/bỏ (kèm lý do), tham chiếu
  code. Chủ đề liên mod và tư liệu dùng chung (trang Nexus, prompt
  logo/thumbnail, spec Nexus API) nằm ở `docs/` gốc repo. Tư liệu về cách các
  module của crate `common` hoạt động (AOB, hook, lý do thiết kế, bằng chứng
  kiểm chứng) nằm ở `shared/docs/<chủ đề>.md`, cùng quy ước `Status` như trên;
  doc đầu file `.rs` chỉ giữ vài dòng rồi link sang đó. Công cụ dịch ngược
  nằm ở `scripts/` (IDA: `scripts/ida/`); dữ liệu sinh ra từ dịch ngược nằm ở `dumps/` (git-ignored,
  ngoại lệ `dumps/seamless-diff/`).

Sau khi hoàn thành một thay đổi đáng kể ở 1 mod (thêm tính năng, đổi ini
key, sửa bug về hành vi, đổi cấu trúc code/thư mục):

1. **Luôn thêm 1 dòng** vào bảng `HISTORY.md`, ghi ngày hôm nay: đã đổi gì và
   vì sao (1-2 câu, đặc biệt nếu sửa 1 hiểu lầm/bug trong thiết kế trước đó).
2. **Nếu người dùng nhìn thấy thay đổi đó** (tính năng, sửa lỗi, đổi hành vi) **và
   mod đã phát hành**: thêm 1 bullet vào `## [Unreleased]` trong `CHANGELOG.md`.
   Mod chưa phát hành thì bỏ qua bước này (xem quy tắc "Initial release" ở trên).
3. **Sửa `README.md` tại chỗ** chỉ khi cách hoạt động, giới hạn hay trạng thái
   (đã phát hành / nội bộ / đã bỏ) thay đổi. Đổi key hay giá trị mặc định chỉ cần
   sửa chú thích trong file config (`.ini`/`.toml`), không đụng README (không thêm mục có ngày); sửa
   đường dẫn file lỗi thời được nhắc tới.
4. **Viết mới hoặc bổ sung `mods/<mod>/docs/<chủ đề>.md`** chỉ khi có phát hiện
   đáng giữ: nguyên nhân gốc, hướng đã thử/bỏ, số liệu, RVA/AOB. Cập nhật dòng
   `Status`, rồi link từ dòng HISTORY và từ README.

Mọi mod hiện có đều đã theo cấu trúc này; mod mới tạo thì làm theo khuôn mẫu
`mods/dropmultiplier/`. `CHANGELOG.md` có cho mọi mod đăng Nexus (không có ở
`sometweaks` và `windowresize`: không đăng).

README gốc repo (`README.md`) có bảng liệt kê mod (tên, chức năng, Nexus ID)
và badge số mod. Khi **thêm, xoá, đổi tên** 1 mod, hoặc mod lần đầu lên
Nexus / đổi trạng thái, cập nhật bảng và badge đó cùng lúc. Không ghi số
phiên bản vào README gốc.

Không cần hỏi lại người dùng trước khi làm việc này - tự làm ngay sau khi
code đã ổn định, coi đây là 1 bước không thể thiếu của việc "xong việc",
giống như build/test.
