# Giữ spirit qua grace và dịch chuyển (nghiên cứu)

**Status 2026-10-26: chỉ phân tích tĩnh (IDA, exe 2.7.1.0), chưa viết code, chưa test trong game. Kết luận: không giữ được chr sống qua lần reload; hướng khả thi là lưu snapshot các nhóm rồi gọi lại sau khi load. Việc cần làm tiếp: probe để biết tín hiệu "mất do reload".**

Hai tính năng muốn làm (người dùng, 2026-10-26):

1. Ngồi grace thì spirit không mất.
2. Dịch chuyển (fast travel...) sang nơi khác thì spirit đi theo.

Người dùng xác nhận ngồi grace làm thế giới reload, nên nhiều khả năng cả hai là một cơ chế.

## Cách game quản lý và huỷ spirit (2.7.1.0, từ IDA)

`SummonBuddyManager+120` là `std::map` khoá theo **map id (block) nơi người chơi đứng lúc triệu hồi**; mỗi node giữ danh sách nhóm spirit (node: key `+32`, danh sách `+48`).

- `DisappearAll` = `sub_1404B8160`: chỉ đánh dấu thu hồi (`+49 = 1`, áp SpEffect 60505 hoặc 1840 lên chr) các nhóm nằm dưới **map id hiện tại** của người chơi.
- `sub_1404B92B0` (cập nhật mỗi frame) gọi `DisappearAll` ở 4 điều kiện:
  1. người chơi có SpEffect 202 (`sub_1403F21B0(player, 202)`);
  2. cờ yêu cầu thu hồi `manager+0x28` (bấm lại Ash);
  3. "rời khu bia đá": `manager+0x20 < 0` và cờ `+181`, `+183` đều tắt;
  4. người chơi chết: `sub_1404B7710()` (trạng thái 13) và `player+312` (HP) <= 0.
- Cũng trong `sub_1404B92B0`, vòng thứ hai: với mỗi map id trong map, tra `ChrSet` của map đó bằng `sub_140509200(WorldChrMan, key)` (duyệt danh sách ở `WorldChrMan+69344`). **Không thấy, tức block đã unload, thì huỷ cả nhóm** qua `sub_1404BBAA0` (gỡ mục warp bằng `sub_1404C26F0`, gỡ chr).
- `sub_140C987E0` cũng gọi `DisappearAll` (kèm một số thao tác phiên chơi); được gọi từ bộ đếm giờ trong `sub_140B019A0` và từ `sub_14065B780`. Chưa rõ ngữ cảnh.

Nghĩa là spirit là chr của block đang nạp, bị huỷ cùng block khi unload. Warp manager hiện có của mod (`warp.rs`) chỉ dịch chuyển spirit khi chúng và người chơi cùng nằm trong thế giới đã nạp, không giúp được ở đây.

## Hướng khả thi: lưu snapshot rồi gọi lại

Không giữ được chr sống. Cách duy nhất là gọi lại spirit sau khi load xong:

1. Theo dõi `groups` khi đang trong thế giới (`WorldChrMan.main_player` có), lưu snapshot: SpEffect kích hoạt của từng nhóm, số lượng đang sống, số bản Clone.
2. Phân biệt "mất do reload" với "mất có chủ ý": hook lời gọi `sub_1404BBAA0` trong vòng unload (đổi đích `call`, `codepatch::redirect_rel32`, vì Seamless quét chữ ký byte) cho trường hợp reload; hook `DisappearAll` (đã có `disappear_all_hook` ở `0x1404B9587`) cho thu hồi, chết, Spirit-Severing Blade thì xoá snapshot.
3. Sau khi load xong (có `main_player`, hết màn đen, chờ vài giây), gọi `DoSummon` (`sub_1404B85B0`) cho từng nhóm, cách nhau một khoảng, rồi để `Update` spawn như bình thường.

Ghi chú:

- Ngồi grace dễ hơn dịch chuyển: chỗ đứng không đổi, không lo điểm triệu hồi hay vùng cấm ở nơi đến. Spirit gọi lại đầy máu, hợp với việc nghỉ ngơi hồi máu.
- Dịch chuyển cần `SummonAnywhere` bật, nếu không điều kiện "rời khu bia đá" thu hồi ngay.
- `DoSummon` gọi trực tiếp **không tốn FP**, vì phí tính ở bước dùng item (`sub_1403C0930`). Cần chốt: miễn phí hay phải tính phí.
- Phải tôn trọng chỗ game cấm triệu hồi (đấu trường: game cố ý không đăng ký spirit với warp manager ở đó).
- Gọi lại số lượng lớn đúng lúc load dễ dính crash đồ hoạ giống Mimic Tear trong Seamless ([mimic_tear.md](mimic_tear.md)); trong Seamless còn có đồng bộ mạng chưa rõ. Đề xuất bản đầu chỉ offline.
- Đăng ký sớm với `warp_manager` khi chr chưa tra được từng gây crash ([warp_research.md](warp_research.md)); gọi lại phải chờ `resolvable()`.

## Chưa biết

- Lúc ngồi grace game huỷ spirit bằng đường nào: vòng unload (`sub_1404BBAA0`), một lời gọi `DisappearAll` (điều kiện 1 hay 3 có thể bị kích khi cờ sự kiện bia đá reset), hay đường khác. Nếu là thu hồi có chủ ý thì chỉ cần chặn đúng lời gọi đó (như `+0x28` ở `multi_spirit.rs`), đơn giản hơn nhiều và giữ nguyên máu spirit.
- Overworld: đi xa qua nhiều ô thì spirit có bị vòng unload huỷ không, hay game đổi khoá map id cho nhóm.
- SpEffect 202 là gì (tra Smithbox).

## Bước tiếp theo (pha 0, probe, mặc định tắt)

Thêm log chẩn đoán: lời gọi nào tới `DisappearAll` (điều kiện nào trong 4) và tới `sub_1404BBAA0`; map id người chơi; số nhóm theo từng map id. Test: ngồi grace, fast travel trong cùng map, fast travel xa, chết, đi xa trong overworld, một lần trong Seamless. Kết quả cho biết tín hiệu nhận biết reload, rồi mới viết tính năng.

Tham chiếu: `src/multi_spirit.rs` (`disappear_all_hook`, `groups`), `src/buddy_stone.rs` (`NoRestResummon`, `SummonAnywhere`), `src/warp.rs`, [multi_spirit.md](multi_spirit.md), [summon_anywhere.md](summon_anywhere.md).
