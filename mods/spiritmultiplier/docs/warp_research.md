# Dịch chuyển spirit về người chơi

**Status 2026-10-09: hoạt động (đã test in-game: hạ ngưỡng và `WarpWhenFar`). Mã: `src/warp.rs`. Mặc định: `WarpDistance=25`, `WarpBlockedTime=1`, `WarpStuckTime=2`, `WarpStuckRange=3` (các giá trị đã test), `WarpWhenFar=false`.**

## Cơ chế của game (2.7.1.0, từ IDA)

`SummonBuddyManager.warp_manager` giữ một `std::map<FieldInsHandle, entry>` (sentinel ở `+8`), mỗi spirit một mục. Node: left +0, parent +8, right +16, isnil +25, key +32, giai đoạn (stage) +48, cờ +96, bộ đếm tia chặn +100, bộ đếm +104, thời gian kẹt +128/+132.

- `sub_1404C28D0` tính lại cờ mỗi frame: bit0 cờ của chr, bit1 chr không có group sống, bit2 `chr->vtable[464]()->+192->+55752->+40 == 82` (hay tắt), bit3 kẹt đường > `+32`, bit4 tia chặn tới người chơi (thời gian cộng dồn ở +100), bit5 thời gian chặn > `+24`, bit6 khoảng cách bình phương >= `+28`², bit7 một trạng thái chung (`sub_14037C970(.., 31) == 6`), bit8 xa quá ngưỡng 1,5 s.
- `sub_1404C2FD0` đặt stage 0 → 1 (RequestWarp) khi (cờ & 7) == 7 và: (bit3 và (bit4 hoặc bit6)), hoặc bit5, hoặc (bit7 và ...). Bit 8 một mình không ai đọc: spirit chỉ ở xa mà còn thấy người chơi thì chạy bộ.
- `sub_1404C3180` đẩy stage 1 → 2 (chọn điểm gần người chơi bằng `sub_1404C0EC0`, chuyển chr) → 3 (hiện ra) → 0.
- Bốn ngưỡng runtime nằm ở `warp_manager` (`+0x18` thời gian chặn, `+0x1C` khoảng cách, `+0x20` thời gian kẹt, `+0x24` quãng đường kẹt), sao chép từ `GameSystemCommonParam` `buddyWarp_*` (mặc định 5 s, 33 m, 3 s, 1 m) và đặt lại khi tải khu vực. Tooltip Smithbox: khoảng cách đường thẳng để dịch chuyển; thời gian dịch chuyển khi tia bị chặn; thời gian phán định kẹt; khoảng cách coi là tắc.

## Mod làm gì

- `WarpDistance`, `WarpBlockedTime`, `WarpStuckTime`, `WarpStuckRange` (0 = của game): ghi mỗi giây vào bốn ngưỡng trên, nhớ giá trị của game để trả lại.
- `WarpWhenFar`: 10 lần mỗi giây, spirit của người chơi này ở stage 0 mà có cờ bit8, bit5, hoặc (bit3 và (bit4 hoặc bit6)) thì đặt stage = 1; engine lo phần còn lại. Không đụng mục của người chơi khác (so key với `groups` không remote).
- `WarpProbe`: ghi log trạng thái manager khi đổi.
- An toàn: `layout_known` tìm 2 đoạn mã của chính engine đọc các offset trên (AOB), không thấy (game cập nhật) thì tắt tính năng; mọi node được kiểm tra đọc được trước khi đụng.

## Kết quả test (Convergence, 2026-10-09)

- Hạ ngưỡng: spirit dịch chuyển sớm hơn khi khuất tầm nhìn (cờ có bit5, `0x177`).
- Chạy nhanh và xa mà không bị chặn: vanilla chạy bộ; với `WarpWhenFar` spirit được dịch chuyển sau ~1,5 s (cờ `0x147`, stage 1 → 2 → 3 → 0).
- Bị chặn 4,8 s mà cờ là `0x033` (thiếu bit2): engine không dịch chuyển; `WarpWhenFar` xử lý trường hợp này.

## Điều chưa làm

Cả nhóm thường dịch chuyển cùng lúc (cùng bị xa); chưa tách từng spirit. Lần thử cũ (chỉ hạ ngưỡng khi chưa có công cụ xem trạng thái) kết luận sai là không có tác dụng.

## Seamless Co-op (2026-10-09, báo cáo Nexus: spirit không dịch chuyển)

Cùng ini, cùng DLL, cùng khu vực: vanilla có 3 mục trong danh sách dịch chuyển ngay khi triệu hồi; Seamless `entries: none` suốt phiên. Nguyên nhân (từ `dumps/seamless-diff/` + IDA):

- Game đăng ký spirit với warp manager ở cuối `sub_1404BAEA0` (tạo chr của spirit, gọi `sub_1404C0DD0(warp_manager, handle)`; huỷ ở `sub_1404BBAA0` qua `sub_1404C26F0`). Seamless chuyển hướng lời gọi tới `sub_1404BAEA0` (`0x1404B7E0E`) và hook đầu `DoSummon`, `sub_1404B92B0`: spirit của Seamless không bao giờ được đăng ký.
- Chỉ đăng ký thì chưa đủ: cờ vẫn 0 và stage yêu cầu kẹt ở 1, tức hàm cập nhật dịch chuyển (`sub_1404C2890`, gọi từ `SummonBuddyManager::Update` ở `0x1404B8B79`) cũng không chạy cho các spirit này.

Cách mod xử lý (`warp.rs`): mỗi 0,1 s đăng ký các spirit sống của người chơi này chưa có mục (và gỡ mục của spirit đã đi, chỉ những mục mod đã đăng ký); nếu có mục do mod đăng ký thì mỗi frame tự gọi `sub_1404C2890` với một `FrameTime` giả (chỉ đọc số giây ở +8); đo khoảng cách tới người chơi và đặt stage = 1 khi xa hơn ngưỡng liên tục 1,5 s. Sau đó engine tự chạy cờ và stage 1 → 2 → 3 → 0 (log: spirit hiện tới chỗ người chơi). Vanilla không đổi: game tự đăng ký nên mod không có mục nào của riêng nó.

Solo khi `ersc.dll` đã nạp (kiểm tra sau khi `CSTaskImp` có, lúc mọi DLL đã nạp): vanilla không đổi hành vi, vì game tự đăng ký spirit (và cố ý không đăng ký trong đấu trường).

## Crash với Mimic Tear trong Seamless (2026-10-10)

Báo cáo Nexus, tái hiện được: bật Seamless + triệu hồi Mimic Tear thì game crash ngay sau khi mod đăng ký spirit với `warp_manager` (vanilla không crash). `sub_1404C28D0` (update) lấy chr bằng `sub_140508A50(handle)` rồi đọc `chr+400` không kiểm null. Sửa: `resolvable()` (`WorldChrMan::chr_ins_by_handle`) trước khi đăng ký và trước mỗi lần gọi update; entry của mod không tra được chr thì gỡ ngay. Đã test in-game, hết crash. Chưa có dump nên nguyên nhân chính xác (Mimic không tra được hay biến mất giữa 2 nhịp) chưa chốt.

## Không phụ thuộc phiên bản game (2026-10-22)

Warp tìm hàm của game bằng AOB và kiểm bố cục node (`layout_known`), nên không gắn với một bản game. Chỗ duy nhất gắn phiên bản là `WorldChrMan::chr_ins_by_handle` của `fromsoftware-rs`: nó ép kiểu chr qua `rva::get()` (chỉ có bảng cho 2.7.1.0), nên trên 2.7.2.0 Warp panic mỗi frame ngay khi có spirit. `resolvable` giờ gọi thẳng `get_chr_ins_by_handle` của ChrSet (vtable slot 3, không ép kiểu). Đã rà: SpiritMultiplier và `shared/` không còn gọi API nào của `eldenring` đi qua `rva::get()` (GameMan, GameDataMan, `ChrInsExt`, `as_subclass`...). Chưa test trên 2.7.2.0.

## Ghi chú kỹ thuật chuyển từ code: `warp.rs` (2026-10-22)

Nội dung này trước đây là doc đầu file `.rs`; chuyển sang đây nguyên văn (tiếng Anh) để doc code chỉ còn vài dòng.

Spirit warp-to-player (experimental; analysis and results in
`docs/warp_research.md`).

The engine keeps `SummonBuddyManager.warp_manager`, a machine that
teleports a spirit back to the player. It only fires when the spirit's
line of sight to the player is blocked or the spirit is stuck - one that is
merely far away runs after the player on foot. Three parts here:

- `WarpDistance` / `WarpBlockedTime` / `WarpStuckTime` / `WarpStuckRange`
  (0 = the game's own value, hot reload): written every second into the
  manager's runtime copy of the `GameSystemCommonParam` `buddyWarp_*`
  thresholds, so spirits warp back sooner. The game resets them on area
  load, hence the periodic write; its value is remembered and put back
  when a key goes to 0.
- `WarpWhenFar` (hot reload): ten times a second, a spirit of the local player
  that sits idle (stage 0) while one of the engine's own warp conditions
  holds gets its stage set to 1 (RequestWarp), see [request_warps].
- `WarpProbe` (diagnostics only): logs the manager's state when it changes.

What IDA shows (2.7.1.0, `sub_1404C2890` = the warp manager's update;
entries = an `std::map<FieldInsHandle, entry>` at `a1 + 8` (sentinel
node); node: left +0, parent +8, right +16, isnil byte +25, key +32,
stage byte +48, flags dword +96):
- every frame `sub_1404C28D0` rebuilds the flag dword from zero: bit0 = a
  flag from the chr, bit1 = the chr has no live group entry in
  `SummonBuddyManager`, bit2 = the chr's `vtable[464]()->+192 ->+55752
  ->+40 == 82`, bit3 = path-stuck time (node +132) > `a1+32`, bit4 = a ray
  from the chr to the player is blocked (time accumulates in node +100),
  bit5 = that time > `a1+24`, bit6 = squared distance to the player >=
  `a1+28` squared, bit7 = some game state, bit8 = far for 1.5 s;
- `sub_1404C2FD0` raises the stage from 0 to 1 (RequestWarp) when
  (flags & 7) == 7 and one of: (bit3 and (bit4 or bit6)), bit5, or (bit7
  and ...) - bit 2 is often 0, so spirits blocked for 5 s still stayed;
- `sub_1404C3180` then moves stage 1 -> 2 (spawn point from
  `sub_1404C0EC0`, chr teleported) -> 3 (fade in) -> 0.

The node layout is hard-coded, so it is checked first: [layout_known]
looks for the engine's own code that reads those offsets and, if it is not
there (the game was updated), the entry features stay off. Every node is
also checked to be readable before it is touched.
