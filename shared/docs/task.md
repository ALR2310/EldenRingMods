# task / task_hook: đăng ký tác vụ chạy theo frame không qua `rva::get()`

**Status 2026-10-13: hoạt động, mọi mod có tick theo frame đều dùng. Pattern đã kiểm chứng trên 1.16.2 và 1.17.0; chưa ghi nhận kiểm chứng trên các bản sau. Code ở `shared/src/task.rs`, `shared/src/task_hook.rs`.**

## Vấn đề

`fromsoftware_shared::task::SharedTaskImpExt::run_recurring` tìm hàm đăng ký task
thật của game qua bảng RVA theo phiên bản (`rva::get().register_task`). Bảng đó
chỉ chứa **một** chuỗi phiên bản mà crate được phát hành lần cuối, và panic
thẳng với mọi phiên bản khác. Thử một mod trên bản game mới hơn bản
`fromsoftware-rs` được ghim làm panic ngay lúc khởi động, thay vì chỉ một tính
năng bị lỗi (câu chuyện đầy đủ ở `mods/autoregen/docs/task_and_startup.md`).

## Giải pháp: AOB

Kiểm chứng 2026-09-09: hàm game mà `register_task` trỏ tới giống từng byte (trừ
toán hạng rip-relative của `call`/`lea`, vốn đổi theo lần link lại) giữa:

| Bản game | Nguồn RVA | RVA |
|---|---|---|
| 1.16.2 | `eldenring` 0.14.0 | `0xeb1fe0` |
| 1.17.0 | `fromsoftware-rs` commit `acb2a19` | `0xeb3de0` |

Một pattern neo vào mọi byte **trừ** các toán hạng đó là duy nhất trong `.text`
trên cả hai. Quét pattern bằng `common::memscan` (cùng kỹ thuật với `hit_hook`
của autoregen) tìm ra hàm bất kể phiên bản nào làm nó dịch chỗ, tránh hẳn cổng
phiên bản của `rva::get()` ở đúng chỗ gọi này. Địa chỉ được cache cho cả process
(`CachedAddr`) nên nhiều tác vụ chỉ quét một lần.

## Đối tượng tác vụ giả

Lời gọi đăng ký vẫn phải đưa cho game một con trỏ **trông như** đối tượng task
native của nó: một vtable C++ ở offset 0, được gọi khi nhóm task chạy.
`fromsoftware_shared::task` làm đúng mẹo này cho địa chỉ được giải bằng bảng RVA,
nhưng phần `RecurringTask` / `self_ref` của nó là private nên không dùng lại được
cho một địa chỉ khác. `Task` trong `task_hook.rs` là bản độc lập của cùng mẹo (vtable
giả dựng bằng crate `vtable-rs` mà `fromsoftware-shared` cũng dùng). Game chỉ đọc
con trỏ vtable ở offset 0; phần sau (closure) với game là mờ, chỉ chạm tới qua
`execute`, nên không cần phản chiếu bố cục `RecurringTask`.

`get_runtime_class` và `destructor` là hàm rỗng vô hại, **không** phải
`unimplemented!()`. Đến 2026-09-12 hai hàm này còn panic; nghi (chưa tái hiện
được) là nguyên nhân giật hình sau khi autoregen 2.5.0 phát hành: nếu `CSTaskImp`
có lúc gọi trực tiếp một trong hai (ví dụ một đợt duyệt/bảo trì danh sách task
định kỳ), khác với `execute` luôn đi qua `catch_unwind` của người gọi, panic sẽ
unwind thẳng vào frame C++ của game vốn không có landing pad: hành vi không xác
định. Task này bị `Box::leak`, sống suốt đời DLL và không bao giờ bị duyệt hay huỷ,
nên không hàm nào cần làm gì thật.

## `task.rs`

- `wait_for_cs_task` tìm `CSTaskImp` bằng tra singleton theo tên của Dantelion2 (không
  qua bảng RVA), thử lại mỗi 500 ms, không bao giờ bỏ cuộc, và ghi **một** dòng
  lỗi nếu quá 15 giây (thường là game cần bản cập nhật mod, không chỉ khởi động
  chậm). Địa chỉ cache cho cả process để `common::reload` và tính năng của mod chờ
  một lần.
- `run_recurring_safe` bọc tác vụ trong `catch_unwind` và ghi `tag` của tính năng
  khi một tick panic (tick đó bị bỏ qua). Cần `panic = "unwind"` ở `[profile.release]`,
  vì `catch_unwind` không bắt được gì khi panic làm process abort.

Gom từ `autoregen::regen` (2026-09-11/12), thay cho kiểu cũ: chờ
`wait_for_system_init` rồi thử lại `CSTaskImp::wait_for_instance` qua
`SystemInitError::InvalidRva`. Tra singleton theo tên tránh hẳn lớp lỗi đó thay vì
thử lại cho qua. `tag` lấy từ bản `task.rs` của `sometweaks`/`risearcher`.
