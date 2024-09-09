use std::fmt::{Debug, Display};

use filter::Comparator;
use insta::glob;
use pretty_assertions::assert_eq;
use rstest::{fixture, rstest};
use strum::IntoEnumIterator;

use crate::app::data_display_options::DataDisplayOptions;

use super::*;

const PATH_PROJECT_ROOT: &str = "../../../";
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
        insta_settings.bind(|| insta::assert_ron_snapshot!(format!("{log_filename}_ron"), data));
        insta_settings.bind(|| insta::assert_yaml_snapshot!(format!("{log_filename}_yaml"), data));
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
                SerdeFormat::Ron => {
                    ron::from_str(&as_string).unwrap_or_else(|e| fail_with(path, i, e, as_string))
                }
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

#[rstest]
fn comparisons_specific_field(insta_settings: insta::Settings) {
    let row0 = create_log_row_no_extra();
    let row1 = create_log_row_with_extra();
    let mut data = Data {
        rows: vec![row0.clone(), row1.clone()],
        ..Default::default()
    };

    data.filter = Some(FilterConfig {
        search_key: "200".to_string(),
        filter_on: filter::FilterOn::Field(FieldSpecifier {
            name: "http.status_code".to_string(),
        }),
        is_case_sensitive: false,
        comparator: Default::default(),
    });

    let display_options = DataDisplayOptions::default();
    let common_fields = display_options.common_fields();

    for comparator in Comparator::iter() {
        data.filter.as_mut().unwrap().comparator = comparator;
        data.apply_filter(common_fields);
        insta_settings.bind(|| insta::assert_yaml_snapshot!(data));
    }
}

#[rstest]
fn comparisons_any(insta_settings: insta::Settings) {
    let row0 = create_log_row_no_extra();
    let row1 = create_log_row_with_extra();
    let mut data = Data {
        rows: vec![row0.clone(), row1.clone()],
        ..Default::default()
    };

    data.filter = Some(FilterConfig {
        search_key: "20".to_string(),
        filter_on: filter::FilterOn::Any,
        is_case_sensitive: false,
        comparator: Default::default(),
    });

    let display_options = DataDisplayOptions::default();
    let common_fields = display_options.common_fields();

    for comparator in Comparator::iter() {
        data.filter.as_mut().unwrap().comparator = comparator;
        data.apply_filter(common_fields);
        insta_settings.bind(|| insta::assert_yaml_snapshot!(data));
    }
}

#[test]
fn selected_maintenance_with_filtering() {
    let test_field = String::from("test field");
    let rows = (5..10)
        .map(|i| {
            let mut row = create_log_row_no_extra();
            row.data.insert(test_field.clone(), i.into());
            row
        })
        .collect();
    let mut data = Data {
        rows,
        ..Default::default()
    };
    let display_options = DataDisplayOptions::default();
    let common_fields = display_options.common_fields();

    // Set "7" as selected
    data.selected_row = Some(2);

    // Save selected row from before
    let expected = data
        .selected_row_data_as_slice(common_fields)
        .unwrap()
        .to_vec();

    data.filter = Some(FilterConfig {
        search_key: "7".to_string(),
        ..Default::default()
    });
    data.apply_filter(DataDisplayOptions::default().common_fields());

    // Test that 7 is still selected
    let actual = data
        .selected_row_data_as_slice(common_fields)
        .unwrap()
        .to_vec();

    assert_eq!(actual, expected);

    // Then reverse
    data.unfilter();

    // Test that 7 is still selected
    let actual = data
        .selected_row_data_as_slice(common_fields)
        .unwrap()
        .to_vec();

    assert_eq!(actual, expected);
}

#[test]
fn selected_unselected_when_not_present() {
    let test_field = String::from("test field");
    let rows = (5..10)
        .map(|i| {
            let mut row = create_log_row_no_extra();
            row.data.insert(test_field.clone(), i.into());
            row
        })
        .collect();
    let mut data = Data {
        rows,
        ..Default::default()
    };
    let display_options = DataDisplayOptions::default();
    let common_fields = display_options.common_fields();

    // Set "7" as selected
    data.selected_row = Some(2);

    // Filter for 6, so 7 is not included
    data.filter = Some(FilterConfig {
        search_key: "6".to_string(),
        ..Default::default()
    });
    data.apply_filter(DataDisplayOptions::default().common_fields());

    let actual = data.selected_row_data_as_slice(common_fields);

    assert!(actual.is_none());
}
