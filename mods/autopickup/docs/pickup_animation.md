# Tắt animation nhặt vật phẩm (`NoPickupAnimation`)

**Status 2026-10-27: hoạt động, đã thử trong game và người dùng xác nhận: `NoPickupAnimation` ghi 999999 vào `GameSystemCommonParam.msbEventGeomTreasureInfo_itemGetAnimId_shine` (nút 4250, animation gốc 60070) và `PlayerCommonParam.animeID_DropItemPick` thì animation nhặt vật phẩm trên đất biến mất. `animeID_DropItemPick` riêng nó không đủ (thử lần đầu không có tác dụng); hài cốt (`corpse`) và rương (`box`) cũng đã thêm vào cùng công tắc và được người dùng xác nhận ổn.**

## Vì sao có

Bình luận trên Nexus (2026-10-27): "add option to disable pick up animations, it's playing even while I'm mounted". Người dùng thử tự nhặt: nguyên liệu và đồ trên xác không có animation, chỉ vật phẩm trên đất (và vật phẩm bị vứt ra) có.

## Cách làm

Giống `vanilla_pickup_animations` của ErdTools (`tmp/repos/Erd-Tools-CPP/Erd-Tools-CPP/Hook/ParamHook.cpp`, `EditPlayerCommonParam`): sửa dòng 0 của `PlayerCommonParam`. ErdTools ghi `animeID_DropItemPick = 60070`, `animeID_SleepCollectorItemPick = 50250`, `animeID_MaterialItemPick = 999999` (ID không tồn tại); tên tùy chọn của họ nói là trả về animation "vanilla", tức mục đích khác. Mod này mượn đúng kỹ thuật "ghi ID không tồn tại" nhưng chỉ cho `animeID_DropItemPick`, vì đó là trường của vật phẩm trên đất. Code ở `src/animation.rs`: giá trị gốc lưu ở lần ghi đầu (log `Animation: drop item pick is <id>`), tắt tùy chọn thì khôi phục.

## Chưa biết, cần thử

- Có đúng trường này không: bật `NoPickupAnimation`, nhặt vật phẩm trên đất, animation phải biến mất.
- Vật phẩm có vào túi không khi animation không tồn tại (nếu game chờ animation xong mới cấp vật phẩm thì không nhặt được).
- Animation khi cưỡi ngựa có cùng trường không (`PlayerCommonParam` không có trường animation riêng cho cưỡi ngựa).
- Log ghi giá trị gốc của game, đối chiếu với `60070` của ErdTools.

## Kết quả thử và hướng tiếp (2026-10-27)

- `animeID_DropItemPick = 60070` ở game vanilla 2.7.2.0, ghi thành 999999 mà animation nhặt vật phẩm trên đất vẫn phát: trường này không điều khiển nó.
- Nghi ngờ đúng chỗ là `ObjActParam` (hành động của đối tượng trên bản đồ): mỗi dòng có `player_anim_id` (animation của nhân vật), `action_button_param_id` (nối tới `ActionButtonParam`, tức ID trong nhóm Items) và `event_kick_timing` (thời điểm trao vật phẩm trong animation). Nguyên liệu và đồ trên xác không có animation, nên so dòng `ObjActParam` của chúng với dòng của vật phẩm trên đất sẽ cho biết giá trị "không animation". Cần export `ObjActParam` (vanilla, Convergence) để so trước khi ghi gì vào game.
- Rủi ro: nếu vật phẩm chỉ được trao ở `event_kick_timing` bên trong animation thì bỏ animation có thể làm vật phẩm không vào túi.
- `elden_ring_item_randomiser.dll` đang bật cùng lúc khi thử lần này; nên tắt nó khi thử nhặt đồ (theo quy tắc "tin kết quả thử có điều kiện" trong `CLAUDE.md`).

## Mặc định

`NoPickupAnimation = true` ở file mẫu, tức animation nhặt bị tắt sẵn cho mọi người (2026-10-27, người dùng chốt giữ khi phát hiện bản 1.1.0 đã đăng mang giá trị này). Người đã cài 1.0.0 nhận key này từ file mẫu khi cập nhật nên cũng bị tắt animation; changelog 1.1.0 chỉ ghi "Added option".

## Nguồn animation tìm được (2026-10-27, từ `GameSystemCommonParam` và `ObjActParam` vanilla)

`GameSystemCommonParam` (một dòng) có ba bộ `msbEventGeomTreasureInfo_*` cho kho báu đặt trên bản đồ:

| Loại | `actionButtonParamId` | `itemGetAnimId` |
|---|---|---|
| shine (vật phẩm phát sáng trên đất) | 4250 "Pick up item" | 60070 |
| corpse (hài cốt) | 4200 "Pillage remains" | 60070 |
| box (rương) | 4300 "Search treasure chest" | 60090 |

`ObjActParam`: nguyên liệu (nút 7800, dòng 90017) có `playerAnimId = 0`, khớp việc nguyên liệu không có animation; nhóm Items không có dòng `ObjActParam` nào, nên animation của chúng đi qua đường kho báu ở bảng trên chứ không qua `ObjActParam`.

Code (`animation.rs`) đổi `itemGetAnimId_shine` và `animeID_DropItemPick` thành 999999 khi bật `NoPickupAnimation`, khôi phục giá trị gốc khi tắt. Từ cùng ngày, `corpse` (60070) và `box` (60090) cũng đổi thành 999999 bởi cùng công tắc (gộp chung vì chỉ có một yêu cầu "tắt animation nhặt"); đã thử và ổn. Log ghi giá trị gốc: `Animation: drop item pick <id>, shine <id>, corpse <id>, chest <id>`.
