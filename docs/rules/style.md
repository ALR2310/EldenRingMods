# Quy tắc: văn phong và TODO.md

Tách từ `AGENTS.md` (2026-10-30). Đọc khi viết log, Features Nexus, CHANGELOG hoặc sửa `TODO.md`.

## Viết `TODO.md` (gốc repo) gọn

`TODO.md` chỉ trả lời "còn gì phải làm", không phải nơi ghi phân tích:

- Chỉ liệt kê việc **còn mở**, chia theo mod. Việc xong thì **xoá luôn**
  (không tick `[x]` rồi để lại; lịch sử đã có ở `HISTORY.md`).
- Mỗi việc **1 dòng** (~150 ký tự): việc gì + nguồn báo (người dùng, ngày) nếu
  có. Không bullet con. Lý do, bằng chứng, hướng đã thử, RVA/số liệu thì viết
  vào `mods/<mod>/docs/<chủ đề>.md` (mod chưa có `docs/` thì tạo) rồi link từ
  dòng TODO hoặc từ tiêu đề mod.
- Việc đã chốt không làm thì **xoá khỏi TODO** và ghi vào `HISTORY.md` (1 dòng)
  hoặc `docs/` của mod; TODO không có mục "Đã dừng".
- Khi gộp/xoá mục có chi tiết chưa nằm ở đâu khác, chuyển chi tiết sang
  `docs/` trước, không xoá mất.
- Cần ưu tiên thì thêm nhãn ngắn (`[bug]`, `[research]`, `[chờ phản hồi]`).

## Văn phong: log, danh sách Features, CHANGELOG (ngắn, đúng đối tượng)

Người dùng (2026-10-16) phải nhắc lại nhiều lần; viết đúng ngay từ đầu:

- **Log (`logger::log/warn/error`) viết cho dev, không cho người dùng cuối:**
  ngắn, kỹ thuật, dạng `<Thành phần>: <trạng thái>` (vd. `Menu: registered`,
  `SoloParamRepository: waiting for world`, `Reload: deferred, not in world`),
  không câu dài kiểu giải thích, không "..." kể lể. Không in danh sách ID hay
  dump nhiều dòng ở lần chạy bình thường (chỉ ghi số lượng, vd. `set on 8 Black
  Flame row(s)`); dump chi tiết chỉ để dò lỗi tạm rồi bỏ. Lỗi phải kèm lý do
  (`... failed (<reason>)`). Mod đơn tính năng không gắn tên mod vào dòng log.
  Cảnh báo cố ý nói với người dùng (vd. "check Nexus") là ngoại lệ.
- **Features trên `nexus_page.bbcode`:** mỗi bullet **một ý, một câu ngắn**
  (vd. `Longer Black Flame and Destined Death burn duration.`); ý khác thì tách
  bullet khác. Không nhồi số liệu, mặc định, "(adjustable)" hay giải thích vào
  một dòng.
- **`CHANGELOG.md`:** mỗi bullet ngắn nhất có thể, nêu tên tính năng/sửa lỗi,
  không kể chi tiết (giá trị mặc định, "last forever", cơ chế). Dạng
  `Added <X> and <Y> options.` / `Fixed <loại lỗi>.`; chi tiết để ở HISTORY và docs.
