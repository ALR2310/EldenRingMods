# JourneyHud: bản đồ thế giới dưới minimap

**Status 2026-10-27: đọc archive và giải mã tile đã kiểm trên bản cài thật (2.7.2.0, ra ảnh Limgrave đúng); vẽ lên minimap chưa thử trong game. Chỉ overworld (area 60), hầm ngầm và dungeon qua bảng đổi toạ độ; chưa có bản đồ Shadow of the Erdtree (area 61).**

Bản đồ là chính các texture của game, đọc từ thư mục cài lúc chạy: DLL **không đóng gói** ảnh nào của game.

## Cách làm (theo kubilaiswf-minimap, MIT)

Mã trong `src/map/` có 4 file (`reader.rs`, `archive.rs`, `formats.rs`, `tiles.rs`) chép từ `kubilaiswf-minimap` rồi sửa đường dẫn
`use`, kèm chú thích nguồn ở đầu file và `LICENSE-kubilaiswf-minimap` cạnh `Cargo.toml`. `world.rs` và `layer.rs` viết theo ý tưởng
của `game.rs` và `minimap.rs`. Năm khoá RSA công khai trong `src/map/keys/` là khoá của từng archive (cộng đồng đã công bố qua UXM
và SoulsFormats), không phải khoá bí mật của ai.

1. **Archive:** `Data0..3` và `DLC` (`.bhd` mục lục + `.bdt` dữ liệu). Mục lục mã hoá RSA (mỗi khối 256 byte, nâng luỹ thừa số mũ công
   khai), tìm file theo hash đường dẫn (`hash = hash*0x85 + byte` trên đường dẫn viết thường), dữ liệu có đoạn mã hoá AES-128.
2. **File dùng:** `/menu/71_maptile.mtmskbnd.dcx` (mặt nạ mảnh bản đồ cho từng ô), `/menu/71_maptile.tpfbhd` + `.tpfbdt`
   (tile `MENU_MapTile_M00_L0_XX_YY_<flags>.tpf.dcx`, mỗi tile 256x256, 1 pixel = 1 m, M00 = overworld, M01 = hầm ngầm).
3. **Giải nén:** DCX kiểu KRAK bằng Oodle của chính game (`oo2core_6_win64.dll` trong thư mục game, nạp bằng `libloading`), rồi TPF, rồi
   DDS BC7 hoặc BC1 (`texture2ddecoder`).
4. **Cửa sổ 3x3 ô** (768x768 RGBA) quanh người chơi chạy ở luồng nền, ghép thành một texture; luồng render chỉ nạp texture đó.
5. **Vẽ:** quạt 72 hình nêm cắt texture thành đĩa, mỗi đỉnh viền có UV riêng đã xoay theo camera (hướng nhìn là phía trên).
6. **Toạ độ:** ô overworld `m60_XX_ZZ_LL` rộng `256 << LL` mét, vị trí tương đối tâm ô; ô bản đồ (i, j) ứng với `m60_{i+28}_{j+24}`. Dungeon
   đổi qua `WorldMapLegacyConvParam` (điểm tham chiếu gần nhất), hầm ngầm dùng toạ độ overworld nhưng bản đồ riêng (M01).

## Kiểm chứng ngoại tuyến

Test bị bỏ qua mặc định `map::tests::map_window_from_the_installed_game` đọc archive của game thật, dựng cửa sổ quanh ô (13, 11) và ghi PNG:
`ER_MAP_PNG=<file.png> cargo test -p journeyhud --release -- --ignored map_window --nocapture`. Trên 2.7.2.0 mất khoảng 2 giây và ra đúng
bản đồ Limgrave (có icon, nét vẽ tay). Hai test nhỏ khác kiểm công thức đổi toạ độ overworld.

## Khác với minimap gốc

- Không khoá cứng bản game: nếu archive hay bảng tham số không đọc được thì minimap vẫn chạy, chỉ không có nền (log `Map: loader stopped (...)`).
- Luôn hiện toàn bộ bản đồ (chưa theo mảnh bản đồ người chơi đã nhặt).
- Không có grace, beacon, marker bản đồ, vệt đường đi.
- Tầm tối đa 200 m: cửa sổ chỉ rộng 768 m nên phải chừa chỗ để dời cửa sổ trước khi người chơi tới mép.

## Lưu ý kỹ thuật (lấy từ minimap gốc)

- hudhook rò bộ đệm trung gian ở mỗi lần nạp texture, nên chỉ xin cửa sổ mới khi người chơi tới gần mép cửa sổ cũ (`margin = tầm + 20 m`).
- Nạp texture trên màn hình tải có thể treo (hàng đợi GPU chờ game), nên chỉ nạp khi `hud_visible` và không fade (`Radar::in_control`).

## Việc chưa làm

- Thử trong game: bản đồ có khớp vị trí người chơi (so với bản đồ thế giới trong game), xoay đúng khi quay camera, đổi cửa sổ không giật.
- Hầm ngầm (M01) và dungeon qua bảng đổi toạ độ.
- Mảnh bản đồ theo cờ sự kiện, grace, beacon, bản đồ Shadow of the Erdtree.
