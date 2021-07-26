# Actix-loginmanager
a simple loginmanager on actix web

# Usage example
```
use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer};
 
use actix_loginmanager as loginmanager;
use loginmanager::{CookieSession, LoginManager, UserMinix, UserWrap};
use loginmanager_codegen::login_required;

use futures::{future, future::Ready};
 
#[derive(Clone)]
struct User {
    id: i32,
    name: &'static str,
}
 
impl UserMinix for User {
    type Future = Ready<Option<Self>>;
    type Key = i32;
    fn get_user(i: &i32, _: &HttpRequest) -> Self::Future {
        for index in 0..USERS.len() {
            if &USERS[index].id == i {
                return future::ready(Some(USERS[index].clone()));
            }
        }
        future::ready(None)
    }
 
    fn get_id(&self) -> i32 {
        self.id
    }
}
 
const USERS: [User; 3] = [
    User { id: 1, name: "Tom" },
    User { id: 2, name: "Jerry" },
    User { id: 3, name: "Spike" },
];

#[login_required(User)]
async fn hello()->impl actix_web::Responder{
    return "hello";
}
 
#[actix_web::main]
async fn main() {
    HttpServer::new(|| {
        App::new()
            .wrap(
                LoginManager::new(
                    CookieSession::new(&[0; 32]).secure(false)
                ),
            )
            .route(
                "/",
                web::get().to(|UserWrap(user): UserWrap<User>| {
                    HttpResponse::Ok().body(format!(
                        "Hello:{:?} is_authenticated:{}",
                        user.name,
                        user.is_authenticated()
                    ))
                }),
            )
            .route(
                "/login",
                web::get().to(|req: HttpRequest| {
                    let user = UserWrap::from(USERS[0].clone());
                    loginmanager::login(&user, &req);
                    HttpResponse::Ok().body(format!("login:{:?} ", user.user().name))
                }),
            )
            .route(
                "/logout",
                web::get().to(|req: HttpRequest, UserWrap(user): UserWrap<User>| {
                    loginmanager::logout(&user, &req);
                    HttpResponse::Ok().body(format!("logout:{:?} ", user.name))
                }),
            )
            .route("/hello", web::get().to(hello))
    })
    .bind("0.0.0.0:7081")
    .unwrap()
    .run()
    .await
    .unwrap();
}
```