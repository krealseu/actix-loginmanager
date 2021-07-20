//! A loginmanager for actix-web
//!
//! ## Example
//! ```rust
//! use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer};
//! 
//! use actix_loginmanager as loginmanager;
//! use loginmanager::{CookieSession, LoginManager, UserMinix, UserWrap};
//! 
//! use futures::{future, future::Ready};
//! 
//! #[derive(Clone)]
//! struct User {
//!     id: i32,
//!     name: &'static str,
//! }
//! 
//! impl UserMinix for User {
//!     type Future = Ready<Option<Self>>;
//!     type Key = i32;
//!     fn get_user(i: &i32, _: &HttpRequest) -> Self::Future {
//!         for index in 0..USERS.len() {
//!             if &USERS[index].id == i {
//!                 return future::ready(Some(USERS[index].clone()));
//!             }
//!         }
//!         future::ready(None)
//!     }
//! 
//!     fn get_id(&self) -> i32 {
//!         self.id
//!     }
//! }
//! 
//! const USERS: [User; 3] = [
//!     User { id: 1, name: "Tom" },
//!     User { id: 2, name: "Jerry" },
//!     User { id: 3, name: "Spike" },
//! ];
//! 
//! #[actix_web::main]
//! async fn main() {
//!     HttpServer::new(|| {
//!         App::new()
//!             .wrap(
//!                 LoginManager::<i32, CookieSession>::new(
//!                     CookieSession::new(&[0; 32]).secure(false)
//!                 ),
//!             )
//!             .route(
//!                 "/",
//!                 web::get().to(|UserWrap(user): UserWrap<User>| {
//!                     HttpResponse::Ok().body(format!(
//!                         "Hello:{:?} is_authenticated:{}",
//!                         user.name,
//!                         user.is_authenticated()
//!                     ))
//!                 }),
//!             )
//!             .route(
//!                 "/login",
//!                 web::get().to(|req: HttpRequest| {
//!                     let user = UserWrap::from(USERS[0].clone());
//!                     loginmanager::login(&user, &req);
//!                     HttpResponse::Ok().body(format!("login:{:?} ", user.user().name))
//!                 }),
//!             )
//!             .route(
//!                 "/logout",
//!                 web::get().to(|req: HttpRequest, UserWrap(user): UserWrap<User>| {
//!                     loginmanager::logout(&user, &req);
//!                     HttpResponse::Ok().body(format!("logout:{:?} ", user.name))
//!                 }),
//!             )
//!     })
//!     .bind("0.0.0.0:7081")
//!     .unwrap()
//!     .run()
//!     .await
//!     .unwrap();
//! }
//! ```

mod cooke_session;
mod loginmanager;
mod user;
pub use crate::cooke_session::CookieSession;
pub use crate::loginmanager::{DecodeRequest, KeyState, KeyWrap, LoginManager};
pub use crate::user::{UserMinix, UserWrap};

/// The method of user login
pub fn login<U>(user: &dyn AsRef<U>, req: &actix_web::HttpRequest)
where
    U: 'static + UserMinix,
{
    let mut extensions = req.extensions_mut();
    let id = user.as_ref().get_id();
    extensions.insert(KeyWrap::new(id, KeyState::Login));
}

/// The method of user logout
pub fn logout<U>(user: &dyn AsRef<U>, req: &actix_web::HttpRequest)
where
    U: 'static + UserMinix,
{
    let mut extensions = req.extensions_mut();
    let id = user.as_ref().get_id();
    extensions.insert(KeyWrap::new(id, KeyState::Logout));
}
