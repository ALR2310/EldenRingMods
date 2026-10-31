# Project Rules

## Nhánh git: mỗi mod một nhánh `mod/<tên thư mục mod>`

Mọi việc thuộc 1 mod (tính năng, sửa lỗi, nghiên cứu, tài liệu, chỉnh config)
làm trên **1 nhánh duy nhất của mod đó**: `mod/<tên thư mục trong mods/>`, vd.
`mod/nofalldamage`. Không tách `feat/`, `fix/`, `docs/`... theo loại việc; gom
hết vào nhánh `mod/` của mod.

- Bắt đầu việc của 1 mod: nếu nhánh `mod/<mod>` chưa có thì tạo từ `main`, có
  rồi thì chuyển sang nó (kiểm `git branch` trước, đừng tạo nhánh trùng ý).
- Thay đổi ở `shared/` (crate `common`) hay file gốc repo (README gốc, `TODO.md`,
  `.vscode/tasks.json`, `Cargo.toml`) mà việc của mod đó cần thì cùng nhánh với
  mod. Việc không thuộc mod nào (hạ tầng, quy tắc chung) thì hỏi người dùng
  nhánh nào trước khi làm.
- **Ngoại lệ: thay đổi cùng một kiểu trên nhiều mod** (vd. thêm một trường vào
  `[package.metadata.winresource]` của mọi mod) làm và commit trên `main`, không
  rải lên từng nhánh `mod/` (người dùng, 2026-10-27). Mod chỉ có trên nhánh của
  nó (chưa vào `main`) thì sửa trên nhánh đó.
- **Ngoại lệ: InputBridge** (người dùng, 2026-10-27): cải tiến `inputbridge` làm
  thẳng trong phiên đang làm mod khác, không cần chuyển nhánh.
- Không tự merge, xoá hay đổi tên nhánh; không tự commit khi chưa được kiểm
  (xem mục "Quy tắc code và quy trình thử" bên dưới).

## Quy tắc code và quy trình thử

- **Thử trong game:** sau khi build mod để người dùng thử, chép DLL (và file ini
  nếu đổi) vào thư mục `mod` của game Steam, ghi đè, không đợi nhắc:
  `E:\SteamLibrary\steamapps\common\ELDEN RING\Game\mod`. Thư mục khác (vd.
  `E:\Games\ELDEN.RING_LinkNeverDie.Com\Game\mod_test`) chỉ khi người dùng nói
  ra. Mod phải có trong `external_dlls` của `config_eldenring.toml` ở đó; nếu
  chép báo `Device or resource busy` thì game đang giữ DLL. Nếu người dùng đã
  bảo chép (lời họ là uỷ quyền) thì tự `taskkill /IM eldenring.exe /F`, đợi
  thoát, chép rồi `cmp` kiểm, nói 1 dòng là đã đóng game (người dùng, 2026-10-26,
  chán phải nhắc "tôi đã thoát game, chép lại"). Khi tự build theo ý mình mà chép
  lỗi thì không kill, chỉ báo game đang giữ DLL.
- **Chơi game qua MCP `eldenring-input`:** gọi `notes()` trước mỗi phiên; khi bị
  người dùng sửa phím hay thử ra điều mới (thời gian, quy trình) thì `add_note`
  ngay (source `user` / `observed`), ghi sai thì sửa thẳng file, không thêm dòng
  mâu thuẫn. Sổ tay nằm ở `mods/inputbridge/docs/game_notes.md` (nhánh
  `mod/inputbridge`). Phím của người dùng: Space = nhảy, Shift = lăn (2026-10-27
  từng nhầm Space là lăn).
- **Không commit khi chưa được kiểm:** không commit tính năng hay bản sửa mà
  người dùng chưa thử và xác nhận chạy trong game, trừ khi họ yêu cầu commit
  (2026-10-09: commit CloneSpirit chưa thử bị phản đối). Code và dòng HISTORY /
  docs của nó cùng để chưa commit và ghi "chưa test"; các bước tài liệu "xong
  việc" vẫn làm, chỉ commit sau khi được xác nhận.
- **Không dọn `builds/`** (trước đây tên `build/`): đó là DLL, config, zip mà
  người dùng giữ dùng (thử, đăng Nexus), không phải rác (2026-10-05). Không
  `rm`, `Remove-Item`, `git clean` lên nó, kể cả file cũ; chỉ ghi đè file của mod
  đang build bằng `scripts/build-mod.ps1`.
- **Tin kết quả thử có điều kiện:** trước khi coi một kết quả thử là bằng chứng
  (nhất là khi bất ngờ, hoặc làm cơ sở một kết luận lớn), xác nhận
  `external_dlls` không còn DLL tham khảo hay DLL cộng đồng nào khác bật cùng
  DLL đang thử (er10x.dll, các DLL dịch ngược trong `.docs/`). 2026-09-04: nhiều
  vòng "đã xác nhận chạy" của sometweaks hoá ra là `er10x.dll` tự chạy.
- **Không dùng API `eldenring::rva::get()`:** bảng RVA của fromsoftware-rs chỉ có
  2.7.1.0 và JP 2.7.1.1, nên trên 2.7.2.0 chúng panic (`Unsupported game version`,
  rồi `LazyLock poisoned` ở mọi lần gọi sau): `WorldChrMan::chr_ins_by_handle` /
  `as_subclass`, các static `GameMan` / `GameDataMan`, `ChrInsExt::apply_speffect`
  / `remove_speffect`. Dùng AOB (`common::memscan`, kiểm duy nhất ở mọi bản exe
  bằng `scripts/ida/run.ps1 aob -Version all`) hoặc gọi thẳng slot vtable (vd.
  `get_chr_ins_by_handle` = slot 3 của ChrSet, xem `warp.rs::resolvable`).
  2026-10-22: SpiritMultiplier Warp panic mỗi frame trên 2.7.2.0.
- **Không đặt hardware breakpoint (debug register) trong DLL:** game crash, nghi do
  chống can thiệp (`docs/crash_dump_analysis.md`). Muốn quan sát một hàm thì hook
  inline kiểu `mods/autoregen/src/hit_hook.rs`, và trong hook không cấp phát bộ
  nhớ hay lấy khoá.
- **Mod không chạy với Seamless Co-op:** tra `dumps/seamless-diff/` (có README)
  trước khi dịch ngược `ersc.dll`.
- **Mod đã bỏ:** `sometweaks` (2026-09-29, không có kế hoạch). Khi chuyển tính năng
  từ SomeTweaks sang mod khác thì sao chép và cải tiến ở mod đích; không đề xuất
  sửa hay refactor SomeTweaks, và "SomeTweaks cũng dùng" không phải lý do đưa code
  lên `common`. `windowresize` chỉ dùng nội bộ (2026-10-07): không đề xuất phát
  hành, không có CHANGELOG.
- **Tài liệu dùng chung:** viết trang Nexus, changelog, gọi API Nexus, prompt logo:
  `docs/nexus.md`. Phân tích dump crash: `docs/crash_dump_analysis.md`. Cổng "đã
  vào game", đọc/ghi param an toàn, hot reload, log riêng tư: `shared/docs/`.

## Mod nội bộ và mod public: `[package.metadata.mod] audience`

Mỗi mod khai báo trong `Cargo.toml` của nó `[package.metadata.mod] audience =
"internal"` (dùng nội bộ, hiện: `windowresize`, `inputbridge`, `toolbox`) hoặc
`"public"` (lên Nexus). Mod `internal` không CHANGELOG, không Nexus ID, không phát
hành, không có trên VirusTotal; chỉ cần README, HISTORY, docs. Mod mới phải khai
nhãn này. Không tách thư mục (người dùng, 2026-10-31, chỉ gắn nhãn cho nhẹ).

## Thư mục `tmp/` (git-ignored)

- `tmp/repos/`: mã nguồn tham khảo clone về (souls_vision, kubilaiswf-minimap,
  ERNativeUI...); trong tài liệu dẫn bằng `tmp/repos/<tên>`.
- `tmp/csv/`: dữ liệu param xuất ra (bản vanilla, Convergence...).
- `tmp/scratchpad/`: **mọi file tạm của phiên làm việc** (script sửa hàng loạt,
  ảnh chụp, dump IDE...) lưu ở đây, không để ở thư mục scratchpad của hệ thống vì
  Windows dọn nó (người dùng, 2026-10-27). Dump giải mã IDA lẻ để ở
  `tmp/scratchpad/ida/`. Không để file lẻ ngay trong `tmp/`.

## Quy tắc theo việc: ĐỌC file tương ứng TRƯỚC khi làm

Các quy tắc dưới đây chỉ áp dụng cho từng loại việc nên để ở `docs/rules/`.
Gặp việc nào thì mở file đó ra đọc đủ, đừng làm theo trí nhớ:

- **Xong một thay đổi ở 1 mod** (HISTORY, CHANGELOG, README, docs của mod; thêm/xoá/đổi
  tên mod, README gốc): `docs/rules/mod_docs.md`. Luôn làm bước tài liệu này, không cần hỏi.
- **Viết log, Features Nexus, CHANGELOG, sửa `TODO.md`:** `docs/rules/style.md`
  (ngắn, đúng đối tượng; TODO chỉ liệt kê việc còn mở, 1 dòng/việc).
- **Phát hành / chốt version / cần Nexus ID / thêm mod mới** (đối chiếu changelog thật
  qua API, bump `version`, `.vscode/tasks.json`, thông tin DLL, VirusTotal):
  `docs/rules/release.md`.
- **Trang Nexus, prompt logo, API Nexus:** `docs/nexus.md`. **Dump crash:**
  `docs/crash_dump_analysis.md`. **Công cụ IDA:** `scripts/ida/README.md`.

## Trước khi viết helper mới: tìm trong `shared/` (`common`)

Trước khi viết hàm/struct/module mới (gate "đã vào game", chờ singleton, đăng ký
task, quét AOB, đọc/ghi config, log, banner...), **tìm xem đã có chưa**: trước hết
`shared/src`, rồi các mod khác trong `mods/`; tìm theo cả chức năng (grep
`main_player`, `WorldChrMan`, `instance()`), không chỉ theo tên định đặt. Đã có
trong `common` thì dùng luôn; có ở mod khác mà mod đang sửa cũng cần thì đề xuất
đưa lên `common`; chỉ viết mới khi chắc không có, và nói rõ lý do trong comment.
Ví dụ (2026-09-23): tự viết `in_game()` trong `passiverunes` trong khi
`common::player::main_player_chr_ins_ptr()` đã có.

## Ghi nhớ nằm trong repo, không nằm ở memory máy riêng

Người dùng (2026-10-30) muốn quy tắc, phản hồi, phát hiện của dự án nằm trong repo,
không ghi vào `~/.claude/projects/.../memory/`: quy tắc làm việc vào file này hoặc
`docs/rules/`, phát hiện kỹ thuật vào `mods/<mod>/docs/`, `shared/docs/`, `docs/`.
Khi người dùng sửa cách làm, ghi vào đúng file rồi báo lại. Giữ file này dưới
~150 dòng: quy tắc dài chỉ dùng cho một loại việc thì đưa sang `docs/rules/`.
