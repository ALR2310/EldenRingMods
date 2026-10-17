# Xung đột với Seamless Co-op: đổi đích `call` thay vì sửa byte

**Status 2026-10-08: sửa ở 1.1.4, đã test với Seamless Co-op 2.0.1 và offline. Quy tắc cho mọi patch code của mod này. Mã: `common::codepatch::{rel32_target, redirect_rel32}`.**

## Sự cố

Từ 1.1.0 tới 1.1.3, game dừng ngay với Seamless Coop 2.0.1 "Fatal Error: No such pattern `E8 ? ? ? ? 83 F8 02 41 0F 44 FC` ... signatures.cpp 1426". Đó là chỗ tính HP cost `0x1403C0C2F` (`call GetBuddyState; cmp eax,2; cmovz edi,r12d`) mà patch FP/HP của MultiSpirit thay bằng `jmp` inline (`E8` → `E9` + NOP). Seamless tìm chỗ patch của nó bằng chữ ký byte và **tự huỷ game** khi không khớp. Từ 1.0.0 tới lúc đó toàn test offline nên không lộ.

## Điều tra

- `ersc.dll` (13 MB, không có version resource): database IDA ở `dumps/ersc/fcd11a18/` (gitignore). Chuỗi gần như đều mã hoá; `ersc\buddy\buddy.cpp` chỉ là cấp slot spirit (65 slot / người chơi, `sub_18008A7D0`).
- Dump crash: chữ ký của Seamless chỉ được giải mã lúc dùng → không liệt kê được hết → phải giả định bất kỳ byte nào quanh chỗ patch đều có thể nằm trong 1 chữ ký của họ.

## Quy tắc

Chỉ ghi lại 4 byte displacement của 1 lệnh `call`/`jmp rel32` sẵn có sang stub gần đó; opcode và mọi byte xung quanh giữ nguyên (chữ ký Seamless wildcard đúng phần `E8 ? ? ? ?`). Nếu mod khác đã đổi đích cùng lệnh, `rel32_target` trả về stub của họ → 2 mod nối tiếp nhau. Stub là "hàm được gọi": vào với `rsp % 16 == 8`, tự `sub rsp,28h/38h` trước mỗi lời gọi bên trong, kết thúc bằng `ret` hoặc tail `jmp`.

Đã chuyển: `multi_spirit.rs` (UI x3, CanUseItem, FP/HP x2, recall, DoSummon, DisappearAll), `buddy_stone.rs` (stateInfo call/tail, in-range). Hai patch inline cũ (giữ bia đá 8 byte, xoá `+0x38` 7 byte) bỏ, logic chuyển vào stub (`decide_state`, stub stateInfo). `band.rs` giữ nguyên (đã chạy với Seamless từ 1.0.0).

## Test

Seamless 2.0.1 (`ersc.dll` nạp): game không còn bị huỷ, mọi dòng cài đặt `was` trỏ về hàm gốc của game; MultiSpirit gọi nhiều Ash và cho về từng Ash (cả Ash nâng cấp `258001`, Ash 15 spirit `240000`); SummonAnywhere dùng bia dự phòng `10000100`. Offline: 12 patch cài đủ. Reforged (`reforged.dll` nạp): 12 patch, không lỗi.

## Seamless còn gây

Spirit mất màu ma khi chơi Seamless (hook `sub_1403F0C20`): [open_issues.md](open_issues.md).

## Không triệu hồi lại được với Seamless (Nexus, 2026-10-10)

**Status 2026-10-10: không tái hiện được, không phải lỗi của mod (đã kiểm tra). Dừng điều tra; nếu có thêm báo cáo cùng lỗi thì xem "Hướng tiếp".**

Người dùng (TechnoMonkeyDJ) báo không triệu hồi lại được sau khi đuổi spirit về / spirit chết, với mọi nhân vật; chỉ xảy ra khi bật Seamless, và theo họ Seamless một mình cũng chặn (nên họ quay lại mod cũ "Resummon Spirit Ashes For Seamless Co-op"). Ở máy tác giả, kể cả dùng ME3 và ERSS (DLSS frame generation) như họ, triệu hồi lại luôn được. Log của họ không có lỗi nào từ mod.

Manh mối bên ngoài: tác giả Seamless trả lời các mod spirit rằng "SpEffect `2147300000`" là cách sửa; mod đổi animation chuông phải gộp nó vào `c0000` (TAE); mod tham chiếu `zibinha_infinite_summoning` (giải ngược trong `D:\tmp\zibinha_infinite_summon`) sửa dòng `503045` thành `2147300000` trong bảng param nội bộ của `ersc.dll`; người dùng báo ép `2147300000` bằng Cheat Engine ~1 giây sau khi đuổi spirit thì triệu hồi lại được.

Đã kiểm tra bằng log chẩn đoán (`SlotProbe=true`: các SpEffect của người chơi mỗi lần đổi, và dòng param của `503045`, `2147300000`), có và không có Seamless:

- `SpEffectParam 2147300000`: **không có dòng** (cả khi có Seamless) → không phải hiệu ứng thật của game, là mã hiệu Seamless tự xử lý trong code.
- `503045`: hiệu ứng vanilla trống, thời gian vô hạn; xuất hiện ~1 s sau khi vào game (cả khi **không** có Seamless), mất khi nạp lại map. Không đổi khi triệu hồi / đuổi về / spirit chết → không phải marker của Seamless.
- Quanh lần triệu hồi chỉ có `232000` (SpEffect của Ash) và `6518201`, `9540`.

Nên cơ chế chặn nằm trong code của Seamless, không thấy được từ SpEffect; áp `2147300000` bằng `apply_speffect` rất có thể không có tác dụng (không có dòng param). Không giải mã `ersc.dll` (packer/VM): tốn công, và lỗi không tái hiện được nên không kiểm chứng được bản sửa.

**Hướng tiếp (nếu có thêm báo cáo):** tra trong `dumps/seamless-diff/.../seamless_only_ida.txt` các vá nằm trên đường triệu hồi (DoSummon, `sub_1404B92B0`, `sub_1404BAEA0`, kiểm tra stone/`stateInfo`), rồi dùng breakpoint phần cứng/log xem Seamless đọc cờ nào lúc bấm triệu hồi lại, thay vì đọc mã của nó.

## MaxSpirits > 60 làm Seamless dừng "No such pattern" (2026-10-18)

**Status 2026-10-18: đã sửa (`band.rs`, `seamless_loaded`); đã tái hiện lỗi, chưa test lại bản sửa trong game.**

Seamless 2.0.1 hiện hộp thoại `Fatal Error: No such pattern "BA 00 05 00 00 41 B8 08 00 00 00 FF 50 50 48 89 46 18"` (`ersc\signatures\signatures.cpp`) khi cài cùng mod này với `MaxSpirits` > 60. Mẫu đó là phần đầu của `CAPACITY_INIT_AOB` (khởi tạo ChrSet của spirit, `sub_1404954B0`): Seamless quét nó để nới capacity lên 1000, còn phép vá capacity của mod viết đè `BA 00 05 00 00` (kích thước cấp phát) nên Seamless không tìm thấy. Quy tắc cũ ở trên (không đụng chữ ký của Seamless) bị phép vá này vi phạm.

Cách sửa: khi `MaxSpirits` > 60, đợi tối đa 10 giây (thread riêng, không phải `DllMain`) xem `ersc.dll` có nạp không; có thì bỏ phép vá capacity và để Seamless nới (dải slot vẫn rộng `MaxSpirits`, `band_layout` chia theo capacity thật); không có thì vá như cũ. Phép vá chỉ cần xong trước khi thế giới được tạo nên chờ vài giây không sao. Còn sót: nếu `ersc.dll` nạp sau 10 giây thì lỗi cũ vẫn xảy ra.
