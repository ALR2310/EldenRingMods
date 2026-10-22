# MultiSpirit: gọi nhiều Ash khác nhau cùng lúc

**Status 2026-10-08: hoạt động (vanilla, Reforged, Seamless), đã test. Mã: `src/multi_spirit.rs`. Reforged + Fury xem [open_issues.md](open_issues.md).**

## Vanilla (2.7.1.0, offset theo `SummonBuddyManager` của fromsoftware-rs)

- `GetBuddyState(mgr, goodsId)` = `sub_1404B72F0`: 2 = đang có spirit và `goodsId` trùng Ash đang ra (`+0xE4`) hoặc `goodsId = -1`; 1 = đã sang khu bia đá khác; -1 = không có gì.
- `DoSummon(mgr, speffect)` = `sub_1404B85B0`: `GetBuddyState(mgr, -1)` → hễ có spirit là bật `disappear_requested` (`+0x28`) và thoát (luật "1 nhóm spirit"); không thì spawn, `+0x20 = speffect`. SpEffect được làm tròn `100 * (a2 / 100)` trước khi tra.
- `Update` = `sub_1404B86D0`: có request thì tìm nhóm trong `groups` (`+0x70`), nhóm có spirit thì `sub_1404BB9A0` thu hồi cả nhóm cũ, rồi spawn nhóm mới.
- `CanUseItem` = `sub_14068EE60`: với Ash (loại 7/8) `usable = GetBuddyState(mgr, goodsId) != -1` (`0x140690383`) → đây là chỗ làm xám Ash khác.
- `sub_1403C0930` dựng chi phí dùng item: nếu `GetBuddyState(mgr,-1) == 2` thì FP (`+0xB8`) và HP (`+0xB4`) = 0 (vanilla coi là "cho về" nên miễn phí).
- Nối goods → SpEffect: mọi Ash có `refCategory = 2` và `refId_default` = SpEffect kích hoạt (khoá của `trigger_speffect_to_buddy_map`); mỗi cấp nâng là 1 goods riêng (+0 = `207000`, +4 = `207004`), làm tròn `100 * (id / 100)` cho 86 khoá, không 2 Ash chung khoá.

## Các patch (từ 1.1.4 đều đổi đích `call`, xem [seamless_conflict.md](seamless_conflict.md))

Viết lại (không copy) từ phân tích Solid Uncapper 2.3.3 (MojoW):

1. **DoSummon** (AOB `E8 ?? ?? ?? ?? 83 F8 01 76 1B 83 F8 02 75 04 C6 43 28 01`, `0x1404B85C9`): đổi đích `call GetBuddyState`; `decide_state` trả 2 (cho về) hoặc 0 (gọi thêm). Nhận "cùng Ash" bằng `decide_dismiss`: lấy BuddyParam id của SpEffect (đã làm tròn) trong `trigger_speffect_to_buddy_map`, xem `groups` còn entry sống (`!disappear_requested`, `!is_remote`) có `buddy_param_id` thuộc danh sách không. Có → nhớ `DISMISS_TARGET`, state 2; không → state 0. Cũng gán `+0x38 = +0x3C` khi `+0x38 = 0`.
2. **Update** (AOB `8B 10 49 8B CF E8 ?? ?? ?? ?? 41 83 BF 88 00 00 00 00`, `0x1404B89AB`): bỏ qua `call sub_1404BB9A0` (thu hồi nhóm cũ) khi bật.
3. **UI x3** (`0x1407C3C9E`, `0x140847A1B`, `0x140847CDF`): `GetBuddyState` thật rồi `eax = 0` nếu bật và kết quả >= 1 (bỏ hộp thoại "gửi spirit về?").
4. **CanUseItem** (AOB `48 8B C8 41 8B D5 E8 ?? ?? ?? ?? 83 F8 FF 41 0F 95 C5`, `0x14069037D`): kết quả -1 thì hỏi lại với goodsId -1, ra 2 → trả 0; ngoài khu bia đá vẫn xám như vanilla.
5. **Giữ bia đá**: `+0x38` về 0 sau lần gọi đầu nên lần 2 bị coi "rời khu" và cả 2 nhóm biến mất sau ~10 s; `decide_state` giữ `+0x3C` cũ (mục "bia đá" của [summon_anywhere.md](summon_anywhere.md)).
6. **Cho về từng Ash**: vanilla chỉ có `DisappearAll` nên bấm lại 1 Ash cho về cả các nhóm. Đổi đích `call DisappearAll` của nhánh `+0x28` trong `sub_1404B92B0` (`0x1404B9587`, AOB `41 80 7F 28 00 74 08 49 8B CF E8 ...`) sang `disappear_all_hook`: khi bật, `+0x28` đang set và có `DISMISS_TARGET` thì đánh dấu tạm `disappear_requested` mọi entry sống của Ash khác, gọi hàm gốc, gỡ dấu; còn nhóm khác thì khôi phục `+0x3C` mà hàm gốc vừa xoá; Ash đích hết spirit sống thì không gọi hàm gốc. Mọi lý do khác (rời khu, chết, SpEffect 202) giữ vanilla (cho về hết). Log: `sent back Ash (SpEffect N): X spirit(s), Y other(s) kept`.
7. **FP/HP cho Ash gọi thêm** (`0x1403C0B79` FP / `0x1403C0C2F` HP): `cost_state` trả 2 (miễn phí) chỉ khi chính Ash được bấm còn spirit sống (= cho về), ngược lại tính giá bình thường. Solid Uncapper 2.3.3 không có phần này.

**Không phải Spirit Ash** (vd. Spirit-Severing Blade của Reforged đi qua cùng `DoSummon` với SpEffect không thuộc Ash nào): `ash_alive` trả `None` khi SpEffect không có chain → `decide_state`/`cost_state` giữ hành vi vanilla (cho về tất cả / miễn phí).

## Bài học / lỗi đã gặp

- Ash đã nâng cấp (+1..+10) bấm lại gọi thêm thay vì cho về (Nexus MonkeyDLuffy2426, 1.1.2): `buddy_ids` tra thẳng `207004` không thấy. Sửa làm tròn `100 * (speffect / 100)` ở 1 chỗ, dùng chung `decide_dismiss`, `disappear_all_hook`, `cost_state` (1.1.3). Mimic Tear: 3 con `133201000` tự biến mất sau ~8 s là hiệu ứng xuất hiện của Mimic, không qua send-back.
- So `[+0x24]` (active) thay vì chỉ `[+0x20]` (pending) như Solid Uncapper, vì `Update` đặt `+0x20 = -1` sau khi spawn nên dùng lại cùng Ash sẽ bị coi là "Ash khác".
- Test lần 3 lẫn `Solid_Uncapper.dll` trong thư mục mod; lần 4 mới sạch (xem memory test isolation).
- Hàm UI `sub_140847830`/`sub_140847B20` chỉ hiển thị chi phí, không phải chỗ làm xám; không có hàm nào vừa đọc `EquipParamGoods+0x79` (`useLimitSummonBuddy`) vừa gọi thẳng API summon.

## Xung đột

Solid Uncapper (Multi Spirit / Max Spirits Out) và bất kỳ mod nào patch các hàm `SummonBuddyManager` cùng chỗ - không dùng chung; mô tả Nexus có ghi Notes.

## CloneSpirit: gọi thêm cùng một Ash (thử nghiệm 2026-10-09)

**Status 2026-10-09: hoạt động, đã test in-game (gọi nhiều bản sao, chi phí FP/HP và kiểm tra đủ mana). `true`/`false`, mặc định `false`.** Đề xuất của almasakmal123 (Nexus): gọi cùng một Ash nhiều lần (vd. Mimic Tear mỗi con một bộ trang bị), quá giới hạn thì cho về.

`CloneSpirit=true` (cần `MultiSpirit`; bật/tắt, mặc định false): `dismisses_on_reuse` trong `multi_spirit.rs` thay cho kiểm tra "Ash đã out thì cho về". Ash chưa có spirit sống: gọi như thường. Đã có: nếu tổng spirit sống (của người chơi) cộng số spirit một lần cast (độ dài chain) vẫn không vượt `MaxSpirits` thì `decide_state` trả 0 (gọi thêm, vẫn tính phí FP/HP vì `cost_state` trả 0); vượt thì như cũ (cho về cả Ash đó). `Update` đã bỏ `call` thu hồi nhóm cũ khi MultiSpirit bật, nên nhóm mới được nối vào cùng danh sách `groups` (khoá theo chủ, không theo Ash: `sub_1404B86D0`).

Điều đã kiểm chứng khi test: game cho spawn lần 2 cùng Ash (nhóm được nối vào `groups`). Điều còn chưa rõ: game có chặn spawn lần 2 của cùng BuddyParam/slot không; `active_summon_speffect_id` giống nhau có làm `Update` coi là cast trùng không; Mimic Tear sao chép trang bị lúc xuất hiện nên mỗi con có thể khác bộ. Giới hạn: `MaxSpirits` (band) là trần dùng để quyết định gọi thêm hay cho về; còn `ActiveLimit`, chr set 80.

Cùng ngày: `CanUseItem` còn 2 chỗ `GetBuddyState(mgr, goodsId) == 2` (`0x14069018A` HP, `0x14069022D` FP) đặt chi phí bắt buộc về 0 khi Ash được bấm chính là Ash đang out, nên với `CloneSpirit` bấm lại Ash đó vẫn gọi được dù không đủ FP/HP. Patch 8 cho 2 chỗ này hỏi `cost_state` (miễn phí chỉ khi lần bấm sẽ cho về). Hàng goods là `[rsp+0x50]` của hàm gọi. Đã test in-game: đúng.

## Ghi chú kỹ thuật chuyển từ code: `multi_spirit.rs` (2026-10-22)

Nội dung này trước đây là doc đầu file `.rs`; chuyển sang đây nguyên văn (tiếng Anh) để doc code chỉ còn vài dòng.

`MultiSpirit` (hot reload): summon a DIFFERENT Spirit Ash while spirits
are already out and the old ones stay, instead of being sent back.
Re-using an Ash that is already out sends back only that Ash's spirits.

Reverse engineered from Solid Uncapper 2.3.3's "Multi Spirit Coexist"
(see README 2026-09-29) and re-implemented here, on `eldenring.exe`
2.7.1.0. **Every patch only redirects an existing `call` (or `jmp`)**
([codepatch::redirect_rel32]: the 4 displacement bytes change, nothing
else) - since 2026-10-01. Until 1.1.3 most were inline `jmp` hooks that
rewrote the instructions around the call; Seamless Co-op 2.0.1 looks up
its own patch sites by byte signature (`E8 ? ? ? ? 83 F8 02 41 0F 44 FC`
= patch 7's HP call, among others) and aborted the game with "No such
pattern" whenever MultiSpirit was installed. Its signatures wildcard a
call's displacement, so a redirected call still matches.

1. `DoSummon` (`sub_1404B85B0`) asks `GetBuddyState(mgr, -1)` and, on
   state 2 (a spirit is out), raises `disappear_requested` (+0x28) and
   returns - vanilla's "one group at a time". That `call`
   ([DOSUMMON_AOB]) goes through a stub: on state 2, [decide_state]
   checks whether the requested Ash (SpEffect `edi`) still has live
   spirits in `groups` - yes → 2 (dismiss request), no → 0 (summon it
   too). (First version compared `edi` with the active/pending SpEffect
   `+0x24`/`+0x20` - that only knows the LAST Ash summoned.)
2. `SummonBuddyManager::Update` (`sub_1404B86D0`) calls `sub_1404BB9A0`
   to recall the player's whole existing group right before spawning the
   new one ([RECALL_AOB]). Skipped while the feature is on.
3. The item UI: the 3 UI callers of `GetBuddyState` ([UI_CALL_SITES]:
   `sub_1407C3930` - the "use item" prompt, state 2 = "send the spirit
   back?" dialog 20000600 - and the two FP-cost/label helpers
   `sub_140847830`/`sub_140847B20`) see any state >= 1 as 0 while the
   feature is on (other Ashes no longer greyed out / no dialog).
4. `CanUseItem` (`sub_14068EE60`) marks a buddy item usable only when
   `GetBuddyState(mgr, goodsId) != -1` ([CANUSE_AOB]); with spirits out
   every other Ash answers -1. On -1 it asks again with goodsId -1; 2
   (spirits out, same area) → 0 (usable). Away from any summoning area it
   stays -1, like vanilla.
5. `DoSummon` copies the current stone `+0x38` into the active one
   `+0x3C` on every summon; `+0x38` can already be 0 by a second summon,
   which zeroed `+0x3C` and made `sub_1404B92B0`'s "left the summoning
   area" branch dismiss everything ~10 s later (test 3/4, 2026-09-29).
   [decide_state] sets `+0x38 = +0x3C` first when it is 0 (until 1.1.3:
   an inline patch on the copy itself).
6. Per-Ash send-back: vanilla only has `DisappearAll` (`sub_1404B8160`).
   The `call` serving the dismiss request in `sub_1404B92B0` (the
   `+0x28` branch, [DISAPPEAR_CALL_AOB]) goes through
   [disappear_all_hook]: every live entry of the other Ashes is masked
   (its `disappear_requested` raised) around the real call, so only the
   pressed Ash's spirits leave - Solid Uncapper's approach. The active
   stone `+0x3C` is restored afterwards while others remain. (Until
   1.1.3: a detour on `DisappearAll`'s own prologue.)
7. FP/HP cost: `sub_1403C0930` builds an item's use cost; for Ashes it
   zeroes the FP (`+0xB8`) and HP (`+0xB4`) cost when `GetBuddyState(mgr,
   -1) == 2` (vanilla: using an Ash with a spirit out only sends it back).
   Both calls ([COST_CALL_SITES]) ask [cost_state] instead: free only if
   the pressed Ash itself has live spirits. The goods row is the
   function's `[rbp-0x38]`; an Ash's `refId_default` (+4) is its trigger
   SpEffect.
8. "Can I afford it" in `CanUseItem`: two more `GetBuddyState(mgr,
   goodsId) == 2` checks ([CANUSE_COST_SITES]) zero the HP / FP cost the
   check requires when the pressed Ash is the one out. With `CloneSpirit`
   that press summons, so they ask [cost_state] too: free only when the
   press sends the Ash back (user report, 2026-10-09: a clone could be
   summoned with too little FP/HP).

All stubs read one byte ([ENABLED]) that the tick below keeps in sync
with the ini, so F5 switches the behavior live without re-patching.
