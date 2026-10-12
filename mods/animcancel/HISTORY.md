# AnimCancel: dòng thời gian

Mỗi dòng một thay đổi, mới nhất ở dưới cùng. Chuyện nào có phân tích dài thì
link sang `docs/`.

| Ngày | Thay đổi | Chi tiết |
|---|---|---|
| 2026-10-12 | Nghiên cứu tĩnh: cancel do TAE bật qua `CSChrActionRequestModule`, hướng đi là nới `possible_action_inputs/cancels`; chưa biết HKS có chuyển state | [docs](../../docs/animation_cancel.md) |
| 2026-10-12 | Tạo crate `animcancel` (probe): log cửa sổ cancel theo anim và phím F7 ép các hành động chọn được; chưa test trong game | [docs](../../docs/animation_cancel.md) |
| 2026-10-12 | Test 1-3: ép bitmask không đủ (HKS đọc queue theo id TAE); ghi vào `input_entries` làm lệnh giả | [docs](../../docs/animation_cancel.md) |
| 2026-10-12 | Ép vào `cancel_entries[id]` (tạo entry nếu thiếu), gỡ `disabled_action_inputs`, bit 37 cho đổi vũ khí: guard, lăn, đổi vũ khí huỷ được đòn đánh, đã test trong game | [docs](../../docs/animation_cancel.md) |
