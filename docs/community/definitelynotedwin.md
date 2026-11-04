# DefinitelyNotEdwin (Nexus, trang SpeedMultiplier)

**Status 2026-11-04: cuộc trao đổi còn mở, tin cuối của Edwin (2026-11-03 23:38) đang chờ trả lời. Người dùng (ALR2310) đã trả lời các câu trước; câu hỏi cuối về RVA `register_task` trên 1.17.2 trả lời ở mục "Việc còn mở".**

Edwin viết mod riêng bằng Rust với `fromsoftware-rs`: tốc độ đánh, lăn, Torrent co giãn theo
Dexterity. Họ hỏi cách làm của SpeedMultiplier. Bản mod của họ chạy ở 1.17.0 và 1.17.1 nhưng
ngừng chạy ở 1.17.2. Mọi điều người dùng đã nói với họ nằm dưới đây để lần sau trả lời nhất quán.

## Dòng thời gian

| Ngày | Ai | Nội dung |
|---|---|---|
| 2026-10-17 | Edwin | Hỏi có decompile DLL được không; người dùng: được. |
| 2026-10-17 | Edwin | Muốn làm mod tốc độ theo Dex, không biết tốc độ animation ở đâu; hỏi công cụ. |
| 2026-10-17 | người dùng | Trả lời đầy đủ (mục "Đã nói với Edwin"). |
| 2026-10-20 | Edwin | Dùng game 1.17.0 (exe 2.7.0.0); crate panic vì RVA cứng cho 2.7.1.0; `CSTaskImp::run_recurring` làm game giết thread. |
| 2026-10-20 | người dùng | Đưa AOB `register_task`, vtable giả, `catch_unwind`, hoặc ghim crate về commit 2.7.0.0. |
| 2026-10-20 | Edwin | Mod đã chạy cho vũ khí và ash of war; hỏi cách làm cho Torrent. |
| 2026-10-21 | người dùng | Cách tìm Torrent và dải anim ID. |
| 2026-10-27 | CrimeEpiiC | Hỏi Edwin có tải lên hay gửi mod không; Edwin (2026-11-03): chưa, còn lỗi. |
| 2026-11-03 | Edwin | Mod chạy ở 1.17.0 và 1.17.1, hỏng ở 1.17.2; nghi `CSTaskImp::instance_mut()` crash. Hỏi 4 câu. |
| 2026-11-03 | người dùng | Trả lời 4 câu (mục dưới). |
| 2026-11-03 23:38 | Edwin | Quét pattern ở 1.17.2 (regulation 1.17.3) không thấy; tìm được wrapper ở `+0x125060` nhưng gọi thì game crash; xin RVA đúng. |
| 2026-11-04 | người dùng | Trả lời (bản dự kiến, nếu đã đăng thì cập nhật): RVA `register_task` 4 bản, gợi ý kiểm tra CE, mã mẫu rút gọn ở https://pastebin.com/f6pA3tfw (Nexus không hỗ trợ spoiler trong reply). |

## Đã nói với Edwin

- **Tốc độ animation:** mỗi nhân vật có `CSChrBehaviorModule` với `animation_speed` (float), tới được qua
  `ChrIns+0x190 → +0x28 → +0x17C8` (2.7.1.0, có thể lệch ở bản khác). Game không tự reset, nên ghi mỗi frame.
  Hook `ChrIns_PreBehavior`, chọn giá trị theo anim ID đang chạy (đánh, lăn, đi/chạy, Torrent). Đọc Dex từ
  dữ liệu người chơi mỗi frame để làm hệ số.
- **Công cụ:** Rust + `fromsoftware-rs`, bảng CE Grand Archives (địa chỉ animation speed), IDA (Ghidra cũng được),
  danh sách TAE ID (Syyyke, bản cập nhật cho SOTE), ModEngine 2.
- **Không dùng `run_recurring`** của crate vì phụ thuộc RVA của một bản game, panic ở mọi bản khác. Quét AOB hàm
  `register_task`, dựng đối tượng task riêng (vtable giả: `get_runtime_class`, `destructor`, `execute`); game chỉ
  đọc con trỏ đầu và gọi `execute`. Giữ đối tượng sống suốt đời DLL. Chi tiết: `shared/docs/task.md`.
- **Pattern** (một dòng, không thừa dấu cách; `??` là toán hạng rip-relative của call/lea):
  `48 89 5c 24 08 57 48 83 ec 40 48 8d 4c 24 20 49 8b d8 8b fa e8 ?? ?? ?? ?? 48 8b 0d ?? ?? ?? ?? 48 85 c9 75 2e 48 8d 0d ?? ?? ?? ?? e8 ?? ?? ?? ?? 4c 8b c8 4c 8d 05 ?? ?? ?? ?? ba b4 00 00 00 48 8d 0d ?? ?? ?? ?? e8 ?? ?? ?? ?? 48 8b 0d`
- **Hai mẹo:** destructor và `get_runtime_class` là hàm rỗng, không `unimplemented!()`; bọc closure mỗi frame
  trong `catch_unwind` (cần `panic = "unwind"`).
- **Ghim crate về 2.7.0.0:** `eldenring = { git = "https://github.com/vswarte/fromsoftware-rs", rev = "e0948c8d64594b79785e9490877baacea5605a9d" }`.
- **Torrent:** là một mục trong `WorldChrMan.summon_buddy_chr_set` (cùng tập với spirit ash), `npc_param_id == 80020000`,
  chỉ chạm các mục đang Active. Ghi `modules.behavior.animation_speed` trong cùng task `ChrIns_PreBehavior`, chọn theo anim ID
  (2.7.1.0): đi `0021xx`, chạy `0022xx`, nhảy `0061xx` (cất cánh) và `0074xx` (hạ cánh). Gợi ý: log anim ID khi cưỡi để xác nhận dải.
- **Skin Torrent** (NpcParam, Smithbox): `80020000` thường, `80030000` Tree Sentinel, `80040000`, `80050000` các skin khác;
  coi cả dải là Torrent để skin có cùng tốc độ.
- **Khi crash ở 1.17.2:** gợi ý log kết quả quét và việc task system đã có chưa trước khi đăng ký, để biết cái nào hỏng;
  lấy `CSTaskImp` theo tên singleton, tránh mọi thứ đi qua bảng RVA của crate, quét một lần.

## Việc còn mở

- **RVA `register_task` theo bản exe** (tra bằng `scripts/ida/run.ps1 aob -Version all`, 2026-11-04, mẫu duy nhất ở cả 4 bản):
  `2.6.2.0` = `0xEB1FE0`, `2.7.0.0` = `0xEB3DE0`, `2.7.1.0` = `0xEB3E50`, `2.7.2.0` (1.17.2) = `0xEB5540`.
  Đã bổ sung vào `shared/docs/task.md`. Trả lời Edwin kèm RVA `0xEB5540`, nhắc dán pattern một dòng.
- Phiên bản của Edwin đã đúng: exe 1.17.2 (2.7.2.0), regulation 1.17.3; không cần nhắc lại điều này với họ.
- File IDA (`dumps/eldenring/2.7.2.0/eldenring.exe`) giống từng byte exe cài trong game, và mẫu có sẵn trong file tĩnh, nên khác biệt IDA/CE
  không phải do mã hoá exe ở hàm này. Khả dĩ cho "CE báo Not Found": đầu hàm đã bị DLL khác hook (mẫu bắt đầu đúng ở byte đầu hàm),
  CE không quét vùng thực thi của module `eldenring.exe`, pattern bị ngắt dòng hoặc thừa dấu cách. Chưa biết nguyên nhân thật.
  Phép thử đã gợi ý: quét từ `48 8d 4c 24 20 49 8b d8 8b fa e8` rồi lùi 10 byte (5 + 1 + 4) về đầu hàm.
- Wrapper ở `+0x125060` Edwin tìm thấy không phải `register_task`; gọi nó làm game crash là đúng dự đoán.
- Mod của Edwin chưa phát hành; nếu họ đăng, có thể được nhắc trên trang SpeedMultiplier.
