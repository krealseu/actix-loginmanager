use crate::loginmanager::{KeyState, KeyWrap};
use actix_http::error::ErrorUnauthorized;
use actix_web::{
    dev::{Extensions, Payload, ServiceRequest, ServiceResponse},
    http::header,
    Error, FromRequest, HttpRequest, HttpResponse, HttpServer,
};
use futures::Future;
use serde::{de::DeserializeOwned, Serialize};
use std::pin::Pin;
use std::rc::Rc;

pub trait UserMinix: Sized {
    type Future: Future<Output = Option<Self>>;
    type Key: Serialize + DeserializeOwned;

    fn get_user(id: &Self::Key, req: &HttpRequest) -> Self::Future;

    fn get_id(&self) -> Self::Key;

    fn is_authenticated(&self) -> bool {
        true
    }

    fn is_atived(&self) -> bool {
        true
    }
}

/// User instance
pub struct UserWrap<T>(pub Rc<T>);

impl<T> Clone for UserWrap<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T: 'static> UserWrap<T>
where
    T: UserMinix,
{
    pub fn new(user: T) -> Self {
        Self(Rc::new(user))
    }

    pub fn user(&self) -> &T {
        self.0.as_ref()
    }
}

impl<U> From<U> for UserWrap<U> {
    fn from(u: U) -> Self {
        UserWrap(Rc::new(u))
    }
}

impl<U> AsRef<U> for UserWrap<U> {
    fn as_ref(&self) -> &U {
        self.0.as_ref()
    }
}

impl<T: 'static> FromRequest for UserWrap<T>
where
    T: UserMinix,
{
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self, Self::Error>>>>;

    type Config = ();
    #[inline]
    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let req_clone: HttpRequest = req.clone();
        Box::pin(async move {
            let extensions = &mut req_clone.extensions_mut();
            if let Some(user) = extensions.get::<Self>() {
                return Ok(user.clone());
            } else {
                if let Some(keywrap) = extensions.get::<KeyWrap<T::Key>>() {
                    if let Some(id) = &keywrap.key {
                        let real_user = T::get_user(&id, &req_clone).await;
                        if let Some(real_user) = real_user {
                            let user = UserWrap(Rc::new(real_user));
                            extensions.insert(user.clone());
                            return Ok(user);
                        } else {
                        }
                    };
                };
            };
            return Err(ErrorUnauthorized("No authentication."));
        })
    }
}
