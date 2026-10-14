# alloc_hook: tìm allocator của game bằng AOB

**Status 2026-10-13: hoạt động. Code ở `shared/src/alloc_hook.rs`, dùng bởi `common::announce::show_announcement`. Pattern đã kiểm chứng trên 1.16.2 và 1.17.0; chưa ghi nhận kiểm chứng trên các bản sau.**

## Vấn đề

Tạo `DLString` (vd. banner thông báo trong game) cần `DLAllocator::runtime_heap_allocator()`.
Hàm này của `fromsoftware-rs` đi qua `eldenring::rva::get()`, bảng RVA gắn theo
từng version game. Trên bản game mà `fromsoftware-rs` chưa được cập nhật, nó
panic. Đây cũng là lý do `task_hook` tồn tại (xem `shared/docs/task.md`):
`register_task` từng panic y như vậy.

## Vì sao phải quét AOB bằng byte

`wait_for_cs_task` tìm `CSTaskImp` theo tên qua reflection Dantelion2. Cách đó
không dùng được ở đây: allocator **không** phải singleton reflected
(`DLAllocator` không có `#[shared::singleton(...)]`), nên cần một mốc byte nằm
trong code game.

## Mốc đã chọn

Biến global giữ con trỏ allocator (allocator heap duy nhất của DLKR) được đọc ở
hàng trăm call site bị inline khắp exe, nên không bám vào call site nào được.
Có đúng 1 hàm getter nhỏ, tự chứa ("đọc, hoặc khởi tạo lười"), có prologue và
epilogue riêng:

```
sub rsp,0x28
call <ctor?>
mov rax,[rip+X]        ; <- global cần lấy (offset 9 trong pattern)
test rax,rax
jnz +0xc
call <lazy-init>
mov [rip+Y],rax
mov [rip+Z],rax
add rsp,0x28
ret
```

Kiểm chứng ngày 2026-09-11: pattern là duy nhất trong `.text`, và giống từng
byte (trừ các toán hạng rip-relative) giữa:

| Bản game | Nguồn RVA | RVA getter |
|---|---|---|
| 1.16.2 | `eldenring` 0.14.0 | `0x4842d40` |
| 1.17.0 | `fromsoftware-rs` commit `acb2a19` | `0x4846dc0` |

## Quyết định về cache

Chỉ cache **địa chỉ của biến global**, không cache giá trị trong nó. Code có
mặt ngay khi pattern khớp, nhưng game có thể chưa dựng allocator; con trỏ vẫn
null thì lần gọi sau phải đọc lại, không được cache null vĩnh viễn. Cache địa
chỉ là đủ để tránh quét lại mỗi lần bấm phím reload.

## Khi hỏng

Hàm trả `None` và ghi log nếu không thấy pattern hoặc global còn null. Người
gọi coi đó như mọi tính năng dựa trên AOB bị lỗi: không nghiêm trọng, chỉ mất
banner.

## Tham chiếu

- `shared/src/alloc_hook.rs`: cài đặt.
- `shared/docs/task.md`: cùng kỹ thuật cho `register_task`.
- `mods/autoregen/docs/task_and_startup.md`: lịch sử tách `task_hook`/`alloc_hook` khỏi autoregen.
