use actix_web::http::{header, header::SET_COOKIE, HeaderValue};
use actix_web::HttpMessage;
use actix_web::{
    dev::{ServiceRequest, ServiceResponse},
    HttpRequest, HttpResponse,
};

use time::{Duration, OffsetDateTime};

use actix_web::cookie::{Cookie, CookieJar, Key, SameSite};
use serde::{Deserialize, Serialize};

use crypto::sha2::Sha512;

use crypto::digest::Digest;

use crate::loginmanager::DecodeRequest;

/// use cookie as session to storage the info of user key.
pub struct CookieSession {
    key: Key,
    name: String,
    path: String,
    domain: Option<String>,
    secure: bool,
    http_only: bool,
    max_age: Option<Duration>,
    expires_in: Option<Duration>,
    same_site: Option<SameSite>,
}

fn __create_identifier(request: &ServiceRequest) -> String {
    let mut sha512 = Sha512::new();
    if let Some(addr) = actix_web::dev::ConnectionInfo::get(request.head(), request.app_config())
        .realip_remote_addr()
    {
        if let Some(ip) = addr.split(":").next() {
            sha512.input_str(ip);
        };
    }
    if let Some(agent) = request.headers().get(header::USER_AGENT) {
        if let Ok(agent) = agent.to_str() {
            sha512.input_str(agent);
        };
    };
    return sha512.result_str();
}

fn _create_identifier(request: &HttpRequest) -> String {
    let mut sha512 = Sha512::new();
    if let Some(addr) = actix_web::dev::ConnectionInfo::get(request.head(), request.app_config())
        .realip_remote_addr()
    {
        if let Some(ip) = addr.split(":").next() {
            sha512.input_str(ip);
        };
    }
    if let Some(agent) = request.headers().get(header::USER_AGENT) {
        if let Ok(agent) = agent.to_str() {
            sha512.input_str(agent);
        };
    };
    return sha512.result_str();
}

#[derive(Serialize, Deserialize)]
struct Session {
    id: String,
    user_id: Option<String>,
}

impl CookieSession {
    pub fn new(key: &[u8]) -> Self {
        Self {
            key: Key::derive_from(key),
            name: "_session".to_owned(),
            path: "/".to_owned(),
            domain: None,
            secure: true,
            http_only: true,
            max_age: None,
            expires_in: None,
            same_site: None,
        }
    }

    pub fn name(mut self, name: &'static str) -> Self {
        self.name = name.to_owned();
        self
    }

    pub fn secure(mut self, secure: bool) -> Self {
        self.secure = secure;
        self
    }

    pub fn http_only(mut self, http_only: bool) -> Self {
        self.http_only = http_only;
        self
    }

    pub fn domain(mut self, domain: Option<String>) -> Self {
        self.domain = domain;
        self
    }

    pub fn max_age(mut self, max_age: Option<Duration>) -> Self {
        self.max_age = max_age;
        self
    }

    pub fn expires_in(mut self, expires_in: Option<Duration>) -> Self {
        self.expires_in = expires_in;
        self
    }

    pub fn same_site(mut self, same_site: Option<SameSite>) -> Self {
        self.same_site = same_site;
        self
    }
}

impl DecodeRequest for CookieSession {
    fn decode(&self, req: &ServiceRequest) -> Option<String> {
        if let Some(cookie) = req.cookie(&self.name) {
            let mut jar = CookieJar::new();
            jar.add_original(cookie.clone());
            let cookie_opt = jar.private(&self.key).get(&self.name);
            if let Some(cookie) = cookie_opt {
                if let Ok(val) = serde_json::from_str::<Session>(cookie.value()) {
                    if val.id == __create_identifier(&req) {
                        return val.user_id;
                    };
                }
            }
        };
        None
    }

    fn update_<B>(&self, key: Option<String>, res: &mut ServiceResponse<B>) -> Result<(), ()> {
        let key = match key {
            Some(x) if x == "".to_owned() => None,
            Some(key) => Some(key),
            _ => return Ok(()),
        };

        let session = Session {
            id: _create_identifier(res.request()),
            user_id: key,
        };

        let value = serde_json::to_string(&session).map_err(|_| ())?;

        let mut cookie = Cookie::new(self.name.clone(), value);

        cookie.set_path(self.path.clone());
        cookie.set_secure(self.secure);
        cookie.set_http_only(self.http_only);

        if let Some(ref domain) = self.domain {
            cookie.set_domain(domain.clone());
        }

        if let Some(expires_in) = self.expires_in {
            cookie.set_expires(OffsetDateTime::now_utc() + expires_in);
        }

        if let Some(max_age) = self.max_age {
            cookie.set_max_age(max_age);
        }

        if let Some(same_site) = self.same_site {
            cookie.set_same_site(same_site);
        }

        let mut jar = CookieJar::new();

        jar.private(&self.key).add(cookie);

        for cookie in jar.delta() {
            let val = HeaderValue::from_str(&cookie.encoded().to_string()).map_err(|_| ())?;
            res.headers_mut().append(SET_COOKIE, val);
        }

        Ok(())
    }
}
