type AwaitingType = futures::channel::oneshot::Receiver<LoadingStatus>;

#[derive(Default, Debug)]
pub enum LoadingStatus {
    #[default]
    NotInProgress,
    InProgress(AwaitingType),
    Failed(String),
    Success(String),
}
