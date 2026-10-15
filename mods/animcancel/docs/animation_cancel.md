# Animation cancel (guard / dodge / đổi vũ khí cắt animation): nghiên cứu khả thi

**Status 2026-10-12: ĐÃ LÀM, test trong game: guard, lăn và đổi vũ khí huỷ được
đòn đánh, weapon skill và phép (người dùng xác nhận bằng mod thật: cấu hình TOML,
lọc theo loại anim, hot reload).** Cách làm: ghi quyền cancel vào `cancel_entries`
của hàng đợi của `CSChrActionRequestModule` (người chơi luôn ở queue mode), xem
mục "Cơ chế đúng". Nguồn yêu cầu: tin nhắn Nexus của DAM127 (2026-10-12). Người
dùng đã hỏi lại họ muốn cancel animation nào, lúc nào; chưa có trả lời. Probe
(log `WINDOW`/`FLAG`) vẫn còn trong mod làm tuỳ chọn (`CancelProbe`, `FlagProbe`).

## Cơ chế của game (đã xác nhận một phần)

Cancel trong Elden Ring không nằm ở animation mà ở **cửa sổ do TAE bật**: trong
anim đòn đánh, các event `CANCEL_*` (R1/R2, L1/L2, magic, đổi style/slot, item
nhanh, `CANCEL_LS_MOVEMENT`...) báo cho `CSChrActionRequestModule` rằng từ frame
này hành động nào được phép cắt. Module này nằm ở `ChrIns.modules.action_request`
(`fromsoftware-rs`: `crates/eldenring/src/cs/chr_ins/module/action_request.rs`,
size 0x140), các trường chính:

| Offset | Trường | Ý nghĩa |
|---|---|---|
| +0x10 | `action_requests` | phím đang giữ (bitmask `ChrActions`, bit 5 = `sp_move` lăn/lùi, 24 = `guard`, 9/10 = đổi vũ khí R/L...) |
| +0x30 | `cancel_ready_actions` | = `queued_action_inputs` AND `possible_action_cancels`, trừ khi `cancel_disable` |
| +0x38 | `queued_action_inputs` | = `new_action_presses` AND `possible_action_inputs` |
| +0x98 | `possible_action_inputs` | hành động nào được xếp hàng ở anim hiện tại (TAE đặt) |
| +0xA0 | `possible_action_cancels` | hành động nào được cắt anim hiện tại (TAE đặt) |
| +0x100 | `tae_cancels` | cờ cancel của TAE; bit 11 `cancel_disable` chặn mọi cancel |

Kiểm chứng trong IDA (db 2.7.1.0): `sub_1404085A0` (RVA 0x4085A0) là bước reset
đầu frame của module, gọi từ `sub_1403CD510`/`sub_1403CDAB0` (hàm của ChrIns):
chép `action_requests` sang `previous_action_requests`, **xoá `cancel_ready_actions`
(+0x30)**, đặt `requested_gesture = -1` (+0xF8), **xoá bit `cancel_disable`**
(+0x100 bit 11) rồi chỉ đặt lại nếu cờ `falling_jump_frames` (bit 10) đang bật và
điều kiện về vật lý thoả. Vậy cancel được tính lại từ đầu mỗi frame, không có
trạng thái nào "kẹt" cần dọn. Vtable của module chỉ có 2 slot (huỷ và 1 hàm
khác), hàm tính `cancel_ready` không đi qua vtable; chưa lần ra chỗ tính (hàm
lớn `0x409270` trong vùng module chỉ là code vẽ debug, không liên quan).

Thứ tự task trong frame (`CSTaskGroupIndex`): `ChrIns_PreBehavior` (module đọc
pad, tính `cancel_ready`) → `HavokBehavior` (HKS và TAE chạy) →
`ChrIns_BehaviorSafe` → `ChrIns_PrePhysics`. HKS đọc kết quả qua
`env(ActionRequest, ACTION_ARM_*)`, `env(IsMoveCancelPossible)`...

## Kết quả probe 1 và 2 (2026-10-12, test trong game, log `AnimCancel.log`)

Hàm tính cancel là `sub_140408190` (RVA 0x408190, 2.7.1.0): `queued = press AND
possible_inputs`, `cancel_ready = queued AND possible_cancels AND NOT
cancel_disable`, khớp mô hình ở trên. Các getter HKS đọc module này (có `AND NOT
cancel_disable`): `sub_140407930` (`env(ActionRequest)`: bit của `cancel_ready`,
hoặc của `new_action_presses` khi `stay_state`), `sub_140407B90` (bit của
`possible_cancels`), `sub_140407B50/C90/B10/C50/...` (các cờ `tae_cancels`).

- Vanilla, đòn đánh `a020_032000` (1,87 s): cửa sổ lăn mở ở t≈0,55-0,57
  (`possible_cancels` có bit 16-18), game chuyển sang anim lăn ngay sau đó.
- **Ép bit (guard/lăn/đổi vũ khí, kèm bit 18 và cờ `tae_cancels` 0x218, ghi ở
  `PreBehaviorSafe`+`BehaviorSafe`): module làm đúng, `cancel_ready` có bit từ
  t=0,17-0,28, nhưng HKS không chuyển state cho tới t≈0,57**, đúng lúc vanilla.
  Lăn và đổi vũ khí đều thế (đổi vũ khí: `ready=0x200` từ 0,28, anim đổi vũ khí
  `a000_029001` bắt đầu ở 0,62). Mọi lần chuyển state đều trùng lúc cờ
  `tae_cancels` bit 10 (0x400) tắt (0x618 → 0x218/0x219).
- Kết luận: **nới bitmask của module không đủ**. HKS còn điều kiện khác, có thể
  là cờ do TAE event đặt ở nơi khác (`CSChrActionFlagModule`, biến HKS/EzState).
- Lỗi test guard: lần 1 chỉ ép bit 24 `ACTION_ARM_GUARD`, trong khi giữ guard là
  L1 (bit 2) nên không có press nào được xếp hàng; đã thêm L1 vào nhóm Guard.
- Bước tiếp: log mọi thay đổi của `CSChrActionFlagModule` (FLAG ...) để tìm cờ
  lật đúng lúc cancel được thực hiện.

## Cơ chế đúng (queue mode) và cách ép

Người chơi luôn có `queue_mode_enabled = true`; id của hàng đợi (`current_tae_id`,
hoặc `queue_tae_id_override` nếu >= 0) là id anim dạng `aXXX_YYYYYY` bỏ dấu `a`
và `_` (vd. `a020_032000` = 20032000). HKS đọc **theo entry của id hiện tại**:

- `cancel_entries[id]` = *quyền* cancel của state, TAE ghi dần theo cửa sổ
  (`sub_140408020`/`sub_1404088F0`).
- `input_entries[id]` = *yêu cầu đang thực hiện*: mỗi frame `sub_140408420` điền
  `queued AND cancel_entries[id]` (xoá đầu frame ở `sub_1404085A0`). `env(ActionRequest)`
  (`sub_140407930` -> `sub_140407A70`) đọc entry này.
- Bit chỉ có trong queue: 35 (movement), 36 (movement prev), 37 (slot switch).
  Đổi vũ khí cần thêm bit 37 trong `cancel_entries[id]` (`sub_140407CC0`).
- `queued = new_presses AND possible_action_inputs`; `new_presses` bỏ các bit trong
  `disabled_action_inputs` (+0x40) trước (`sub_140408190`).

Cách ép đã chạy (probe `1b4e3e8`, nay là `src/cancel.rs`, `open_windows()`): mỗi
frame, ở `ChrIns_PreBehaviorSafe` và `ChrIns_BehaviorSafe`: OR mask vào `possible_action_inputs`
và `possible_action_cancels`, OR vào `cancel_entries[id]` (**tạo entry nếu thiếu**),
xoá các bit đó khỏi `disabled_action_inputs`, OR cờ `tae_cancels` 0x218 và xoá
`cancel_disable`. Mask: Guard = bit 24 + L1 (2), Dodge = bit 5, 16, 17, 18, 25,
Weapon = bit 4, 9, 10, 37.

**Không** được ghi vào `input_entries`: lần test 3 (ghi mask vào cả hai danh sách)
làm mỗi lần bấm biến thành một lệnh giả (đánh -> đổi vũ khí -> đổi vũ khí -> lăn).

## Kết quả test (2026-10-12)

| Lần | Thay đổi | Kết quả |
|---|---|---|
| 1 | ép bitmask ở `BehaviorSafe` | `ready` có bit từ t=0,2 nhưng không cancel được |
| 2 | + bit 18, cờ `tae_cancels`, ghi ở `PreBehaviorSafe` | vẫn chỉ cancel ở t=0,57 (đúng vanilla) |
| 3 | + ép vào `cancel_entries` và `input_entries` | hành vi lạ (lệnh giả), chứng tỏ HKS đọc queue |
| 4 | chỉ `cancel_entries` (tạo entry nếu thiếu) | **lăn và đỡ huỷ được đòn đánh** |
| 5 | + gỡ `disabled_action_inputs`, nhóm Weapon thêm bit 4 | đổi vũ khí được ghi nhận từ t=0,28 nhưng chỉ chạy ở t=0,83 |
| 6 | + bit 37 cho Weapon | **đổi vũ khí huỷ được đòn đánh (t=0,20-0,32 của đòn)** |

Cửa sổ vanilla của đòn đánh (`a020_032000`, 1,87 s): lăn mở ở t≈0,55-0,57.
Cờ `CSChrActionFlagModule` (log `FLAG`) đổi theo anim nhưng không có cờ nào chặn
cancel: giả thuyết đó đã loại.

## Mod thật (khác với probe)

- Cấu hình `AnimCancel.toml`: mỗi hành động một section (`[Guard]`, `[Dodge]`,
  `[WeaponSwap]`) có `Enabled` và bộ animation riêng (Attack, Skill, Cast, Item),
  mặc định bật hết (2026-10-15, trước đó `[Cancel]` + `[Animations]` dùng chung). Hot reload qua
  `common::reload` (F5).
- Anim nào đang chạy do **id hàng đợi** quyết định (`group_for_cancel(queue_id)`),
  không phải anim đọc từ `anim_queue`: khi niệm phép lúc đang đi, `anim_queue` hiện
  lớp chân `a000_020010` (nhóm Other) một lúc dù id hàng đợi vẫn là của phép, nên
  dùng anim id thì phép không huỷ được trong lúc đó (log 2026-10-12, `grp=Other
  qgrp=Cast`). Id trễ vài frame lúc vào anim mới: chỉ có nghĩa là anim trước vẫn
  đang được cắt.
- `group_for_cancel` = `group_of` (`common::anim`) trừ việc mọi anim TAE a400-a599
  là Cast: `group_of` xếp tia của Placidusax's Ruin (`a451_045110`, ~9 s) vào Other
  vì SpeedMultiplier không được tăng tốc anim mà tia gắn theo, còn với cancel thì
  không có lý do loại. Đòn chí mạng không thuộc Attack nên không bị cắt.
- Nhóm Dodge/Guard/Weapon chỉ ép cờ `tae_cancels` cần cho nó (`action_general`,
  thêm `slot_switch` cho Weapon); probe ép thêm `rh_attack` (bit 4), mod bỏ vì đó
  là cancel đòn đánh sang đòn đánh. Test xong, không thấy thiếu gì.
- Probe còn lại làm tuỳ chọn trong `[Logging]`; không còn phím F7.

## Chưa làm

1. Tác dụng phụ: i-frame/hitbox của anim bị cắt, stamina, hướng nhân vật, FP bị
   trừ khi cắt phép (chưa kiểm tra kỹ).
2. Seamless Co-op (anim id đồng bộ qua mạng, người khác có thể thấy giật hình);
   `cancel_entries.push_back` dùng bộ cấp phát của game (chưa thấy lỗi trong test).
3. Chờ DAM127 nói rõ muốn cancel gì (có thể thêm Item, Jump, cancel từ lăn...).
4. Trang Nexus (`nexus_page.bbcode`), CHANGELOG, logo khi quyết định đăng.
