use crate::loginmanager::{KeyState, KeyWrap};
use actix_web::{
    dev::{Extensions, Payload, ServiceRequest, ServiceResponse},
    http::header,
    web, App, Either, Error, FromRequest, HttpRequest, HttpResponse, HttpServer, Responder,
};
use core::cell::RefCell;
use futures::{
    future,
    future::{ok, Ready},
    Future,
};
use std::pin::Pin;
use std::rc::Rc;

use serde::{de::DeserializeOwned, Serialize};

pub trait UserMinix<K>: Sized {
    type Future: Future<Output = Option<Self>>;

    fn get_user(id: &K, req: &HttpRequest) -> Self::Future;

    fn get_id(&self) -> K;

    fn is_authenticated(&self) -> bool {
        true
    }

    fn is_atived(&self) -> bool {
        true
    }
}

struct Inner<K, U>
where
    U: UserMinix<K>,
    K: Serialize + DeserializeOwned,
{
    id: Option<K>,
    user: Option<U>,
}

pub struct User<K, U>(Rc<Inner<K, U>>)
where
    U: UserMinix<K>,
    K: Serialize + DeserializeOwned;

impl<K, U> Default for User<K, U>
where
    U: UserMinix<K>,
    K: Serialize + DeserializeOwned,
{
    fn default() -> Self {
        Self(Rc::new(Inner {
            id: None,
            user: None,
            // state: State::Ok,
        }))
    }
}

impl<K: 'static, U: 'static> User<K, U>
where
    U: UserMinix<K>,
    K: Serialize + DeserializeOwned,
{
    pub fn new(user: U) -> Self {
        Self(Rc::new(Inner {
            id: None,
            user: Some(user),
            // state: State::Ok,
        }))
    }

    pub fn get_id(&self) -> Option<K> {
        if let Some(user) = &self.0.user {
            Some(user.get_id())
        } else {
            None
        }
    }

    pub fn user(&self) -> Option<&U> {
        self.0.user.as_ref()
    }

    pub fn is_authenticated(&self) -> bool {
        if let Some(user) = self.user() {
            user.is_authenticated()
        }else{
            false
        }
    }

    pub fn login(&self, req: &HttpRequest) {
        let mut extensions = req.extensions_mut();
        extensions.insert(KeyWrap {
            key: self.get_id(),
            state: KeyState::Login,
        });
    }

    pub fn logout(&self, req: &HttpRequest) {
        let mut extensions = req.extensions_mut();
        extensions.insert(KeyWrap {
            key: self.get_id(),
            state: KeyState::Logout,
        });
    }

    async fn get_user_from_req(req: &HttpRequest) -> Self {
        let mut extensions = req.extensions_mut();
        if let Some(user) = extensions.get::<Rc<Inner<K, U>>>() {
            return User(user.clone());
        } else {
            let mut user = Self::default();
            if let Some(keywrap) = extensions.get::<KeyWrap<K>>() {
                if let Some(id) = &keywrap.key {
                    let real_user = U::get_user(&id, req).await;
                    user = User(Rc::new(Inner {
                        id: None,
                        user: real_user,
                        // state: State::Pending,
                    }))
                };
            };
            extensions.insert(User(user.0.clone()));
            return user;
        }
    }
}

impl<K: 'static, U: 'static> FromRequest for User<K, U>
where
    U: UserMinix<K>,
    K: Serialize + DeserializeOwned,
{
    type Error = Error;
    type Future = future::LocalBoxFuture<'static, Result<Self, Self::Error>>;
    type Config = ();
    #[inline]
    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let req_clone: HttpRequest = req.clone();
        Box::pin(async move { Ok(User::get_user_from_req(&req_clone).await) })
    }
}
