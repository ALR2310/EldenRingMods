# JourneyHud: minimap, lớp chấm (radar)

**Status 2026-10-27: bản đầu đã có code, chưa thử trong game. Chưa có nền bản đồ (giai đoạn 4).**

Tên người dùng thấy là **Minimap**; "radar" là lớp chấm (đồng đội, spirit, kẻ địch, hướng) trong code (`radar*.rs`). Nền bản đồ sẽ là lớp
riêng (`map`) vẽ dưới các chấm, nên tên tệp `radar*` vẫn đúng sau giai đoạn 4.

Một ô minimap tròn ở góc trên phải, trên cùng cột; thanh mục tiêu nằm ngay dưới nó (nhích theo kích thước minimap): mũi tên người chơi ở giữa, xoay theo camera (hướng camera nhìn là
phía trên), chấm cho đồng đội, spirit của mình và kẻ địch đang khoá, chữ N/E/S/W trên viền. Code: `src/radar.rs` (đọc game),
`src/radar_ui.rs` (vẽ). Tự vẽ bằng hình ImGui, không dùng asset nào.

## Học từ kubilaiswf-minimap (MIT)

Đã đọc `tmp/kubilaiswf-minimap/src/game.rs` và `minimap.rs`. Những gì lấy làm ý tưởng:

- **Chụp snapshot trên task game, luồng render chỉ đọc bản sao** (`game::LATEST`). Cùng mô hình với panel mục tiêu.
- **Hướng camera:** `CSCamera::instance().pers_cam_1.matrix` (ma trận nhìn): `camera_right = (m.0.0, m.0.2)`,
  `camera_forward = (m.2.0, m.2.2)` trên mặt đất. Quy ra màn hình: `[dx*rx + dz*rz, -(dx*fx + dz*fz)]` (forward là lên trên).
- **Hướng nhân vật:** `chr_ctrl.physics_model_matrix`, nhân vật nhìn theo trục **-Z** của model: `facing = (-m.2.0, -m.2.2)`.
- **Vị trí tương đối, không cần toạ độ thế giới:** ma trận vật lý của mọi nhân vật chung không gian Havok, nên
  chỉ cần hiệu `m.3 - player.m.3`. Chỉ nền bản đồ mới cần đổi block sang toạ độ thế giới (`locate`,
  `WorldMapLegacyConvParam`), phần đó để giai đoạn 4.
- **Đồng đội Seamless:** vị trí block của họ không được đặt, nhưng vị trí vật lý thì sống, nên đặt theo hiệu so với người
  chơi chính, qua `world_chr_man.player_chr_set` bỏ chính mình.
- **Mũi tên mượt:** hướng người chơi đổi tức thì nên làm mượt theo góc (có `wrap_angle`); đồng đội ngoài tầm thì ghim
  vào viền và mờ đi.
- **Khi nào ẩn:** `CSFeManImp.hud_state != Default` (menu đang đổi HUD) hoặc `CSFade` đang phủ (màn hình tải, warp, tối
  đa các `fade_plates[i].current_color.a`) thì mờ dần rồi ẩn, vì menu và bản đồ thế giới vẽ ngay chỗ đặt radar.
- **hudhook:** nạp texture ở thời điểm lạ có thể làm treo trên màn hình tải, và mỗi lần nạp lại rò bộ đệm; JourneyHud nạp
  texture đúng một lần lúc khởi tạo và radar không dùng texture.

Không lấy: toàn bộ phần bản đồ (giải mã archive của game, ghép tile, `maptiles.rs`, `archive.rs`, `formats.rs`), icon của Map
for Goblins (`mfg.rs`), khung và mũi tên bằng ảnh (`assets/`).

## Khác với minimap

- Radar không có nền bản đồ; ô tròn chỉ có nền tối mờ, hai vòng chỉ dẫn và viền be (cùng màu đường dưới thanh máu).
- Spirit là chấm của riêng JourneyHud (lấy từ `summon_buddy_chr_set`, gồm cả spirit của người khác nếu có).
- Chỉ đọc các singleton qua `instance()` theo tên (không `rva::get()`).

## Ghi chú và việc chưa kiểm chứng

- Offset của `CSCamera`, `CSFade`, `CSFeManImp` và `physics_model_matrix` lấy từ `fromsoftware-rs` (rev của workspace);
  minimap xác nhận cách đọc ma trận trong game nhưng ở rev khác, nên cần thử: mũi tên có chỉ đúng hướng, camera quay
  thì radar xoay đúng chiều, đồng đội và spirit nằm đúng chỗ.
- Lọc "cùng khu vực": chỉ lấy nhân vật cùng `block_id.area()` và trong 3 lần tầm radar, vì ở khu khác toạ độ chỉ
  trông gần.
- Chưa có: kẻ địch xung quanh (ngoài mục tiêu khoá), nền bản đồ, thu phóng bằng phím (tầm chỉnh trong menu/toml).
