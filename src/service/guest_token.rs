use crate::error::{BoxError, HttpError};

use http::header::{HeaderValue, InvalidHeaderValue, USER_AGENT};
use http::{Method, Request, Response};
use http_body::Body;
use serde::Deserialize;
use std::convert::{TryFrom, TryInto};
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::{Arc, RwLock};
use std::task::{Context, Poll};
use tower::{Service, ServiceExt};
use tower_http::auth::add_authorization::AddAuthorization;
use tower_http::set_header::SetRequestHeader;

#[derive(Debug)]
pub struct GuestTokenService<S, B> {
    service: AddAuthorization<SetRequestHeader<S, HeaderValue>>,
    guest_token: Arc<RwLock<Option<String>>>,
    body: PhantomData<fn() -> B>,
}

impl<S, B> Clone for GuestTokenService<S, B>
where
    S: Clone,
{
    fn clone(&self) -> Self {
        Self {
            service: self.service.clone(),
            guest_token: self.guest_token.clone(),
            body: PhantomData,
        }
    }
}

impl<S, ReqB, RespB> GuestTokenService<S, ReqB>
where
    S: Service<Request<ReqB>, Response = Response<RespB>> + Send + Clone,
{
    pub fn new(
        service: S,
        user_agent: String,
        bearer_token: String,
    ) -> Result<Self, InvalidHeaderValue> {
        let service =
            SetRequestHeader::overriding(service, USER_AGENT, HeaderValue::try_from(user_agent)?);

        let service = AddAuthorization::bearer(service, &bearer_token);

        Ok(Self {
            service,
            guest_token: Arc::new(RwLock::new(None)),
            body: PhantomData,
        })
    }
}

impl<S, ReqB, RespB> Service<Request<ReqB>> for GuestTokenService<S, ReqB>
where
    S: Service<Request<ReqB>, Response = Response<RespB>> + Send + Clone + 'static,
    S::Error: Into<BoxError> + Send,
    S::Future: Send + 'static,
    ReqB: Body + Send + Default + 'static,
    RespB: Body + Send + 'static,
    RespB::Error: Into<BoxError>,
    RespB::Data: Send,
{
    type Response = Response<RespB>;
    type Error = BoxError;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx).map_err(Into::into)
    }

    fn call(&mut self, mut req: Request<ReqB>) -> Self::Future {
        let guest_token_lock = self.guest_token.clone();
        let mut service = self.service.clone();

        Box::pin(async move {
            let stored_token = (*guest_token_lock.read().unwrap()).clone();
            let guest_token = match stored_token {
                Some(token) => token,
                None => {
                    let req = Request::builder()
                        .uri("https://api.twitter.com/1.1/guest/activate.json")
                        .method(Method::POST)
                        .body(Default::default())?;

                    let resp = service
                        .ready()
                        .await
                        .map_err(Into::into)?
                        .call(req)
                        .await
                        .map_err(Into::into)?;

                    // TODO: Status code checking could be moved into a tower layer
                    let status = resp.status();
                    let body = hyper::body::to_bytes(resp.into_body())
                        .await
                        .map_err(Into::into)?;

                    if !status.is_success() {
                        if status.is_client_error() {
                            *guest_token_lock.write().unwrap() = None;
                        }
                        return Err(HttpError::new(status, body).into());
                    } else {
                        let token = serde_json::from_slice::<Activate>(&body)?.guest_token;
                        *guest_token_lock.write().unwrap() = Some(token.clone());
                        token
                    }
                }
            };

            req.headers_mut()
                .insert("X-Guest-Token", guest_token.try_into()?);

            let resp = service
                .ready()
                .await
                .map_err(Into::into)?
                .call(req)
                .await
                .map_err(Into::into)?;

            let status = resp.status();

            // TODO: Status code checking could be moved into a tower layer
            if !status.is_success() {
                let body = hyper::body::to_bytes(resp.into_body())
                    .await
                    .map_err(Into::into)?;

                if status.is_client_error() {
                    *guest_token_lock.write().unwrap() = None;
                }

                return Err(HttpError::new(status, body).into());
            }

            Ok(resp)
        })
    }
}

#[derive(Deserialize)]
pub struct Activate {
    pub guest_token: String,
}
