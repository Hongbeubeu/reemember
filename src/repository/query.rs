/// Query builder and options for database queries.
/// Provides flexible query construction for sorting, filtering, and pagination.

/// Sorting field options for word queries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortBy {
    /// Sort alphabetically by word name (A-Z)
    Word,
    /// Sort by creation date (newest first)
    CreatedAt,
    /// Sort by review count (highest first, for words needing most review)
    ReviewCount,
}

impl SortBy {
    /// Convert SortBy variant to SQL ORDER BY clause.
    pub fn to_sql(&self) -> &'static str {
        match self {
            SortBy::Word => "words.word ASC",
            SortBy::CreatedAt => "words.created_at DESC",
            SortBy::ReviewCount => "words.review_count DESC",
        }
    }
}

/// Options for querying word records from the repository.
/// All fields are optional to allow flexible query construction.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct QueryOptions {
    /// How to sort results (defaults to no specific order)
    pub sort: Option<SortBy>,
    /// Maximum number of records to return
    pub limit: Option<usize>,
    /// Filter by a specific tag (returns only words with this tag)
    pub tag_filter: Option<String>,
    /// Filter to words belonging to a specific collection
    pub collection_id: Option<i64>,
    /// Filter to words belonging to a specific topic
    pub topic_id: Option<i64>,
}

impl QueryOptions {
    /// Create a new QueryOptions with default (no filtering, sorting, or limit).
    pub fn new() -> Self {
        QueryOptions::default()
    }

    /// Set the sort order for results.
    pub fn sort(mut self, sort: SortBy) -> Self {
        self.sort = Some(sort);
        self
    }

    /// Set a maximum limit on results.
    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Filter results to only include words with a specific tag.
    pub fn with_tag(mut self, tag: String) -> Self {
        self.tag_filter = Some(tag);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sort_by_to_sql() {
        assert_eq!(SortBy::Word.to_sql(), "words.word ASC");
        assert_eq!(SortBy::CreatedAt.to_sql(), "words.created_at DESC");
        assert_eq!(SortBy::ReviewCount.to_sql(), "words.review_count DESC");
    }

    #[test]
    fn test_query_options_builder() {
        let opts = QueryOptions::new()
            .sort(SortBy::Word)
            .limit(10)
            .with_tag("mindset".to_string());

        assert_eq!(opts.sort, Some(SortBy::Word));
        assert_eq!(opts.limit, Some(10));
        assert_eq!(opts.tag_filter, Some("mindset".to_string()));
    }
}

