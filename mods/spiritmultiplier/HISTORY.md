# SpiritMultiplier: dòng thời gian

Mỗi dòng một thay đổi, mới nhất ở dưới cùng. Chuyện nào có phân tích dài thì
link sang file trong `docs/` của mod. Bản nhật ký chi tiết cũ (trước 2026-10-08) nằm trong
lịch sử git của `README.md`.

| Ngày | Thay đổi | Chi tiết |
|---|---|---|
| 2026-09-25 | Tạo mod, thêm `SlotProbe`: đo `summon_buddy_chr_set` có capacity 80, band người chơi bắt đầu ở slot 20, trần 10 nằm ở `sub_140493380` | [docs](docs/slots_and_band.md) |
| 2026-09-25 | 0.2.0: nới band lên 60 slot (patch tìm slot + next-cursor) và `chain.rs` (`Multiplier`/`Amount`); test 30 sói, dismiss sạch | [docs](docs/slots_and_band.md) |
| 2026-09-25 | `MaxSpirits`: nới luôn capacity của ChrSet (test 100 con, offline); chốt mặc định `Multiplier=2`, `Amount=0`, `MaxSpirits=60` là mức tối thiểu | [docs](docs/slots_and_band.md) |
| 2026-09-26 | Seamless Co-op: chia band theo capacity cho người chơi index 1-5 (Seamless nới capacity lên 1000); sửa next-cursor cho band không bắt đầu ở #20 | [docs](docs/slots_and_band.md) |
| 2026-09-26 | Kẻ địch biến mất khi có 60 spirit: spirit chiếm suất trong giới hạn 60 nhân vật active; `activate_limit.rs` nâng giới hạn theo số spirit (luôn bật, bỏ key `KeepEnemySlots`); `EnemyProbe` gây crash nên tắt | [docs](docs/activate_limit.md) |
| 2026-09-27 | Log phiên bản game + danh sách DLL đã nạp (`common::diag`); `GhostColor` ban đầu gỡ SpEffect 295xxx mỗi frame, rồi chuyển sang sửa NpcParam (hot reload 2 chiều) | [docs](docs/ghost_color.md) |
| 2026-09-29 | `Regen` (spirit và Torrent tự hồi máu, copy từ SomeTweaks); tra cứu cờ hành vi spirit (297xxx) | [docs](docs/ghost_color.md) |
| 2026-09-29 | Thử `WarpDistance`/`WarpBlockedTime`: ghi được ngưỡng nhưng spirit không dịch chuyển, gỡ trước 1.0.0, hoãn | [docs](docs/warp_research.md) |
| 2026-09-29 | 1.0.0 phát hành (Nexus mod 11168) | |
| 2026-09-29 | `MultiSpirit`: gọi nhiều Ash khác nhau cùng lúc (patch DoSummon/Update/UI/CanUseItem, giữ bia đá), cho về từng Ash, Ash gọi thêm tốn FP/HP; viết lại từ phân tích Solid Uncapper | [docs](docs/multi_spirit.md) |
| 2026-09-29 | `NoRestResummon` và `SummonAnywhere` (sửa `BuddyStoneParam`) | [docs](docs/summon_anywhere.md) |
| 2026-09-29 | 1.1.0 phát hành (`SummonAnywhere` mặc định `true`) | |
| 2026-09-30 | 1.1.1: `SummonAnywhere` không còn phụ thuộc bia đá được nạp (patch stateInfo 373, in-range, bia dự phòng) | [docs](docs/summon_anywhere.md) |
| 2026-09-30 | 1.1.2: `GhostColor` tắt ở `SpEffectVfx` (`phantomParamOverwriteType = 0`) thay vì gỡ SpEffect ô 26, chạy được trên Reforged; quy tắc tỉ lệ spirit/không-spirit để khỏi tắt vfx của kẻ địch | [docs](docs/ghost_color.md) |
| 2026-10-01 | 1.1.3: Ash đã nâng cấp (+1..+10) cho về đúng thay vì gọi thêm (làm tròn SpEffect `100 * (id / 100)` như `DoSummon`) | [docs](docs/multi_spirit.md) |
| 2026-10-01 | 1.1.4: mọi patch code đổi sang đổi đích `call` (`common::codepatch::redirect_rel32`) vì Seamless Co-op 2.0.1 tự huỷ game khi chữ ký byte không khớp | [docs](docs/seamless_conflict.md) |
| 2026-10-01 | 1.1.4: Reforged - sửa Spirit Ring của Spiritcaller hỏng do `SummonAnywhere` ép cờ in-range mỗi frame; Spirit-Severing Blade không bị `MultiSpirit` chặn | [docs](docs/summon_anywhere.md) |
| 2026-10-05 | Version thật trong thuộc tính file DLL (`build.rs` + `winresource`); thêm key `ReloadBanner` | |
| 2026-10-07 | Đổi thư mục `crates/` thành `mods/`; đường dẫn script/dump dịch ngược đổi chỗ (`.docs/` thành `tmp/`, script IDA ở `scripts/ida/`) | |
| 2026-10-08 | Tách README thành README + HISTORY + docs/; các vấn đề còn mở (Reforged Fury, Seamless mất màu ma, warp) ghi ở `docs/open_issues.md` | [docs](docs/open_issues.md) |
| 2026-10-08 | `MultiSpirit`: spirit đã chết vẫn nằm trong `groups` nên bấm lại Ash của nó (khi Ash khác còn sống) bị coi là "đang out" và chỉ thu hồi; nay kiểm tra HP chr_ins. Đã test in-game (2026-10-08): đúng | |
| 2026-10-08 | Thêm `SpiritThink.ini` (tuỳ chọn, thử nghiệm): chỉnh field NpcThinkParam của spirit khi đang chơi, không cần build lại, để nghiên cứu độ hung hăng | [docs](docs/think_override.md) |
| 2026-10-09 | Sửa `NoRestResummon=false` không có tác dụng khi `SummonAnywhere=true` (báo cáo trên Nexus): stub stateInfo 373 ép "được dùng" nên đè luôn khoá của vanilla; nay mod tự đọc cờ `summonedEventFlagId` gốc của bia và tắt stub khi cờ bật và không còn spirit. Đã test in-game (2026-10-09): đúng | [docs](docs/summon_anywhere.md) |
| 2026-10-09 | Thêm `CloneSpirit` (thử nghiệm, true/false, mặc định false): bấm lại Ash đang out thì gọi thêm thay vì cho về, tới `MaxSpirits` thì mới cho về; theo đề xuất của almasakmal123 trên Nexus. Đã test in-game (2026-10-09): đúng | [docs](docs/multi_spirit.md) |
| 2026-10-09 | Sửa chi phí FP/HP: `cost_state` đọc `refId_default` ở +0 (luôn 0) thay vì +4 nên chỉ lần gọi đầu tốn mana; thêm patch 8 (2 chỗ trong `CanUseItem`) để `CloneSpirit` không gọi được khi thiếu FP/HP. Đã test in-game: đúng | [docs](docs/multi_spirit.md) |
| 2026-10-09 | Dịch chuyển spirit: đọc máy trạng thái `warp_manager` trong IDA; thêm `WarpDistance`/`WarpBlockedTime`/`WarpStuckTime`/`WarpStuckRange` (hạ ngưỡng) và `WarpWhenFar` (yêu cầu dịch chuyển cả khi chỉ ở xa); kiểm AOB bố cục, bỏ spirit của người chơi khác. Đã test in-game bản trước khi thêm kiểm tra an toàn | [docs](docs/warp_research.md) |
| 2026-10-09 | Thêm `ImproveSenses` / `ImproveAggression` / `ImproveFollow` (mặc định true): bộ giá trị AI spirit lấy từ Age of Spirit, ghi vào NpcThinkParam theo kiểu chỉ nâng / chỉ hạ; bỏ log tìm SpEffect rơi. Chưa test từng nhóm | [docs](docs/think_override.md) |
| 2026-10-09 | Thêm `RegenMode` (1 HP cố định, 2 % HP tối đa, 3 % HP đã mất) và `RegenValue` thay cho key `Regen` (tự đổi tên trong ini cũ, giữ nguyên giá trị; mặc định mode 2 = hành vi cũ). Chưa test in-game | |
| 2026-10-09 | Sửa dịch chuyển spirit trong Seamless Co-op (báo cáo Nexus): Seamless bỏ qua bước đăng ký spirit với `warp_manager` và hàm cập nhật dịch chuyển; mod tự đăng ký, tự chạy cập nhật và tự đo khoảng cách cho các spirit đó | [docs](docs/warp_research.md) |
| 2026-10-10 | Sửa crash khi triệu hồi Mimic Tear trong Seamless Co-op (báo cáo Nexus, tái hiện được): hàm update của engine tra chr theo handle và không kiểm null; giờ chỉ đăng ký spirit tra được chr và gỡ entry ngay khi chr không còn tra được. Đã test in-game | [docs](docs/warp_research.md) |
| 2026-10-10 | Điều tra báo cáo "không triệu hồi lại được trong Seamless" (SpEffect `2147300000`): log chẩn đoán SpEffect người chơi + dòng param `503045`/`2147300000` (sau `SlotProbe`); `2147300000` không có trong param, `503045` là hiệu ứng vanilla; không tái hiện được, dừng | [docs](docs/seamless_conflict.md) |
| 2026-10-10 | Điều tra báo cáo "spirit biến mất khi triệu hồi Ash thứ 3" (Convergence): spirit vô hình (`camouflage_transparency` về 0, thanh máu còn) khi triệu hồi thêm Ash sau Cemetery Shade; không phải lỗi mod, chưa vá, ghi lại | [docs](docs/open_issues.md) |
| 2026-10-13 | Mặc định `ReloadBanner` đổi thành `false`: nhiều mod cùng bật thì mỗi mod hiện một banner vài giây khi bấm reload | |
| 2026-10-13 | Menu trong game tùy chọn qua ERNativeUI (`src/menu.rs`): 5 trang con (số spirit, triệu hồi, máu và màu, hành vi, dịch chuyển) + bật/tắt log; giá trị số gõ vào hàng chữ (không dùng slider/bảng chọn), mọi key đổi được ngay khi chơi (mod vốn đọc lại mỗi giây), riêng `MaxSpirits` phải khởi động lại game | [docs](../../docs/ingame_menu.md) |
| 2026-10-14 | Sửa lỗi cũ: các dòng `Warp*` trong ini có chú thích ngay sau giá trị (`WarpWhenFar=true   ; ...`) bị đọc cả chuỗi làm giá trị nên rơi về mặc định (Warp không bao giờ chạy theo ini). `common::config` giờ cắt chú thích cuối dòng (sau khoảng trắng, `;` hoặc `#`) và ini mẫu đưa chú thích lên dòng riêng | |
| 2026-10-15 | Chữ menu theo ngôn ngữ: `locales/en.json` + `locales/vi.json` nhúng vào DLL qua `common::i18n` (host báo `vietnamese`), `menu.rs` chỉ nêu khoá, ngôn ngữ khác tiếng Anh; nhãn kiểu hồi máu cũng dịch; test khớp khoá menu với các tệp | [docs](../../shared/docs/i18n.md) |

| 2026-10-15 | Menu trong game: Multiplier (0..10, bước 0.25) và Stuck Range (0..100, bước 1) thành slider native khi host có API 1.2 (ERNativeUI >= 1.2), phạm vi slider hẹp hơn phạm vi mod chấp nhận (ô gõ và tệp cấu hình vẫn nhận số lớn); chưa thử trong game | [docs](../../docs/ingame_menu.md) |
| 2026-10-16 | Log gọn lại theo kiểu ngắn, kỹ thuật cho dev: bỏ dòng lặp ý, địa chỉ patch, câu dài; chỉ ghi 1 lần khi chờ thế giới | |
| 2026-10-16 | `Amount` không đếm spirit tạm (Mimic Tear - Blob): `Amount=10` ra 10 Mimic thay vì 5; chưa test trong game | [docs](docs/mimic_tear.md) |
| 2026-10-16 | Điều tra báo cáo Mimic Tear/Seamless: thêm `common::crash` (log lỗi nghiêm trọng); crash khi gọi nhiều Mimic là lỗi bộ vẽ của game, không phải mod; `Amount` đếm spirit thật thay vì cả Blob | [docs](docs/mimic_tear.md) |
| 2026-10-18 | Sửa lỗi Seamless 2.0.1 báo "No such pattern" khi `MaxSpirits` > 60: phép vá capacity của mod viết đè đúng các byte (`BA 00 05 00 00 41 B8 08 ...`) mà Seamless quét để tự nới capacity; nay mod chờ tối đa 10 giây thấy `ersc.dll` thì bỏ phép vá capacity (Seamless đã nới lên 1000) | [docs](docs/seamless_conflict.md) |
| 2026-10-19 | Sửa lỗi phím reload lọt ra ngoài game: `common::reload` đọc bằng `common::input` (`GetAsyncKeyState`, chỉ nhận phím khi cửa sổ game có focus) thay cho `GetKeyState` của `eldenring`, và antivirus gắn ít cờ hơn (RuneMultiplier 6 xuống 2 cờ trên VirusTotal); chưa test phím reload trong game | [docs](../../shared/docs/reload.md) |
| 2026-10-22 | Thêm `InstantMimic` (port từ er10x): Mimic Tear bỏ qua Blob; dòng BuddyParam của Blob nhận trường của Mimic (giữ ID để game vẫn copy build), chuỗi còn 1 link/Mimic; fade-in (SpEffect 16316) giữ nguyên theo ý người dùng (chỉ là hiệu ứng hình, 3 s); chưa test trong game | [docs](docs/mimic_tear.md) |
| 2026-10-22 | Sửa panic `Unsupported game version 2.7.2.0` lặp mỗi frame ở Warp khi gọi spirit: `resolvable` gọi thẳng `get_chr_ins_by_handle` của ChrSet thay vì `WorldChrMan::chr_ins_by_handle` (hàm này kiểm kiểu qua `rva::get()`, chỉ hỗ trợ 2.7.1.0) | [docs](docs/warp_research.md) |
| 2026-10-22 | Tách log SpEffect của người chơi khỏi `SlotProbe` sang `EffectProbe` (`src/effect_probe.rs`), mặc định tắt | [docs](docs/seamless_conflict.md) |
| 2026-10-22 | `InstantMimic`: dòng Blob dùng animation spirit thường (1830) thay vì animation trồi lên của Mimic (60500), mọi link chuỗi là dòng Blob; chưa test | [docs](docs/mimic_tear.md) |
| 2026-10-22 | Hoàn tác thử `generateAnimId` 1830 + chuỗi toàn dòng Blob: ra 1 Mimic đứng yên mãi cạnh 1 Mimic chạy được; `InstantMimic` còn lại chỉ ghi đè dòng Blob như bản trước | [docs](docs/mimic_tear.md) |
| 2026-10-22 | Giải mã er10x: `InstantMimic` chép thêm 3 byte cờ (follow type...) từ dòng Mimic sang Blob và đặt animation 1830 cho cả Blob lẫn Mimic, như `fix_fields` của er10x; sửa Mimic đứng yên mãi; chưa test | [docs](docs/mimic_tear.md) |
| 2026-10-22 | `InstantMimic`: HUD bỏ thanh HP của chr sinh từ dòng Blob (`sub_1404B7A10` trong `sub_140771ED0`); đổi đích call sang stub trả "không phải Blob" khi tính năng bật; đã test in-game | [docs](docs/mimic_tear.md) |
| 2026-10-22 | Nối `common::conflict` (`src/conflicts.rs`): mỗi chỗ patch (band, SummonAnywhere, MultiSpirit, HUD Mimic) kiểm tra hook của mod khác trước khi patch, log nêu tên DLL và hiện 1 hộp thoại (luôn bật, không có khoá cấu hình); hook của Seamless (`ersc`) không bị báo; chưa thử với mod xung đột thật | [docs](../../shared/docs/conflict.md) |
| 2026-10-22 | Dọn code: module doc dài chuyển sang docs/, regen dùng common::player::for_each_active_chr, log ngắn, sửa cảnh báo clippy, viết lại README. Không đổi hành vi. | |
| 2026-10-22 | Phân tích crash Seamless 2 người với Mimic (Multiplier=25, 1.3.1): cùng chữ ký lỗi bộ vẽ, xảy ra khi gọi lần hai lúc 25 Mimic cũ còn sống; chưa thử lại với `InstantMimic` | [docs](docs/mimic_tear.md) |
| 2026-10-22 | Sửa `InstantMimic` ghi animation spirit thường lên 96 dòng BuddyParam (mọi nhóm có dòng không-Blob) thay vì chỉ dòng Mimic; giờ chỉ nhóm có Blob | [docs](docs/mimic_tear.md) |
| 2026-10-23 | Thêm `HubHpBars` (5-30, đổi tên từ `HpBars` khi chưa phát hành): game cứng 5 slot dữ liệu, 5 slot hiển thị và vector 5 hàng trong view; nay sửa movie `01_000_fe.gfx` lúc game mở tệp (thêm hàng) và dựng thêm view cho mỗi 5 hàng trên cùng clip; đã test trong game (10 và 30 thanh) | [docs](docs/buddy_hud.md) |
| 2026-10-23 | `HubHpBars`: panel không còn nâng cố định trong movie mà đặt vị trí lúc chạy theo số thanh đang dùng (`sub_140D872C0` trên clip `Anim`): 5 thanh trở xuống ở chỗ vanilla, nhiều hơn thì mọc lên trên; bỏ `shift_stage_placement` khỏi `common::gfx`; đã test trong game | [docs](docs/buddy_hud.md) |
| 2026-10-26 | Phân tích tĩnh (IDA 2.7.1.0) giữ spirit qua grace và dịch chuyển: spirit là chr của block nên bị huỷ khi reload, hướng khả thi là lưu snapshot rồi gọi lại sau load; chưa có code | [docs](docs/persist_spirits.md) |
