# Giữ spirit qua grace và dịch chuyển (nghiên cứu)

**Status 2026-11-03: ngồi grace giữ được spirit (đã test solo, 2.7.2.0): `KeepSpiritsOnRest` (`src/keep_on_rest.rs`) chặn hai chỗ, SpEffect 202 (`DisappearAll`) và lời gọi `sub_1404B83B0` trong hàm reset chr `sub_14050E0E0` (đánh cờ xoá mọi nhóm spirit); log: 2 nhóm còn nguyên, bộ đếm huỷ nhóm đều 0. Chưa test: Seamless, fast travel, spirit có còn đi theo / đánh được sau khi nghỉ. Khoá của `groups` là id chủ spirit, không phải map id.**

Hai tính năng muốn làm (người dùng, 2026-10-26):

1. Ngồi grace thì spirit không mất.
2. Dịch chuyển (fast travel...) sang nơi khác thì spirit đi theo.

Người dùng xác nhận ngồi grace làm thế giới reload, nên nhiều khả năng cả hai là một cơ chế.

## Cách game quản lý và huỷ spirit (2.7.1.0, từ IDA)

`SummonBuddyManager+120` là `std::map` khoá theo **id của người chơi chủ spirit** (không phải map id/block như bản đầu ghi nhầm; kiểm lại 2026-11-03 trên 2.7.2.0): khoá = `*(PlayerIns+1408 -> +8)` qua `sub_1406576E0`, và `fromsoftware-rs` cũng ghi "maps character event id to groups it owns". Node: key `+32`, danh sách nhóm `+48`, số nhóm `+56`. Vì vậy co-op mỗi người một khoá riêng.

- `DisappearAll` (2.7.2.0 `sub_1404B8240`, 2.7.1.0 `sub_1404B8160`): chỉ đánh dấu thu hồi (`+49 = 1`, `+52 = manager+140`, áp SpEffect 60505 hoặc 1840 lên chr) các nhóm dưới khoá của **người chơi local**. Nó không xoá chr.
- `sub_1404B9390` (2.7.2.0; 2.7.1.0 `sub_1404B92B0`; cập nhật mỗi frame) gọi `DisappearAll` ở 4 điều kiện:
  1. người chơi có SpEffect 202 (`sub_1403F21B0(player, 202)`);
  2. cờ yêu cầu thu hồi `manager+0x28` (bấm lại Ash);
  3. "rời khu bia đá": `manager+0x20 < 0` và cờ `+181`, `+183` đều tắt;
  4. người chơi ở trạng thái 13 (`sub_1404B77F0()`) và HP <= 0 (`sub_140437B10`).
- Cùng hàm, vòng thứ hai: với mỗi khoá còn nhóm, tra người chơi có id đó bằng `sub_140509480(WorldChrMan, key)` (duyệt `player_chr_set` ở `WorldChrMan+69344`, so `sub_1406576E0(player)` với khoá). **Không thấy người chơi đó thì huỷ cả nhóm** qua `sub_1404BBB80` (2.7.1.0 `sub_1404BBAA0`; gỡ mục warp rồi gỡ chr). Bản đầu hiểu nhầm đây là "block đã unload"; thực ra là dọn khi **chủ spirit rời phiên** (co-op: người chơi thoát thì spirit của họ bị dọn trên máy khác).
- Vòng thứ ba: nhóm của người chơi local có `+48` (đã bị thu hồi) thì xoá khi hết `+52` giây.
- `sub_140C98CA0` (2.7.2.0; 2.7.1.0 `sub_140C987E0`) gọi `DisappearAll(local)` sau một loạt thao tác dọn phiên; nơi gọi: `sub_14065BC30` (cập nhật người chơi, khi có SpEffect 405) và `sub_140B01E60` (bộ đếm > 10 giây).

Hệ quả: spirit không bị huỷ vì "unload block"; chúng nằm trong `summon_buddy_chr_set` toàn cục, và đường huỷ duy nhất ngoài thu hồi là chủ spirit biến mất khỏi `player_chr_set`. Ngồi grace reset thế giới (quái hồi sinh, vật phá hồi lại; hành vi mặc định) nhưng không rõ nó gọi `DisappearAll` (nhiều khả năng qua `sub_140C98CA0` hay điều kiện 3) hay làm gì khác với chr của spirit: cần `PersistProbe`.

## Hướng khả thi: lưu snapshot rồi gọi lại

Không giữ được chr sống. Cách duy nhất là gọi lại spirit sau khi load xong:

1. Theo dõi `groups` khi đang trong thế giới (`WorldChrMan.main_player` có), lưu snapshot: SpEffect kích hoạt của từng nhóm, số lượng đang sống, số bản Clone.
2. Phân biệt "mất do reload" với "mất có chủ ý": nếu probe cho thấy nhóm bị gỡ không qua `DisappearAll` thì hook lời gọi `sub_1404BBB80`/`sub_1404BBAA0` (đổi đích `call`, `codepatch::redirect_rel32`, vì Seamless quét chữ ký byte); hook `DisappearAll` (đã có `disappear_all_hook` ở `0x1404B9587`) cho thu hồi, chết, Spirit-Severing Blade thì xoá snapshot.
3. Sau khi load xong (có `main_player`, hết màn đen, chờ vài giây), gọi `DoSummon` (`sub_1404B85B0`) cho từng nhóm, cách nhau một khoảng, rồi để `Update` spawn như bình thường.

Ghi chú:

- Ngồi grace dễ hơn dịch chuyển: chỗ đứng không đổi, không lo điểm triệu hồi hay vùng cấm ở nơi đến. Spirit gọi lại đầy máu, hợp với việc nghỉ ngơi hồi máu.
- Dịch chuyển cần `SummonAnywhere` bật, nếu không điều kiện "rời khu bia đá" thu hồi ngay.
- `DoSummon` gọi trực tiếp **không tốn FP**, vì phí tính ở bước dùng item (`sub_1403C0930`). Cần chốt: miễn phí hay phải tính phí.
- Phải tôn trọng chỗ game cấm triệu hồi (đấu trường: game cố ý không đăng ký spirit với warp manager ở đó).
- Gọi lại số lượng lớn đúng lúc load dễ dính crash đồ hoạ giống Mimic Tear trong Seamless ([mimic_tear.md](mimic_tear.md)); trong Seamless còn có đồng bộ mạng chưa rõ. Đề xuất bản đầu chỉ offline.
- Đăng ký sớm với `warp_manager` khi chr chưa tra được từng gây crash ([warp_research.md](warp_research.md)); gọi lại phải chờ `resolvable()`.

## Chưa biết

- Lúc ngồi grace game huỷ spirit bằng đường nào: vòng dọn khi chủ rời phiên (`sub_1404BBB80`; ít khả năng, người chơi vẫn còn), một lời gọi `DisappearAll` (điều kiện 1 hay 3 có thể bị kích khi cờ sự kiện bia đá reset), hay đường khác. Nếu là thu hồi có chủ ý thì chỉ cần chặn đúng lời gọi đó (như `+0x28` ở `multi_spirit.rs`), đơn giản hơn nhiều và giữ nguyên máu spirit.
- Chr của spirit có sống sót qua lần reset thế giới của grace không (vị trí, vật lý, tải lại model), kể cả khi chặn được `DisappearAll`: probe chưa log trạng thái chr, cần thêm nếu cần.
- SpEffect 202 là gì (tra Smithbox).

## Cập nhật 2026-11-03: thêm 2 nơi gọi `DisappearAll` và probe `PersistProbe`

Kiểm lại trên 2.7.2.0 (IDA): mọi hàm trên còn nguyên cấu trúc, chỉ lệch địa chỉ `+0xE0` (AOB `DISAPPEAR_CALL_AOB` của mod vẫn duy nhất, `0x4B965D`). Xref `DisappearAll`: 4 chỗ trong `sub_1404B9390` và `sub_140C98CA0` (`0xC98E43`). `sub_140C98CA0` (gọi `sub_1401D2610/2620`, kiểm `sub_1405F0210(0x40000000, ...)`, đặt cờ `+223/+224/+225` của một singleton) là ứng viên "kết thúc phiên"; chưa xác định nó có chạy lúc ngồi grace không. Chưa xác nhận bằng game.

Probe thụ động `PersistProbe` (ini `[Logging]`, mặc định tắt, `src/persist_probe.rs`, không hook): mỗi lần đổi, log block người chơi, từng khoá chủ spirit (id người chơi), từng nhóm trong `groups` (khoá, số nhóm, số `disappear_requested`, số HP <= 0) và SpEffect của người chơi. Spirit mất sau khi nhóm có cờ `disappear` = đường `DisappearAll` (xem SpEffect 405/202 lúc đó); nhóm biến mất không có cờ = đường chủ rời phiên (`sub_1404BBB80` từ vòng 2) hoặc đường khác chưa biết. Chưa chạy.

## Kết quả probe ngồi grace và `KeepSpiritsOnRest` (2026-11-03)

Log solo (không Seamless): 2 nhóm sống; lúc ngồi grace SpEffect của người chơi có thêm `202` (kèm `100010`, rồi `9608/9609/9610/100690`); cùng nhịp cả 2 nhóm có cờ `disappear`, sau khoảng 1 giây nhóm biến mất khỏi `groups`. Không có nhóm nào biến mất mà không qua cờ `disappear`. Kết luận: đường `DisappearAll` điều kiện 1, không phải đường chủ rời phiên hay unload. SpEffect 202 (`SpEffectParam` base): hiệu ứng thời hạn 0,1 s, không có hiệu ứng nào khác trong hàng; nguồn gắn nó là script nghỉ của grace.

`KeepSpiritsOnRest` (ini `[Spirit]`, mặc định `true`, hot reload): đổi đích `call sub_1403F21B0(player, 202)` trong hàm update (AOB duy nhất ở 2.6.2.0 - 2.7.2.0, `codepatch::redirect_rel32`) sang hàm trả "không có" khi bật, nên `DisappearAll` không chạy lúc ngồi grace; tắt thì gọi hàm gốc. Rủi ro chưa kiểm: SpEffect 202 có thể còn do nơi khác gắn (cửa sương, cắt cảnh, đấu trường cấm triệu hồi) và nơi đó cũng không còn thu hồi spirit; cần thử vài tình huống. Chưa rõ điều gì xảy ra với chr của spirit sau lần reset thế giới của grace (vị trí, vật lý): xem khi test.

**Test `KeepSpiritsOnRest` (2026-11-03, solo): vẫn mất.** SpEffect 202 vẫn xuất hiện nhưng không nhóm nào có cờ `disappear`; cả 2 nhóm biến mất cùng lúc khoảng 3 giây sau (`2 groups` → `0 groups`), mục warp cũng mất theo. Vậy có đường huỷ không qua cờ. Ứng viên (2.7.2.0): (a) `sub_1404B87B0` (cập nhật chính của manager, gọi `sub_1404BBA80(manager, local key)` tại `0x4B8A90` khi `manager+0x20 >= 0`, tức `request_summon_speffect_id` có yêu cầu triệu hồi, rồi triệu hồi lại bằng `sub_1404BBEB0`): xoá ngay mọi nhóm của người chơi local; (b) vòng chủ rời phiên của `sub_1404B9390` (`0x4B9517`, nhóm có khoá không còn người chơi tương ứng trong `player_chr_set`), nếu người chơi tạm không ở `player_chr_set` lúc reset thế giới; (c) chr bị gỡ bởi reset thế giới rồi mục nhóm bị dọn ở chỗ khác. Probe lần 2 (đọc đúng trường manager): lúc nhóm biến mất `req` (+0x20) vẫn -1, `dismiss` 0, không cờ `disappear`; bia (+0x38/+0x3C) và cờ vùng/`player_has_alive_summon` chỉ đổi SAU khi nhóm đã mất (chúng tính lại từ nhóm). Loại ứng viên (a) (yêu cầu triệu hồi) và đường `DisappearAll`; còn (b) chủ rời phiên và (c) chr bị gỡ. Probe lần 3 (đọc đúng): `players 6["0", null...]` và id chủ `0` giữ nguyên suốt lúc nhóm mất: ứng viên (b) bị loại, người chơi vẫn trong `player_chr_set` và khớp khoá nhóm. Còn (c) và đường loại 4 của `sub_1404B9390` (nhóm local có byte `+48` của node = `group+0x20`, trong `fromsoftware-rs` là `warp_requested`, thì bị huỷ ngay tại `0x4B97D5`; chưa tìm được nơi đặt byte đó). Probe lần 4 dump byte thô `+0x08..+0x2E` của từng nhóm mỗi lần đổi, để thấy byte nào lật trước khi mất. Probe lần 4 (byte thô): không byte nào của nhóm lật trước khi nhóm mất (kể cả `group+0x20`, `+0x21`); `+0x08` 0 → 1 và `+0x28` 0 → 1 chỉ lúc mới triệu hồi. Nhóm bị gỡ nguyên khối trong một frame. Vòng "chủ rời phiên" (b) vẫn là nghi phạm chính: probe chỉ đọc ở đầu frame nên có thể bỏ lỡ lúc người chơi vắng mặt giữa frame khi reset thế giới. Thử nghiệm: `keep_on_rest.rs` đổi đích thêm `call sub_140509480` ở `0x4B94FA` (AOB `OWNER_AOB`) sang `owner_lookup_hook`: nếu game không thấy người chơi cho khoá và khoá là id của người chơi local thì trả người chơi local (không huỷ nhóm); log 10 lần đầu `owner lookup found no player for key ...`. Kết quả: chưa test.
Probe lần 5 (thử nghiệm chặn tra cứu chủ): không có dòng `owner lookup found no player` nào, tức tra cứu chủ chưa từng thất bại: (b) bị loại hẳn. Đường huỷ còn lại: sự kiện TAE 2030 phát trên chính chr của spirit (cờ chr `+457 & 4`, chr của buddy) gọi `sub_1404B8440(manager, handle)` (TAE handler `sub_14040D190` tại `0x40DC38`), đặt byte `+48` của node (= `group+0x20`); vòng 4 của `sub_1404B9390` (`0x4B97D5`) huỷ nhóm đó ngay. Đó là sự kiện cuối của hoạt ảnh biến mất; ở lượt probe trước cờ này được đặt và nhóm bị huỷ trong cùng frame nên probe không thấy. (`sub_1404B7080` là đường thứ tư, chỉ chạy trong phiên nhiều người, từ `sub_14050E410`.) Vậy ngồi grace buộc chr của spirit chạy hoạt ảnh biến mất bằng đường khác `DisappearAll` (thường qua SpEffect 60505/1840 trên chr). Probe giờ log SpEffect của chr spirit đầu tiên (`spirit fx`) để thấy hiệu ứng nào được gắn lúc đó.
Probe lần 6 (`spirit fx`): SpEffect trên chr spirit đầu tiên không đổi từ lúc ngồi đến lúc nhóm mất (không có 60505/1840 hay hiệu ứng thu hồi nào; chỉ có hiệu ứng lúc mới triệu hồi `35, 5255, 290000`). Vậy chr và nhóm bị xoá trong cùng một frame bởi chính lần reset thế giới của grace (khoảng 1 giây sau khi người chơi nhận `9608/9609/9610`), không qua cờ, hiệu ứng hay sự kiện nào mà probe thấy được. Sau khi mất, `manager+0x24` (`active_summon_speffect_id`) vẫn giữ SpEffect của Ash (ví dụ `213000`): có thể gọi lại bằng đường của game (ghi `request_summon_speffect_id` ở `+0x20`, hàm update huỷ nhóm cũ rồi `sub_1404BBEB0` triệu hồi lại). Hướng thực tế: lưu snapshot các SpEffect Ash đang hoạt động, phát hiện "bị xoá bởi reset" (nhóm mất không cờ, người chơi vừa có SpEffect 202), rồi gọi lại sau khi thế giới sẵn sàng.
**Tìm được đường huỷ (2026-11-03, bộ đếm tại 4 chỗ gọi `sub_1404BBB80`): chỉ chỗ số 2 (vòng nhóm có cờ `group+0x20`) chạy, 2 lần cho 2 nhóm.** Cờ được đặt hàng loạt cho nhóm của MỌI chủ bởi `sub_1404B83B0(manager)`, gọi từ `sub_14050E0E0(WorldChrMan)` (2.7.2.0, `0x50E140`; 2.7.1.0 `0x50DE60` vùng tương ứng), hàm "reset mọi chr" mà `sub_14050F220` chạy lúc reset thế giới của grace (cũng gọi slot `+0x50` của từng ChrSet trong danh sách `WorldChrMan+0x10DC8`, đếm ở `+0x10ED8`). Khớp quan sát của người dùng: spirit của người chơi khác cũng bị xoá trong Seamless, trong khi người chơi đó không bị reset. `KeepSpiritsOnRest` giờ đổi đích lời gọi `sub_1404B83B0` đó (AOB `RESET_AOB`, duy nhất ở 2.6.2.0 - 2.7.2.0) sang hàm không làm gì khi bật; bỏ hook tra cứu chủ (vô tác dụng). Chưa biết slot `+0x50` của ChrSet có xoá chr spirit hay không (summon ChrSet có trong danh sách đó không): nếu có, nhóm còn mà chr mất, probe/mod đọc `chr_ins` có thể lỗi. Chưa test.
Probe lần 3 log thêm id chủ của người chơi (`PlayerIns+1408 -> +8`, thứ vòng dọn so với khoá nhóm) để xem nó có đổi lúc reset không.
`PersistProbe` log các trường manager: `+0x20` yêu cầu, `+0x24` đang hoạt động, `+0x28` cờ thu hồi, `+0x38/+0x3C` bia, `+0x88`, cờ `+0xB4..B7`; xem trường nào đổi ngay trước khi nhóm mất.

Fast travel chưa xử lý: chưa có log; điều kiện 3 (`manager+0x20 < 0`) hay SpEffect khác có thể kích khi dịch chuyển.

## Bước tiếp theo (pha 0, probe, mặc định tắt)

Thêm log chẩn đoán: lời gọi nào tới `DisappearAll` (điều kiện nào trong 4) và tới `sub_1404BBAA0`; map id người chơi; số nhóm theo từng map id. Test: ngồi grace, fast travel trong cùng map, fast travel xa, chết, đi xa trong overworld, một lần trong Seamless. Kết quả cho biết tín hiệu nhận biết reload, rồi mới viết tính năng.

Tham chiếu: `src/multi_spirit.rs` (`disappear_all_hook`, `groups`), `src/buddy_stone.rs` (`NoRestResummon`, `SummonAnywhere`), `src/warp.rs`, [multi_spirit.md](multi_spirit.md), [summon_anywhere.md](summon_anywhere.md).
