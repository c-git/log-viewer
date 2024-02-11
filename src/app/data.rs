use std::collections::BTreeMap;

use anyhow::Context;

#[derive(serde::Deserialize, serde::Serialize, Default, Debug, PartialEq, Eq)]
pub struct Data {
    pub selected_row: Option<usize>,
    rows: Vec<LogRow>,
}

#[derive(serde::Deserialize, serde::Serialize, Default, Debug, PartialEq, Eq)]
pub struct LogRow {
    time: Option<String>,
    request_id: Option<String>,
    #[serde(rename = "otel.name")]
    otel_name: Option<String>,
    msg: Option<String>,

    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_json::Value>,
}

impl LogRow {
    const TEXT_FOR_EMPTY: &'static str = "[ --- ]";

    pub(crate) fn time(&self) -> &str {
        if let Some(val) = &self.time {
            val
        } else {
            Self::TEXT_FOR_EMPTY
        }
    }

    pub(crate) fn request_id(&self) -> &str {
        if let Some(val) = &self.request_id {
            val
        } else {
            Self::TEXT_FOR_EMPTY
        }
    }

    pub(crate) fn otel_name(&self) -> &str {
        if let Some(val) = &self.otel_name {
            val
        } else {
            Self::TEXT_FOR_EMPTY
        }
    }

    pub(crate) fn msg(&self) -> &str {
        if let Some(val) = &self.msg {
            val
        } else {
            Self::TEXT_FOR_EMPTY
        }
    }
}

impl Data {
    pub fn rows(&self) -> &[LogRow] {
        &self.rows
    }
}

impl TryFrom<&str> for Data {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let mut result = Data::default();
        for (i, line) in value.lines().enumerate() {
            result.rows.push(
                serde_json::from_str(line)
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
            for (i, row_before) in rows_before.rows().iter().enumerate() {
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

    fn create_log_row_no_extra() -> LogRow {
        LogRow {
            time: Some("time value".to_string()),
            request_id: None,
            otel_name: Some("otel value".to_string()),
            msg: None,
            extra: BTreeMap::new(),
        }
    }

    fn create_log_row_with_extra() -> LogRow {
        let mut result = LogRow {
            time: Some("time value".to_string()),
            request_id: None,
            otel_name: Some("otel value".to_string()),
            msg: None,
            extra: BTreeMap::new(),
        };
        result.extra.insert("key".into(), "value".into());
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
}
