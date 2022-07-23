use crate::error::{BoxError, HttpError};

use crate::UserId;
use futures::future;
use tower::retry::{Policy, Retry};
use tower::Layer;

#[derive(Debug, Clone)]
pub struct RetryOnHttpError {
    retries_remaining: usize,
}

impl RetryOnHttpError {
    pub fn new(retries: usize) -> Self {
        Self {
            retries_remaining: retries,
        }
    }
}

impl<S> Layer<S> for RetryOnHttpError {
    type Service = Retry<Self, S>;

    fn layer(&self, service: S) -> Self::Service {
        let policy = self.clone();
        Retry::new(policy, service)
    }
}

impl<Resp> Policy<UserId, Resp, BoxError> for RetryOnHttpError {
    type Future = future::Ready<Self>;

    fn retry(&self, _req: &UserId, result: Result<&Resp, &BoxError>) -> Option<Self::Future> {
        match result {
            Ok(_) => None,
            Err(e) if self.retries_remaining > 0 => {
                if let Some(error) = e.downcast_ref::<HttpError>() {
                    tracing::warn!(
                        %error,
                        retries_remaining = self.retries_remaining,
                        "Retrying request due to HTTP error"
                    );
                    Some(future::ready(RetryOnHttpError {
                        retries_remaining: self.retries_remaining - 1,
                    }))
                } else {
                    None
                }
            }
            Err(_) => None,
        }
    }

    fn clone_request(&self, req: &UserId) -> Option<UserId> {
        Some(*req)
    }
}
