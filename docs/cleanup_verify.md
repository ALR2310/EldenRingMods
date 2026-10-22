# Kiểm tra lại trong game sau đợt dọn code

**Status 2026-10-22: chờ kiểm tra. Đợt dọn code `shared/` và từng mod chỉ được build và chạy unit test, chưa chạy trong game (đang tập trung dọn trước). Danh sách này gom những thay đổi có thể ảnh hưởng hành vi để kiểm một lượt sau khi dọn xong. Xong mục nào thì xoá mục đó, và xoá dòng tương ứng trong `TODO.md`.**

Khi thử: tắt mọi DLL tham chiếu khác (xem memory về cô lập kiểm thử) và bật `LogFile`.

## Chung (`shared/`)

- **codepatch** (`shared/src/codepatch.rs`): `overwrite_bytes` nay có `FlushInstructionCache`; `install_jmp_hook` / `redirect_rel32` viết lại qua `with_writable_code`; thêm `make_executable`.
  - weightmultiplier: hook chạy, đổi `Mode` rồi reload vẫn đúng.
  - spiritmultiplier: các patch `buddy_stone`, `band`, `multi_spirit` còn hiệu lực.
  - speedmultiplier với Seamless: `redirect_rel32` vẫn chuyển hướng và nối chuỗi với hook của Seamless.
- **diag**: module đầu tiên trong `Loaded modules` là exe (không còn dòng `Game:`); `is_module_loaded("ersc.dll")` nhận ra Seamless ở speedmultiplier ("detected") và spiritmultiplier (đường warp theo registry).
- **memscan**: thuật toán tìm mới (`windows().position`) và hạn chót tính bằng `Instant`: mọi hook dùng AOB vẫn tìm thấy pattern như trước (autoregen, runemultiplier, weightmultiplier, task_hook, alloc_hook).
- **config / toml_config**: comment cuối dòng trong ini (`Key=1   ; note`) đọc đúng; lưu từ menu và reload không để lại file `.tmp`, không mất comment.
- **task / logger**: log chờ `CSTaskImp` chỉ một dòng; `LogFile` bật tắt đúng khi reload.

## AnimCancel

- Guard, Dodge, Jump, Weapon swap cắt đúng như cũ; guard không cắt khi dual wield / Twin Blades / giữ guard từ trước (`weapon.rs` được gom lại).
- Probe (`CancelProbe`, `FlagProbe`) vẫn ghi `WINDOW` / `STYLE` / `FLAG` khi bật.
- Log khởi động theo dạng mới (`AnimCancelV1.toml: created` ...).

## AutoRegen

- Hồi theo tick: `Trigger` 0 (luôn), 1 (ngoài combat), 2 (trong combat), 3 (idle 5 giây), 4 (gesture); `ValueType` 0/1/2 và `Cap`.
- Gesture: ngồi thật hồi, bấm gesture khi đang cast thì không hồi, bấm lại để đứng dậy thì dừng (`trigger.rs` viết lại thành máy trạng thái).
- Hồi khi đánh trúng: `Mode` 0/1/2, `DamageType` 0/1/2, `ExcludeAow` (kể cả Ash dạng Stance như Unsheathe); hook cài lại đúng khi reload.
- `hit_hook.rs` mở quyền thực thi cho bộ đệm trampoline **trước** khi patch hàm gốc (trước đó là sau): đánh quái và bị đánh không crash, kể cả đòn đâm lén / chí mạng (lỗi từng gặp 2026-09-03).

## DropMultiplier

- `Mode=0` (nhân) và `Mode=1` (ép tỉ lệ, kể cả `Percentage=100`): tỉ lệ rơi đúng như trước; đổi chế độ rồi reload không cộng dồn.
- `[Materials]`: số lượng Crafting / Upgrade / Unique nhân đúng, nhặt đồ hái ngoài thế giới mở.
- Log đổi dạng (`Drops: Multiplier=... -> N row(s)`, `Materials: ... -> N slot(s)`).

## FasterRevival

- Chết thường, chết giữa lúc đang tấn công, đòn tóm, rơi vực: hồi sinh vẫn khoảng 4-5 giây (rơi vực không đổi).
- Log `Death: died / killed early / registered / respawned` và `Fade: delay 0s` còn đủ; `DeathProbe` bật vẫn ghi vết.

## InfiniteAilment

- Cả 4 hiệu ứng (kể cả Destined Death giảm máu tối đa) áp đúng thời lượng và sát thương; reload và menu không cộng dồn.
- Số dòng khớp mỗi hiệu ứng như trước (log `<Hiệu ứng>: duration=... -> N row(s)`), kể cả trên Convergence.

## PassiveRunes

- Mỗi chu kỳ cộng đúng tổng cố định + % chi phí lên cấp + lãi kép; giá trị âm trừ rune, không xuống dưới 0.
- Mốc thời gian vẫn được thưởng đúng một lần khi vượt mốc; đồng hồ tạm dừng ở màn hình tiêu đề và màn hình tải.
- Reload và menu đổi giá trị có hiệu lực ngay; log `Tick:` và `Milestone:` còn đúng.

## RiseArcher

- Sát thương, scaling, trọng lượng, giá bán, số đạn mang tối đa của Bow, Crossbow, Ballista áp đúng như trước.
- Rain of Arrows (tầm, độ loe, số mũi tên) và tốc độ đạn áp đúng; reload và menu không cộng dồn.
- Log `Applied:` / `Reload:` còn đúng số dòng, không có cảnh báo `expected ID(s) not found` trên regulation vanilla.

## RuneMultiplier

- Giết quái và nhặt vật phẩm nhận rune nhân đúng hệ số; lên cấp và mua đồ không bị nhân (stub dùng `overwrite_bytes` và `alloc_executable` mới).
- Reload và ô nhập trong menu đổi hệ số ngay; log `Hook: installed` còn xuất hiện và không còn dòng địa chỉ hay dump byte.

## WeightMultiplier

- Mode 0 nhân đúng Trọng Tải (số hiển thị và roll / chạy), Mode 1 đặt đúng giá trị cố định.
- Đổi Mode rồi reload và qua menu vá lại opcode trong stub mà không crash; log `Weight: mode=... value=...` mới.
- Với mod trọng lượng khác hook cùng chỗ (vd. ZibaPeso): hộp cảnh báo xung đột vẫn hiện và mod không cài hook.

## WindowResize

- Kéo từng cạnh và góc: tỉ lệ 16:9 giữ đúng, cạnh đối diện đứng yên, không nhỏ hơn MinWidth và MinHeight.
- Width / Height lúc khởi động (cả hai, chỉ một) áp đúng; Fullscreen và Borderless không bị thêm viền.

## SpeedMultiplier

- Tốc độ người chơi theo từng nhóm và hệ số tổng; Torrent đi bộ, chạy, nhảy; spirit và kẻ địch theo hệ số chung.
- Nạn nhân của backstab và riposte chạy theo tốc độ người chơi rồi về 1; khối [[Override]] theo SpEffect và lớp tải trọng.
- Với Seamless Co-op: stub vẫn chuyển hướng hook (`is_module_loaded` mới); probe (SpeedProbe, EffectProbe, EnemyProbe) vẫn ghi log.

## SoulsTeleport

- Hai máy Seamless Co-op: hỏi vị trí (WHERE) và nhận (HERE), teleport tới đúng chỗ qua màn hình tải.
- Gói tin sai độ dài hoặc sai người gửi bị bỏ; invader và người thù địch vẫn không teleport được; menu và chặn chuột vẫn đúng.

## SpiritMultiplier

- Triệu hồi nhiều spirit vượt 10 con, kiểm tra band và Seamless Co-op vẫn đúng
- Hồi máu (RegenMode 1/2/3) vẫn hồi cho spirit và Torrent
- Dịch chuyển spirit (WarpWhenFar) vẫn yêu cầu warp, log ghi 'warp requested'
