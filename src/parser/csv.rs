use csv::{ReaderBuilder, StringRecord};

use crate::model::{Definition, Metadata, WordRecord};

use super::ParseError;

/// Represents a single row-level error that occurred during CSV parsing.
/// The parser doesn't fail on first error; instead it collects per-row errors
/// so partial imports can still be processed.
///
/// # Fields
/// - `row`: The CSV row number (1-indexed header, so data rows start at 2)
/// - `message`: Human-readable description of the error
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CsvRowError {
    pub row: usize,
    pub message: String,
}

/// Report from parsing a CSV file.
/// Contains both successfully parsed records and any errors encountered.
/// This allows partial import—if rows 1-5 are good but row 6 has a missing field,
/// you still get records 1-5 plus an error for row 6.
///
/// # Fields
/// - `records`: Successfully parsed and validated WordRecords
/// - `errors`: Row-level errors (missing fields, invalid values, etc.)
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CsvImportReport {
    pub records: Vec<WordRecord>,
    pub errors: Vec<CsvRowError>,
}

/// Parse CSV data into WordRecords with per-row error handling.
/// Supports both minimal and extended CSV formats as defined in the README.
/// - Minimal: `word,meaning_vi`
/// - Extended: `word,meaning_vi,phonetic,pos,examples,tags,created_at,review_count`
///
/// # Parameters
/// - `input`: CSV data as a string (UTF-8)
///
/// # Returns
/// - `Ok(CsvImportReport)`: Always succeeds (errors are collected, not fatal)
/// - `Err(ParseError::InvalidData(...))`: Header validation failed (missing required columns)
/// - `Err(ParseError::Csv(...))`: Underlying CSV reader error
///
/// # Parsing Rules
/// - Headers are case-insensitive (normalized to lowercase)
/// - Required columns: `word`, `meaning_vi`
/// - Optional columns: `phonetic`, `pos`, `examples`, `tags`, `created_at`, `review_count`
/// - Blank lines are skipped silently
/// - Multiple meanings split by `;`, examples/tags split by `|`
/// - All cells are trimmed of leading/trailing whitespace
///
/// # Example
/// ```ignore
/// let csv = "word,meaning_vi\ntest,a trial\nvalid,ok\n";
/// let report = parse_csv(csv)?;
/// assert_eq!(report.records.len(), 2);
/// assert_eq!(report.errors.len(), 0);
/// ```
pub fn parse_csv(input: &str) -> Result<CsvImportReport, ParseError> {
    // Create a CSV reader that trims all whitespace from cells
    let mut rdr = ReaderBuilder::new().trim(csv::Trim::All).from_reader(input.as_bytes());

    // Parse and validate CSV headers to find column positions
    let headers = rdr.headers()?.clone();
    let indexes = HeaderIndexes::new(&headers)?;

    // Initialize the report to collect records and errors
    let mut report = CsvImportReport::default();

    // Process each data row
    for (idx, row_result) in rdr.records().enumerate() {
        let row_number = idx + 2; // Header is row 1, first data row is row 2

        // Handle CSV read errors (e.g., invalid field count)
        let row = match row_result {
            Ok(row) => row,
            Err(err) => {
                report.errors.push(CsvRowError {
                    row: row_number,
                    message: err.to_string(),
                });
                continue; // Skip this row but keep processing others
            }
        };

        // Skip blank rows (all cells empty or whitespace)
        if is_blank_row(&row) {
            continue;
        }

        // Parse and validate this row
        match parse_row(&row, row_number, &indexes) {
            Ok(record) => report.records.push(record),
            Err(message) => report.errors.push(CsvRowError {
                row: row_number,
                message,
            }),
        }
    }

    Ok(report)
}

/// Internal structure to track column positions in the CSV header.
/// Used to extract fields in the correct order regardless of column order in the file.
struct HeaderIndexes {
    word: usize,
    meaning_vi: usize,
    phonetic: Option<usize>,
    pos: Option<usize>,
    examples: Option<usize>,
    tags: Option<usize>,
    created_at: Option<usize>,
    review_count: Option<usize>,
}

impl HeaderIndexes {
    /// Parse CSV headers and find column indices.
    /// Validates that required columns exist.
    ///
    /// # Parameters
    /// - `headers`: The CSV header row
    ///
    /// # Returns
    /// - `Ok(HeaderIndexes)`: Column positions found
    /// - `Err(ParseError::InvalidData(...))`: Missing required columns
    fn new(headers: &StringRecord) -> Result<Self, ParseError> {
        // Normalize headers to lowercase for case-insensitive matching
        let normalized: Vec<String> = headers.iter().map(normalize_header).collect();

        // Find required columns and fail if missing
        let word = find_required(&normalized, "word")?;
        let meaning_vi = find_required(&normalized, "meaning_vi")?;

        Ok(Self {
            word,
            meaning_vi,
            // Optional columns return None if not present
            phonetic: find_optional(&normalized, "phonetic"),
            pos: find_optional(&normalized, "pos"),
            examples: find_optional(&normalized, "examples"),
            tags: find_optional(&normalized, "tags"),
            created_at: find_optional(&normalized, "created_at"),
            review_count: find_optional(&normalized, "review_count"),
        })
    }
}

/// Find a required column by name in the header list.
/// If not found, returns an error.
///
/// # Parameters
/// - `headers`: Normalized (lowercase) header names
/// - `key`: The column name to find
///
/// # Returns
/// - `Ok(usize)`: The column index (0-based)
/// - `Err(ParseError::InvalidData(...))`: Column not found
fn find_required(headers: &[String], key: &str) -> Result<usize, ParseError> {
    find_optional(headers, key)
        .ok_or_else(|| ParseError::InvalidData(format!("missing required CSV header: {}", key)))
}

/// Find an optional column by name in the header list.
/// Returns None if the column is not present.
///
/// # Parameters
/// - `headers`: Normalized (lowercase) header names
/// - `key`: The column name to find
///
/// # Returns
/// Option containing the column index (0-based), or None if not found
fn find_optional(headers: &[String], key: &str) -> Option<usize> {
    headers.iter().position(|value| value == key)
}

/// Normalize a CSV header name for comparison.
/// Trims whitespace and converts to lowercase.
fn normalize_header(value: &str) -> String {
    value.trim().to_lowercase()
}

/// Parse a single CSV row into a WordRecord.
/// Handles field extraction, splitting, and validation at the row level.
///
/// # Parameters
/// - `row`: The CSV row data
/// - `row_number`: The row number (for error messages)
/// - `indexes`: Column position information from HeaderIndexes
///
/// # Returns
/// - `Ok(WordRecord)`: Successfully parsed and validated record
/// - `Err(String)`: Human-readable error message about what failed
fn parse_row(row: &StringRecord, row_number: usize, indexes: &HeaderIndexes) -> Result<WordRecord, String> {
    // Extract required fields
    let word = get_required_cell(row, indexes.word, "word")?;
    let meaning_vi = get_required_cell(row, indexes.meaning_vi, "meaning_vi")?;

    // Extract optional part-of-speech field
    let pos = indexes.pos.and_then(|idx| get_optional_cell(row, idx));

    // Split meanings by `;` and create Definition objects for each
    let definitions: Vec<Definition> = split_with_delimiter(&meaning_vi, ';')
        .into_iter()
        .map(|meaning| Definition {
            pos: pos.clone(),
            meaning,
        })
        .collect();

    // Validate that we have at least one definition with meaning
    if definitions.is_empty() {
        return Err("missing required field: definitions[].meaning".to_string());
    }

    // Extract optional fields
    let phonetic = indexes.phonetic.and_then(|idx| get_optional_cell(row, idx));
    let examples = indexes
        .examples
        .map(|idx| split_with_delimiter(&get_optional_cell(row, idx).unwrap_or_default(), '|'))
        .unwrap_or_default();
    let tags = indexes
        .tags
        .map(|idx| split_with_delimiter(&get_optional_cell(row, idx).unwrap_or_default(), '|'))
        .unwrap_or_default();
    let created_at = indexes.created_at.and_then(|idx| get_optional_cell(row, idx));

    // Parse review_count as unsigned integer, defaulting to 0 if not present
    let review_count = match indexes.review_count.and_then(|idx| get_optional_cell(row, idx)) {
        Some(value) => value.parse::<u32>().map_err(|_| {
            format!(
                "invalid review_count at row {}: expected unsigned integer, got '{}'",
                row_number, value
            )
        })?,
        None => 0,
    };

    // Construct the WordRecord
    let record = WordRecord {
        word,
        phonetic,
        definitions,
        examples,
        synonyms: vec![],
        antonyms: vec![],
        family_words: vec![],
        metadata: Metadata {
            tags,
            created_at,
            review_count,
        },
    };

    // Validate the complete record before returning
    record.validate().map_err(|err| err.to_string())?;
    Ok(record)
}

/// Extract a required cell value from a row and check that it's not empty.
///
/// # Parameters
/// - `row`: The CSV row
/// - `idx`: Column index
/// - `field_name`: Field name (for error messages)
///
/// # Returns
/// - `Ok(String)`: The trimmed cell value
/// - `Err(String)`: Error message if cell is empty or missing
fn get_required_cell(row: &StringRecord, idx: usize, field_name: &str) -> Result<String, String> {
    let value = row.get(idx).unwrap_or("").trim().to_string();
    if value.is_empty() {
        return Err(format!("missing required field: {}", field_name));
    }
    Ok(value)
}

/// Extract an optional cell value from a row.
/// Returns None if the cell is missing or empty after trimming.
///
/// # Parameters
/// - `row`: The CSV row
/// - `idx`: Column index
///
/// # Returns
/// Option containing the trimmed cell value, or None if empty/missing
fn get_optional_cell(row: &StringRecord, idx: usize) -> Option<String> {
    let value = row.get(idx).unwrap_or("").trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

/// Split a delimited string into a Vec of trimmed, non-empty strings.
/// Used to split multiple meanings, examples, or tags from a single CSV cell.
///
/// # Parameters
/// - `value`: The input string containing delimited items
/// - `delimiter`: Character used to separate items (`;` for meanings, `|` for examples/tags)
///
/// # Returns
/// Vec of trimmed strings, with empty items filtered out
///
/// # Example
/// ```ignore
/// assert_eq!(split_with_delimiter("a; b; c", ';'), vec!["a", "b", "c"]);
/// assert_eq!(split_with_delimiter("x | | y", '|'), vec!["x", "y"]);
/// ```
fn split_with_delimiter(value: &str, delimiter: char) -> Vec<String> {
    value
        .split(delimiter)
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(ToString::to_string)
        .collect()
}

/// Check if a CSV row is completely blank (all cells empty or whitespace).
/// Used to skip empty rows in the CSV data.
///
/// # Parameters
/// - `row`: The CSV row to check
///
/// # Returns
/// true if all cells are empty/whitespace, false otherwise
fn is_blank_row(row: &StringRecord) -> bool {
    row.iter().all(|value| value.trim().is_empty())
}



