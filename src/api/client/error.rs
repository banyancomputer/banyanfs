#[derive(Debug, thiserror::Error)]
pub enum ApiClientError {
    #[error("provided URL wasn't valid: {0}")]
    BadUrl(#[from] url::ParseError),

    #[error("underlying HTTP client error: {0}")]
    Reqwest(#[from] reqwest::Error),
}
