# InfiniteAilment: Black Flame và Destined Death

**Status 2026-10-15: ĐÃ LÀM cả hai, test trong game ổn (2026-10-16, log khớp 8 / 4 / 2 dòng và hiệu ứng chạy đúng), `status_effect.rs` (`BLACK_FLAME`, `DESTINED_DEATH`, `DESTINED_DEATH_MAX_HP`).**

Yêu cầu từ người dùng Nexus (long41, 2026-10-13): "make the black flame or destined death as ailment, I want blackflame last forever" - tức hiệu ứng đốt và hiệu ứng giảm máu tối đa kéo dài mãi, không phải thêm cơ chế mới.

## Dữ liệu (SmithBox export `SpEffectParam`, bản 2.7.x)

Vết cháy của Black Flame là các dòng DoT 2 giây, nhịp 0.1 giây, 0.1% máu tối đa + 1 máu cố định mỗi nhịp, `StateInfo 0`, `SpCategory` 10 hoặc 20 (khác nhau tuỳ nguồn):

| ID | Tên | Ghi chú |
|---|---|---|
| 835 | [Weapon] Black Flame Tornado - HP Burn | cat 10 |
| 855 / 860 | Godslayer's Greatsword / Gargoyle's Blackblade - HP Burn | cat 20 |
| 1621000, 1623000, 1625000, 1626001 | Black Flame, Scouring Black Flame, Black Flame Ritual, Black Flame Blade (DoT Black) | cat 20 |
| 1624010 | Noble Presence (DoT Black) | cat 20 |

Không đưa vào: `29000`/`29001`.. `29517` (NPC: HP Burn on Hit, quái gây ra), `6840..6851` (thuộc Destined Death, xem dưới), `1627001` (Black Flame's Protection, không có `ChangeHpRate`).

## Vì sao dùng danh sách ID thay vì chữ ký

Poison và Scarlet Rot có `SpCategory`/`StateInfo` riêng nên lọc theo chữ ký được. Black Flame thì không: chữ ký (`EffectEndurance 2`, nhịp 0.1, `ChangeHpRate 0.1`, `ChangeHpPoint 1`) trùng với Destined Death và các dòng NPC. Nên mod chỉ xét 8 ID ở trên và vẫn kiểm `EffectEndurance == 2`, `StateInfo == 0`, `ChangeHpRate != 0` trước khi ghi (overhaul đổi giá trị gốc thì dòng đó bị bỏ qua, như hai ailment kia). ID của DLC/overhaul không có trong danh sách sẽ không được sửa.

## Mặc định

`BlackFlame.Duration=10` và `DestinedDeath.Duration=10` (vanilla 2 giây; cháy mãi (-1) quá mạnh nên mặc định giảm còn 10 giây, người dùng quyết định 2026-10-16), phần trăm và sát thương cố định mặc định bằng vanilla (0.1 / 1). `DestinedDeath.MaxHpDuration` mặc định vẫn -1 (mãi).

## Destined Death

Hai nhóm dòng, mỗi nhóm một ailment trong mod:

- **Cháy (DoT):** `6840`, `6841`, `6850` ("DoT Main"), `6851`: cùng số liệu với Black Flame (2 giây, nhịp 0.1, 0.1% + 1), chỉ khác `vfxId` (9011/9012). Chỉnh y như Black Flame, mục `[Destined Death]` (`DestinedDeath.Duration/PercentDamage/FixedDamage`). `6840`, `6841`, `6851` không có tên trong SmithBox; được đưa vào vì nằm cùng khối ID 684x-685x với `6850`, khớp trong log và chạy đúng khi test (2026-10-16).
- **Giảm máu tối đa:** `6860` (15 giây) và `6865` (45 giây), `MaxHpRate 0.9`, không có `ChangeHpRate`. Mod chỉ ghi `EffectEndurance` (`DestinedDeath.MaxHpDuration`, mặc định -1 = mãi); không đụng `MaxHpRate`. Dòng thuộc nhóm này chỉ cần `EffectEndurance > 0` (vì 15 và 45 khác nhau, không dùng một giá trị vanilla chung).

CSV không có dòng nào tham chiếu tới 6840-6865, nên chưa biết nguồn nào kích chúng (chỉ đoán từ tên: Maliketh / Gurranq); nếu nguồn đó không nằm trong 6860/6865 thì giảm máu tối đa vẫn theo thời lượng gốc.
