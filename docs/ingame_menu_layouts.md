# Bố cục menu trong game của từng mod

**Status 2026-10-13: bố cục hiện tại của 10 mod có menu (ERNativeUI), chưa kiểm trong game.** Hình vẽ lấy giá trị mặc định của
ini mẫu từng mod. Nguồn: `mods/<mod>/src/menu.rs`. Quy ước chung và giới hạn của host: [ingame_menu.md](ingame_menu.md).

## Cách đọc hình

```
 ▶ Tên mod            hàng nút ở Game Options, bấm để mở trang
 ▷ Nhóm               hàng nút mở trang con
 ◄ On ►               công tắc bật / tắt
 ◄ Giá trị ►          hàng chọn: bấm trái/phải đổi giữa vài lựa chọn
 ━━━━●━━━━━━━  30     thanh trượt, số nguyên
 [ 1.5 ]              ô gõ số: bấm để mở bàn phím, gõ rồi xác nhận
 ⟨ 0 … 100 ⟩         phạm vi hợp lệ: nhỏ nhất … lớn nhất (giá trị ngoài phạm vi bị kẹp về mép)
 ℤ                    chỉ nhận số nguyên (gõ 1.5 sẽ thành 2)
 (0 = tắt)            ghi chú ngắn, không phải một phần của hàng

 ⚡ áp sau khi lưu  mod đọc cấu hình liên tục: có tác dụng khoảng 1 giây sau khi bạn ngừng thao tác (RuneMultiplier: ngay khi xác nhận)
 ↻ áp lại        mod tính lại từ dữ liệu gốc sau khi bạn ngừng thao tác 1 giây
 ⟲ khởi động lại   chỉ có tác dụng sau khi khởi động lại game
```

Mọi mod đều có công tắc **Log file** ở trang đầu và hàng gốc ở System > Game Options. Có GFX thì Game Options có 9 chỗ trống,
10 mod sẽ phân trang (Next/Previous); không GFX thì chỉ 2 chỗ.

---

## Drop Multiplier  ↻

```
▶ Drop Multiplier
  ├─ ▷ Drop Chance
  │     Mode          ◄ Multiply chance ►
  │     Multiplier    [ 3 ]   ⟨0 … 1000⟩
  │     Percentage    ━━━●━━━━━━━━━━  30   ⟨0 … 100⟩     (khi Mode = Fixed chance)
  ├─ ▷ Materials
  │     Crafting      [ 2 ]   ⟨0 … 1000⟩
  │     Upgrade       [ 2 ]   ⟨0 … 1000⟩
  │     Unique        [ 1 ]   ⟨0 … 1000⟩
  └─ Log file         ◄ On ►
```

## Rune Multiplier  ⚡

```
▶ Rune Multiplier
  ├─ Multiplier       [ 2 ]   ⟨-1000 … 1000⟩                  (âm = bị trừ rune)
  └─ Log file         ◄ Off ►
```

## Auto Regen  ⚡

```
▶ Auto Regen
  ├─ ▷ Per Tick                              (hồi theo thời gian)
  │     Enabled         ◄ On ►
  │     When            ◄ Always ►
  │     Interval (ms)   [ 1000 ]   ⟨ℤ 0 … 3600000⟩             (0 = tắt)
  │     Value Type      ◄ Percent of max ►
  │     HP              [ 0.5 ]   ⟨-1000 … 1000⟩
  │     FP              [ 1 ]   ⟨-1000 … 1000⟩
  │     SP              [ 5 ]   ⟨-1000 … 1000⟩
  │     Max Fill        ━━━━━━━━━━━━●  100   ⟨1 … 100⟩
  ├─ ▷ Per Hit                               (hồi khi đánh trúng)
  │     Enabled         ◄ Off ►
  │     Mode            ◄ Points ►
  │     Hit Type        ◄ Melee ►
  │     Skip Ash of War ◄ Off ►
  │     HP              [ 0.5 ]   ⟨-1000 … 1000⟩
  │     FP              [ 1 ]   ⟨-1000 … 1000⟩
  │     SP              [ 5 ]   ⟨-1000 … 1000⟩
  └─ Log file           ◄ On ►
```

## Infinite Ailment  ↻

```
▶ Infinite Ailment
  ├─ ▷ Scarlet Rot
  │     Duration (s)    [ -1 ]   ⟨-1 … 86400⟩               (-1 = vô hạn)
  │     Damage (%)      [ 0.18 ]   ⟨-100 … 100⟩        (âm = hồi máu)
  │     Damage          [ 15 ]   ⟨ℤ -1000 … 1000⟩     (âm = hồi máu)
  ├─ ▷ Poison
  │     Duration (s)    [ -1 ]   ⟨-1 … 86400⟩
  │     Damage (%)      [ 0.07 ]   ⟨-100 … 100⟩        (âm = hồi máu)
  │     Damage          [ 7 ]   ⟨ℤ -1000 … 1000⟩      (âm = hồi máu)
  └─ Log file           ◄ On ►
```

## Passive Runes  ⚡

```
▶ Passive Runes
  ├─ Enabled          ◄ On ►
  ├─ Interval (ms)    [ 10000 ]   ⟨ℤ 0 … 3600000⟩              (0 = tắt)
  ├─ Amount           [ 50 ]   ⟨ℤ -100000 … 100000⟩
  ├─ Next Level (%)   [ 0.25 ]   ⟨-100 … 100⟩
  ├─ Interest (%)     [ 0 ]   ⟨-100 … 100⟩
  └─ Log file         ◄ Off ►
```

## Rise Archer  ↻

```
▶ Rise Archer
  ├─ ▷ Damage
  │     Bow Damage          [ 1.5 ]   ⟨-1000 … 1000⟩
  │     Crossbow Damage     [ 3 ]   ⟨-1000 … 1000⟩
  │     Ballista Damage     [ 4 ]   ⟨-1000 … 1000⟩
  │     Bow Scaling         [ 2 ]   ⟨-1000 … 1000⟩
  │     Bow Ash of War      ◄ On ►
  ├─ ▷ Weight & Value
  │     Bow Weight          [ 0.5 ]   ⟨-1000 … 1000⟩
  │     Crossbow Weight     [ 0.5 ]   ⟨-1000 … 1000⟩
  │     Ballista Weight     [ 0.5 ]   ⟨-1000 … 1000⟩
  │     Bow Sell Value      [ 2 ]   ⟨-1000 … 1000⟩
  │     Crossbow Sell Value [ 2 ]   ⟨-1000 … 1000⟩
  │     Ballista Sell Value [ 2 ]   ⟨-1000 … 1000⟩
  ├─ ▷ Ammo & Shots
  │     Arrow Limit         ━━━━━━━━━━━●  255   ⟨0 … 255⟩
  │     Bolt Limit          ━━━━━━━━━━━●  255   ⟨0 … 255⟩
  │     Projectile Speed    [ 3.5 ]   ⟨-1000 … 1000⟩
  │     Rain Range          [ 2 ]   ⟨-1000 … 1000⟩
  │     Rain Spread         [ 2 ]   ⟨-1000 … 1000⟩
  │     Rain Arrows         [ 6 ]   ⟨ℤ 0 … 1000⟩
  └─ Log file               ◄ On ►
```

## Speed Multiplier  ⚡

```
▶ Speed Multiplier                           (mọi ô: ⟨0.1 … 10⟩, số thập phân)
  ├─ All Player Actions   [ 1.0 ]            (khác 1 thì ghi đè các hàng riêng)
  ├─ ▷ Player Movement
  │     Walk  [ 1.1 ]   Run  [ 1.2 ]   Sneak [ 1.1 ]
  │     Jump  [ 1.0 ]   Roll [ 1.1 ]   Ladder [ 1.2 ]
  ├─ ▷ Player Actions
  │     Attack [ 1.2 ]  Critical [ 1.0 ]  Skill [ 1.2 ]
  │     Cast   [ 1.2 ]  Item     [ 1.0 ]  Other [ 1.0 ]
  ├─ ▷ Torrent
  │     All [ 1.0 ]  Walk [ 1.1 ]  Run [ 1.3 ]  Jump [ 1.0 ]  Other [ 1.0 ]
  ├─ ▷ Spirit & Enemy
  │     Spirit [ 1.0 ]   Enemy [ 1.0 ]
  └─ Log file             ◄ On ►
```
(mỗi giá trị là một hàng riêng; xếp ngang ở đây cho gọn; tất cả cùng phạm vi ⟨0.1 … 10⟩)

## Spirit Multiplier  ⚡  (Max Spirits: ⟲)

```
▶ Spirit Multiplier
  ├─ ▷ Spirit Count
  │     Multiplier      [ 2 ]   ⟨0 … 1000⟩
  │     Fixed Count     [ 0 ]   ⟨ℤ 0 … 1000⟩                (0 = dùng Multiplier)
  │     Max Spirits     [ 60 ]   ⟨ℤ 60 … 1000⟩               ⟲
  ├─ ▷ Summoning
  │     Multi Spirit            ◄ On ►
  │     Clone Spirit            ◄ Off ►
  │     Resummon Without Rest   ◄ On ►
  │     Summon Anywhere         ◄ On ►
  ├─ ▷ Health and Look
  │     Regen Mode      ◄ % of max HP ►
  │     Regen Amount    [ 0.75 ]   ⟨0 … 1000⟩             (0 = không hồi)
  │     Ghost Color     ◄ Off ►
  ├─ ▷ Behavior
  │     Senses ◄ On ►   Aggression ◄ On ►   Follow ◄ On ►
  ├─ ▷ Teleport
  │     Warp When Far   ◄ On ►
  │     Distance (m)    [ 25 ]   ⟨0 … 1000⟩
  │     Blocked Time (s) [ 1 ]   ⟨0 … 600⟩
  │     Stuck Time (s)  [ 2 ]   ⟨0 … 600⟩
  │     Stuck Range (m)  [ 3 ]   ⟨0 … 100⟩
  └─ Log file           ◄ On ►
```
Lưu ý: nhóm Teleport hiện không có tác dụng theo ini mẫu (chú thích cuối dòng làm parse lỗi), chờ quyết định.

## Weight Multiplier  ↻

```
▶ Weight Multiplier
  ├─ Mode             ◄ Multiply ►
  ├─ Multiplier       [ 0.5 ]   ⟨-1000 … 1000⟩                (khi Mode = Multiply)
  ├─ Fixed Value      [ 0 ]   ⟨-1000 … 1000⟩                  (khi Mode = Fixed value)
  └─ Log file         ◄ On ►
```

## Anim Cancel  ⚡

```
▶ Anim Cancel
  ├─ ▷ Guard
  │     Enabled       ◄ On ►
  │     Attacks       ◄ On ►
  │     Skills        ◄ On ►
  │     Spells        ◄ On ►
  │     Items         ◄ On ►
  ├─ ▷ Dodge
  │     Enabled       ◄ On ►
  │     Attacks       ◄ On ►
  │     Skills        ◄ On ►
  │     Spells        ◄ On ►
  │     Items         ◄ On ►
  ├─ ▷ Weapon swap
  │     Enabled       ◄ On ►
  │     Attacks       ◄ On ►
  │     Skills        ◄ On ►
  │     Spells        ◄ On ►
  │     Items         ◄ On ►
  └─ Log file         ◄ On ►
```

---

## Không có menu

FasterRevival (không có thiết lập đáng chỉnh), SoulsTeleport (đã có giao diện riêng), SomeTweaks (đã dừng), WindowResize (mod nội bộ, không đăng).

## Ghi chú kỹ thuật

- Phạm vi `⟨ … ⟩` của từng ô khai báo ở đầu mỗi `menu.rs` (`NumberSetting`, slider). Gõ ngoài phạm vi thì bị kẹp về mép và ô
  hiện lại số đã kẹp; chữ không phải số thì ô trả về giá trị cũ. Phạm vi là do menu chặn, còn ini sửa tay vẫn nhận mọi giá trị mà
  mod chấp nhận (ví dụ hệ số RuneMultiplier ngoài ±1000).
- Danh sách gesture của AutoRegen, mốc rune của PassiveRunes, khối `[[Override]]` và các probe chỉ sửa trong file cấu hình.
