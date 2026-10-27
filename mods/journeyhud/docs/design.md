# JourneyHud: thiết kế và nguồn tham khảo

**Status 2026-10-27: giai đoạn 1 (thanh máu/poise) xong và đã thử; giai đoạn 3 (minimap, lớp chấm) có code, chưa thử ([radar.md](radar.md)); giai đoạn 4 (nền bản đồ) chưa làm.** Nhánh `mod/journeyhud`. Kiểm chứng offset theo bản exe: [offsets.md](offsets.md).

## Mục tiêu

HUD overlay (DX12, ImGui qua hudhook, cùng nền với SoulsTeleport) ở góc phải màn hình:

1. **Giai đoạn 1, panel mục tiêu:** tên, HP và Poise của kẻ địch đang khoá. Không stamina, không FP, không thanh tích lũy trạng thái, không bảng sát thương (đã chốt với người dùng).
2. **Giai đoạn 2, khung HUD:** neo bố cục để panel nằm ngay dưới ô minimap, không có minimap thì lên góc.
3. **Giai đoạn 3, radar:** chấm người chơi, đồng đội, spirit, mục tiêu khoá, xoay theo camera.
4. **Giai đoạn 4, nền bản đồ:** tuỳ chọn, làm sau cùng.

## Nguồn tham khảo (chỉ lấy ý tưởng, tự viết lại)

| Repo | Giấy phép | Dùng cho |
| --- | --- | --- |
| `tmp/souls_vision` (C++, samjviana) | **GPL-3.0** | Cách tìm mục tiêu, nơi đọc HP/stagger. Không copy code, không dịch từng dòng. |
| `tmp/kubilaiswf-minimap` (Rust, kubilaiswf) | MIT | Đổi toạ độ ô sang bản đồ, đọc tile bản đồ từ bản cài game, camera, vẽ khung. Được tham khảo và dùng lại có ghi nguồn. |

Vì souls_vision là GPL-3.0 nên phần panel mục tiêu phải viết độc lập: dựa vào kết quả dịch ngược và kiểu Rust của `fromsoftware-rs`, không nhìn code C++ khi viết.

## Bố cục và màu

- **Khu vực vẽ:** game vẽ HUD trong khung 16:9 vừa khít cửa sổ (dải đen ở trục còn lại; màn 2560x1600 có dải đen 80 px trên và dưới). Overlay neo vào khung đó, không vào cả màn hình, và co giãn theo độ cao của nó (thử game 2026-10-26: tính theo cả màn hình làm panel to và đè dải đen). Siêu rộng (21:9) coi như viền đen hai bên; bản mod mở rộng HUD sẽ khác.
- **Cỡ:** thanh HP của người chơi đo từ ảnh 2560x1440: dài khoảng 949 px (khoảng 712 ở 1080p, đổi theo máu tối đa của nhân vật), dày khoảng 16 px (12 ở 1080p), mép trên cách đỉnh khung 63 px (47 ở 1080p). Thanh mục tiêu dày 12 (poise 6), cùng độ dày với thanh người chơi, nhưng ngắn (mặc định 220, khoá `Width`): thử game cho thấy dài bằng thanh người chơi (700) quá to, người dùng chọn cắt ngắn.
- **Kiểu thanh:** đo từ ảnh game 2560x1440: lõi đỏ sậm phẳng (khoảng RGB 92,23,20) cao khoảng 8 px, mép dưới ngả hồng nhạt (110,67,55), viền be mảnh (135,128,95) có nắp hai đầu, nền gần đen. Thanh boss (cùng ngày): lõi đỏ phẳng (76,0,0), khối vàng (143,117,23) cho máu vừa mất kèm vạch kem (226,228,191) ở chỗ ngăn, chỉ một đường be (150,143,114) dưới đáy và nắp nhỏ hai đầu, không có viền quanh. Không hộp nền và không số (game không có). Thử game 2026-10-26: bản đầu vẽ hộp, đỏ tươi và chữ trong thanh nên bị chê lạc quẻ.

## Texture của game

`assets/Bar.png`, `BarBG.png`, `BarEdge.png`, `BarEdge2.png` là texture thanh máu trích từ HUD gốc của game (cùng bản có trong `tmp/souls_vision/sv_assets/` và `tmp/Elden_Ring_game_tools/Resources/OverlayAssets/`, trùng hash), nhúng vào DLL bằng `include_bytes!` và nạp một lần qua `RenderContext::load_texture` (`src/textures.rs`). Người dùng quyết định dùng lại (2026-10-26).

- `Bar.png` (820x26, xám, 24 hàng thấy được): tô màu bằng vertex color rồi cắt theo UV theo tỉ lệ máu; cũng dùng cho khối máu vừa mất (vàng) và poise (vàng nâu). Màu chia cho độ sáng trung bình 0,86 của texture để ra đúng màu đã đo.
- `BarBG.png`: nền trong mờ, tô tối, đặt trên nền đen.
- `BarEdge.png` (6x36): vạch ngăn ở đầu máu khi có khối vàng.
- `BarEdge2.png` (98x40): vệt sáng ở đầu máu, mờ dần trong lúc giữ khối vàng.
- Đường be dưới đáy và hai nắp vẫn tự vẽ (game không có texture riêng trong bộ này).
- Texture tải lỗi: log `Overlay: textures failed (...)` rồi vẽ thanh phẳng như bản trước.

## Ý tưởng triển khai rút ra từ souls_vision

- **Mục tiêu khoá = một handle trên người chơi**, không phải suy luận theo vị trí. souls_vision đọc `targetHandle` ở ChrIns + `0x6B0` rồi đổi sang ChrIns bằng hàm `GetChrInsFromHandle` của game. Trong `fromsoftware-rs` (rev đang dùng) trường này đã có kiểu: `locked_on_enemy: FieldInsHandle` ở `PlayerIns`, ngay sau `player_menu_ctrl`. Cách ghép theo `lock_on_target_position` của [probe.rs](../../speedmultiplier/src/probe.rs) sai số 8-25 m nên bỏ.
- **Đổi handle sang ChrIns:** dùng slot 3 của vtable ChrSet (`get_chr_ins_by_handle`), mẫu ở `mods/spiritmultiplier/src/warp.rs::resolvable`. Không dùng `WorldChrMan::chr_ins_by_handle` (`rva::get()` panic trên 2.7.2.0).
- **HP:** `chr.modules.data.hp` và `max_hp` (đã có kiểu trong `fromsoftware-rs`; souls_vision đọc `ChrStatModule` + `0x138/0x13C`).
- **Poise (stagger):** `chr.modules.super_armor.sa_durability` và `sa_durability_max` (souls_vision đọc `ChrSuperArmorModule` + `0x10/0x14`). Cần kiểm tra giá trị thật khi đang bị đánh; một số boss không có thanh này nên ẩn hàng Poise.
- **Tên kẻ địch:** từ `npc_param_id` / `npc_id`, tên hiển thị tra qua text của game (souls_vision không làm, cần tự tra).
- **Cạm bẫy souls_vision đã gặp:** crash lúc khởi động khi con trỏ menu còn null, xung đột với MSI Afterburner (cùng hook DXGI), cần chờ trước khi khởi tạo overlay. Ta dùng cổng "đã vào game" của `common::player` và hudhook có sẵn.

## Ý tưởng triển khai rút ra từ kubilaiswf-minimap (cho giai đoạn 2-4)

- Chạy trên `CSTaskGroupIndex::FrameBegin`, **chụp snapshot** trên task game rồi luồng render chỉ đọc snapshot; không chạm đối tượng game từ luồng render. Cùng mô hình với SoulsTeleport (`SHARED`).
- Khoá phiên bản exe và chỉ đọc khi khớp, nếu không thì để yên (tránh crash khi game cập nhật). Repo của ta dùng AOB và kiểm theo từng bản exe thay vì khoá cứng.
- Toạ độ: ô overworld `mAA_XX_ZZ_LL` rộng `256 << LL` mét, vị trí tương đối tâm ô. Dungeon cũ đổi sang toạ độ thế giới qua `WorldMapLegacyConvParam` (chọn điểm tham chiếu gần nhất). Hầm ngầm dùng toạ độ overworld nhưng bản đồ riêng.
- Nền bản đồ lấy từ chính bản cài game lúc chạy (giải mã archive, giải nén texture), **không đóng gói ảnh của game**. Đây là phần nặng nhất, để giai đoạn 4.
- Đồng đội co-op: vị trí khối của người chơi khác chưa đặt, nên suy ra từ chênh lệch vị trí vật lý so với người chơi chính (cùng không gian Havok).
- Ẩn khi: menu, bản đồ thế giới, màn hình tải, fade màn hình, tuỳ chọn khi đánh boss.

## Việc tiếp theo

1. Dựng khung mod (`Cargo.toml`, `build.rs`, ini, `.vscode/tasks.json`, workspace, README gốc) theo `mods/dropmultiplier/`.
2. Task game đọc `locked_on_enemy`, đổi sang ChrIns, log HP/Poise để kiểm trên 2.7.1.0 và 2.7.2.0 (chưa vẽ gì).
3. Overlay ImGui panel mục tiêu, thử trong game rồi mới commit.
