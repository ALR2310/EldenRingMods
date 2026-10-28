# JourneyHUD: bản đồ thế giới dưới minimap

**Status 2026-10-27: đọc archive và giải mã tile đã kiểm trên bản cài thật (2.7.2.0, ra ảnh Limgrave đúng); vẽ lên minimap chưa thử trong game. Chỉ overworld (area 60), hầm ngầm và dungeon qua bảng đổi toạ độ; đã nối bản đồ Land of Shadow (gốc toạ độ khớp) (area 61).**

Bản đồ là chính các texture của game, đọc từ thư mục cài lúc chạy: DLL **không đóng gói** ảnh nào của game.

## Cách làm (theo kubilaiswf-minimap, MIT)

Mã trong `src/map/` có 4 file (`reader.rs`, `archive.rs`, `formats.rs`, `tiles.rs`) chép từ `kubilaiswf-minimap` rồi sửa đường dẫn
`use`, kèm chú thích nguồn ở đầu file và `THIRD-PARTY-NOTICES.txt` cạnh `Cargo.toml`. `world.rs` và `layer.rs` viết theo ý tưởng
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
- Có grace (chấm vàng, xem dưới); không có beacon, marker bản đồ, vệt đường đi.
- Tầm tối đa 200 m: cửa sổ chỉ rộng 768 m nên phải chừa chỗ để dời cửa sổ trước khi người chơi tới mép.

## Grace

`map/graces.rs`: bảng grace là `BonfireWarpParam` (chỉ dòng có cờ sự kiện), mỗi grace đổi sang toạ độ bản đồ bằng `world::locate` (cần bảng đổi
toạ độ dungeon đã nạp xong, nên đọc lại mỗi frame cho tới khi được), tìm thấy khi cờ sự kiện của nó bật (`CSEventFlagMan.virtual_memory_flag`).
Mỗi frame trên task game chỉ lấy grace cùng không gian và trong 1,2 lần tầm minimap, ra danh sách khoảng cách tới người chơi. Vẽ là chấm
vàng có quầng sáng nhỏ ở `radar_ui.rs`. Khoá `MinimapGraces` (menu có hàng bật tắt). **Chưa thử trong game.**

## Icon thật của game

Các dấu trên minimap dùng chính icon của bản đồ thế giới trong game, đọc lúc chạy như tile (không đóng gói ảnh nào). `map/sprites.rs`:

- Sprite sheet là một texture trong `/menu/hi/01_common.tpf.dcx` (TPF, tên texture UTF-16, khoảng 200 MB sau khi giải nén), còn vị trí từng sprite nằm ở
  layout `/menu/hi/01_common.sblytbnd.dcx` dạng XML `<TextureAtlas imagePath="SB_MapCursor_02.png"><SubTexture name=... x y width height/>`. Đọc một lần ở luồng nền
  (khoảng 1,3 giây, vài trăm MB tạm thời), cắt từng sprite, rồi `MapLayer` nạp thành texture khi đang chơi bình thường.
- Dùng: `SB_MapCursor_02/MENU_MAP_01_Bonfire` (grace, 156x156, đĩa vàng có quầng sáng), `SB_MapCursor/MENU_MAP_Player_01` (kim vàng của người chơi, chỉ có vòng tròn rỗng) cộng `MENU_MAP_Player_02`
  (hình nhỏ đặt giữa vòng), `MENU_MAP_Coop_01` + `MENU_MAP_Coop_02` (kim hồng của người chơi khác, cũng gồm hai phần), `MENU_MAP_Friend_00` (spirit, kiếm chéo xanh), `MENU_MAP_Enemy_00` (kẻ địch khoá, kiếm chéo đỏ). Cũng có sẵn `MENU_MAP_Raid_01/02`
  (kẻ xâm nhập), `Host`, `Guests`, `Marker` (beacon), `memo_*` và các icon điểm `MENU_MAP_NN` cho lần sau.
- Con trỏ của game gồm hai sprite chồng nhau: kim có vòng tròn rỗng và hình nhỏ ở giữa vòng (thử game lần đầu chỉ vẽ kim nên giữa vòng bị trống). Cả hai quay
  quanh tâm vòng tròn (khoảng 54% chiều cao kim), không quanh giữa ảnh.
- Không đọc được thì log `Map: icons not read (...)` và dùng lại các hình vẽ tay (chấm vàng, mũi tên, chấm đỏ).
- Kiểm ngoại tuyến: test bỏ qua `sprite_sheet_from_the_installed_game` (`ER_MAP_PNG`, `ER_SPRITES="Sheet:Name;..."`) ghi các sprite cạnh nhau ra PNG; đã ra đúng cả 13 sprite thử.

## Địa điểm (di tích, nhà thờ, hang...)

`map/points.rs`: các địa điểm của bản đồ thế giới nằm trong `WorldMapPointParam` (mỗi dòng có `iconId`, vị trí theo block, cờ mở, các dòng text và cờ bật tắt
chúng). Quy tắc hiện theo game (qua `kubilaiswf-minimap`, vốn lấy từ ghi chú EROverlay của soarqin): cờ mở phải bật và, trừ dòng cho phép không text, phải có ít nhất một
text đang hoạt động; text loại 1 đổi sang icon thay thế (`altIconId`); cờ hoàn thành bật thì mờ đi (boss đã hạ). Bỏ các dòng nhãn vùng, icon 0, grace (1, 2, vẽ từ bảng grace riêng)
và các tia sáng dẫn đường (83 đến 85).

Icon của mỗi điểm là sprite `MENU_MAP_<iconId>` (riêng icon 3, nhà thờ, tên là `MENU_MAP_Church`) trong ba sheet `SB_MapCursor`, `SB_MapCursor_02`, `SB_MapCursor_03_dlc` (cho Shadow of the Erdtree). `sprites::load_map_icons` đọc
cả 98 sprite một lần ở luồng nền (khoảng 1,4 giây), `MapLayer` thu nhỏ (tối đa 96 px) và xếp thành **một atlas 1024x1024** chỉ nạp một lần (hudhook rò bộ đệm mỗi lần nạp).
Vẽ ở kích thước 0,11 lần sprite, không cao quá 30 px. Dòng log `Map: N point(s)` và `Map: N point icon(s) uploaded`. Khoá `MinimapPoints`. **Chưa thử trong game.**

Kiểm ngoại tuyến: test bỏ qua `map_icons_from_the_installed_game` ghi cả 97 icon ra PNG lưới. Còn thiếu: beacon (xem dưới).

## Beacon

`map/beacons.rs`. Beacon người chơi đặt không có kiểu trong `fromsoftware-rs`. Game giữ chúng trong `CSMenuMarkersSaveData`:

```text
CSMenuMan -> +0x80 -> +0x250 (view bản đồ thế giới) -> +0x338 (danh sách marker ở view +0x300, con trỏ dữ liệu ở +0x38 của nó)
  -> +0x08 mảng slot (mỗi slot 16 byte), +0x10 số slot
slot: id i32 (âm = trống), x f32, y f32 (toạ độ màn hình bản đồ của game, Y xuống, 41 ô 256 px), map u8 (0 overworld, 1 hầm ngầm), icon u8
```

**Kiểm trong IDA:** hàm thêm marker (chuỗi `CSMenuMarkersSaveData.cpp`) ghi đúng các trường này và nơi gọi nó giữ danh sách ở view +0x300 trên **cả bốn bản exe**
(2.6.2.0 `0x818ED0`, 2.7.0.0 và 2.7.1.0 `0x819D50`, 2.7.2.0 `0x81A200`): mảng slot ở +0x08, số slot ở +0x10, phần tử 16 byte, bộ đếm id ở +0x48, con trỏ dữ liệu ở +0x38 của danh sách
(`a1[7]`), danh sách ở `a1 + 768`. **Chưa kiểm riêng** phần đầu chuỗi (`CSMenuMan +0x80 +0x250`): nó lấy từ minimap gốc, tác giả đã thử trong game trên 2.7.2.0.

**An toàn:** mọi con trỏ đọc bằng `ReadProcessMemory` (lỗi thì trả `None` thay vì crash), và kết quả bị bỏ hết nếu có slot nào không giống beacon (số slot 0 hoặc quá 10, id quá lớn, map
khác 0 và 1, vị trí ngoài bản đồ hoặc không hữu hạn). Bản exe nào chuỗi sai thì chỉ không có beacon.

Chuyển toạ độ: `from_game_map` đổi (x, y) của màn hình bản đồ sang mét trên lưới overworld, ô (i, j) của bản đồ ứng với `m60_{i+28}_{j+24}`. Số slot (1 đến 5...) là số game hiển thị cạnh beacon;
chưa vẽ số. Icon là `MENU_MAP_Marker` (64x324, tia sáng xanh nhạt, đầu mũi tên ở dưới là vị trí): vẽ cao 36 px, mũi dưới đặt đúng chỗ; beacon ngoài tầm ghim vào viền như người chơi khác.
Khoá `MinimapBeacons`. **Chưa thử trong game.**

## Thứ tự lớp và cỡ icon

Như game (người dùng, 2026-10-27), từ dưới lên: **địa điểm < NPC < grace < beacon < spirit, đồng đội, kẻ địch khoá < người chơi**. NPC là các icon 80, 86, 87, 88 (người, dấu
triệu hồi, kẻ xâm nhập), còn lại là địa điểm. Cỡ theo sprite của game: địa điểm 0,2 lần (to nhất, nhà thờ cao hơn đĩa grace nên vẫn thấy khi grace nằm đè lên), NPC 0,17 lần, grace 0,15 lần,
không icon điểm nào quá 42 px. Cỡ sau thử game 2026-10-27: gấp đôi bản đầu rồi bớt 30% (0,28 / 0,24 / 0,21 lần sprite), rồi người dùng thấy icon tàn tích còn to nên chốt **một hệ số chung cho địa điểm, NPC và grace** (`ICON_SCALE`, thay cho ba hệ số riêng; 0,18 rồi người dùng nâng lên 0,20); icon gốc 156 px thành khoảng 31 px, người dùng thấy vậy là vừa. Bản trước dời grace xuống góc phải khi che địa điểm; bỏ vì game không làm vậy và lớp mới đã giải quyết.

## Điểm không hiện (đã tra, 2026-10-27)

Đã từng có khoá config `MinimapDebug` ghi log các dòng bản đồ gần người chơi; đã gỡ sau khi tra xong. Kết quả thử game ở nhà thờ Elleh (bảng điểm 472 dòng): nhà thờ là dòng icon 3 (cờ mở bật,
text hoạt động) nên chỉ thiếu sprite `MENU_MAP_03`, sửa bằng tên `MENU_MAP_Church`. Quy đổi toạ độ đúng (người chơi `m60_42_36` cục bộ (-63, 90), nhà thờ cách 25 m, grace 20 m). NPC khung hình người
đội mũ trùm không có dòng nào gần: 71 dòng icon 80 nhưng gần nhất cách 208 m, icon 87 cách 738 m, icon 86 không có dòng nào. Ở chỗ khác NPC hiện bình thường (người dùng), nên chỗ Elleh có thể là
trường hợp riêng (NPC không đi qua `WorldMapPointParam` hoặc là dấu của người chơi); chưa kết luận.

## Vẽ cho dễ nhìn

Làm tối dần vòng ngoài của đĩa (4 vòng mờ) để viền ăn khớp với bản đồ, và có chấm nền tối sau mỗi chữ N/E/S/W cho dễ đọc trên nền bản đồ sáng.

## Khi nào bắt đầu đọc

Luồng đọc archive (bản đồ, icon, địa điểm) chỉ khởi động **khi người chơi đã vào thế giới** (frame đầu tiên có `in_control`), không phải lúc game mới chạy. Lý do: nó cần vài trăm MB và vài giây
đĩa và CPU, không nên tranh với game đang khởi động. Thử game 2026-10-27 có một lần crash ở engine khoảng 3 giây sau khi móc DX12, trước khi overlay kịp khởi tạo (AV đọc `0x30` trong getter
`sub_141E937F0` do `sub_141E8B8B0`, hàm liên quan `CGShaderResourceView`, trên luồng không có khung nào của mod; cùng họ với crash 2026-10-04 khi chưa có JourneyHUD, xem `docs/crash_dump_analysis.md`).
Chưa chứng minh mod liên quan; chuyển sang đọc muộn để loại một nguồn nhiễu.

## Lưu ý kỹ thuật (lấy từ minimap gốc)

- hudhook rò bộ đệm trung gian ở mỗi lần nạp texture, nên chỉ xin cửa sổ mới khi người chơi tới gần mép cửa sổ cũ (`margin = tầm + 20 m`).
- Nạp texture trên màn hình tải có thể treo (hàng đợi GPU chờ game), nên chỉ nạp khi `hud_visible` và không fade (`Radar::in_control`).

## Map for Goblins

Lớp `Minimap.Layers.MapForGoblins`, code ở `src/map/mfg.rs`. Mod Map for Goblins DLL (từ 2.1.6) xuất hàm `MfgGetMapAPI` (header `sdk/mfg_map_api.h` của nó, ABI 1) để mod khác đọc đúng những marker nó đang vẽ; cách đọc và giải mã icon theo kubilaiswf-minimap (MIT, `mfg.rs`).

- **Tìm API:** `GetModuleHandleW("MapForGoblins.dll")` rồi `MfgGetMapAPI(1)`, kiểm cỡ struct. Chưa thấy thì hỏi lại mỗi 2 giây (thứ tự nạp DLL không cố định); không có DLL thì lớp không làm gì và không tốn gì.
- **Marker:** `view_points(layer)` với layer 0 = overworld, 1 = hầm ngầm (layer DLC bỏ qua, chưa có bản đồ). Mỗi điểm có `raw_map_id` (area, grid X, grid Z) và `x, z` trong ô đó, đổi qua `world::locate` như hàng `WorldMapPointParam`. Danh sách dựng lại tối đa 3 lần mỗi giây; vòng highlight (bit 62 của id) bị bỏ, huy hiệu hoàn thành (bit 63) vẽ như icon thường. Danh sách đã gồm việc người chơi tắt loại trong menu của nó và vật phẩm đã nhặt, nên không phải kiểm cờ ở đây.
- **Icon:** `get_icon` trả thân thẻ SWF DefineBitsLossless2 (định dạng 3 bảng màu hoặc 5 ARGB, nhân alpha sẵn); giải nén zlib rồi chia lại alpha. Giải mã trên luồng riêng khi API xuất hiện lần đầu, xếp lưới ô 96 px (16 cột) thành một texture; icon quá 96 px bị bỏ. Không đóng gói ảnh nào của mod đó.
- **Vẽ:** icon 96 px vẽ khoảng 29 px ở 1080p (hệ số 0,30, `GOBLIN_SCALE`; bản đầu 1/6 thì người dùng thấy nhỏ hơn icon của game), dưới grace và trên các địa điểm của game; ngoài tầm thì không vẽ (không ghim viền).
- **Chưa thử trong game.** Bản Map for Goblins DLL cài trên máy là 2.1.6 (có API); mod này chưa nằm trong `external_dlls` của `config_eldenring.toml` ở thư mục game nên cần bật nó mới thấy marker.

## Bản đồ Land of Shadow (DLC, 2026-10-27)

Bản đồ DLC nằm trong cùng archive `/menu/71_maptile.tpfbhd`: `M10` (Land of Shadow, lưới 41x41 ô 256 px, cùng định dạng tile như `M00`) và `M11` (một vùng nhỏ 16x10 ô, bản đồ hầm ngầm của DLC, chưa dùng). Tile giải mã được bằng đúng bộ đọc hiện có (đã ghép thử cả 41x41 ô `M10` thành ảnh để xem: một đảo lớn ở nửa dưới, Scadutree ở phía trên, viền tối quanh).

- **Cách nối vào:** `MapId::Shadow` (`M10`), `world::locate` trả `Some(Shadow)` cho block area 61 (và đích area 61 của bảng đổi toạ độ dungeon), điểm đánh dấu theo `disp_mask02` của `WorldMapPointParam`, beacon theo bản đồ 2 và marker Map for Goblins theo layer 2. Mảnh bản đồ của DLC chưa đọc: `Shadow` luôn hiện đủ (`FullMap` không áp dụng).
- **`M11`:** DLC không có bản đồ hầm ngầm. Bảng tên mask trong exe (`menu:/71_MapTile.mtmskbnd`, 8 ô) chỉ liệt kê `M00`, `M01`, `M10`, `M03` đến `M07`; `M11` có trong archive (1521 ô, mọi mask bằng 0, không phải bản đồ theo mảnh) nhưng game không nạp. Người dùng nhớ DLC không có khu ngầm, khớp.
- **Mảnh bản đồ DLC:** mask của `M10` dùng 5 bit (0 đến 4; `M00` 21 bit, `M01` 5 bit), tức 5 mảnh. Dòng log `Map: piece rows 0-20, 70-70, 100-105, 1000-1004, 1070-1070` (2026-10-28, 2.7.2.0) cho thấy hàng của DLC là **1000 đến 1004** (5 hàng, khớp 5 bit), không phải 200 đến 204 như đoán lúc đầu (khi đó `FullMap = false` ở DLC tối cả vùng đã nhặt mảnh). Hàng 70 và 1070 chưa rõ (không thuộc 5 bit). `FullMap = false` ở DLC dùng gốc 1000.
- **Gốc toạ độ:** ô (0, 0) của `M10` ứng với tile m61 (28, 24), cùng khung với bản đồ cơ sở (m60). Đây là giả định đầu tiên và người dùng thử trong game thấy minimap DLC khớp, nên không cần chỉnh (đảo trên ảnh chiếm ô x 12..31, z 10..33, tức tile m61 khoảng x 40..59, z 34..57).
- **`M11`:** chưa dùng; cần biết khối nào của game chọn nó và toạ độ riêng của nó.

## Việc chưa làm

- Thử trong game: bản đồ có khớp vị trí người chơi (so với bản đồ thế giới trong game), xoay đúng khi quay camera, đổi cửa sổ không giật.
- Hầm ngầm (M01) và dungeon qua bảng đổi toạ độ.
- Mảnh bản đồ theo cờ sự kiện, grace, beacon, bản đồ Shadow of the Erdtree.
