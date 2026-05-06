# Grammar Data Files

Mỗi file `.md` trong thư mục này là một bài ngữ pháp độc lập, có thể import trực tiếp vào Reemember.

## Tenses (14 files)

| File | Topic | Level |
|------|-------|-------|
| `tenses/00_overview.md` | **Tổng quan 13 thì + cheatsheet** | A1 |
| `tenses/01_present_simple.md` | Present Simple | A1 |
| `tenses/02_present_continuous.md` | Present Continuous | A1 |
| `tenses/03_present_perfect.md` | Present Perfect | B1 |
| `tenses/04_present_perfect_continuous.md` | Present Perfect Continuous | B1 |
| `tenses/05_past_simple.md` | Past Simple (+ used to / would) | A1 |
| `tenses/06_past_continuous.md` | Past Continuous | A2 |
| `tenses/07_past_perfect.md` | Past Perfect | B1 |
| `tenses/08_past_perfect_continuous.md` | Past Perfect Continuous | B2 |
| `tenses/09_future_will.md` | Future – Will | A2 |
| `tenses/10_future_going_to.md` | Future – Going To (+ was/were going to) | A2 |
| `tenses/11_future_continuous.md` | Future Continuous | B1 |
| `tenses/12_future_perfect.md` | Future Perfect | B2 |
| `tenses/13_future_perfect_continuous.md` | Future Perfect Continuous | C1 |

## Cách Import

Trong Reemember: tab **Grammar** → click **Import** → chọn file `.md`.

Mỗi file gồm:
- **Frontmatter** (`title`, `category`, `level`)
- **Nội dung Markdown** (cấu trúc, cách dùng, dấu hiệu, so sánh, lỗi thường gặp...)
- **Khối exercises** trong HTML comment `<!-- EXERCISES [...] -->` chứa JSON các bài tập.

## Cấu Trúc Mỗi Bài

Tất cả 13 bài thì đã được chuẩn hoá theo template:

1. **Cấu trúc** — bảng 3 dòng (khẳng định / phủ định / nghi vấn)
2. **Quy tắc biến đổi động từ** — khi liên quan (chia ngôi 3, -ing, -ed)
3. **Cách sử dụng** — bảng các tình huống + ví dụ
4. **Dấu hiệu nhận biết** — phân nhóm
5. **So sánh** với thì gần nhất (named "So Sánh: X vs Y")
6. **Stative verbs / Lưu ý đặc biệt** — khi liên quan
7. **Cách dùng khác / mở rộng** — khi có (used to/would, was/were going to, ...)
8. **Lỗi thường gặp** — bảng 3 cột (sai / đúng / giải thích)
9. **Exercises** — 12 bài tập đa dạng

## Format Schema

Xem `GRAMMAR_DATA_FORMAT.md` ở thư mục gốc dự án để biết schema đầy đủ và các loại exercise type.
