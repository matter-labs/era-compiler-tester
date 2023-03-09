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
        self.mode_filters.is_empty()
            || self
                .mode_filters
                .iter()
                .any(|filter| Self::normalize_mode(mode, filter).contains(filter))
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

    ///
    /// Checks if the mode is compatible with the specified filters.
    ///
    pub fn check_mode_filters(mode: &Mode, filters: &[String]) -> bool {
        if filters.is_empty() {
            return true;
        }
        for filter in filters.iter() {
            let mut split = filter.split_whitespace();
            let mode_filter = split.next().unwrap_or_default();
            let normalized_mode = Self::normalize_mode(mode, mode_filter);
            if !normalized_mode.contains(mode_filter) {
                continue;
            }

            let version = match split.next() {
                Some(version) => version,
                None => return true,
            };
            if let Ok(version_req) = semver::VersionReq::parse(version) {
                if mode.check_version(&version_req) {
                    return true;
                }
            }
        }
        false
    }

    ///
    /// Normalizes the mode according to the filter.
    ///
    fn normalize_mode(mode: &Mode, filter: &str) -> String {
        let mut current = mode.to_string();
        if filter.contains("Y*") {
            current = regex::Regex::new("Y[-+]")
                .expect("Always valid")
                .replace_all(current.as_str(), "Y*")
                .to_string();
        }
        if filter.contains("E*") {
            current = regex::Regex::new("E[-+]")
                .expect("Always valid")
                .replace_all(current.as_str(), "E*")
                .to_string();
        }
        if filter.contains("M^") {
            current = regex::Regex::new("M[3z]")
                .expect("Always valid")
                .replace_all(current.as_str(), "M^")
                .to_string();
        }
        if filter.contains("M*") {
            current = regex::Regex::new("M[0123sz]")
                .expect("Always valid")
                .replace_all(current.as_str(), "M*")
                .to_string();
        }
        if filter.contains("B*") {
            current = regex::Regex::new("B[0123]")
                .expect("Always valid")
                .replace_all(current.as_str(), "B*")
                .to_string();
        }
        current
    }
}
