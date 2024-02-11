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
    use insta::glob;
    use pretty_assertions::assert_eq;
    use rstest::{fixture, rstest};

    use super::*;

    const PATH_PROJECT_ROOT: &str = "../../";
    const PATH_TEST_SAMPLES: &str = "tests/sample_logs/*.*";

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
    fn round_trip() {
        glob!(PATH_PROJECT_ROOT, PATH_TEST_SAMPLES, |path| {
            let input = std::fs::read_to_string(path).unwrap();
            let rows_before = Data::try_from(&input[..]).unwrap();

            // Test single row
            for row_before in rows_before.rows() {
                let as_ron = ron::to_string(&row_before).unwrap();
                let row_after: LogRow = ron::from_str(&dbg!(as_ron)).unwrap();
                assert_eq!(&row_after, row_before);
            }

            // Test all rows
            let as_ron = ron::to_string(&rows_before).unwrap();
            let rows_after: Data = ron::from_str(&dbg!(as_ron)).unwrap();
            assert_eq!(rows_after, rows_before);
        });
    }
}
