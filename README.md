# Actix-loginmanager
a simple loginmanager on actix web

# Usage example
```
use actix_web::{
    web, App, Either, Error, FromRequest, HttpRequest, HttpResponse, HttpServer, Responder,
};

use actix_loginmanager::{LoginManager,CookieSession,UserMinix,User};

use futures::{
    future,
    future::{ok, Ready},
    Future,
};

#[derive(Clone)]
struct User2{
    id:i32,
    name:&'static str
}

const users:[User2;3] = [
    User2{id:1,name:"Tom"},
    User2{id:2,name:"Jerry"},
    User2{id:3,name:"Spike"},
];

impl UserMinix<i32> for User2 {
    type Future = Ready<Option<Self>>;
    fn get_user(i: &i32, _: &HttpRequest) -> Self::Future {
        for index in 0..users.len() {
            if &users[index].id == i{
                return future::ready(Some(users[index].clone()));
            }
         }
        future::ready(None)
    }

    fn get_id(&self) -> i32 {
        self.id
    }
}

fn main() {
    actix_rt::System::new("loginmanager")
        .block_on(async {
            HttpServer::new(|| {
                App::new()
                    .wrap(LoginManager::<i32, CookieSession>::new(
                        CookieSession::new(b"1231231231231231231231231231231231231231")
                            .secure(false),
                    ))
                    .route(
                        "/",
                        web::get().to(|req: HttpRequest, user: User<i32, User2>| {
                            if !user.is_authenticated(){
                                User::new(users[0].clone()).login(&req);
                            }else{
                                user.logout(&req);
                                return HttpResponse::Ok().body(format!("is_authenticated:{:?} current user:{:?}", user.is_authenticated(), user.user().unwrap().name));
                            };
                            HttpResponse::Ok().body(format!("is_authenticated:{:?} ", user.is_authenticated()))
                        }),
                    )
            })
            .bind("0.0.0.0:7081").unwrap()
            .run()
            .await
        }).unwrap();
}

```