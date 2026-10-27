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
- Không tự merge, xoá hay đổi tên nhánh; không tự commit khi chưa được kiểm
  (xem mục "Quy tắc code và quy trình thử" bên dưới).

## Quy tắc code và quy trình thử

- **Thử trong game:** sau khi build mod để người dùng thử, chép DLL (và file ini
  nếu đổi) vào thư mục `mod` của game Steam, ghi đè, không đợi nhắc:
  `E:\SteamLibrary\steamapps\common\ELDEN RING\Game\mod`. Thư mục khác (vd.
  `E:\Games\ELDEN.RING_LinkNeverDie.Com\Game\mod_test`) chỉ khi người dùng nói
  ra. Mod phải có trong `external_dlls` của `config_eldenring.toml` ở đó; nếu
  chép báo `Device or resource busy` thì game đang giữ DLL, bảo người dùng thoát
  game.
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

## Tài liệu của mỗi mod: README, CHANGELOG, HISTORY, docs

Mỗi mod có 4 loại tài liệu, mỗi loại 1 vai trò, không trộn lẫn:

- `mods/<mod>/README.md` = **mô tả chung, ít đổi** (không phải nhật ký): mod làm
  gì, cấu hình ở mức khái niệm, cách hoạt động, giới hạn; link tới CHANGELOG,
  HISTORY và docs. **Không** ghi Nexus ID, trạng thái phát hành hay "đã test"
  (Nexus ID nằm ở danh sách bên dưới và README gốc). Kiểu trình bày: header căn
  giữa + badge, Giới thiệu, Tính năng, Cấu hình, Cách hoạt động, Giới hạn, Tài liệu.
  **Không** ghi bảng key, giá trị mặc định hay số phiên bản: tên key và mặc
  định nằm ở chú thích của file config `.ini`/`.toml` (nguồn sự thật duy nhất), phiên bản ở
  `CHANGELOG.md` và `Cargo.toml`. Số liệu hay đổi (số dòng param, RVA, bảng
  tương thích) để ở `docs/`. Khuôn mẫu: `mods/spiritmultiplier/README.md`.
- `mods/<mod>/CHANGELOG.md` = ghi chú phát hành cho **người dùng cuối**, theo
  từng version (mới nhất ở trên); mục `## [Unreleased]` gom các thay đổi chưa
  phát hành. Mỗi bullet 1 dòng ngắn, văn bản thuần không định dạng, nói điều
  người dùng thấy, không ghi tên key ini hay chi tiết nội bộ (văn phong: xem
  memory `feedback_changelog_style`). Skill `deploy-nexus-mod` gửi nội dung này
  lên tab Changelog của Nexus; `nexus_page.bbcode` (trang mod) **không** chứa
  changelog. **Mod chưa phát hành** (chưa lên Nexus lần nào, chưa có Nexus ID
  ở danh sách bên dưới): `[Unreleased]` chỉ giữ đúng một dòng `- Initial
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

## Viết `TODO.md` (gốc repo) gọn

`TODO.md` chỉ trả lời "còn gì phải làm", không phải nơi ghi phân tích:

- Chỉ liệt kê việc **còn mở**, chia theo mod. Việc xong thì **xoá luôn**
  (không tick `[x]` rồi để lại; lịch sử đã có ở `HISTORY.md`).
- Mỗi việc **1 dòng** (~150 ký tự): việc gì + nguồn báo (người dùng, ngày) nếu
  có. Không bullet con. Lý do, bằng chứng, hướng đã thử, RVA/số liệu thì viết
  vào `mods/<mod>/docs/<chủ đề>.md` (mod chưa có `docs/` thì tạo) rồi link từ
  dòng TODO hoặc từ tiêu đề mod.
- Việc đã chốt không làm thì **xoá khỏi TODO** và ghi vào `HISTORY.md` (1 dòng)
  hoặc `docs/` của mod; TODO không có mục "Đã dừng".
- Khi gộp/xoá mục có chi tiết chưa nằm ở đâu khác, chuyển chi tiết sang
  `docs/` trước, không xoá mất.
- Cần ưu tiên thì thêm nhãn ngắn (`[bug]`, `[research]`, `[chờ phản hồi]`).

## Văn phong: log, danh sách Features, CHANGELOG (ngắn, đúng đối tượng)

Người dùng (2026-10-16) phải nhắc lại nhiều lần; viết đúng ngay từ đầu:

- **Log (`logger::log/warn/error`) viết cho dev, không cho người dùng cuối:**
  ngắn, kỹ thuật, dạng `<Thành phần>: <trạng thái>` (vd. `Menu: registered`,
  `SoloParamRepository: waiting for world`, `Reload: deferred, not in world`),
  không câu dài kiểu giải thích, không "..." kể lể. Không in danh sách ID hay
  dump nhiều dòng ở lần chạy bình thường (chỉ ghi số lượng, vd. `set on 8 Black
  Flame row(s)`); dump chi tiết chỉ để dò lỗi tạm rồi bỏ. Lỗi phải kèm lý do
  (`... failed (<reason>)`). Mod đơn tính năng không gắn tên mod vào dòng log.
  Cảnh báo cố ý nói với người dùng (vd. "check Nexus") là ngoại lệ.
- **Features trên `nexus_page.bbcode`:** mỗi bullet **một ý, một câu ngắn**
  (vd. `Longer Black Flame and Destined Death burn duration.`); ý khác thì tách
  bullet khác. Không nhồi số liệu, mặc định, "(adjustable)" hay giải thích vào
  một dòng.
- **`CHANGELOG.md`:** mỗi bullet ngắn nhất có thể, nêu tên tính năng/sửa lỗi,
  không kể chi tiết (giá trị mặc định, "last forever", cơ chế). Dạng
  `Added <X> and <Y> options.` / `Fixed <loại lỗi>.`; chi tiết để ở HISTORY và docs.

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

## Kiểm tra `shared/` (`common`) trước khi viết helper mới

Trước khi viết 1 hàm/struct/module mới (helper gate "đã vào game", chờ
singleton, đăng ký task, quét AOB, đọc/ghi config, log, banner thông
báo...), **luôn tìm xem đã có sẵn chưa** - trước hết trong `shared/src`
(crate `common`, mọi mod dùng chung), rồi đến các mod khác trong `mods/`
(có thể đã tự viết 1 bản chưa kịp đưa lên `common`). Tìm theo cả tên lẫn
chức năng (vd. grep `main_player`, `WorldChrMan`, `instance()`), không chỉ
theo đúng tên định đặt.

- Đã có trong `common` → dùng luôn, không tự viết bản riêng.
- Có ở 1 mod khác nhưng chưa ở `common`, và mod đang sửa cũng cần → đề
  xuất đưa lên `common` thay vì copy thêm 1 bản.
- Chỉ viết mới khi chắc chắn không có gì tương đương; nếu bản sẵn có
  không đủ (vd. chỉ đọc, cần ghi), nói rõ lý do trong comment.

Ví dụ đã xảy ra (2026-09-23): tự viết `in_game()` trong
`passiverunes/src/rune.rs` trong khi `common::player::main_player_chr_ins_ptr()`
đã làm đúng việc đó và được `DropMultiplier`/`SomeTweaks`/`AutoRegen` dùng
sẵn.

