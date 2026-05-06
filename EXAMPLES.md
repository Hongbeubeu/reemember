# Examples

Real-world code examples demonstrating the four principles. Each example shows what LLMs commonly do wrong and how to fix it.

---

## 1. Think Before Coding

### Example 1: Hidden Assumptions

**User Request:** "Add a feature to export user data"

**❌ What LLMs Do (Wrong Assumptions)**

```rust
fn export_users(format: &str) {
    let conn = Connection::open("app.db").unwrap();
    let mut stmt = conn.prepare("SELECT id, email, name FROM users").unwrap();
    let users: Vec<User> = stmt
        .query_map([], |row| Ok(User { id: row.get(0)?, email: row.get(1)?, name: row.get(2)? }))
        .unwrap()
        .map(|u| u.unwrap())
        .collect();

    if format == "json" {
        std::fs::write("users.json", serde_json::to_string(&users).unwrap()).unwrap();
    } else if format == "csv" {
        let mut wtr = csv::Writer::from_path("users.csv").unwrap();
        for u in &users { wtr.serialize(u).unwrap(); }
    }
}
```

**Problems:**
- Assumed it should export ALL users (what about pagination? privacy?)
- Assumed file location without asking
- Assumed which fields to include
- Panics with `.unwrap()` on every operation

**✅ What Should Happen (Surface Assumptions)**

```
Before implementing, I need to clarify:

1. **Scope**: Export all users or filtered subset? (privacy implications)
2. **Format**: You mentioned "export" — did you mean:
   - Download file in the Tauri frontend?
   - Write to disk at a user-chosen path (use rfd for file dialog)?
   - Return bytes from a Tauri command?
3. **Fields**: Which user fields? (some might be sensitive)
4. **Volume**: How many users typically? (affects whether to stream or load all)

Simplest approach: Tauri command that returns JSON string, frontend triggers download.
Would need more info for file-based exports.

What's your preference?
```

### Example 2: Multiple Interpretations

**User Request:** "Make the search faster"

**❌ What LLMs Do (Pick Silently)**

```rust
// Adds an LRU cache, full-text index, and async execution
// without asking which "faster" matters
static CACHE: Lazy<Mutex<LruCache<String, Vec<SearchResult>>>> =
    Lazy::new(|| Mutex::new(LruCache::new(NonZeroUsize::new(1000).unwrap())));

pub async fn search(query: &str) -> Vec<SearchResult> {
    if let Some(cached) = CACHE.lock().unwrap().get(query) {
        return cached.clone();
    }
    // ... 200 lines of optimisation
}
```

**✅ What Should Happen**

```
"Make the search faster" could mean:

1. **Faster response time** (< 50ms instead of 300ms)
   - Add a SQLite FTS5 virtual table
   - Estimated effort: 1–2 hours

2. **Handle more concurrent searches** (throughput)
   - Use a connection pool (r2d2 + rusqlite)
   - Estimated effort: 2–3 hours

3. **Faster perceived speed** (UX)
   - Debounce the input in the frontend JS
   - Show a loading spinner immediately
   - Estimated effort: 30 minutes

Current query does a LIKE scan with no index.
Which aspect matters most for your use case?
```

---

## 2. Simplicity First

### Example 1: Over-abstraction

**User Request:** "Add a function to calculate discount"

**❌ What LLMs Do (Overengineered)**

```rust
trait DiscountStrategy {
    fn calculate(&self, amount: f64) -> f64;
}

struct PercentageDiscount { percentage: f64 }
impl DiscountStrategy for PercentageDiscount {
    fn calculate(&self, amount: f64) -> f64 { amount * (self.percentage / 100.0) }
}

struct FixedDiscount { fixed_amount: f64 }
impl DiscountStrategy for FixedDiscount {
    fn calculate(&self, amount: f64) -> f64 { self.fixed_amount.min(amount) }
}

struct DiscountConfig {
    strategy: Box<dyn DiscountStrategy>,
    min_purchase: f64,
    max_discount: f64,
}

struct DiscountCalculator { config: DiscountConfig }
impl DiscountCalculator {
    fn apply_discount(&self, amount: f64) -> f64 {
        if amount < self.config.min_purchase { return 0.0; }
        self.config.strategy.calculate(amount).min(self.config.max_discount)
    }
}

// Requires 30+ lines of setup just to call it
```

**✅ What Should Happen (Simple)**

```rust
fn calculate_discount(amount: f64, percent: f64) -> f64 {
    amount * (percent / 100.0)
}

// Usage
let discount = calculate_discount(100.0, 10.0); // 10.0
```

**When to add complexity:** Only when you actually need multiple discount types. If that requirement comes later, refactor then.

### Example 2: Speculative Features

**User Request:** "Save user preferences to database"

**❌ What LLMs Do (Unnecessary Features)**

```rust
struct PreferenceManager {
    conn: Connection,
    cache: HashMap<i64, serde_json::Value>,
    validator: Box<dyn Validator>,
}

impl PreferenceManager {
    fn save(
        &mut self,
        user_id: i64,
        prefs: serde_json::Value,
        merge: bool,
        validate: bool,
        notify: bool,
    ) -> Result<(), AppError> {
        if validate {
            self.validator.validate(&prefs)?;
        }
        let final_prefs = if merge {
            let existing = self.get(user_id)?;
            merge_json(existing, prefs)
        } else {
            prefs
        };
        self.conn.execute(
            "UPDATE users SET preferences = ?1 WHERE id = ?2",
            params![final_prefs.to_string(), user_id],
        )?;
        if let Some(cached) = self.cache.get_mut(&user_id) { *cached = final_prefs.clone(); }
        if notify { self.notify_preference_change(user_id)?; }
        Ok(())
    }
    // Another 40 lines for a feature nobody asked for
}
```

**✅ What Should Happen (Just What Was Asked)**

```rust
fn save_preferences(conn: &Connection, user_id: i64, preferences: &serde_json::Value) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE users SET preferences = ?1 WHERE id = ?2",
        params![preferences.to_string(), user_id],
    )?;
    Ok(())
}
```

**Add later if needed:** Caching (when performance matters), validation (when bad data appears), merging (when requirement emerges).

---

## 3. Surgical Changes

### Example 1: Drive-by Refactoring

**User Request:** "Fix the bug where empty emails crash the validator"

**❌ What LLMs Do (Too Much)**

```diff
- fn validate_user(data: &HashMap<String, String>) -> Result<(), String> {
+ fn validate_user(data: &HashMap<String, String>) -> Result<(), ValidationError> {
+     // Validate email
-     // Check email
      let email = data.get("email").ok_or("Email required")?;
-     if !email.contains('@') {
-         return Err("Invalid email".to_string());
-     }
+     let email = email.trim();
+     if email.is_empty() { return Err(ValidationError::missing("email")); }
+     if !email.contains('@') || !email.contains('.') {
+         return Err(ValidationError::invalid("email"));
+     }
+     // Validate username
-     // Check username
      let username = data.get("username").ok_or("Username required")?;
+     let username = username.trim();
+     if username.len() < 3 { return Err(ValidationError::too_short("username")); }
+     if !username.chars().all(|c| c.is_alphanumeric()) {
+         return Err(ValidationError::invalid("username"));
+     }
      Ok(())
  }
```

**Problems:**
- Changed the error type to a new struct nobody asked for
- "Improved" email validation beyond the bug fix
- Added username length/character validation nobody requested
- Changed comments

**✅ What Should Happen (Surgical)**

```diff
  fn validate_user(data: &HashMap<String, String>) -> Result<(), String> {
      // Check email
-     let email = data.get("email").ok_or("Email required")?;
+     let email = data.get("email").map(|s| s.trim()).unwrap_or("");
+     if email.is_empty() { return Err("Email required".to_string()); }
      if !email.contains('@') {
          return Err("Invalid email".to_string());
      }
  
      // Check username
      let username = data.get("username").ok_or("Username required")?;
      Ok(())
  }
```

**Only changed:** The specific lines that fix empty email handling.

### Example 2: Style Drift

**User Request:** "Add logging to the upload function"

**❌ What LLMs Do (Reformat Everything)**

```diff
- fn upload_file(file_path: &str, destination: &str) -> bool {
+ fn upload_file(file_path: &Path, destination: &str) -> Result<(), UploadError> {
+     tracing::info!(path = %file_path.display(), "Starting upload");
      let data = match std::fs::read(file_path) {
          Ok(d) => d,
-         Err(_) => return false,
+         Err(e) => return Err(UploadError::Io(e)),
      };
      let client = reqwest::blocking::Client::new();
      let resp = client.post(destination)
          .body(data)
          .send();
      match resp {
-         Ok(r) if r.status().is_success() => true,
-         _ => false,
+         Ok(r) if r.status().is_success() => {
+             tracing::info!("Upload successful");
+             Ok(())
+         }
+         Ok(r) => Err(UploadError::Http(r.status())),
+         Err(e) => Err(UploadError::Request(e)),
      }
  }
```

**Problems:**
- Changed parameter type from `&str` to `&Path`
- Changed return type from `bool` to `Result<(), UploadError>` (new type)
- Switched from `eprintln!` to `tracing` (different logging system)

**✅ What Should Happen (Match Existing Style)**

```diff
  fn upload_file(file_path: &str, destination: &str) -> bool {
+     eprintln!("Starting upload: {file_path}");
      let data = match std::fs::read(file_path) {
          Ok(d) => d,
          Err(_) => return false,
      };
      let client = reqwest::blocking::Client::new();
      let resp = client.post(destination).body(data).send();
      match resp {
-         Ok(r) if r.status().is_success() => true,
+         Ok(r) if r.status().is_success() => {
+             eprintln!("Upload successful: {file_path}");
+             true
+         }
-         _ => false,
+         Ok(r) => {
+             eprintln!("Upload failed: {file_path}, status={}", r.status());
+             false
+         }
+         Err(e) => {
+             eprintln!("Upload error: {file_path}: {e}");
+             false
+         }
      }
  }
```

**Matched:** `&str` parameters, `bool` return, `eprintln!` for logging, existing match structure.

---

## 4. Goal-Driven Execution

### Example 1: Vague vs. Verifiable

**User Request:** "Fix the authentication system"

**❌ What LLMs Do (Vague Approach)**

```
I'll fix the authentication system by:
1. Reviewing the code
2. Identifying issues
3. Making improvements
4. Testing the changes

[Proceeds to make changes without clear success criteria]
```

**✅ What Should Happen (Verifiable Goals)**

```
To "fix authentication," I need to define success criteria.
What specific issue are we solving?

If the issue is "sessions remain valid after password change":

Plan:
1. Write test: change password → verify old token rejected by verify_session()
   Verify: test fails (reproduces bug)

2. Implement: invalidate all sessions for user_id on password update
   Verify: test passes

3. Check edge cases: multiple active sessions, concurrent requests
   Verify: additional tests pass

4. Verify no regression: cargo test — all existing auth tests green

What's the specific auth issue you're seeing?
```

### Example 2: Multi-Step with Verification

**User Request:** "Add rate limiting to the API"

**❌ What LLMs Do (All at Once)**

```rust
// Implements full rate limiting with DashMap, sliding window,
// per-user and per-IP limits, Redis backend, and metrics
// in one 400-line commit — no incremental verification
```

**✅ What Should Happen (Incremental with Verification)**

```
Plan for rate limiting in Tauri commands:

1. Add in-memory rate limiter (single command)
   Verify:
   - Test: call command 11 times → first 10 succeed, 11th returns Err("rate limit")
   - cargo test rate_limit — passes

2. Extract to a shared RateLimiter type, apply to all commands
   Verify:
   - cargo test — all existing command tests still pass
   - Rate limit applies to both search_words and import commands

3. Make limits configurable (per-command, via app config)
   Verify:
   - Test: config { search: 20/min, import: 5/min } applied correctly

Each step is independently verifiable.
Start with step 1?
```

### Example 3: Test-First Verification

**User Request:** "The sorting breaks when there are duplicate scores"

**❌ What LLMs Do (Fix Without Reproducing)**

```rust
// Immediately changes sort logic without confirming the bug
fn sort_scores(scores: &mut Vec<Score>) {
    scores.sort_by(|a, b| b.value.cmp(&a.value).then(a.name.cmp(&b.name)));
}
```

**✅ What Should Happen (Reproduce First)**

```rust
// 1. Write a test that reproduces the issue
#[test]
fn sort_scores_stable_on_duplicate_values() {
    let mut scores = vec![
        Score { name: "Charlie".into(), value: 100 },
        Score { name: "Alice".into(),   value: 100 },
        Score { name: "Bob".into(),     value: 90  },
    ];

    sort_scores(&mut scores);

    // Bug: order of Alice/Charlie was non-deterministic
    assert_eq!(scores[0].name, "Alice");   // tie broken by name asc
    assert_eq!(scores[1].name, "Charlie");
    assert_eq!(scores[2].name, "Bob");
}

// Verify: cargo test → FAILS (reproduces bug)

// 2. Now fix with a stable secondary key
fn sort_scores(scores: &mut Vec<Score>) {
    scores.sort_by(|a, b| b.value.cmp(&a.value).then(a.name.cmp(&b.name)));
}

// Verify: cargo test → PASSES
```

---

## Anti-Patterns Summary

| Principle | Anti-Pattern | Fix |
|-----------|-------------|-----|
| Think Before Coding | Assumes file path, fields, and format silently | List assumptions explicitly, ask for clarification |
| Simplicity First | `trait DiscountStrategy` + `Box<dyn>` for one call site | One `fn calculate_discount(amount, percent) -> f64` |
| Surgical Changes | Changes return type and parameter types while fixing a bug | Only change the lines that fix the reported issue |
| Goal-Driven | "I'll review and improve the auth code" | "Write a test that fails → make it pass → cargo test green" |

## Key Insight

The "overcomplicated" examples aren't obviously wrong — they follow Rust idioms and design patterns. The problem is **timing**: they add complexity before it's needed, which:

- Makes code harder to understand
- Introduces more surface area for bugs
- Takes longer to implement and review
- Harder to test in isolation

The "simple" versions are:
- Easier to understand
- Faster to implement
- Easier to test with `cargo test`
- Can be refactored later when complexity is actually needed

**Good code is code that solves today's problem simply, not tomorrow's problem prematurely.**
