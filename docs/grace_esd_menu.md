# Chèn mục vào menu Site of Grace: hook EnterState, mẫu AOB, dải shop

**Status 2026-10-26: dự án GraceExpanded (mục Upgrade + Purchase trong menu Grace) đã dừng, không phát hành vì mod [5513](https://www.nexusmods.com/eldenring/mods/5513) đã làm việc này và tốt hơn (người dùng). Công thức chèn mục đã test trong game ở SomeTweaks (2026-08-28, xem [../mods/sometweaks/docs/grace_menu.md](../mods/sometweaks/docs/grace_menu.md)); phần hook mới (dưới đây) đã build và kiểm bằng IDA nhưng chưa chạy trong game. Doc này giữ những gì đáng dùng lại.**

## Công thức chèn mục

Giữ nguyên như [SomeTweaks](../mods/sometweaks/docs/grace_menu.md): mỗi lần vào Grace game dựng một bản sao ESD graph, hook bắt lúc máy ESD vào state đầu, nhận Grace theo nội dung (có `add_talk_list_data` message 15000395 "Sort Chest"), chèn state mới (lệnh mở menu + điều kiện chờ `menu_closed_expression(loại menu)` + trả về state vanilla "dựng lại danh sách") rồi chèn dòng danh sách và transition chọn mục. Bảng lệnh:

| Mục | Lệnh | Loại menu | Index | Chữ (FMG vanilla) |
|---|---|---|---|---|
| Upgrade | 4x `CombineMenuFlagAndEventFlag(6001, 232..235)`, `Unknown141(9)`, `OpenEnhanceShop(0)` | 9 | 71 | 22130001 |
| Purchase | `OpenRegularShop(đầu, cuối)` | 5 | 72 | 26000010 |
| Sell (chưa làm) | `OpenSellShop(-1, -1)` | 6 | 73 | 20000011 |

- **Loại menu của điều kiện chờ phải khớp lệnh**, sai thì nhân vật đứng dậy. Biểu thức chỉ khác 1 byte giữa các loại: `[7b, loại+64, 40, 86, 41, 95, 7a, 40, 85, 40, 95, 98, 40, 95, a1]`.
- Nhận biết "đã chèn" theo từng bản đồ thị bằng index (71/72), không bằng cờ toàn cục (cờ toàn cục chỉ chèn ở Grace đầu tiên). Mỗi lần vào Grace rò ~1 KB (`Box::leak`).
- **Chữ**: dùng thẳng ID FMG vanilla, không cần hook `get_message`. SomeTweaks có hook đó (332 dòng) nhưng cuối cùng cả ba mục đều dùng chữ vanilla.

## Hook `EzState::EnterState` (mới, chỉ kiểm bằng IDA)

Hook chỉ quan sát: ghi đè 15 byte mở đầu hàm bằng `jmp` vào stub (`codepatch::install_jmp_hook`); stub lưu `rcx/rdx/r8/r9`, `sub rsp, 28h`, `mov rax, <observer>; call rax`, `add rsp, 28h`, khôi phục, rồi chạy lại đúng 15 byte đã ghi đè và nhảy về phần còn lại. `observed(state, machine)` nhận hai đối số đầu của `EnterState` (rcx = state, rdx = machine); `machine+0x28` là `state_group`. Đích hook lấy từ lệnh `call` trong mẫu AOB (không hardcode RVA).

- **Mẫu AOB cũ của SomeTweaks khớp 2 chỗ** ở cả bốn bản exe: hai hàm anh em (vd. 2.7.2.0: `sub_14208A4E0`, `sub_14208A7B0`) cùng gọi một `EnterState`. Đuôi thêm sau `call` (`lea r13d` khác `lea r12d`) làm mẫu duy nhất:

  `80 7e 18 00 74 15 4c 8d 44 24 40 48 8b d6 48 8b 4e 20 e8 ?? ?? ?? ?? c6 46 18 00 45 33 ff 45 8d 6f 01` (lệnh `call` ở offset 18)

- Kiểm bằng `scripts/ida/run.ps1 aob -Version all`: `UNIQUE` ở 2.6.2.0 (`2086FE2`), 2.7.0.0 (`2088DE2`), 2.7.1.0 (`2088E52`), 2.7.2.0 (`208A582`).
- Phần mở đầu `EnterState` giống hệt ở cả bốn bản, dừng đúng ranh giới lệnh: `48 8b c4 57 41 56 41 57 48 81 ec 20 04 00 00` (`mov rax,rsp; push rdi; push r14; push r15; sub rsp,420h`); đích `2088860` (2.6.2.0), `208A660` (2.7.0.0), `208A6D0` (2.7.1.0), `208BE00` (2.7.2.0). Hook nên so 15 byte này trước khi vá và từ chối nếu khác.
- Trong hook có cấp phát (`Box::leak`, `Vec`) và ghi log, như SomeTweaks (chạy được). Nếu treo hoặc crash ở lần vào Grace thì chuyển phần dựng sang thread riêng, hook chỉ ghi nhận.

## Shop Twin Maiden Husks: dải ID

- **`ShopLineupParam` không có cột "thuộc shop nào".** Shop nào bán dòng nào do ESD của NPC quyết định, qua cặp ID trong `OpenRegularShop(đầu, cuối)`; dải chính là hai số đó. Muốn biết dải thật phải đọc ESD của Husks (`talkesdbnd` vùng Roundtable Hold, mở bằng DSMapStudio hoặc ESDLang), hoặc xem các dòng `ShopLineupParam` từ `101800` trở đi trong `regulation.bin` (Smithbox).
- Dải `101800-101899` (~100 dòng) lấy từ ghi chú SomeTweaks ("bản đầu chỉ mở Husks, không giật"), **chưa tự đối chiếu** (workspace không có dump param).
- Dải cố định trong mã sẽ **không hiện hàng mod lớn thêm ngoài dải** (Convergence, Reforged chưa kiểm). Hướng chưa làm: key ini cho dải; hoặc hook lệnh `OpenRegularShop` khi người chơi nói chuyện với Husks ở Roundtable Hold, ghi lại cặp ID và dùng lại ở Grace.
- `OpenRegularShop(0, 9999999)` (mọi thương nhân) dựng ~1277 dòng đồng bộ nên giật 1-2 giây, và thu hẹp dải không giúp (các loại item trải gần hết `0..9999999`).

## Các mod tham khảo

`EldenConvenienceMod` (chèn tĩnh bằng `SoulsIds.ESDEdits`, nguồn của chuỗi lệnh Upgrade và `MenuCloseExpr`), `erdGameTools` (kỹ thuật nhận Grace theo "Sort Chest"), `Elden-Ring-CT-TGA` (tên lệnh ESD). Mod đã làm tốt việc này: [Nexus 5513](https://www.nexusmods.com/eldenring/mods/5513).
