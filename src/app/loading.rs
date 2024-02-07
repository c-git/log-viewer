#[derive(Default)]
pub enum LoadingStatus {
    #[default]
    NotInProgress,
    InProgress(poll_promise::Promise<Box<LoadingStatus>>),
    Failed(String),
    Success(String),
}
