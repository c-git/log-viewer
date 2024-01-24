#[derive(Default, Debug)]
pub enum LoadingStatus {
    #[default]
    NotInProgress,
    InProgress(),
    Failed(String),
    Success(String),
}
