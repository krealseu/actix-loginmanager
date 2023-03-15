# Actix-loginmanager
a simple loginmanager on actix web

# Dependencies
```toml
actix-web = "^4"
serde = {version="^1.0", features = ["derive"] }
serde_json = "^1.0"
rust-crypto = "^0.2"
```

# Usage example
```rust
use std::pin::Pin;
use actix_web::{get, web, App, HttpRequest, HttpResponse, HttpServer};
use actix_loginmanager as loginmanager;
use loginmanager::{CookieSession, LoginManager, UserMinix, UserWrap};

use futures::Future;
use loginmanager_codegen::login_required;

use futures::{future, future::Ready};
 
#[derive(Clone)]
struct User {
    id: i32,
    name: &'static str,
}
 
impl UserMinix for User {
    type Future = Pin<Box<dyn Future<Output = Option<Self>>>>;
    type Key = i32;
    fn get_user(i: &Self::Key, _: &HttpRequest) -> Self::Future {
        // let req = req.clone();
        let i = i.clone();
        Box::pin(async move {
            for id in 0..USERS.len() {
                if USERS[id].id == i {
                    return Some(USERS[id].clone());
                }
            }
            None
        })
    }

    fn get_id(&self) -> &Self::Key {
        &self.id
    }
}
 
const USERS: [User; 3] = [
    User { id: 1, name: "Tom" },
    User { id: 2, name: "Jerry" },
    User { id: 3, name: "Spike" },
];

#[login_required(User)]
async fn hello() -> impl actix_web::Responder {
    return format!("hello {}",user.name);
}

async fn auto_login(req: HttpRequest) -> impl actix_web::Responder {
    let user = UserWrap::from(USERS[0].clone());
    loginmanager::login(&user, &req);
    HttpResponse::Ok().body(format!("login:{:?} ", user.user().name))
}

async fn logout(req: HttpRequest, UserWrap(user): UserWrap<User>) -> impl actix_web::Responder {
    loginmanager::logout(&user, &req);
    HttpResponse::Ok().body(format!("logout:{:?} ", user.name))
}

#[get("/")]
async fn index(UserWrap(user): UserWrap<User>) -> impl actix_web::Responder {
    HttpResponse::Ok().body(format!(
        "Hello:{:?} is_authenticated:{}",
        user.name,
        user.is_authenticated()
    ))
}

#[actix_web::main]
#[test]
async fn main() {
    HttpServer::new(|| {
        App::new()
            .wrap(LoginManager::new(
                CookieSession::new(&[0; 32]).secure(false),
            ))
            .service(index)
            .route("/hello", web::get().to(hello))
            .route("/login", web::get().to(auto_login))
            .route("/logout", web::get().to(logout))
    })
    .bind("0.0.0.0:7081")
    .unwrap()
    .run()
    .await
    .unwrap();
}
```