use std::{
    borrow::Cow,
    collections::{BTreeMap, BTreeSet},
};

use anyhow::Context;
use data_iter::DataIter;
use filter::{FieldSpecifier, FilterConfig};
use log::warn;

use super::calculate_hash;
mod data_iter;
pub mod filter;

// TODO 1: Create access method that returns enum indicating value or not
// TODO 2: Create an iterator that allows for selection of first fields to show if present

#[derive(serde::Deserialize, serde::Serialize, Default, Debug, PartialEq, Eq)]
#[serde(default)]
pub struct Data {
    pub selected_row: Option<usize>,
    pub filter: Option<FilterConfig>,
    rows: Vec<LogRow>,
    filtered_rows: Option<Vec<usize>>,
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

impl<'a> FieldContent<'a> {
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

    pub fn as_slice(&mut self, common_fields: &BTreeSet<String>) -> &[(String, String)] {
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
}

impl Data {
    pub fn rows_iter(&self) -> impl Iterator<Item = &'_ LogRow> {
        DataIter::new(self)
    }

    pub fn selected_row_data_as_slice(
        &mut self,
        common_fields: &BTreeSet<String>,
    ) -> Option<&[(String, String)]> {
        let selected_row_index = self.selected_row?;
        Some(self.rows[selected_row_index].as_slice(common_fields))
    }

    pub fn move_selected_to_next(&mut self) {
        // TODO 1: Fix index values used
        if let Some(selected) = self.selected_row.as_mut() {
            if *selected < self.rows.len() - 1 {
                *selected += 1;
            } else {
                // Do nothing already on last row
            }
        } else {
            self.move_selected_to_last();
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
            self.move_selected_to_first()
        }
    }

    pub fn move_selected_to_first(&mut self) {
        if !self.rows.is_empty() {
            self.selected_row = Some(0)
        } else {
            // No rows to select
        }
    }

    pub fn move_selected_to_last(&mut self) {
        if !self.rows.is_empty() {
            self.selected_row = Some(self.rows.len() - 1);
        } else {
            // No rows to select
        }
    }

    pub fn is_filtered(&self) -> bool {
        self.filtered_rows.is_some()
    }

    pub fn unfilter(&mut self) {
        self.selected_row = None;
        self.filtered_rows = None;
    }

    pub fn apply_filter(&mut self, common_fields: &BTreeSet<String>) {
        if let Some(filter) = self.filter.as_ref() {
            self.selected_row = None;
            self.filtered_rows = Some(
                self.rows
                    .iter_mut()
                    .enumerate()
                    .filter_map(|(i, row)| {
                        if is_included(row, filter, common_fields) {
                            Some(i)
                        } else {
                            None
                        }
                    })
                    .collect(),
            );
        } else {
            warn!("Apply called but no filter is available")
        }
    }
}

fn is_included(
    row: &mut LogRow,
    filter: &filter::FilterConfig,
    common_fields: &BTreeSet<String>,
) -> bool {
    let FilterConfig {
        search_key,
        filter_on,
        comparator,
        is_case_sensitive,
    } = filter;
    let fields_and_values = row.as_slice(common_fields);
    let search_key = if *is_case_sensitive {
        search_key
    } else {
        &search_key.to_lowercase()
    };
    let mut iter = fields_and_values.iter().map(|(k, v)| {
        if *is_case_sensitive {
            (Cow::Borrowed(k), Cow::Borrowed(v))
        } else {
            (Cow::Owned(k.to_lowercase()), Cow::Owned(v.to_lowercase()))
        }
    });

    match filter_on {
        filter::FilterOn::Any => {
            iter.any(|(_, value)| comparator.apply(search_key, value.as_str()))
        }
        filter::FilterOn::Field(FieldSpecifier { name }) => {
            let name = if *is_case_sensitive {
                name
            } else {
                &name.to_lowercase()
            };
            iter.any(|(field_name, value)| {
                name == field_name.as_str() && comparator.apply(search_key, value.as_str())
            })
        }
    }
}

impl TryFrom<&str> for LogRow {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Ok(Self {
            data: serde_json::from_str(value)?,
            cached_display_list: None,
        })
    }
}

impl TryFrom<&str> for Data {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let mut result = Data::default();
        for (i, line) in value.lines().enumerate() {
            result.rows.push(
                LogRow::try_from(line)
                    .with_context(|| format!("failed to parse line {}", i + 1))?,
            );
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use std::fmt::{Debug, Display};

    use insta::glob;
    use pretty_assertions::assert_eq;
    use rstest::{fixture, rstest};

    use super::*;

    const PATH_PROJECT_ROOT: &str = "../../";
    const PATH_TEST_SAMPLES: &str = "tests/sample_logs/*.*";

    /// Formats to test serializing with
    ///
    /// Even though in the application only RON is used for serialization we do round
    /// trip testing on Json because it helps identify problems that are format specific
    /// and avoid unnecessary debugging
    enum SerdeFormat {
        Ron,
        Json,
    }

    #[fixture]
    pub(crate) fn insta_settings() -> insta::Settings {
        let mut result = insta::Settings::clone_current();
        let cwd = std::env::current_dir().expect("failed to get cwd");
        let path = cwd.join("tests").join("snapshots");
        result.set_snapshot_path(path);
        result
    }

    #[rstest]
    fn deserialize_rows_from_string(insta_settings: insta::Settings) {
        glob!(PATH_PROJECT_ROOT, PATH_TEST_SAMPLES, |path| {
            let input = std::fs::read_to_string(path).unwrap();
            let data = Data::try_from(&input[..]).unwrap();
            let log_filename = path.file_name().unwrap().to_string_lossy().to_string();
            insta_settings
                .bind(|| insta::assert_ron_snapshot!(format!("{log_filename}_ron"), data));
            insta_settings
                .bind(|| insta::assert_yaml_snapshot!(format!("{log_filename}_yaml"), data));
            insta_settings
                .bind(|| insta::assert_debug_snapshot!(format!("{log_filename}_debug"), data));
        });
    }

    #[rstest]
    #[case::ron(SerdeFormat::Ron)]
    #[case::json(SerdeFormat::Json)]
    fn round_trip_from_samples(#[case] serde_format: SerdeFormat) {
        // Function needed because rustfmt doesn't play nicely with formatting long strings in macros
        fn fail_with(path: impl Debug, row: usize, e: impl Debug, s: impl Display) -> LogRow {
            panic!(
                "failed to deserialize back into struct.\nFile: {path:?}\nRow: {row}\nError: {e:?}\nSerialized Data: {s}"
            )
        }

        glob!(PATH_PROJECT_ROOT, PATH_TEST_SAMPLES, |path| {
            let input = std::fs::read_to_string(path).unwrap();
            let rows_before = Data::try_from(&input[..]).unwrap();

            // Test individual rows
            for (i, row_before) in rows_before.rows_iter().enumerate() {
                let as_string = match serde_format {
                    SerdeFormat::Ron => ron::to_string(&row_before).unwrap(),
                    SerdeFormat::Json => serde_json::to_string(&row_before).unwrap(),
                };

                let row_after: LogRow = match serde_format {
                    SerdeFormat::Ron => ron::from_str(&as_string)
                        .unwrap_or_else(|e| fail_with(path, i, e, as_string)),
                    SerdeFormat::Json => serde_json::from_str(&as_string)
                        .unwrap_or_else(|e| fail_with(path, i, e, as_string)),
                };
                assert_eq!(&row_after, row_before);
            }

            // Test composition of all rows
            let as_string = match serde_format {
                SerdeFormat::Ron => ron::to_string(&rows_before).unwrap(),
                SerdeFormat::Json => serde_json::to_string(&rows_before).unwrap(),
            };
            let rows_after: Data = match serde_format {
                SerdeFormat::Ron => ron::from_str(&dbg!(as_string)).unwrap(),
                SerdeFormat::Json => serde_json::from_str(&dbg!(as_string)).unwrap(),
            };
            assert_eq!(rows_after, rows_before);
        });
    }

    pub fn create_log_row_no_extra() -> LogRow {
        let mut result = LogRow::default();
        result.data.insert("time".into(), "time value".into());
        result
            .data
            .insert("otel.name".into(), "HTTP GET /status".into());
        result
    }

    pub fn create_log_row_with_extra() -> LogRow {
        let mut result = create_log_row_no_extra();
        result.data.insert("http.status_code".into(), 200.into());
        result
    }

    #[rstest]
    fn round_trip_from_manual(
        #[values(SerdeFormat::Ron, SerdeFormat::Json)] serde_format: SerdeFormat,
        #[values(create_log_row_no_extra(), create_log_row_with_extra())] before: LogRow,
    ) {
        let as_string = match serde_format {
            SerdeFormat::Ron => ron::to_string(&before).unwrap(),
            SerdeFormat::Json => serde_json::to_string(&before).unwrap(),
        };
        println!("Serialized data:\n{as_string}");
        let after: LogRow = match serde_format {
            SerdeFormat::Ron => ron::from_str(&as_string).unwrap(),
            SerdeFormat::Json => serde_json::from_str(&as_string).unwrap(),
        };
        assert_eq!(after, before);
    }

    // TODO 1: Add tests for filters
}
