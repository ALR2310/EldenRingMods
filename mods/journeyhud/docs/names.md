# JourneyHud: tên kẻ địch

**Status 2026-10-26: ĐÃ BỎ, không có trong mod. Người dùng không cần tên (souls_vision gốc cũng không có). Phần dưới là kết quả dịch ngược, giữ lại nếu sau này cần.**

Code đã thử (đã xoá) tra ra `None` cho Tree Sentinel ở 2.7.2.0 (`npc 32510010`, 7045 dòng NpcParam): `name_id` đọc theo fromsoftware-rs hoặc danh mục text chưa đúng, chưa tìm nguyên nhân.

Tên = `name_id` của dòng `NpcParam` (`chr.npc_param_id`), tra trong text của chính game nên theo đúng ngôn ngữ
game đang chạy. Code: `src/names.rs`.

## Hàm tra text (`get_message`)

Chữ ký: `(repository, version, category, id) -> const wchar_t*` (rcx, edx, r8d, r9d; NULL nếu không có).
Game luôn truyền `version = 0`. Tìm bằng AOB (đầu hàm, tới lệnh `jmp` đuôi):

```text
3B 51 10 73 ?? 44 3B 41 14 73 ?? 48 8B 41 08 8B D2 48 8B 0C D0 48 85 C9 74 ?? 41 8B C0 48 8B 0C C1 48 85 C9 74 ?? 41 8B D1 E9
```

`scripts/ida/run.ps1 aob -Version all`: **UNIQUE ở cả 4 bản**, trúng đúng hàm (RVA 266D3C0 / 266FBD0 / 266FC40 / 2671370).
Tiền tố ngắn hơn (15 byte đầu, như SomeTweaks dùng) khớp 2 chỗ vì có một bản sao trong vùng dữ liệu (`26713E0` ở
2.7.2.0, không phải hàm), nên mẫu dài là bắt buộc.

`repository` là `MsgRepositoryImp::instance()` của fromsoftware-rs (singleton tra theo tên, không qua `rva::get()`).

## Category của tên NPC

Hàm menu `CS::MenuString::NpcName` (2.7.2.0: `0xD13910`) thử lần lượt các category **119, 18, 328, 428** và trả
chuỗi đầu tiên có, không thì `"?NpcName?"`. Cùng 4 số và cùng thứ tự ở 2.6.2.0 (`D10AE0`), 2.7.0.0 (`D121B0`),
2.7.1.0 (`D12220`). Mod làm đúng như vậy; các chuỗi `?...` và `%...` coi như không có tên.

## Giới hạn

- Phông mặc định của ImGui chỉ có Latin cơ bản: ngôn ngữ cần chữ khác (Nhật, Trung, Hàn, Nga...) sẽ ra `?`. Cần nạp phông hệ thống như menu của SoulsTeleport (`docs/menu_ui.md`) nếu muốn hỗ trợ.
- `NpcParam` đọc qua `common::params::row_ids` (không dùng `rows()` / `get()`), chỉ khi đã vào game.
- `name_id` đọc theo offset của fromsoftware-rs (+0xC); đối chiếu bằng log `Target: npc <id> Some("...")` khi thử.
