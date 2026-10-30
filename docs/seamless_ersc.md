# ersc.dll (Seamless Co-op): cấu trúc và mức đọc được

**Status 2026-10-29: `.text` đọc được bằng IDA; chỉ ~21 hàm `.text` nhảy vào
vùng bị che. Chưa dịch ngược logic nào (ghost color, scaling).**

Phiên bản đã xem: 2.0.1 (SHA256 `fcd11a18…`) và bản 2026-10-27 (`0a093d84…`).
Dữ liệu ở `dumps/ersc/<hash8>/` (git-ignored): `ersc.dll`, `ersc.dll.i64`,
`unpacked/` (bản dump bộ nhớ của 2.0.1).

## Kết luận

- Protector là Themida 3.x (section `ERSC` ~11 MB, RWX, entry point nằm trong
  đó). `unlicense` không dump được 2.0.1: tự nhận sai phiên bản, ép
  `--target_version=3` thì không tới OEP trước timeout (cả 60 s lẫn 240 s).
- Dump bộ nhớ lúc đã vào world (`scripts/dump_module.py ersc.dll`) cho thấy
  `.text` giống 100% file trên đĩa và `ERSC` giống 99,94%: Themida **không giải
  mã gì lúc chạy**. Code thật nằm sẵn trong file; dump chỉ thêm base thật
  (`0x180000000`) và dữ liệu `.data` đã khởi tạo.
- Bản 2026-10-27 (IDA 9.3, auto analysis): 10112 hàm, 4868 ở `.text`, 5244 ở
  `ERSC`. Mẫu 300 hàm `.text` đầu decompile được 300/300. Chỉ 21 hàm `.text`
  gọi/nhảy vào `ERSC` (đếm bằng xref của IDA; quét byte `E8/E9` thô ra 119 là
  dương tính giả). Đó là các hàm bị Themida mutate/virtualize, phần duy nhất
  không đọc thẳng được.
- Seamless không để chữ ký tìm hook ở dạng thô: 24 byte đầu của
  `sub_1403F0C20` (và các đoạn con 9 đến 16 byte) không có trong file nào.
- Hook ghost color và get-team (bản 2026-10-27, đọc lúc game chạy bằng
  `scripts/dump_module.py eldenring.exe --peek 3F0C20`): `E9` từ
  `sub_1403F0C20` / `sub_1403F1C90` → stub cấp phát ngay trước ảnh exe
  (`jmp [rip+0]`) → thunk `mov rax,imm64; movq xmm5,rax; mov r11,<đích>; jmp r11`
  → đích `ersc.dll` `0x180095B20` / `0x180095E60`. Cả hai nằm trong `.text`
  (đọc được), không phải `ERSC`.
- Hai đích đó chỉ là thunk gọi callback kiểu `std::function`: ngữ cảnh nằm
  trong `xmm5` (`sub_1800EA0D0()` chỉ trả `xmm5`), callback là đối tượng có
  vtable trong `.rdata`, slot 2 lại là thunk `fn(*(obj+8), args)` với `obj+16`
  là hàm thật. Hàm thật chỉ biết lúc chạy (heap): bản 2026-10-27 là
  `ersc+0x83260` (ghost color, thay `sub_1403F0C20`) và `ersc+0x8E600` (get
  team, thay `sub_1403F1C90`), đều ở `.text`, decompile được.
- `sub_180083260` (ghost color): nếu cờ bật thì chọn **PhantomParam row**
  `8380001..8380005` theo vị trí người chơi (bảng `0x1801DD880`), mặc định
  `8380000` (`8380006` ở một điều kiện khác), rồi tra param bằng
  `sub_1800A95C0`. Với spirit (nhóm `WorldChrMan+69528`), nó tìm chủ qua
  `sub_180028700` rồi mới tô màu của chủ. Màu chỉ áp khi mục người chơi của
  Seamless có cờ `0x200`.
- `sub_180028700` (tìm chủ của spirit): duyệt mảng người chơi
  (`WorldChrMan+69360`), khớp nếu `*(player+1408)+2384` == handle của spirit,
  hoặc spirit nằm trong danh sách buddy ở `WorldChrMan+124216` của người chơi
  chính, hoặc id ở `+1456` khớp `*(spirit+536)`. Không khớp thì spirit không
  có chủ và không được tô màu.

## Giả thuyết cho lỗi mất màu ma (chưa kiểm chứng)

Spirit do SpiritMultiplier sinh thêm có thể không khớp bất kỳ điều kiện nào
của `sub_180028700` (handle không nằm ở `+2384` của người chơi, không có trong
danh sách buddy `+124216`, `+536` không khớp), nên Seamless coi là không có
chủ và không tô màu. Cần kiểm bằng cách đối chiếu các trường đó trên một
spirit gốc và một spirit clone.

## Đọc lúc chạy (2026-10-29, `scripts/dump_module.py eldenring.exe --chr`)

AOB `48 8B 05 ?? ?? ?? ?? 48 85 C0 74 0F 48 39 88` tìm `WorldChrMan`
(global ở `exe+0x3D6D018` trên bản đang chạy). Lúc đứng ở Gatefront, chưa có
spirit: mảng người chơi (`+69360` đếm, `+69368` mảng, bước 16) chỉ có 1 phần tử
là người chơi chính (`+0x1E508`), handle `+8` = `0xFFFFFFFF16F00000`,
`*(player+536)` = 0, `*(player+1408)+2384` = `0xFFFFFFFF17100000` (một
handle khác, nhiều khả năng của spirit gần nhất, chỉ 1 ô). Đối tượng buddy ở
`+124216` có vtable của game (`exe+0x3B488C8`), cấu trúc chưa giải.

### Sau khi triệu hồi 2 lần (CloneSpirit=true, Multiplier=3, 6 spirit)

Handle spirit `0xFFFFFFFF17100014` đến `…19` (nhóm 1: 14 đến 16, nhóm 2: 17
đến 19). `*(player+1408)+2384` **vẫn** là `0xFFFFFFFF17100000`, y như trước khi
có spirit, và handle đó là một đối tượng cùng loại với các spirit (vtable
`exe+0x2A49090`, bước 0x5F0, `+536` = 0, `+1408`/`+1456` là con trỏ hợp lệ), nhiều
khả năng Torrent. **Giả thuyết cũ ("`+2384` giữ handle spirit") sai**: ô này là
nhánh dành cho Torrent, không liên quan tới spirit.

Cả 6 spirit đều có `+536` = 0 và `+1408`/`+1456` hợp lệ, **không khác nhau**
giữa spirit gốc và clone ở các trường này. Còn lại để so sánh: danh sách buddy
`*(WorldChrMan+124216)` (hàm `a1[32]`/`a1[33]` của bảng hàm Seamless, chưa giải)
và cờ `0x200` trên mục người chơi của Seamless trong `sub_180083260`.

Chú ý: ini đang để `GhostColor=false` (mod chủ động bỏ tint), nên lỗi "mất màu
ma" chỉ thấy khi `GhostColor=true`; hook Seamless cũng chỉ chạy nhánh tô màu
khi cờ ở `*(ctx+152)+641 & 0x40` bật (có vẻ chỉ khi đang trong phiên co-op).

Công cụ: `scripts/dump_module.py eldenring.exe --chr` (WorldChrMan, người chơi)
và `--handles HEX,HEX` (quét heap theo handle, nhiều kết quả nhiễu, lọc theo
vtable `exe+0x2A49090`).

### Tái hiện lỗi (2026-10-29): không cần người chơi thứ hai

`GhostColor=true` + Seamless bật, chơi solo, triệu hồi Skeletal Bandit Ashes:
spirit hiện đặc, không có lớp ma xanh (log SpiritMultiplier vẫn ghi
`ghost tint restored on 2 vfx row(s)`, nên tint nằm ở chỗ khác với 2 hàng vfx
đó). Cờ `*(*(v3+152)+641) & 0x40` = `0xD3` (bật) ngay khi solo, nên hook
`sub_1403F0C20` của Seamless luôn thay hàm gốc.

Giá trị trên spirit (đọc bằng `--handles`): `+1344 = -1` (không ghi đè),
`+104 = 5`, `+524 = -1`, `*(*(*(+400)+184)+68) = 200`; Torrent (`…17100000`)
cho `-2` ở trường cuối. Theo `sub_180083260`, spirit không khớp mục người chơi
nên Seamless trả đúng `200` (không phải 8380000..8380006), và hàm gốc game
(`mov eax,[rcx+540h]; ...; mov eax,[[[rcx+190h]+0B8h]+44h]; cmp eax,-2; jnz`)
cũng trả giá trị đó, nên hook phantom có vẻ **không phải thủ phạm**. Nghi hơn là
hook get-team (`sub_1403F1C90` → `ersc+0x8E600`): nó ghi `16`/`15` vào byte
đầu ra khi khớp mục người chơi, có thể đổi team của spirit khiến game bỏ tint.

### Cập nhật: get-team không phải thủ phạm; nghi `sub_180083260` gán màu chủ

Hàm gốc `sub_1403F1C90` chỉ ghi `team = *(chr+108)` (và `15` nếu có SpEffect
132). Các hook khác trong cụm `0x3EE7B0..0x3FC4A0` là setter/khoá sắp xếp vẽ
(`sub_1403F0700` dựng khoá thứ tự vẽ 15 bit), không liên quan tint.

Đọc lại đuôi `sub_180083260`: sau khi tìm chủ của spirit (qua danh sách buddy),
nó tra mục người chơi Seamless của chủ; nếu mục đó có cờ `0x200` thì `v6 = 1` và
hàm trả **PhantomParam row của slot chủ** (`8380001..8380005`, bảng
`0x1801DD880`), còn không thì trả `*(*(*(chr+400)+184)+68)` (= `200` với spirit).
Hàm gốc game trả cùng giá trị `200` đó cho spirit, nên màu ma xanh vanilla là
PhantomParam row `200`; dưới Seamless spirit nhận row màu của chủ (đặc) nên mất
tint. Đây là hành vi "spirit mang màu người triệu hồi" của Seamless chứ không
phải bug trong mod.

Cả hai hàm (Seamless và game) đều trả thẳng `*(chr+1344)` khi giá trị này
`>= 0` (bỏ qua mọi logic còn lại). Hướng sửa đề xuất: khi `GhostColor=true` và
phát hiện Seamless, SpiritMultiplier ghi `*(spirit+0x540) = 200` cho từng spirit
(hoặc theo `BuddyParam`/NpcParam của spirit), để cả hai hàm trả `200`. Chưa
kiểm chứng trong game (cần ghi bộ nhớ spirit, hoặc làm trong
`ghost_color.rs`).

### Thử ghi `+1344` (2026-10-29): không có tác dụng nhìn thấy

Ghi `200` vào `*(spirit+1344)` của 3 spirit đang ra (bằng `--handles … --set1344
200`) rồi chụp, sau đó đặt lại `-1` và chụp cùng khung hình: hai ảnh giống hệt
nhau, tint ma không quay lại. Nên hoặc PhantomParam của ChrIns được tính một lần
lúc spawn (ghi sau không có hiệu lực), hoặc nguyên nhân mất tint không nằm ở
giá trị hook này. Giả thuyết "row màu của chủ" ở trên vì vậy **chưa được xác
nhận**.

## Chưa làm

- Chạy cùng cấu hình nhưng **không nạp Seamless** để xem hình dạng spirit
  vanilla (có tint ma) và đọc cùng các trường (`+1344`, `+104`, chain `+68`) để
  so với bản Seamless.
- Thử ghi `+1344` **trước** khi spirit hiển thị (cần hook lúc spawn, trong mod).
- Giải danh sách buddy `*(WorldChrMan+124216)` và bảng hàm `a1[32]/a1[33]` của
  Seamless, rồi xem clone có trong đó không.
- Liệt kê và phân loại 21 hàm `.text` gọi vào `ERSC`.
- Diff 2.0.1 với bản 2026-10-27 (địa chỉ đã dịch chuyển, cần khớp theo hàm).
- Cơ chế scaling enemy (hệ số trong `ersc_settings.ini`, SpEffect gắn lên
  enemy theo số người chơi).

Liên quan: `dumps/seamless-diff/` (các chỗ Seamless vá trong `eldenring.exe`),
`mods/speedmultiplier/docs/seamless.md`.
