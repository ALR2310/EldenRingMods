# Trang menu mới (tab riêng) thay vì dùng lại Game Options

**Status 2026-10-15: nghiên cứu tĩnh (IDA, 2.7.1.0) + phân tích NativeTweaks r96, CHƯA thử trong game. Kết luận: thêm một tab/trang mới vào menu System là khả thi
(NativeTweaks đã làm), nhưng đòi hỏi vá GFX `02_040` + hook; trang có cuộn thật cần dùng `CS::GridControl` (cuộn ảo của màn gán phím) và dựng danh sách C++ tuỳ biến, chưa có
mẫu làm sẵn. Giữ ERNativeUI cho thông số thường; đây là hướng nghiên cứu tiếp.**

Bối cảnh: [ingame_menu.md](ingame_menu.md), [ingame_menu_scroll.md](ingame_menu_scroll.md). Các RVA bên dưới là `eldenring.exe` 2.7.1.0 (imagebase 0x140000000),
đọc từ decompile; phần nào chưa kiểm trong game thì ghi rõ.

## Menu System được dựng thế nào

- Mọi tab dùng chung movie `02_040_OptionSetting` (mở ở `0x809660`; nhánh Trial dùng `02_041`). Mỗi trang là một clip con của `WindowList`.
- Lớp trên cùng `CS::OptionSettingTopDialog` (ctor `0x9672C0`, vtable `0x2B16B48`); thanh tab `_SettingTabControl` (vtable `0x2B16AB8`, input `0x9680D0`)
  gắn vào clip `TabList`; danh sách tab là `BasicViewItemList<MenuOptionCategory, 10>`.
- Mỗi tab là một bản ghi `MenuOptionCategory` (136 byte, ctor `0x86A0A0`, copy `0x967230`: text id, category id, cờ). Thêm vào danh sách: `0x968CF0`.
  Tab chỉ chuyển bằng L1/R1 hoặc chuột, không cuộn.
- Category id → trang: switch `0x93D730` (case 0..9) → hàm bọc → `0x95FD30` (dựng `OptionSettingDialog`, ctor `0x94D710`, rồi gọi materializer).

| id | clip | materializer | tab |
|---|---|---|---|
| 0 | ControllSetting | 0x959DF0 | Game Options |
| 1 | CameraSetting | 0x959320 | Camera |
| 2 | EnvironmentSetting | 0x95D540 | Display |
| 3 | AudioSetting | 0x959090 | Sound |
| 4 | BrightnessSetting | 0x95D200 | không nằm trong danh sách tab |
| 5 | NetworkSetting | 0x95AD00 | Network |
| 6 | PadSetting | 0x953560 | lớp riêng (0x968E30) |
| 7 | PCMouseKey | 0x95CB30 | Keyboard/Mouse |
| 8 | PCGraphic | 0x95C050 | Graphics |
| 9 | GameEnd | 0x959AB0 | chỉ khi đang trong game |

Bảng 15 tên clip `WindowList` nằm ở `0x143B3BB50`; vòng lặp `0x95FD30` ẩn các clip không được chọn.

## Ba giới hạn cứng khi thêm tab
1. Vector tab tối đa **10** (`0x968CF0` báo lỗi khi `count + 1 > 10`). Game dựng tối đa 9 tab (id 4 không được thêm), nên còn đúng 1 chỗ.
2. `CompositeOptionSettingDialog` (ctor `0x93C090`) có mảng 10 con trỏ trang, chỉ số theo category id.
3. Switch `0x93D730` chỉ có case 0..9, mặc định không làm gì.

## NativeTweaks r96 (mod bên thứ ba, đóng)
- Tab "NativeTweaks"/"Mod Settings" do **cả GFX lẫn một bản ERNativeUI.dll đã vá (fork)**: fork nối category thứ 10 sau Graphics, chuyển callback brightness
  (category 4) sang callback Game Options `0x959DF0` với vỏ trang rỗng, rồi bơm hàng. Điểm hook (RVA lấy từ bảng cấu hình của fork): nối category `0x968CF0`,
  builder `0x807E10`, destructor `0x86A190`, hàng `0x95D200`, tuyến `0x93CB22/0x93CB34`.
- `NativeTweaks.dll` chỉ gọi API host (`ERUI_GetApi`), không hook UI. Trang của nó **không cuộn**: 6 hàng vật lý + Next/Previous, nhóm tuỳ chọn là trang con
  (~15 hàng) và INI chung bị chặn 128 hàng/trang.
- GFX (95 KB) so với bản ERNativeUI (45 KB): thêm `Item_0_9` vào `TabList`/`BackTabList`, nén khoảng cách tab, thêm một `PlaceObject3` đặt bản sao cửa sổ tên
  `ERModSettingsHub` trong `WindowList`; **không** nâng số hàng (sprite 127 = 7, sprite 148 = 6). Xung đột với mọi mod cùng thay `02_040_optionsetting.gfx`.
- Đọc INI: quét module đang nạp, mỗi INI cạnh DLL (trừ `NativeTweaks.ini`, `ERNativeUI.ini`, `mod_loader_config.ini`, `eldenmodloader.ini`, `ERModSettingsHub.ini`)
  thành các hàng sửa chữ; lưu `.bak`; nạp lại qua export `<Tên>_RequestReloadV1` / `_GetReloadStatusV1` (hỏi mỗi 20 ms, tối đa 5 s).

## Cuộn thật: `CS::GridControl`
- Cơ chế cuộn của màn gán phím là `CS::GridControl` (vtable `0x2A94438`, ctor `0x7368E0`), dùng chung với `TabList` và `GenericListSelectDialog`
  (ctor `0x78D2F0`). Gắn vào clip có `Item_%d_%d` (`0x737E10`), `TopItem_%d`/`BottomItem_%d`, cộng `ScrollBarV`.
- Bind danh sách vào grid: `0x93E300` (key config) / `0x967140` (tab); đặt số mục `0x739C10`; chọn mục `0x739B90`/`0x73C240`.
- Cuộn ảo: lambda `0x940D10 → 0x941800 → 0x93FEB0` duyệt các vị trí đang hiển thị, đổi vị trí → chỉ số dữ liệu (`0x7382F0`); chỉ số ≥ `count()` thì ẩn vị trí
  (`0x734190`), còn không gọi `GetItem(i)->vfunc+16` để vẽ mục vào vị trí đó.
- Danh sách mục là `MenuViewItemList<T>` (vtable gốc `0x2AA1618`); họ `GenericItemSelectDialog<T>` dùng lại cho nhiều loại mục.
- Không thấy cách "thêm hàng" vào `KeyConfigDialog` đã có: danh sách `CSMenuKeySettingList` (vtable `0x2AD6E70`, mục 80 byte) bị dựng cứng trong ctor
  `0x93E430`/`0x941B80`; chỉ vẽ hàng gán phím (không có slider). Muốn trang riêng thì phải tự dựng lớp danh sách C++ với vtable giả (nguy cơ crash) hoặc hook `0x869580`/`0x869980`.
- Đính chính [ingame_menu_scroll.md](ingame_menu_scroll.md): `0x869580` là dispatcher theo byte `[a2+8]` (0 → `0x869980`, 1 → `0x8697C0`), không phải builder.

## Cấu trúc GFX gốc (đo 2026-10-15 từ bản export vanilla, `D:\MyProjects\EldenRingVietHoa\menu\`)

`win/02_040_optionsetting.gfx` (bản PC, 44 016 B; bản gốc của ERNativeUI và NativeTweaks) và `02_160_keyconfiguration.gfx` (55 600 B). Script: `tree.py`/`po.py` trong scratchpad
(parser SWF/GFX tối thiểu: tag DefineSprite/PlaceObject, cây clip theo tên instance).

**`win/02_040`**
- `TabList`/`BackTabList`: **9** vị trí `Item_0_0..Item_0_8` (cách nhau 90 px) + `LeftArrow`/`RightArrow`.
- `WindowList` có 10 clip trang, mỗi clip là một sprite chứa các hàng `Item_N_0` (bản vanilla): Camera 7 hàng (sprite 127), Controll/Game Options **6** (148),
  Network 7 + `UgcRestricted` (157), Audio và Environment **dùng chung sprite 158, 12 hàng**, PCMouseKey 7 (159), PCGraphic 7 (160), Brightness 2 (147), Pad 1 (165),
  GameEnd 1 (138). Tức một clip trang có thể là bản sao đặt tên khác của sprite có sẵn (NativeTweaks dùng cách này, đặt thêm sprite 148 với tên `ERModSettingsHub`).
- Mỗi hàng `Item_N_0` chứa `Widgets` với 5 widget (`Button`, `TextInput`, `Slider`, `ComboBox`, `DropdownList`, mỗi cái có `Text_0`, `Caption`, `Cursor`...).

**`02_160` (màn gán phím)**
- Gốc `KeySetting` (sprite 198) có `ItemList` (190) gồm **11 vị trí × 2 cột**: `Item_N_0` (sprite 189, cột trái, x≈7) và `Item_N_1` (sprite 182, cột phải, x≈892), N = 0..10,
  bước dọc ≈ 64 px (y 35 … 673); thêm `ScrollBarV` (x 33) và `ScrollBarV2` (x 379).
- Mỗi hàng chỉ có **chữ tĩnh + `Conflict` + `HitArea` + `Cursor`**; **không có Slider/ComboBox/TextInput** như hàng của `02_040`.

**Hệ quả**
- Hướng 1 (hook danh sách màn gán phím) chỉ cho hàng hai cột "nhãn + chữ" chọn được; slider, công tắc hay ô gõ phải tự mô phỏng (chọn hàng rồi mở bàn phím/đổi giá trị bằng nút). Không có widget native.
- Muốn trang cuộn có hàng widget kiểu Options thì phải **tạo movie mới** ghép: cuộn ảo `GridControl` (cấu trúc `ItemList` + `ScrollBarV` của `02_160`) với hàng `Widgets` của `02_040`. Việc này đòi
  chỉnh SWF bằng script (hoặc Flash/JPEXS), chưa thử.
- Tab mới: `TabList` chỉ có 9 vị trí nên phải thêm `Item_0_9` và dịch các tab khác (như NativeTweaks); `Brightness` (id 4) là clip trang sẵn có nhưng không có tab.

## `GridControl` và hàng widget (IDA 2.7.1.0, 2026-10-15, chưa thử trong game)

**`GridControl` không biết loại widget.** Vtable chỉ có 4 slot: dtor `0x735100`, `0x737570`, update mỗi frame `0x73A140` (nhận input), init/bind `0x738AB0`. Nó dò clip `Item_%d_%d`
(tối đa 32 cột x 64 hàng, qua `LayoutProxy` tại `+1688`), `ScrollGuide_UpArrow/DownArrow`, `ScrollBarV` (`+424`, bind `0x74E410`), và chỉ giữ số mục (`+208`), chỉ số chọn (`+212`),
bù cuộn theo hàng (`+840`), hàm kiểm "mục chọn được" (`+1752`) và callback làm mới hiển thị `(GridIterator begin, end)` (`+1816`, gọi từ `0x73D0D0`). Việc vẽ hàng hoàn toàn do callback.
Phím: `0x73BF10`; chuột/con lăn/track: `0x73B410`; cuộn theo hàng `0x73C5B0`; chọn `0x73C240`/`0x73C320`; ẩn/hiện `Cursor` `0x7399F0`; vị trí → clip `0x737C80`.

**Phát hiện đổi bài toán: trang Options (`02_040`) đã là `GridControl` + hàng `Widgets`.** Chuỗi kế thừa `OptionSettingDialog` (`0x94D710`) → `PropertyEditDialog` (`0x91CC20`) →
`GenericItemSelectDialog<EditProperty>` → `GenericListSelectDialog` (`0x78D2F0`), nên mỗi trang Options có một `GridControl` tại `+2616`. Builder hàng nhận clip hàng, ẩn 4 widget còn lại
và dựng controller trên `Widgets/<loại>` (`0x949DC0` slider, `0x9499A0` công tắc/combo, `0x949560` combo có danh sách; thêm vào trang `0x94A590`/`0x94A140`). `SliderCtrl` (`0x74EBC0`) chỉ tra tên
con (`Anchor, LeftArrow, RightArrow, Bar, HitArea, Base, Text_0, Text_1`), không có lớp cha cứng nào ngoài việc clip có đúng cấu trúc tên.
Ràng buộc: controller **buộc vĩnh viễn** vào clip lúc dựng; danh sách thuộc tính là `BasicViewItemList<EditProperty,16>` (tối đa 16, vector 136 byte tại `page+4712`, kiểm ở `0x928630`);
slider ghi vào con trỏ `u8`. **Suy luận (chưa kiểm):** trang Options là ánh xạ 1:1 vị trí-mục, không phải cuộn ảo, vì controller không gắn lại clip. Chưa biết `PropertyEditDialog`
có đăng ký callback làm mới `+1816` không.

**Nơi khác dùng `GridControl`** (ctor `0x7368E0`): `ColorPaletteControl` `0x77E3D0`, `ColorSliderControl` `0x780800` (3 hàng slider màu), `BloodMessageTopDialog` `0x8983C0`, `ChrMakeItemSelectDialog`
`0x8A9930`, `EquipDialog` `0x8DB500`, `Gaitem*TabViewParts` `0x8E1CA0/0x8E2FC0/0x8EBF30`, `GestureEquipDialog` `0x907270/0x909660`, các `PropertyComboBoxController` (`0x94C520`, `0x919660...0x91AAB0`),
`SignRankMultiPlayViewParts` `0x9347D0`, `OptionSettingTopDialog` `0x9672C0/0x969DE0`, `WorldMapBookmark/MemoSelect/SignPuddle/WarpSelect` `0x9BA510/0x9D3410/0x9DDB70/0x9E4410`.
Không nơi nào ngoài Options có hàng slider cuộn ảo.

**Item của màn gán phím (`CSMenuKeySetting`, 80 byte):** vfunc+16 `0x245CE0` → vẽ thật `0x8684D0(item, rowProxy)`: hiện hàng `0x734190`, xám `0x734030`, `Text_0` từ id `GR_MenuText` (`0x7617C0`)
rồi đặt bằng **`0x74AE50(handle, const wchar_t*)`**, `Text_1` là tên phím (`0x868DD0`). Vậy chữ thô tuỳ ý đặt được (không cần id chuỗi). Danh sách dựng bởi `0x869580 → 0x869980/0x8697C0`
từ bảng param id 169 (stride 24 byte). Chưa biết nơi xử lý bấm chọn của `KeyConfigDialog`.

**Hệ quả:**
- Hướng 1 (hook màn gán phím): đặt chữ tuỳ ý làm được (detour `0x8684D0`, gọi bản gốc rồi `0x74AE50` ghi đè `Text_0/Text_1`); cuộn và chọn có sẵn; bắt bấm và đổi giá trị cần tìm
  thêm (callback chọn `+72/+136` hoặc hook `0x73C240`). Không có widget native.
- Hướng widget: đáng thử hơn dự kiến. Gọi trực tiếp `0x94A590`/`0x94A140` trên một trang có sẵn nhiều hàng (sprite 158, 12 hàng) hoặc chèn sau materializer (đúng cách ERNativeUI). Chặn: tối đa 16 thuộc tính,
  slider `u8`, không cuộn ảo. Movie ghép mới có lẽ không cần trừ khi muốn cuộn thật (khi đó phải tạo lại controller/gắn lại clip lúc cuộn, chưa có mẫu trong game).

**Thử nghiệm nhỏ nhất, rủi ro thấp:** (1) detour `0x8684D0` trong màn gán phím, đổi `Text_0` một hàng thành chuỗi thô (xác nhận `0x74AE50` và handle); (2) ghi log ở `0x73C240` và `0x73D0D0` khi di chuyển trong trang
Options để xem bù cuộn (`+840`) và có callback làm mới hay không, rồi thử nhét 16 thuộc tính vào trang 12 hàng xem 4 hàng cuối bị ẩn hay chọn được.

## Kết quả probe v1 (`scratch/menuprobe`, log 2026-10-15 12:24-12:26, game 2.7.1.0)

Cả 5 hook cài được (đúng byte), game không crash, không đổi hành vi. Mọi grid đều có vtable `exe+0x2A94438` (`GridControl` gốc), 50 grid xuất hiện: phần lớn nhỏ (2-5 mục = các hộp
combo, mỗi `PropertyComboBoxController` có một grid riêng).

**Màn gán phím = cuộn ảo thật, đo được:**
- Hai grid (count 52 và 58, tức 26 và 29 hàng x 2 cột; `index` đi bước 2 vì mỗi hàng có 2 mục) với `scroll` (`+840`) chạy 0..15 và 0..18. `scroll` là **chỉ số hàng đầu tiên đang hiển thị**.
- Quy tắc: cửa sổ 11 hàng; `scroll` bắt đầu tăng khi chọn tới hàng 9 (chừa 1 hàng đệm), giữ nguyên khi chọn đi ngược trong cửa sổ, về 0 khi chọn quay lại đầu.
- Mỗi lần cuộn gọi callback làm mới (`0x73D0D0`) và vẽ lại **toàn bộ vị trí** (~22 lần `0x8684D0` cho 11 hàng x 2 cột mỗi sự kiện): vị trí được gán lại dữ liệu, đúng cơ chế cuộn ảo.
- `grid_select(this, cũ, mới, cờ)`: `rdx` = chỉ số cũ, `r8d` = chỉ số mới.
- Item màn gán phím có vtable `exe+0x2AD6A98` (80 byte mỗi mục, liên tiếp).

**Trang Options:** mọi trang đều đẩy thuộc tính qua `0x928630` với `count_before` từ 0 (trang nhiều nhất trong lần chạy: 14 thuộc tính, grid 14 mục, `scroll` luôn 0, đi hết chỉ số 0..13). Chưa có trang nào có
nhiều mục hơn số vị trí nên chưa biết trang Options có cuộn được không. Một grid khác (8 mục) có `scroll` 0..3 (cửa sổ ~5 hàng), chưa xác định là màn nào.

**Đã trả lời bằng probe v2** (mục sau).

## Callback làm mới của trang Options: cuộn ảo KHÔNG áp dụng cho widget (IDA + probe v2, 2026-10-15)

Probe v2 (434 dòng, 52 grid): `GridControl+216` là số **cột**, `+220` là số **hàng** (màn gán phím 2x11, TabList 9x1, trang Options 1x15 / 1x12 / 1x7). **Mọi** grid có callback làm mới.
Một grid 1 cột x 5 hàng (count 8) cũng cuộn (`scroll` 0..3), chưa nhận ra là màn nào. Trong lần chạy mọi trang Options có `count <= số hàng` (ERNativeUI tự phân trang), nên log không cho thấy trường hợp `count > rows`.

Đọc mã (suy luận từ decompile, chưa thử trong game):
- `GridControl+0x6E0` là `std::function<void(GridIterator, GridIterator)>` F1 (`_Ptr` ở `+0x718`, hàm gọi `0x73D0D0`); `+0x720` là F2 `std::function<void(SceneObjProxy&, int)>` "áp mục vào hàng" (`_Ptr` `+0x758`);
  `+0x90` là hàm "chọn được" `bool(int)`. Cả ba gán trong hàm Init mẫu (Options `0x9761E0`, gán phím `0x93E300`, combo `0x94B120/210/300`...), Init còn gọi `0x739C10(grid, count)` rồi `0x73D0D0`.
- Trang Options được "chốt" bởi `0x977A60(page, chỉ_số_đầu)` (gọi từ `0x91B010, 0x929A60, 0x9540F0, 0x9542D0, 0x957DE0, 0x96EC40, 0x9CA630`, sau khi thêm hết thuộc tính): gán F1 (`0x975E80`, thân `0x977650 -> 0x9768E0`,
  cùng khuôn `0x93FEB0` của gán phím), hàm chọn được `0x975F40`, Init `0x9761E0` với F2 `0x975DC0`. Grid ở `page+2616`, danh sách thuộc tính ở `page+4704`.
- Thân F1: với mỗi vị trí `p`, `row = 0x737530(p)`, `idx = 0x738380(p)` với công thức `idx = +224 + cột + cols*(+840 + hàng)` (`0x73ADC0`; cờ wrap `+1988` thì `idx % count`); `idx >= count` thì ẩn hàng (`0x734190(row,0)`),
  ngược lại hiện và gọi `item.vf+16(rowProxy)`. **Callback không dựng hay gắn lại controller.**
- Với `EditProperty`, `vf+16` = `0x245CE0` -> `0x86BFF0` (Apply): ghi chữ `Text_0` bằng `0x74AE50`, nhưng item giữ con trỏ controller ở `item+120`; nếu controller đã có clip riêng (buộc lúc dựng) thì chữ ghi vào **clip buộc**, không vào hàng vị trí được truyền.
  Controller Slider/Combo/Toggle được dựng **một lần** khi thêm thuộc tính (`0x94A590/0x94A140` -> `0x9265E0` lấy clip của vị trí = `list.count` hiện tại -> `0x949DC0/0x9499A0/0x949780` -> `0x928630` đẩy vào danh sách tối đa 16).
- Chọn và nhập liệu dùng chỉ số tuyệt đối (`0x976D40`, `0x976AA0`: `list.vf+40/32(grid.selected)`).

**Kết luận (suy luận):** trang Options là ánh xạ 1:1 cố định thuộc tính <-> clip theo thứ tự thêm. Nếu `count > rows` grid vẫn coi là cuộn được (thanh cuộn có phạm vi, `+840` tăng), nhưng widget
không dịch chuyển vì chữ ghi vào clip buộc; thuộc tính thứ `rows+1` trở đi không có clip hiển thị; con trỏ chọn (theo vị trí) và mục chọn (theo `scroll+p`) lệch nhau. Chưa biết: `LayoutProxy` khi thiếu clip `Item_0_N` (null hay assert),
`SliderCtrl` dựng trên clip rỗng có crash không.

**Hệ quả:** muốn cuộn thật cho hàng widget thì phải **tự gán F1 riêng** cho grid: hàm này gắn lại (hoặc dựng lại) controller Slider/Combo/Toggle vào clip của vị trí theo `scroll`. Màn gán phím không cần vì hàng chỉ có chữ.
Giữ <= số hàng mỗi trang (7/12/15 tuỳ sprite) hoặc tách nhiều trang là cách an toàn nếu không tự viết F1.

## Các hướng, từ rẻ đến đắt (chưa thử)
1. Hook nạp danh sách của `KeyConfigDialog` với mục tuỳ biến (nhãn + ô, không slider): ít việc nhất, hạn chế loại hàng.
2. Vá GFX `02_040` thêm tab + clip trang cuộn (như NativeTweaks) và tự dựng lớp trang bằng `GridControl` + danh sách tuỳ biến.
3. Tự viết movie riêng: nặng nhất.

Rủi ro chính: giả vtable C++ từ Rust (`MenuViewItemList<T>`, mục); xung đột GFX với ERNativeUI/NativeTweaks/Solid Uncapper; không hook được `0x93D730` nếu mod khác cũng hook.

## Chưa biết
- Cách bấm hàng của `02_160` gọi callback, hàm mở `02_160` nằm ở menu nào (layout hàng đã đo ở trên).
- Slot tên clip rảnh (0, 1, 9, 12, 13) có clip tương ứng trong GFX gốc hay không; trường nào của bản ghi 0x88 byte gắn clip cửa sổ với category.
- Fork ERNativeUI của NativeTweaks có công bố mã và giấy phép không (không dùng thay bản mã mở).
- Mọi thứ trên chưa chạy trong game.

## Bước nghiên cứu tiếp
(a) ~~giải nén GFX `02_160`/`02_040`~~ (xong, xem trên); (b) decompile `CSMenuKeySetting` Apply (item vtable +16) để biết gán chữ vào clip nào;
(c) đọc `GridControl` slot 2, 3 (`0x738AB0`/`0x73A140`) cho cuộn bằng phím/chuột; (d) thử hook `0x869580` trả danh sách giả và quan sát trong game;
(e) nếu thêm tab: detour `0x93D730` + bản ghi `0x968CF0` trên GFX vá.
