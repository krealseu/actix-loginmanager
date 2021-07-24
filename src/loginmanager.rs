use actix_http::error::ErrorBadRequest;
use actix_service::{Service, Transform};
use actix_web::HttpMessage;
use actix_web::{
    dev::{ServiceRequest, ServiceResponse},
    http,
    http::header::HeaderValue,
    http::header::LOCATION,
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

/// The Wrap of key ,storage in request extensions
pub struct KeyWrap<I>
where
    I: Serialize + DeserializeOwned,
{
    pub key: Option<I>,
    pub state: KeyState,
}

impl<I> KeyWrap<I>
where
    I: Serialize + DeserializeOwned,
{
    pub fn new(key: I, state: KeyState) -> Self {
        Self {
            key: Some(key),
            state,
        }
    }
}

struct Inner<I, D>
where
    I: Serialize + DeserializeOwned,
    D: DecodeRequest,
{
    key: Option<I>,
    decoder: D,
    login_view: HeaderValue,
    redirect: bool,
}

/// LoginManager<I, D> is implemented as a middleware.   
/// - `I` the type of user key.  
/// - `D` the type of DecodeRequest. It decode the key_string from request.  
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
            login_view: HeaderValue::from_str("/login").unwrap(),
            redirect: true,
        }))
    }

    /// set false, not redirect when user is not authenticated,default true.
    pub fn redirect(mut self, redirect: bool) -> Self {
        Rc::get_mut(&mut self.0).unwrap().redirect = redirect;
        self
    }

    pub fn login_view(mut self, login_view: String) -> Self {
        Rc::get_mut(&mut self.0).unwrap().login_view = HeaderValue::from_str(&login_view).unwrap();
        self
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
                        Err(ErrorBadRequest(
                            "Authentication information is not given in the correct format.",
                        ))
                    });
                }
            }
        };
        let fut = self.service.call(req);
        Box::pin(async move {
            let res = fut.await.map(|mut res| {
                let key: Option<String> =
                    if let Some(key) = res.request().extensions().get::<KeyWrap<I>>() {
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
                inner.decoder.update_(key, &mut res).unwrap();
                if inner.redirect && res.status().as_u16() == 401 {
                    res.response_mut().head_mut().status = http::StatusCode::FOUND;
                    let mut path = String::new();
                    let req = res.request();
                    if inner.redirect {
                        path.push_str(req.path());
                        if req.query_string().len() > 0 {
                            path.push_str("%3F");
                            path.push_str(
                                &req.query_string().replace("&", "%26").replace("=", "%3d"),
                            );
                        }
                    }
                    let headervalue = if path.len() > 0 {
                        let url = format!("{}?next={}", inner.login_view.to_str().unwrap(), path);
                        HeaderValue::from_str(&url).unwrap()
                    } else {
                        inner.login_view.clone()
                    };
                    res.headers_mut().insert(LOCATION, headervalue);
                };
                res
            })?;
            Ok(res)
        })
    }
}
