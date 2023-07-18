//!
//! The compiler tests filters.
//!

use std::collections::HashSet;

use crate::compilers::mode::Mode;

///
/// The compiler tests filters.
///
pub struct Filters {
    /// The path filters.
    path_filters: Vec<String>,
    /// The mode filters.
    mode_filters: Vec<String>,
    /// The group filters.
    group_filters: HashSet<String>,
}

impl Filters {
    ///
    /// A shortcut constructor.
    ///
    pub fn new(
        path_filters: Vec<String>,
        mode_filters: Vec<String>,
        group_filters: Vec<String>,
    ) -> Self {
        Self {
            path_filters,
            mode_filters,
            group_filters: group_filters.into_iter().collect(),
        }
    }

    ///
    /// Check if the test path is compatible with the filters.
    ///
    pub fn check_test_path(&self, path: &str) -> bool {
        self.path_filters.is_empty()
            || self
                .path_filters
                .iter()
                .any(|filter| path.contains(&filter[..filter.find("::").unwrap_or(filter.len())]))
    }

    ///
    /// Check if the test case path is compatible with the filters.
    ///
    pub fn check_case_path(&self, path: &str) -> bool {
        self.path_filters.is_empty() || self.path_filters.iter().any(|filter| path.contains(filter))
    }

    ///
    /// Check if the mode is compatible with the filters.
    ///
    pub fn check_mode(&self, mode: &Mode) -> bool {
        mode.check_filters(self.mode_filters.as_slice())
    }

    ///
    /// Check if the test group is compatible with the filters.
    ///
    pub fn check_group(&self, group: &Option<String>) -> bool {
        if self.group_filters.is_empty() {
            return true;
        }
        if let Some(group) = group {
            !self.group_filters.contains(group)
        } else {
            false
        }
    }
}
