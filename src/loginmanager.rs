use actix_web::dev::{forward_ready, Service, Transform};
use actix_web::HttpMessage;
use actix_web::{
    dev::{ServiceRequest, ServiceResponse},
    http,
    http::header::HeaderValue,
    http::header::LOCATION,
    Error,
};
use futures::{
    future::{ok, Ready},
    Future,
};
use std::pin::Pin;
use std::rc::Rc;

pub trait DecodeRequest: Sized {
    fn decode(&self, req: &ServiceRequest) -> Option<String>;
    fn update_<B>(&self, res: &mut ServiceResponse<B>) -> Result<(), Error> {
        Ok(())
    }
}

pub enum LoginState {
    Login,
    Logout,
    Update,
    Ok,
    Wait,
    Err,
}

pub struct LoginInfo {
    pub key_str: Option<String>,
    pub state: LoginState,
}

impl LoginInfo {
    pub fn new(key_str: Option<String>, state: LoginState) -> Self {
        Self { key_str, state }
    }
}

struct Inner<D>
where
    D: DecodeRequest,
{
    decoder: D,
    login_view: HeaderValue,
    redirect: bool,
}

/// LoginManager<D> is implemented as a middleware.   
/// - `D` the type of DecodeRequest. It decode the key_string from request.  
pub struct LoginManager<D>(Rc<Inner<D>>)
where
    D: DecodeRequest;

impl<D> LoginManager<D>
where
    D: DecodeRequest,
{
    pub fn new(decoder: D) -> Self
    where
        D: DecodeRequest,
    {
        Self(Rc::new(Inner {
            decoder,
            login_view: HeaderValue::from_str("/login").unwrap(),
            redirect: true,
        }))
    }

    /// Set false, not redirect when user is not authenticated. Default true.
    pub fn redirect(mut self, redirect: bool) -> Self {
        Rc::get_mut(&mut self.0).unwrap().redirect = redirect;
        self
    }

    /// Set the login url redirect, default '/login'.
    pub fn login_view(mut self, login_view: String) -> Self {
        Rc::get_mut(&mut self.0).unwrap().login_view = HeaderValue::from_str(&login_view).unwrap();
        self
    }
}

impl<S, B, D: 'static> Transform<S, ServiceRequest> for LoginManager<D>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
    D: DecodeRequest,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = LoginManagerMiddleware<S, D>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(LoginManagerMiddleware {
            service,
            inner: self.0.clone(),
        })
    }
}

pub struct LoginManagerMiddleware<S, D>
where
    D: DecodeRequest,
{
    service: S,
    inner: Rc<Inner<D>>,
}

impl<S, B, D: 'static> Service<ServiceRequest> for LoginManagerMiddleware<S, D>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
    D: DecodeRequest,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>+'static>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let inner = self.inner.clone();
        let key_str = inner.decoder.decode(&req);
        req.extensions_mut()
            .insert(LoginInfo::new(key_str, LoginState::Wait));
        let fut = self.service.call(req);
        Box::pin(async move {
            let res = fut.await.map(|mut res| {
                inner.decoder.update_(&mut res);
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