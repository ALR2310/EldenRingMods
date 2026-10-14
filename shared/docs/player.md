# player: cổng "người chơi đã vào thế giới"

**Status 2026-10-13: hoạt động, `main_player_chr_ins_ptr` được hơn 20 nơi dùng, `wait_for_solo_param_repository` bởi các mod đụng param. Code ở `shared/src/player.rs`. Không có test tự động (cần `WorldChrMan` thật của game).**

## Quy tắc

Mọi tính năng chạm vào singleton mà dữ liệu chỉ được điền khi có nhân vật người
chơi thật (`GameDataMan` với dữ liệu save, `SoloParamRepository` với bảng param)
phải chờ `WorldChrMan::main_player`, **không** chỉ chờ `instance()` /
`instance_mut()` trả `Ok`. `Ok` chỉ nghĩa là đối tượng quản lý đã tồn tại, việc
này xảy ra ngay khi một ô save được nạp, vẫn ở màn hình tiêu đề hoặc đang loading,
rất lâu trước khi dữ liệu có thật.

Đã bị cắn ở `sometweaks` (2026-08-24/25): cộng rune sớm trong `rune::reward`, và
crash trong `drop_rate` trước khi `SoloParamRepository` được chặn theo cùng cách.
Hàm gom từ `sometweaks::player` và `risearcher::player` (hai bản y hệt) khi mod
thứ ba cần cùng cổng này.

## `ActionSnapshot`

Đọc các bit input của pad mỗi frame từ `CSChrActionRequestModule` của người chơi
chính, cho mọi tính năng cần phân loại input ("đòn đánh này là đòn thường hay
Ash of War?", "người chơi có đang rảnh không?", "vừa bắt đầu cử chỉ?").

- `r1/r2/l1/l2/new_gesture` lấy từ `new_action_presses`, đúng 1 frame ở lúc bấm.
  Kiểm trong game (2026-09-03): tương quan với đòn Ash of War/Skill đáng tin hơn
  nhiều so với bất kỳ trường `AtkParam` nào (xem README của autoregen).
- `requested_gesture` là giá trị thường, chỉ có nghĩa trong frame `new_gesture` bật.
- `busy` gồm mọi cách khác khiến người chơi "không rảnh": các hành động đang giữ
  (không chỉ mới bấm) từ `action_requests`, cộng input di chuyển, đọc từ cùng module
  để người gọi khỏi phải gọi thêm `WorldChrMan::instance()`.

Gom từ `autoregen::regen` (2026-09-17) khi `NewActionPresses` hẹp hơn của
`sometweaks::player` (chỉ `r1/r2/l1/l2`) hoá ra là tập con của đúng các trường
này; ai chỉ cần 4 bit bấm thì bỏ qua phần còn lại.

## `wait_for_solo_param_repository`: chờ mãi mãi

Chờ cả `SoloParamRepository` lẫn người chơi trong thế giới. Từ 2026-08-24 đến
2026-09-17 hàm nhận `timeout` và trả `Option`; mọi nơi gọi đều truyền
`Duration::from_secs(300)` và coi hết hạn là "tắt cho phiên này" (có mod, như
`risearcher`, còn không giữ task theo dõi reload sau mốc đó). Thực tế người chơi
alt-tab hoặc chậm chọn save mất tính năng đến hết phiên mà chỉ có một dòng log dễ
bỏ sót, trừ khi họ biết bấm `ReloadKey`. Không có lý do thật sự để bỏ cuộc ở đây:
khác với việc tra cứu khoá theo phiên bản, đây chỉ là "chưa xảy ra", nên hàm giờ
thử mãi, giống `wait_for_cs_task`.

Cứ 30 giây trong lúc chờ có một dòng log nhắc. Không phải cảnh báo (mất vài phút
để chọn save và vào game là bình thường, khác với `CSTaskImp` khởi tạo chậm), chỉ để
thấy thread không chết.
