# AnimCancel: antivirus gắn cờ trên VirusTotal

**Status 2026-10-19: đã xử lý, Nexus nhận bản `AnimCancelV1.toml` (VirusTotal còn 1 cờ ESET-NOD32).** Trước đó DLL bị
13 đến 16 hãng gắn cờ (phần lớn họ `Gen:Variant.Doina.132093`, engine BitDefender) và bị Nexus chặn.

## Nguyên nhân tìm được

1. **Chuỗi đường dẫn `AnimCancel.toml` trong DLL** (tên file cấu hình mod mở lúc chạy). Cùng một mã,
   đổi chuỗi này thành `AnimCancelV1.toml` (mã, log, mẫu nhúng) thì họ Doina biến mất: 14 xuống 6 cờ.
   Đổi riêng tên file log, tên DLL, số phiên bản, metadata PE hay câu log thì không có tác dụng; chuỗi
   `AnimCancel.toml` nằm trong câu log mà đường dẫn đã đổi cũng không bị bắt. Cơ chế chưa rõ (có thể độ giống
   với các mẫu `AnimCancel` đã bị gắn cờ trước đó).
2. **`GetKeyState` + `diag`**: `common::reload` từng gọi `eldenring::util::input::is_key_pressed`
   (`GetKeyState`); cộng với `common::diag` (liệt kê module, đọc phiên bản) thì thêm nhóm 4 hãng Avast, AVG, Avira,
   WithSecure (`MalwareX-gen`, `TR/W64.Agent`). Đổi sang `common::input::is_key_pressed` (`GetAsyncKeyState`
   cùng kiểm tra cửa sổ nền) thì nhóm này biến mất: RuneMultiplier 6 xuống 2 cờ (chỉ Cynet, ESET), trong khi
   `diag` vẫn nguyên. Mới đo trên hai mod; SpiritMultiplier 1.3.2 có cả hai mà chỉ 0 đến 1 cờ nên không phải quy luật chắc.

## Đã loại trừ (đừng thử lại)

- Thư viện TOML (`toml`, `toml_edit`, `basic-toml`, bộ đọc tự viết, phiên bản cũ và mới), menu `erui`,
  `crash.rs`, probe, mã cắt animation, mã migration, nhúng mẫu cấu hình, cờ build (`strip`, `opt-level`,
  `panic = "abort"`).
- Dựng lại `cancel.rs` / `weapon.rs` / `probe.rs` cho sạch (ghi bitfield có kiểu) không tự làm hết Doina:
  bản dựng lại chỉ sạch khi đồng thời đổi tên file cấu hình. Mã mới vẫn đáng giữ (ít `unsafe`, có test).

## Cần biết khi quét

- Kết quả VirusTotal **trôi theo thời gian**: các hãng thêm chữ ký rồi áp lên file cũ (SpeedMultiplier 1.1.0
  từng 0/68, quét lại 18/67 cùng một file). `scripts/vt-scan.py` tra theo hash nên trả kết quả cũ lưu sẵn; muốn
  số hiện tại phải bấm Reanalyze trên web.
- Các bản gần giống nhau có thể chênh vài cờ; nên quét đúng file sẽ phát hành.
- Cynet gắn cờ gần như mọi DLL hook game và ESET (`GameHack_AGen`) gắn cờ hầu hết mod; hai cờ này là mức nền.
