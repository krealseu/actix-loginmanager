mod cooke_session;
mod loginmanager;
mod user;
pub use crate::cooke_session::CookieSession;
pub use crate::loginmanager::{DecodeRequest, LoginManager};
pub use crate::user::{User, UserMinix};

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{
        web, App, Either, Error, FromRequest, HttpRequest, HttpResponse, HttpServer, Responder,
    };
    // use actix_web::{test, HttpResponse, HttpServer};
    use futures::{
        future,
        future::{ok, Ready},
        Future,
    };

    impl UserMinix<i32> for i32 {
        type Future = Ready<Option<Self>>;
        fn get_user(i: &i32, _: &HttpRequest) -> Self::Future {
            future::ready(Some(333 + i))
        }

        fn get_id(&self) -> i32 {
            *self
        }
    }

    #[test]
    fn run_test() {
        assert!(actix_rt::System::new("loginmanager1")
            .block_on(async {
                HttpServer::new(|| {
                    App::new()
                        .wrap(LoginManager::<i32, CookieSession>::new(
                            CookieSession::new(b"1231231231231231231231231231231231231231")
                            .secure(false)
                            ,
                        ))
                        .route(
                            "/",
                            web::get().to(|req: HttpRequest,ss: User<i32, i32>| {
                                println!("{:?} {:?}", ss.user(), ss.get_id());
                                ss.login(&req);
                                HttpResponse::Ok().body("Hello World!")
                            }),
                        )
                })
                .bind("0.0.0.0:7081")?
                .run()
                .await
            })
            .is_ok())
    }
}
