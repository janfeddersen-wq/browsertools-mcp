//! Result pagination utilities.

/// Paginate a slice of items, returning the requested page.
pub fn paginate<T>(items: &[T], page: usize, page_size: usize) -> (&[T], PaginationInfo) {
    let total = items.len();
    let total_pages = if total == 0 {
        1
    } else {
        total.div_ceil(page_size)
    };

    let start = page.saturating_sub(1) * page_size;
    let end = (start + page_size).min(total);

    let slice = if start < total {
        &items[start..end]
    } else {
        &[]
    };

    let info = PaginationInfo {
        page,
        page_size,
        total_items: total,
        total_pages,
        has_next: page < total_pages,
        has_previous: page > 1,
    };

    (slice, info)
}

/// Metadata about a paginated result.
#[derive(Debug, Clone)]
pub struct PaginationInfo {
    pub page: usize,
    pub page_size: usize,
    pub total_items: usize,
    pub total_pages: usize,
    pub has_next: bool,
    pub has_previous: bool,
}

impl std::fmt::Display for PaginationInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Page {}/{} ({} items total)",
            self.page, self.total_pages, self.total_items
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pagination() {
        let items: Vec<i32> = (1..=25).collect();
        let (page, info) = paginate(&items, 1, 10);
        assert_eq!(page, &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
        assert!(info.has_next);
        assert!(!info.has_previous);
        assert_eq!(info.total_pages, 3);
    }

    #[test]
    fn test_last_page() {
        let items: Vec<i32> = (1..=25).collect();
        let (page, info) = paginate(&items, 3, 10);
        assert_eq!(page, &[21, 22, 23, 24, 25]);
        assert!(!info.has_next);
        assert!(info.has_previous);
    }
}
