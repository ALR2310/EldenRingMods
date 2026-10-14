# Crate `common`: vì sao một crate duy nhất

**Status 2026-10-13: một crate (`shared/`, package `common`) cho mọi mod; mô-đun phụ thuộc `eldenring` và mô-đun thuần Windows/ini nằm chung. Code ở `shared/src/lib.rs`.**

## Quyết định

Mỗi mod là một `cdylib`, `common` được link tĩnh vào từng DLL, nên các static của
nó bị nhân bản trong mỗi DLL: các mod chia sẻ **mã nguồn**, không chia sẻ trạng
thái lúc chạy.

Các module dùng struct của game (`task_hook`, `alloc_hook`, `task`, `player`,
`reload`, `announce`, `params`, `anim`, `erui`) từng nằm ở một crate riêng tên
`engine` (2026-09-14), để mod như `weightmultiplier`, chỉ patch mã thô và không
đụng struct engine nào, khỏi phụ thuộc `eldenring` / `fromsoftware-shared`. Cùng
ngày đó gộp lại vào `common`:

- `lto = true` và `codegen-units = 1` (`[profile.release]` ở `Cargo.toml` gốc) đã
  loại phần code không dùng khỏi `.dll` cuối cùng, nên mod không dùng module
  engine không bị phình.
- Chi phí thật của việc tách chỉ là tổ chức (hai thư mục phải nhớ) và một thiệt
  thòi nhỏ: build sạch một mod không dùng module engine vẫn phải biên dịch
  `eldenring` một lần. Không đáng cho hai crate.

## Tham chiếu

- `shared/src/lib.rs`: danh mục module.
- `shared/docs/*.md`: ghi chú thiết kế từng module.
- `mods/autoregen/docs/task_and_startup.md`: lịch sử tách `task_hook` / `alloc_hook` khỏi autoregen.
