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
/// the base user trait
/// ### Example: Get user from database
/// ```rust
/// type Pool = sqlx::SqlitePool;
///
/// #[derive(Serialize, Deserialize)]
/// pub struct User { ... }
///
/// impl UserMinix for User {
///     type Future = Pin<Box<dyn Future<Output = Option<Self>>>>;
///     type Key = i32;
///     fn get_user(id: &i32, req: &HttpRequest) -> Self::Future {
///         let req = req.clone();
///         let id = id.clone();
///         Box::pin(async move {
///             if let Some(pool) = req.app_data::<Data<Pool>>(){
///                 let pool = pool.get_ref();  
///                 todo!()   // get user from pool instance,return Some(user) or None
///             }else{
///                 None
///             }
///         })
///     }
///     fn get_id(&self) -> i32 {
///         self.id
///     }
/// }
/// ```
pub trait UserMinix: Sized {
    ///
    type Future: Future<Output = Option<Self>>;

    /// The type of User, must be same as Loginmanager.
    /// Otherwise no user will be returned.
    type Key: Serialize + DeserializeOwned;

    /// Get user from id and req,Tip:can use req.app_data to obtain
    /// database connection defined in Web app.
    fn get_user(id: &Self::Key, req: &HttpRequest) -> Self::Future;

    /// Return the User id
    fn get_id(&self) -> Self::Key;

    /// return user's actual authentication status, default True.
    fn is_authenticated(&self) -> bool {
        true
    }

    /// return user's actual active status, default True.
    fn is_atived(&self) -> bool {
        true
    }
}

/// The wrap of user Instance. It implements `FromRequest` trait.  
/// It will return `401 Unauthorized` if no key or error key.  
/// If loginmanager set redirect true,then will rediret login_view. 
/// ```rust
/// #[get("/index")]
/// async fn index(UserWrap(user): UserWrap<User>) -> impl Responder{
///     todo()!
/// }
/// ```
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
