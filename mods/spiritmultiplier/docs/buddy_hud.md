# Thanh HP spirit trên HUD (`HubHpBars`)

**Status 2026-10-23: `HubHpBars` (5-30) hoạt động trong game 2.7.2.0: thử 10 và 30 thanh với nhiều spirit, không crash, panel nâng lên nằm gọn giữa khung trạng thái và khung kỹ năng (độ phân giải 2560x1600). Panel tự đặt vị trí lúc chạy theo số thanh đang dùng (mục 7), đã thử trong game: 5 spirit nằm đúng chỗ vanilla, nhiều hơn thì mọc lên trên. Chưa thử các bản exe cũ. Giải mã bằng IDA trên 2.7.2.0; các AOB duy nhất trên 2.6.2 / 2.7.0 / 2.7.1 / 2.7.2.**

## Vì sao game chỉ có 5 thanh

Ba nơi cùng cứng 5, nên nới một chỗ không đủ:

- **Dữ liệu.** `sub_140771ED0` (RVA 0x771ED0, chạy mỗi frame từ `sub_140772F00`) duyệt spirit đã triệu hồi, mỗi con qua 3 kiểm tra (chưa chết `sub_1403F4740`; `sub_1404B7CE0`; không phải Blob Mimic `sub_1404B7A10`) rồi ghi vào **5 slot** ở `a1+19936`, 24 byte/slot: handle chr (8), HP (4), đệm (4), HP tối đa mất đi (4), HP tối đa (4). Slot trống = `-1`. `if (v3 < 5)` chỉ ghi vào mảng, biến đếm vẫn tăng.
- **Vòng lặp hiển thị.** `sub_1408C2600` (cập nhật `CS::BuddyViewParts`, gọi từ `sub_1408CA1C0`) nhận khối `a3 = a1+19800` (header 136 byte, rồi slot) và lặp **đúng 5 slot** (`lea rsi,[rdi+78h]`). Mỗi slot gắn vào một thành phần thanh HP (khoá `+200` = handle, trống = `-1`).
- **Danh sách hàng của view.** Constructor `sub_1408C1D20` gom `Anim/GaugeList/Item_%d` (lặp tới khi thiếu `Item_i`) bằng một lambda; lambda (`sub_1408C24C0`) thêm hàng vào một `DLFixedVector` **dung lượng cố định 5** nằm ngay trong object view (`+896`, đếm ở `+944`), rồi bỏ qua hàng thứ 6 trở đi. Dung lượng này không mở rộng được tại chỗ (thành phần kế tiếp ở `+952`).

Điều dễ nhầm: khi vector đầy, lambda gọi `sub_140734640(row, 0)`, tức **đặt `visible = false` cho clip của hàng**, không phải bỏ qua êm. Vanilla cần vậy để giấu hàng thừa; hook của mod phải tránh gọi nó cho hàng mà view khác dùng.

## Cấu trúc trong `menu/01_000_fe.gfx`

```
Buddy (gốc, depth 447, 18 frame)
└─ Anim
   ├─ Base, ItemIcon, StandbyIcon, ItemName
   └─ GaugeList  → Item_4 (y 0), Item_3 (-300 twips), ... Item_0 (-1200); mỗi mục là Timeline_178 có clip HP
```

Chỉ có `Item_0..Item_4`, cách nhau 300 twips (15 px), `Item_4` thấp nhất. Mã AS3 trong tệp chỉ có timeline; logic nằm hết ở exe.

## Cách làm (`src/buddy_hud.rs`)

Mỗi view giữ 5 hàng, nên cần nhiều view trên **cùng một clip `Buddy`** (không vẽ panel thứ hai):

1. **Movie** (`common::gfx_hook`, `common::gfx`): thêm `Item_5...Item_{N-1}` vào `GaugeList` (cùng nhân vật và scale, cách nhau như hai hàng cuối). Chi tiết: `shared/docs/gfx.md`.
2. **`hud_blob`**: đổi đích `call sub_1404B7A10` trong `sub_140771ED0` (cũng là chỗ `InstantMimic` bỏ Blob). Hàm này chỉ được gọi cho spirit đã qua mọi kiểm tra khác, nên là danh sách chính xác những con được cấp slot; ghi lại theo thứ tự.
3. **`hud_build`**: đổi đích `call sub_140771ED0` trong `sub_140772F00`; chạy hàm gốc (5 slot đầu), rồi dựng các slot sau slot thứ 5 từ danh sách ở bước 2 (handle `chr+8`, `data.hp`, `max_hp`, `max_uncapped_hp`).
4. **`hud_ctor`**: đổi đích `call sub_1408C1D20` trong constructor HUD (`sub_1408C6C80`); dựng view của game rồi `ceil(N/5) - 1` view phụ bằng chính hàm đó, mỗi view cấp phát riêng (0x1000 byte, không giải phóng).
5. **`hud_item`**: đổi đích `jmp` của hàm bọc lambda (`add rcx,8; jmp sub_1408C24C0`, tìm bằng `memscan::find_jumps_to`); view thứ k chỉ nhận hàng `5k..5k+4`, hàng khác bỏ qua **không** qua hàm ẩn clip.
6. **`hud_view`**: đổi đích `call sub_1408C2600` trong `sub_1408CA1C0`; sau view của game, cập nhật từng view phụ với header của game và 5 slot phụ của nó.

7. **Vị trí panel** (trong `hud_view`): `Anim`, clip chứa cả nền, thanh, icon và tên, nằm ở (0, 0) trong `Buddy`. Mỗi frame mod tính số hàng đang dùng (`min(số spirit được cấp slot, N)`, tối thiểu 5) và dời `Anim` lên `(hàng - 5) × 15 px` khi con số đó đổi: 5 thanh trở xuống nằm đúng chỗ vanilla, nhiều hơn thì panel mọc lên khoảng trống phía trên (không xuống đè khung kỹ năng). Cách làm giống mã menu của game: proxy clip của component `Anim` của view (view+776, proxy ở +8) có vtable slot 1 trả về `Value`, rồi gọi `sub_140D872C0(value, x, y)` (đặt `_x`, `_y`). Khi HUD dựng lại, vị trí đã áp dụng được đặt về 0.

Thứ tự cài: view, hàng, list, constructor, nên lỗi giữa chừng chỉ để lại 5 thanh. Kiểm layout (header 136, 5 slot 120) bằng mẫu `LAYOUT_AOB` trước khi cài; không khớp thì không bật. Mọi chỗ đổi đích là `call`/`jmp rel32`, nên mod khác hook cùng chỗ vẫn nối chuỗi được; không có gì ghi đè byte của game.

## AOB (duy nhất trên 2.6.2 - 2.7.2)

| Chỗ | Mẫu (rút gọn) | Vị trí |
|---|---|---|
| Blob check | `84 C0 74 ?? 48 8B D6 49 8B CE E8 ... 83 FF 05` | `call` +10 |
| Gọi list builder | `E8 .. 0F 28 CE 48 8B CB E8 .. 0F 28 CE 48 8B CB E8 .. 4C 8B C7` | `call` +11 |
| Gọi view update | `48 81 C2 .. 00 00 48 8D 8E .. E8 .. 4C 8B 46 60 49 81 C0 .. 00 00 48 8D 8E .. 0F 28 CE E8` | `call` +40 |
| Gọi constructor view | `4C 8D 05 .. 48 8D 54 24 .. 49 8B CE E8 .. 90 49 8D 8E .. 48 8B D0 E8` | `call` +31 |
| Hàm thêm hàng | `40 57 48 83 EC 30 48 C7 44 24 20 FE FF FF FF ... 48 83 B8 B0 03 00 00 05 0F 83` | đầu hàm; wrapper là `jmp` trỏ vào đó, trước nó `48 83 C1 08` |
| Layout slot | `49 8D BF 88 00 00 00 41 BC FF FF FF FF 48 8D 77 78 48 3B FE` | chỉ kiểm |
| Đặt `_x/_y` của clip | `48 8B C4 48 89 58 10 55 48 8D 68 C8 48 81 EC 30 01 00 00 ...` | đầu hàm (`sub_140D872C0`) |

Mẫu gọi view update phải kèm ngữ cảnh phía trước (`mov r8,[rsi+60h]`): bản ngắn khớp 7 chỗ.

## Kiểm trên các bản exe cũ (2026-10-23, IDA)

`scripts/ida/buddy_hud_check.py` (`run.ps1 buddy_hud_check -Version all --all`): trên **2.6.2.0, 2.7.0.0, 2.7.1.0, 2.7.2.0** cả 7 mẫu đều duy nhất và các quan hệ giữa hàm đều đúng: lệnh gọi Blob nằm trong chính hàm mà chỗ gọi list builder trỏ tới; hàm view update chứa cửa sổ slot (136 + 5×24 byte); constructor view tham chiếu `Anim/GaugeList` và một vtable có slot 2 là wrapper `add rcx,8; jmp` của hàm thêm hàng (đúng một wrapper); hàm mở tệp bắt đầu đúng 16 byte trước mẫu và có 3 ô con trỏ trong dữ liệu. 2.7.0.0 và 2.7.1.0 trùng địa chỉ, chỉ khác hàm mở tệp (`D6D210` / `D6D280`). Đây là kiểm trên mã, chưa phải chạy thật trên các bản đó.

## Giới hạn và rủi ro

- Tối đa 30 thanh (6 view); nâng `MAX_BARS` là đủ để thêm view. Giới hạn thực tế là chiều cao màn hình: khi dùng đủ N thanh panel nâng lên `(N-5) × 15 px` (N=30: 375 px), thử ở 30 thanh vẫn nằm giữa khung trạng thái và khung kỹ năng. 30 được chọn vì vừa khung; hơn nữa panel sẽ chạm khung trạng thái.
- View phụ không bao giờ được giải phóng (rò rỉ vài KB nếu game dựng lại HUD) và giữ tham chiếu movie đến khi thoát.
- Header (tên, icon, nền) bị mọi view ghi, nhưng cùng giá trị nên không khác gì khi chỉ có một view.
- Offset trong khối (`136`, `24`) và kích thước view (0x430) đo ở 2.7.2.0; mẫu `LAYOUT_AOB` chặn bản có layout khác, kích thước view chỉ có biên an toàn 0x1000.

## Lỗi đã gặp khi làm (để khỏi lặp lại)

- AOB `gfx_hook` bắt đầu từ prologue không khớp khi ERNativeUI đã hook đầu hàm mở tệp (log `file opener pattern not found`): `shared/docs/gfx.md`.
- Quét `find_data_pointers` theo kích thước section crash (trang guard): `shared/docs/memscan.md`.
- Gọi đường thoát "vector đầy" cho hàng của view khác làm biến mất 5 thanh đầu (clip bị ẩn): xem phần trên.
