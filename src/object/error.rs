#[derive(Debug, thiserror::Error)]
pub enum ObjectError {
    #[error("Unrecognized Object type: {0}")]
    UnrecognizedObjectType(String),
}
