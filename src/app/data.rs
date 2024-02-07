#[derive(serde::Deserialize, serde::Serialize, Default, Debug)]
pub struct Data {
    rows: Vec<LogRow>,
}

#[derive(serde::Deserialize, serde::Serialize, Default, Debug)]
pub struct LogRow {
    time: Option<String>,
    request_id: Option<String>,
    otel_name: Option<String>,
    msg: Option<String>,
    // TODO 2: Capture other info
}

impl LogRow {
    const EMPTY_TEXT: &'static str = "[-]";
    pub(crate) fn time(&self) -> &str {
        if let Some(val) = &self.time {
            val
        } else {
            Self::EMPTY_TEXT
        }
    }

    pub(crate) fn request_id(&self) -> String {
        if let Some(val) = &self.request_id {
            val.clone()
        } else {
            Self::EMPTY_TEXT.to_string()
        }
    }

    pub(crate) fn otel_name(&self) -> String {
        if let Some(val) = &self.otel_name {
            val.clone()
        } else {
            Self::EMPTY_TEXT.to_string()
        }
    }

    pub(crate) fn msg(&self) -> String {
        if let Some(val) = &self.msg {
            val.clone()
        } else {
            Self::EMPTY_TEXT.to_string()
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
        dbg!(value);
        todo!()
    }
}
