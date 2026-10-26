# Tự nhặt qua hook `ExecActionButton`

**Status 2026-10-26: đã thử trong game trên 2.7.2.0 và người dùng xác nhận chạy đúng (các nhóm, tầm nhặt, tạm dừng khi chiến đấu). Hook `CS::CSActionButtonManImp::ExecActionButton` trả "đã kích hoạt" cho ID ActionButtonParam của nhóm được bật; nới tầm bằng cách nhân `radius/height/baseHeightOffset`. Cờ "đang chiến đấu" lấy từ `CSSound` theo offset của Erd-Tools; chạy đúng trong game, nhưng chưa tự tìm được chỗ ghi byte `+4` trong exe.**

## Nguồn tham khảo

Ba mod có cùng tính năng, cùng bốn nhóm và cùng quy tắc "chờ khi đang chiến đấu":

- **Elden_Ring_game_tools** (`tmp/Elden_Ring_game_tools/Source/Features/AutoPickup.cpp` + `.asm`): có mã nguồn, là nguồn ID và offset của mod này. Chứa cả `Erd-Tools-CPP-main` nhưng thư mục đó chỉ còn README và minhook.
- **Erd-Tools-CPP** (`tmp/Erd-Tools-CPP`, bản dựng thành `ErdTools.dll` của Convergence; `Hook/FeHook.cpp`, `Hook/ParamHook.cpp`, `ASM/ExecActionButtonParamWrapper.asm`): có mã nguồn, là gốc của Elden_Ring_game_tools. Cùng mẫu AOB, cùng 4 nhóm ID, cùng cách nhân `radius/height/baseHeightOffset` (một lần lúc khởi động, không ghi lại mỗi frame). Mục `[FRONTEND] pickup_*`, `[PARAM] harvest/item/corpse_loot/lost_runes_*_multiplier` trong `erd_tools.ini`. Có thêm `lock_*` (chặn tương tác khi đang chiến đấu, mod này không có) và `vanilla_pickup_animations`: chỉ sửa `PlayerCommonParam` (`animeID_DropItemPick=60070`, `animeID_SleepCollectorItemPick=50250`, `animeID_MaterialItemPick=999999`), không liên quan tới cơ chế tự nhặt; mod này không đụng animation.
- **Solid_Uncapper.dll**: chỉ có DLL, mục `Auto Pickup` trong ini; không đụng cần gạt, cửa, hội thoại.

Chưa ai trong ba mod được dịch ngược ở đây; mod này chỉ lấy cơ chế và kiểm lại điểm hook trong exe. Solid Uncapper chỉ có DLL nên không đối chiếu được mã.

## Cơ chế

`ExecActionButton(this, id, float, a4, ...)` là hàm game hỏi "tương tác `id` (dòng ActionButtonParam) có được kích hoạt không". Trả 0 = không, 1 = có. Hook trả 1 thẳng cho ID thuộc nhóm đang bật, không chạy hàm gốc; game tiếp tục như thể người chơi đã bấm nút nhặt. ID ngoài các nhóm (cần gạt, cửa, grace, nói chuyện) chạy hàm gốc như cũ.

Nhóm (ID trong `src/groups.rs`): nguyên liệu (112 ID), vật phẩm trên đất (14), đồ trên xác và hài cốt (5), rune rơi (1000), rương (4300, mặc định tắt).

ID khởi đầu lấy từ `AutoPickup.cpp`, rồi đối chiếu với bảng `ActionButtonParam` export của vanilla và Convergence (`tmp/ActionButtonParam.csv`, `tmp/convergence_csv/`; 98 ID gốc đều có ở cả hai và giống hệt nhau). Chỉnh so với bản gốc:

- Mảng gốc khai báo 78 phần tử nhưng chỉ có 76 ID nguyên liệu, hai ô cuối là 0 làm `binary_search` của bản C++ chạy trên mảng không tăng dần.
- `4300` ("Search treasure chest") nằm trong nhóm Items của bản gốc: tách thành nhóm `Chests`, mặc định tắt (rương mimic mở cùng cách).
- `4200-4202` ("Pillage remains") và `9532` ("Pillage corpse") chuyển từ Items sang CorpseLoot.
- Thêm `207850` và `207860-207894` ("Acquire materials", 36 dòng, nguyên liệu DLC; song song với `7850`, `7860-7878` của game gốc, bản gốc chỉ có tới `207844`), và `209210`, `209524`, `4261` ("Pick up item"; `4261` chỉ có ở Convergence). Các ID thêm này dựa vào tên và cấu trúc bảng, chưa thử trong game.

Tầm: nhân `radius`, `height`, `baseHeightOffset` của các dòng nhóm bật theo `Range`. Giá trị gốc lưu ở lần quét đầu (`src/range.rs`), nên đổi `Range` hay tắt nhóm luôn tính từ số gốc của game, không cộng dồn. Cùng giá trị này quyết định cả khoảng cách hiện gợi ý nhặt.

## Xung đột với mod khác

Hook thay 16 byte đầu hàm bằng một lệnh nhảy. Nếu mod khác đã làm vậy trước (nạp trước), mẫu đầy đủ không còn khớp. `hook.rs::scan` khi đó tìm phần đuôi của mẫu (luôn cách đầu hàm đúng 0x10 byte, duy nhất ở cả 4 bản exe) và hỏi `common::conflict::foreign_hook_at` ở địa chỉ đầu hàm; có lệnh nhảy ra ngoài exe thì ghi log, hiện hộp thoại và tắt hook (cùng cách `weightmultiplier` làm, xem `shared/docs/conflict.md`). Chưa thử với mod xung đột thật: chỉ có test của `common` cho việc nhận dạng lệnh nhảy. Mod khác hook ở hàm bọc `A673D0`/`A674B0` (như bản Erd-Tools gốc) thì không xung đột, vì chúng gọi hàm lõi và hook ở đây vẫn chạy.

## Giấy phép và ghi công

Erd-Tools-CPP (Nordgaren) dùng GPLv3; README của nó cho phép dùng trong mod khác với điều kiện dẫn link về repo, và nói tác giả không cho đăng lại **mod của họ** lên Nexus. Mod này là mã Rust viết mới: lấy từ nguồn đó ý tưởng (hook hàm tương tác), danh sách ID ActionButtonParam (dữ liệu của game, đã đối chiếu và chỉnh) và offset của `CSSound`. Trang Nexus ghi công Erd-Tools-CPP và Elden_Ring_game_tools (ankerlcg1996) kèm link. Nếu tác giả phản đối thì gỡ trang, không đổi ghi công.

## Điểm hook

Hàm lõi là `sub_140A64680` (2.7.2.0; chuỗi `CS::CSActionButtonManImp::ExecActionButton`): `a1` = CSActionButtonManImp, `a2` = ID. Hai hàm bọc cạnh nhau đều chuyển tiếp vào nó:

- `A673D0`: gọi thẳng, 21 nơi gọi.
- `A674B0`: lấy khoá `a1 + 144` rồi gọi (qua `A64AA0`), 2 nơi gọi (`3DD060`, `577F90`).

AOB của Erd-Tools (mã Elden_Ring_game_tools) khớp **cả hai hàm bọc** (2 kết quả ở mọi bản exe) và `modutils::scan` lấy kết quả đầu, nên bản C++ chỉ hook một trong hai. Mod này hook hàm lõi để bao phủ cả hai.

Mẫu 61 byte ở `src/hook.rs` (prologue của hàm lõi) **duy nhất ở cả 4 bản exe**: 2.6.2.0 (`A62EE0`), 2.7.0.0 và 2.7.1.0 (`A641C0`), 2.7.2.0 (`A64680`). Kiểm bằng `scripts/ida/run.ps1 aob -Version all`. 16 byte đầu (7 lệnh, tới trước `SUB RSP,0F0h`) không có lệnh RIP-relative nên relocate nguyên văn.

Trampoline lưu `rcx/rdx/r8/r9` và `xmm0-3`, gọi `pickup_filter(this, id)` (chỉ đọc hai atomic, không cấp phát, không khoá). Trả khác 0 thì `eax = 1; ret`, ngược lại khôi phục thanh ghi và nhảy vào 16 byte đã relocate.

## Lỗi đã gặp: nhảy về bằng `rax` (2026-10-26)

Lần thử đầu crash 3 giây sau khi vào thế giới: `illegal instruction at eldenring.exe+0xA64678`, `rdx=0x17D5` (ID 6101, "Rest at site of grace", ngoài mọi nhóm nên hook cho chạy hàm gốc). Nguyên nhân: đường nhảy về sau 16 byte relocate dùng `mov rax, imm64; jmp rax`, trong khi prologue của hàm để `rax` = `rsp` lúc vào và nửa sau hàm lưu thanh ghi theo `rax` (`mov [rax+8], rbx`, `[rax+10h]`, `movaps [rax-38h], xmm6`). Với `rax` = địa chỉ code thì các lệnh đó ghi đè lên chính code của hàm (trang đã là RWX sau khi patch), và hàm chạy vào đoạn hỏng. `hit_hook` của autoregen không dính vì hàm của nó dùng `r11`. Sửa: nhảy về bằng `mov r11, imm64; jmp r11` (13 byte, `hook.rs::jmp_via_r11`). Quy tắc khi relocate prologue: trước khi chọn thanh ghi cho lệnh nhảy, kiểm hàm còn dùng thanh ghi đó sau prologue không.

## Cờ chiến đấu

`CSSound` (singleton theo tên, đăng ký ở `sub_1400B0470`) `+0x328` -> struct, byte `+4` = đang chiến đấu. Offset giống hệt nhau ở Erd-Tools-CPP (`ErdTools_globals.h`: `CSSound.undefined[0x328]` rồi `soundCombatStruct`, `SoundCombatStruct.undefined[4]` rồi `isInCombat`) và Elden_Ring_game_tools. Erd-Tools tìm con trỏ `CSSound` bằng AOB `48 8B 05 ?? ?? ?? ?? 48 85 C0 ?? ?? 83 CD 02 83 B8 ?? ?? ?? ?? 00 89 6C 24 ?? 0F 95 C1`; mẫu này duy nhất ở cả 4 bản exe và trên 2.7.2.0 rơi vào `sub_1404395D0` (RVA `439A63`), đọc global `143D8AD68` rồi so `[+0x314]` với 0, nên đúng là một đối tượng có các field quanh `+0x314..+0x328`. **Chưa tự xác nhận**: chưa chứng minh `143D8AD68` chính là singleton đăng ký tên `CSSound`, và chưa tìm được chỗ ghi byte `+4`; `fieldaccess 328` chỉ ra các stub của CSSoundImp (class khác). Đọc mỗi frame ở task `FrameBegin` (không trong hook: tra singleton không an toàn trong hook) và kiểm con trỏ hợp lệ. Cần xác nhận trong game: vào trận, xem log/hành vi `PauseInCombat`.

## Rủi ro chưa kiểm

- Trả 1 bỏ qua cả điều kiện của hàm gốc (`a4` hoặc `sub_140A656D0(a1)` trả 0 sớm). Mọi mod tham khảo làm vậy; nếu thấy nhặt khi đang mở menu, cutscene hay đang ngồi ngựa thì thêm điều kiện ở đây.
- Chưa thử cùng `Solid_Uncapper`/`ErdTools.dll`: tắt `Auto Pickup` ở các DLL đó khi thử (xem quy tắc "tin kết quả thử có điều kiện" ở `CLAUDE.md`).
