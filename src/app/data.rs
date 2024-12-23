use std::{
    borrow::Cow,
    collections::{BTreeMap, BTreeSet},
};

use anyhow::Context;
use data_iter::DataIter;
use filter::{FieldSpecifier, FilterConfig};
use log::warn;
use serde_json::Value;

use super::calculate_hash;
mod data_iter;
pub mod filter;

type LogRowIdxFieldName<'a> = Option<&'a String>;
type RowSlice<'a> = &'a [(String, String)];

#[derive(serde::Deserialize, serde::Serialize, Default, Debug, PartialEq, Eq)]
#[serde(default)]
pub struct Data {
    pub selected_row: Option<usize>,
    pub filter: Option<FilterConfig>,
    rows: Vec<LogRow>,
    filtered_rows: Option<Vec<usize>>,
    applied_filter: Option<FilterConfig>,
}

#[derive(serde::Deserialize, serde::Serialize, Default, Debug, PartialEq, Eq, Clone)]
pub struct LogRow {
    data: BTreeMap<String, serde_json::Value>,
    #[serde(skip)]
    cached_display_list: Option<CachedDisplayInfo>,
}

#[derive(Default, Debug, PartialEq, Eq, Clone)]
struct CachedDisplayInfo {
    data: Vec<(String, String)>,
    common_fields_hash: u64,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum FieldContent<'a> {
    Present(&'a serde_json::Value),
    Missing,
}

impl FieldContent<'_> {
    pub const TEXT_FOR_EMPTY: &'static str = "[ --- ]";

    pub fn display(&self) -> String {
        // TODO 4: Revisit implementation to see if a more efficient way can be found (should be benchmarked to see if it's worth it)
        match self {
            FieldContent::Present(val) => {
                if let Some(s) = val.as_str() {
                    s.to_string()
                } else {
                    val.to_string()
                }
            }
            FieldContent::Missing => Self::TEXT_FOR_EMPTY.to_string(),
        }
    }
}

impl LogRow {
    pub(crate) fn field_value(&self, field_name: &str) -> FieldContent<'_> {
        match self.data.get(field_name) {
            Some(value) => FieldContent::Present(value),
            None => FieldContent::Missing,
        }
    }

    pub fn as_slice(&mut self, common_fields: &BTreeSet<String>) -> RowSlice<'_> {
        self.ensure_cache_is_populated(common_fields);

        &self
            .cached_display_list
            .get_or_insert_with(|| {
                unreachable!("should have been initialized above if it was empty")
            })
            .data
    }

    fn ensure_cache_is_populated(&mut self, common_fields: &BTreeSet<String>) {
        let common_fields_hash = calculate_hash(common_fields);

        if let Some(cache) = self.cached_display_list.as_ref() {
            if cache.common_fields_hash != common_fields_hash {
                // Hash changed cache no longer valid
                self.cached_display_list = None;
            }
        }

        if self.cached_display_list.is_none() {
            // Build data for sorting
            let mut data: Vec<(bool, (String, String))> = self
                .data
                .iter()
                .map(|(k, v)| {
                    (
                        common_fields.contains(k),
                        (k.clone(), FieldContent::Present(v).display()),
                    )
                }) // Use display to keep formatting consistent
                .collect();

            // Add separator for common fields
            data.push((
                true,
                (
                    format!(" {}", FieldContent::TEXT_FOR_EMPTY), // prefixed with a leading space so it should end up at top of the common section
                    FieldContent::TEXT_FOR_EMPTY.to_string(),
                ),
            ));

            // Sort data based on common fields (to group them at the bottom)
            data.sort_unstable();

            // Remove extra info that was used for sorting
            let data = data.into_iter().map(|x| x.1).collect();

            self.cached_display_list = Some(CachedDisplayInfo {
                data,
                common_fields_hash,
            });
        }
    }

    /// Adds the value passed at the key if the key does not exist
    fn or_insert(&mut self, key: String, value: Value) {
        self.data.entry(key).or_insert(value);
    }
}

impl Data {
    pub fn rows_iter(&self) -> impl Iterator<Item = &'_ LogRow> {
        DataIter::new(self)
    }

    pub fn len(&self) -> usize {
        if let Some(filtered) = self.filtered_rows.as_ref() {
            filtered.len()
        } else {
            self.rows.len()
        }
    }

    pub fn total_len_unfiltered(&self) -> usize {
        self.rows.len()
    }

    /// If the points are not filtered returns the input otherwise translates it from the filtered array
    fn get_real_index(&self, index: usize) -> usize {
        if let Some(filtered) = self.filtered_rows.as_ref() {
            filtered[index]
        } else {
            index
        }
    }

    pub fn selected_row_data_as_slice(
        &mut self,
        common_fields: &BTreeSet<String>,
    ) -> Option<RowSlice<'_>> {
        let selected_row_index = self.selected_row?;
        let real_index = self.get_real_index(selected_row_index);
        Some(self.rows[real_index].as_slice(common_fields))
    }

    pub fn selected_row_data_as_slice_with_filter_matching_fields(
        &mut self,
        common_fields: &BTreeSet<String>,
    ) -> Option<(RowSlice<'_>, Vec<usize>)> {
        // Collect other needed info before taking mutable borrow to appease the borrow checker (couldn't find another readable way)
        let is_filtered = self.is_filtered();
        let filter = if is_filtered {
            self.applied_filter.clone()
        } else {
            None
        };
        let row_slice = self.selected_row_data_as_slice(common_fields)?;
        let matching_fields = if is_filtered {
            if let Some(filter) = filter.as_ref() {
                matching_fields(row_slice, filter).unwrap_or_default()
            } else {
                debug_assert!(false, "No filter but is_filtered is true?");
                Vec::new()
            }
        } else {
            Vec::new()
        };
        Some((row_slice, matching_fields))
    }

    pub fn move_selected_to_next(&mut self) {
        let n = self.len();
        if let Some(selected) = self.selected_row.as_mut() {
            if *selected < n - 1 {
                *selected += 1;
            } else {
                // Do nothing already on last row
            }
        } else {
            self.move_selected_to_first();
        }
    }

    pub fn move_selected_to_prev(&mut self) {
        if let Some(selected) = self.selected_row.as_mut() {
            if *selected > 0 {
                *selected -= 1;
            } else {
                // Do nothing already on first row
            }
        } else {
            self.move_selected_to_last()
        }
    }

    pub fn move_selected_to_first(&mut self) {
        if self.len() > 0 {
            self.selected_row = Some(0)
        } else {
            // No rows to select
        }
    }

    pub fn move_selected_to_last(&mut self) {
        let n = self.len();
        if n > 0 {
            self.selected_row = Some(n - 1);
        } else {
            // No rows to select
        }
    }

    pub fn is_filtered(&self) -> bool {
        debug_assert_eq!(self.applied_filter.is_some(), self.filtered_rows.is_some());
        self.filtered_rows.is_some()
    }

    pub fn unfilter(&mut self) {
        let previous_real_index_selected = self.selected_row.map(|x| self.get_real_index(x));
        self.filtered_rows = None;
        self.applied_filter = None;
        if let Some(old_selected) = previous_real_index_selected {
            self.selected_row = Some(old_selected);
        }
    }

    pub fn apply_filter(&mut self, common_fields: &BTreeSet<String>) {
        if let Some(filter) = self.filter.as_ref() {
            let previous_real_index_selected = self.selected_row.map(|x| self.get_real_index(x));

            self.applied_filter = self.filter.clone();
            self.filtered_rows = Some(
                self.rows
                    .iter_mut()
                    .enumerate()
                    .filter_map(|(i, row)| {
                        if matching_fields(row.as_slice(common_fields), filter).is_some() {
                            Some(i)
                        } else {
                            None
                        }
                    })
                    .collect(),
            );
            if let Some(old_selected) = previous_real_index_selected {
                if let Some(filtered) = self.filtered_rows.as_ref() {
                    self.selected_row = filtered.iter().position(|&idx| idx == old_selected);
                }
            }
        } else {
            warn!("Apply called but no filter is available")
        }
    }

    pub fn take_config(&mut self, other: &mut Self, common_fields: &BTreeSet<String>) {
        let is_filtered = other.is_filtered();
        self.filter = other.filter.take();
        if is_filtered {
            self.apply_filter(common_fields);
        }
        if let Some(i) = other.selected_row {
            if i < self.len() {
                self.selected_row = Some(i);
            }
        }
    }

    pub fn applied_filter_display(&self) -> String {
        let Some(FilterConfig {
            search_key,
            filter_on,
            is_case_sensitive,
            comparator,
        }) = self.applied_filter.as_ref()
        else {
            debug_assert!(false, "We really shouldn't end up here");
            return "No Filter Applied".to_string();
        };
        format!(
            "Search Key: {search_key} | Filter On: {filter_on} | Case Sensitive: {} | Comparator: {comparator}", 
            if *is_case_sensitive {
                "Yes"
            } else {
                "No"
            })
    }
}

/// If the slice of fields and values matches the filter then the indices of the fields that match are returned or None if it does not match
fn matching_fields(fields_and_values: RowSlice<'_>, filter: &FilterConfig) -> Option<Vec<usize>> {
    let FilterConfig {
        search_key,
        filter_on,
        comparator,
        is_case_sensitive,
    } = filter;
    let search_key = if *is_case_sensitive {
        search_key
    } else {
        &search_key.to_lowercase()
    };
    let iter = fields_and_values
        .iter()
        .map(|(k, v)| {
            if *is_case_sensitive {
                (Cow::Borrowed(k), Cow::Borrowed(v))
            } else {
                (Cow::Owned(k.to_lowercase()), Cow::Owned(v.to_lowercase()))
            }
        })
        .enumerate();
    let result: Vec<usize> = match filter_on {
        filter::FilterOn::Any => iter
            .filter_map(|(i, (_, value))| comparator.apply(search_key, value.as_str()).then_some(i))
            .collect(),
        filter::FilterOn::Field(FieldSpecifier { name }) => {
            let name = if *is_case_sensitive {
                name
            } else {
                &name.to_lowercase()
            };
            iter.filter_map(|(i, (field_name, value))| {
                (name == field_name.as_str() && comparator.apply(search_key, value.as_str()))
                    .then_some(i)
            })
            .collect()
        }
    };

    if result.is_empty() {
        None
    } else {
        Some(result)
    }
}

impl TryFrom<(LogRowIdxFieldName<'_>, usize, &str)> for LogRow {
    type Error = anyhow::Error;

    fn try_from(
        (log_row_idx_key, row_idx_val, value): (LogRowIdxFieldName<'_>, usize, &str),
    ) -> Result<Self, Self::Error> {
        let mut result = Self {
            data: serde_json::from_str(value)?,
            cached_display_list: None,
        };
        if let Some(key) = log_row_idx_key {
            result.or_insert(key.to_string(), row_idx_val.into());
        }
        Ok(result)
    }
}

impl TryFrom<(LogRowIdxFieldName<'_>, &str)> for Data {
    type Error = anyhow::Error;

    fn try_from(
        (log_row_idx_key, value): (LogRowIdxFieldName<'_>, &str),
    ) -> Result<Self, Self::Error> {
        let mut result = Data::default();
        for (i, line) in value.lines().enumerate() {
            result.rows.push(
                LogRow::try_from((log_row_idx_key, i, line))
                    .with_context(|| format!("failed to parse line {}", i + 1))?,
            );
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests;
