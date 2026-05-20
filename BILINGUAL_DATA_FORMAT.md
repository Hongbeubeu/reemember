# Bilingual Books Data Format

Use JSON files to import bilingual reading articles. The root value is an array of articles.

```json
[
  {
    "book": "Everyday Stories",
    "level": "A2",
    "title": "A Rainy Morning",
    "paragraphs": [
      [
        { "en": "When Lina woke up,", "vi": "Khi Lina th\u1ee9c d\u1eady," },
        { "en": "rain was tapping gently on the window.", "vi": "m\u01b0a \u0111ang g\u00f5 nh\u1eb9 l\u00ean c\u1eeda s\u1ed5." }
      ]
    ],
    "structures": [
      {
        "pattern": "When + past simple, past continuous",
        "example": "When Lina woke up, rain was tapping gently on the window.",
        "note": "Dùng when để đặt mốc thời gian trong quá khứ, sau đó dùng quá khứ tiếp diễn cho hành động đang xảy ra tại thời điểm đó."
      }
    ]
  }
]
```

Required fields:

- `title`: article title.
- `paragraphs`: array of paragraphs. Each paragraph is an array of phrase segments.
- `paragraphs[].en`: English phrase.
- `paragraphs[].vi`: Vietnamese meaning for that English phrase.

Optional fields:

- `book`: book collection name / đầu sách. Defaults to `Bilingual Books`.
- `level`: CEFR level or any short label.
- `structures`: English sentence structures found in the article. Each `example` should match a full English sentence in `paragraphs`; matching sentences are shown in bold and can be clicked to jump to the grammar explanation.
- `structures[].note`: short explanation for the structure.

Import behavior:

- Open `Bilingual Books`.
- Choose `Import file(s) (.json)` and select one or more JSON files.
- Existing articles are updated when both `book` and `title` match.
- Manifest sync also supports bilingual files with kind `bilingual`, `bilingual_book`, or `bilingual-books`.

Organization:

- Each distinct `book` value is shown as one collection in the sidebar.
- Each article object is one chapter/article inside that book collection.

Manifest example:

```json
{
  "version": "2026-05-12",
  "files": [
    {
      "kind": "bilingual",
      "name": "Sample bilingual stories",
      "url": "./bilingual_data/sample_articles.json"
    }
  ]
}
```
