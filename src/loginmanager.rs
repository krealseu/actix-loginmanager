use actix_service::{Service, Transform};
use actix_web::HttpMessage;
use actix_web::{
    dev::{ServiceRequest, ServiceResponse},
    Error, HttpResponse,
};
use futures::{
    future::{ok, Ready},
    Future,
};
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll};

use serde::{de::DeserializeOwned, Serialize};

pub trait DecodeRequest: Sized {
    fn decode(&self, req: &ServiceRequest) -> Option<String>;
    fn update_<B>(&self, key: Option<String>, header: &mut ServiceResponse<B>) -> Result<(), ()> {
        Ok(())
    }
}

pub enum KeyState {
    Login,
    Logout,
    Update,
    Ok,
}

pub struct KeyWrap<I>
where
    I: Serialize + DeserializeOwned,
{
    pub key: Option<I>,
    pub state: KeyState,
}

struct Inner<I, D>
where
    I: Serialize + DeserializeOwned,
    D: DecodeRequest,
{
    key: Option<I>,
    decoder: D,
    login_view: String,
}

impl<I, D> Inner<I, D>
where
    I: Serialize + DeserializeOwned,
    D: DecodeRequest,
{
    // fn from_request(&self, req: &ServiceRequest)->Option<I>{
    //     None
    // }
}

pub struct LoginManager<I, D>(Rc<Inner<I, D>>)
where
    I: Serialize + DeserializeOwned,
    D: DecodeRequest;

impl<I, D> LoginManager<I, D>
where
    I: Serialize + DeserializeOwned,
    D: DecodeRequest,
{
    pub fn new(decoder: D) -> Self
    where
        D: DecodeRequest,
    {
        Self(Rc::new(Inner {
            key: None,
            decoder,
            login_view: "/login".to_owned(),
        }))
    }
}

impl<S, B, I: 'static, D: 'static> Transform<S> for LoginManager<I, D>
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
    I: Serialize + DeserializeOwned,
    D: DecodeRequest,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = LoginManagerMiddleware<S, I, D>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(LoginManagerMiddleware {
            service,
            inner: self.0.clone(),
        })
    }
}

pub struct LoginManagerMiddleware<S, I, D>
where
    I: Serialize + DeserializeOwned,
    D: DecodeRequest,
{
    service: S,
    inner: Rc<Inner<I, D>>,
}

impl<S, B, I: 'static, D: 'static> Service for LoginManagerMiddleware<S, I, D>
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
    I: Serialize + DeserializeOwned,
    D: DecodeRequest,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&mut self, req: ServiceRequest) -> Self::Future {
        let inner = self.inner.clone();
        let key_str = inner.decoder.decode(&req);
        if let Some(key_str) = key_str {
            match serde_json::from_str::<I>(&key_str) {
                Ok(key) => {
                    req.extensions_mut().insert(KeyWrap {
                        key: Some(key),
                        state: KeyState::Ok,
                    });
                }
                _ => {
                    return Box::pin(async move {
                        HttpResponse::Ok().finish();
                        let res: actix_http::Response = HttpResponse::BadRequest()
                            .body("Authentication information is not given in the correct format.")
                            .into();
                        Ok(req.error_response(Error::from(res)))
                    });
                }
            }
        };
        let fut = self.service.call(req);
        Box::pin(async move {
            let res = fut.await.map(|mut res| {
                let mut key = if let Some(key) = res.request().extensions().get::<KeyWrap<I>>() {
                    match key {
                        KeyWrap {
                            key: Some(key),
                            state: KeyState::Login | KeyState::Update,
                        } => {
                            if let Ok(key) = serde_json::to_string(key) {
                                Some(key)
                            } else {
                                None
                            }
                        }
                        KeyWrap {
                            key: _,
                            state: KeyState::Logout,
                        } => Some("".to_owned()),
                        _ => None,
                    }
                } else {
                    None
                };
                inner.decoder.update_(key, &mut res);
                res
            })?;
            Ok(res)
        })
    }
}
