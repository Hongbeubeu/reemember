# Grammar Data Format — AI Generation Guide

Use this document to instruct an AI to generate grammar content for Reemember.

---

## Prompt template (copy and customize)

```
Generate a grammar lesson for Reemember in the JSON format below.

Topic: [e.g. Present Perfect Tense]
Level: [A1 / A2 / B1 / B2 / C1 / C2]
Category: [e.g. tenses / conditionals / modal verbs / passive voice / articles / prepositions / word formation / sentence structure]
Number of exercises: [e.g. 10] — include at least one of each type if possible.

Output a valid JSON array (one item = one grammar document).
Follow the schema exactly. Do not add extra fields.
```

---

## Full JSON Schema

```json
[
  {
    "title": "string — name of the grammar topic",
    "category": "string — grammar category (tenses, modals, etc.)",
    "level": "string — CEFR level: A1 | A2 | B1 | B2 | C1 | C2",
    "content": "string — explanation text (plain text or markdown). Describe the rule, structure, usage notes.",
    "examples": ["string", "..."],
    "exercises": [ /* see exercise types below */ ]
  }
]
```

---

## Exercise Types

Every exercise object has `"type"` and `"explanation"`. The other fields depend on the type.

---

### 1. fill_blank
User types the missing word/phrase into a blank (`___`).

```json
{
  "type": "fill_blank",
  "prompt": "She ___ (live) here since 2020.",
  "answer": "has lived",
  "explanation": "Use Present Perfect with 'since' + point in time."
}
```

---

### 2. multiple_choice
User selects one correct option from a list.

```json
{
  "type": "multiple_choice",
  "prompt": "Which sentence is grammatically correct?",
  "options": [
    "I have went to Paris.",
    "I have gone to Paris.",
    "I gone to Paris.",
    "I did go to Paris yesterday."
  ],
  "answer": "I have gone to Paris.",
  "explanation": "'gone' is the past participle of 'go'."
}
```

Rules:
- Provide 3–5 options.
- `answer` must be one of the strings in `options` (exact match).

---

### 3. multiple_select
User selects ALL correct options (may be more than one).

```json
{
  "type": "multiple_select",
  "prompt": "Which sentences correctly use Present Perfect?",
  "options": [
    "I have seen him yesterday.",
    "She has just arrived.",
    "They have studied all morning.",
    "We went there last week."
  ],
  "answer": ["She has just arrived.", "They have studied all morning."],
  "explanation": "Present Perfect cannot be used with finished time expressions like 'yesterday' or 'last week'."
}
```

Rules:
- `answer` is an array of strings, each matching an item in `options`.

---

### 4. reorder
User rearranges shuffled words into a correct sentence.

```json
{
  "type": "reorder",
  "prompt": "Arrange the words to form a correct sentence:",
  "words": ["never", "I", "sushi", "have", "eaten"],
  "answer": "I have never eaten sushi",
  "explanation": "Frequency adverbs go between the auxiliary and the main verb."
}
```

Rules:
- `words` can be in any order — the UI will shuffle them automatically.
- `answer` is the complete correct sentence (case-insensitive matching).

---

### 5. matching
User matches each item in the left column to the correct item in the right column.

```json
{
  "type": "matching",
  "prompt": "Match each sentence beginning with its correct ending:",
  "pairs": [
    { "left": "I have lived here", "right": "for ten years." },
    { "left": "She has worked there", "right": "since last month." },
    { "left": "They have studied", "right": "all morning." }
  ],
  "explanation": "'For' is used with a duration; 'since' is used with a point in time."
}
```

Rules:
- `pairs` is an array of `{left, right}` objects that are correctly paired.
- The UI shuffles the right column automatically.
- Provide 3–6 pairs.

---

### 6. error_correction
User identifies the error in the sentence and types the corrected word or phrase.

```json
{
  "type": "error_correction",
  "prompt": "Find and correct the error:\n'I have went to the store yesterday.'",
  "answer": "gone",
  "explanation": "The past participle of 'go' is 'gone', not 'went'. Also, 'yesterday' signals Simple Past, not Present Perfect."
}
```

Rules:
- `answer` is the corrected word or short phrase only (not the whole sentence).

---

### 7. transformation
User rewrites a sentence using a given word (Cambridge Key Word Transformation style).

```json
{
  "type": "transformation",
  "prompt": "Complete the second sentence so that it has a similar meaning, using the word given. Do not change the word. Use 2–5 words.\n\nShe started working here in 2020. She still works here.\nSINCE\nShe _______________ 2020.",
  "answer": "has worked here since",
  "explanation": "Use Present Perfect + 'since' + the year to express an action that started in the past and continues now."
}
```

Rules:
- The given key word appears in ALL CAPS.
- `answer` is the missing words only (case-insensitive).

---

### 8. true_false_ng
User reads a passage and decides if a statement is True, False, or Not Given (IELTS-style).

```json
{
  "type": "true_false_ng",
  "passage": "The company was founded in 1990 and has grown significantly over the past three decades, expanding into 45 countries.",
  "statement": "The company has been operating for more than 30 years.",
  "answer": "true",
  "explanation": "From 1990 to the present is more than 30 years, so the statement is TRUE."
}
```

Rules:
- `answer` must be exactly `"true"`, `"false"`, or `"not_given"`.
- `passage` can be omitted if the question refers to general knowledge.

---

### 9. short_answer
User types a short open-ended answer. Multiple acceptable answers can be listed.

```json
{
  "type": "short_answer",
  "prompt": "Name three signal words commonly used with Present Perfect.",
  "answer": ["since", "for", "just", "already", "yet", "ever", "never", "recently", "lately"],
  "explanation": "These words signal that an action has a connection to the present moment."
}
```

Rules:
- `answer` is an array of accepted strings. The user's input must match any one of them (case-insensitive).
- Use this for questions with multiple valid short answers.

---

### 10. word_form
User provides the grammatically correct form of a given root word.

```json
{
  "type": "word_form",
  "prompt": "Use the correct form of the word in brackets to complete the sentence:\nHer ___ (ACHIEVE) in the competition impressed the judges.",
  "answer": "achievement",
  "explanation": "'Achievement' is the noun form of the verb 'achieve'."
}
```

Rules:
- The root word appears in ALL CAPS inside brackets.
- `answer` is the correct inflected/derived form (case-insensitive).

---

## Full Example Document

```json
[
  {
    "title": "Present Perfect Tense",
    "category": "tenses",
    "level": "B1",
    "content": "# Present Perfect\n\nUsed to connect the past to the present.\n\n**Structure:** Subject + have/has + past participle (V3)\n\n**When to use:**\n- Actions that happened at an unspecified time before now\n- Actions that started in the past and continue now\n- Recent events with present relevance\n\n**Signal words:** since, for, just, already, yet, ever, never, recently, lately",
    "examples": [
      "I have visited Paris three times.",
      "She has worked here since 2018.",
      "Have you ever tried sushi?",
      "They haven't finished the project yet."
    ],
    "exercises": [
      {
        "type": "fill_blank",
        "prompt": "She ___ (not / finish) her homework yet.",
        "answer": "hasn't finished",
        "explanation": "Negative Present Perfect: have/has + not + V3. 'Yet' at the end confirms Present Perfect."
      },
      {
        "type": "multiple_choice",
        "prompt": "Which sentence is correct?",
        "options": [
          "I have went there yesterday.",
          "I went there yesterday.",
          "I have go there yesterday.",
          "I did went there yesterday."
        ],
        "answer": "I went there yesterday.",
        "explanation": "'Yesterday' is a finished time reference → use Simple Past, not Present Perfect."
      },
      {
        "type": "multiple_select",
        "prompt": "Which sentences use Present Perfect correctly?",
        "options": [
          "I have seen him yesterday.",
          "She has just arrived.",
          "He has lived here for five years.",
          "We went to Spain last summer."
        ],
        "answer": ["She has just arrived.", "He has lived here for five years."],
        "explanation": "Present Perfect cannot be used with specific past time expressions like 'yesterday' or 'last summer'."
      },
      {
        "type": "reorder",
        "prompt": "Arrange the words into a correct sentence:",
        "words": ["ever", "Have", "you", "tried", "durian", "?"],
        "answer": "Have you ever tried durian?",
        "explanation": "In Present Perfect questions: Have/Has + subject + ever + V3?"
      },
      {
        "type": "matching",
        "prompt": "Match the sentence halves:",
        "pairs": [
          { "left": "I have lived in Hanoi", "right": "for five years." },
          { "left": "She has been a teacher", "right": "since 2015." },
          { "left": "They have just returned", "right": "from their trip." }
        ],
        "explanation": "'For' + duration; 'since' + point in time; 'just' = very recently."
      },
      {
        "type": "error_correction",
        "prompt": "Find and correct the error:\n'I have saw that movie twice.'",
        "answer": "seen",
        "explanation": "The past participle of 'see' is 'seen', not 'saw'."
      },
      {
        "type": "transformation",
        "prompt": "Complete the second sentence using the word given.\n\nThis is my first time in Japan.\nNEVER\nI _______________ Japan before.",
        "answer": "have never been to",
        "explanation": "\"It's my first time\" → \"I have never + V3\" expresses the same idea."
      },
      {
        "type": "true_false_ng",
        "passage": "Jane has been learning Vietnamese for two years and can now hold basic conversations.",
        "statement": "Jane started learning Vietnamese more than a year ago.",
        "answer": "true",
        "explanation": "Two years > one year, so the statement is TRUE."
      },
      {
        "type": "short_answer",
        "prompt": "What auxiliary verb is used in Present Perfect with 'he', 'she', or 'it'?",
        "answer": ["has"],
        "explanation": "Third-person singular subjects (he, she, it) use 'has' instead of 'have'."
      },
      {
        "type": "word_form",
        "prompt": "Use the correct form of the word in brackets:\nThe team's ___ (ACHIEVE) this year has been remarkable.",
        "answer": "achievement",
        "explanation": "'Achievement' is the noun form of the verb 'achieve', formed by adding the suffix '-ment'."
      }
    ]
  }
]
```

---

## Tips for AI Generation

1. **Variety** — include multiple exercise types per document (ideally all 10 for a comprehensive lesson).
2. **Difficulty progression** — start with easier exercises (fill_blank, multiple_choice) and end with harder ones (transformation, true_false_ng).
3. **Explanations** — always explain WHY the answer is correct, not just what it is.
4. **Realistic distractors** — for multiple_choice, make wrong options plausible common mistakes.
5. **Consistent topic** — all exercises in one document should reinforce the same grammar point.
6. **Passage quality** — for true_false_ng, write natural, paragraph-length passages (3–5 sentences).
7. **Answer format** — answers are case-insensitive in the app; write them in natural case for readability.
