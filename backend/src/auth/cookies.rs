use crate::config::AuthConfig;
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};

pub const ACCESS_COOKIE: &str = "am_access";
pub const REFRESH_COOKIE: &str = "am_refresh";

pub const ACCESS_MAX_AGE_SECS: i64 = 3600;
pub const REFRESH_MAX_AGE_SECS: i64 = 30 * 24 * 3600;

pub fn set_auth_cookies(
    jar: CookieJar,
    cfg: &AuthConfig,
    access_token: &str,
    refresh_token: Option<&str>,
) -> CookieJar {
    let mut jar = jar.add(build_cookie(
        cfg,
        ACCESS_COOKIE,
        access_token.to_string(),
        ACCESS_MAX_AGE_SECS,
    ));
    if let Some(rt) = refresh_token {
        jar = jar.add(build_cookie(
            cfg,
            REFRESH_COOKIE,
            rt.to_string(),
            REFRESH_MAX_AGE_SECS,
        ));
    }
    jar
}

pub fn clear_auth_cookies(jar: CookieJar, cfg: &AuthConfig) -> CookieJar {
    jar.add(build_cookie(cfg, ACCESS_COOKIE, String::new(), 0))
        .add(build_cookie(cfg, REFRESH_COOKIE, String::new(), 0))
}

fn build_cookie(cfg: &AuthConfig, name: &str, value: String, max_age_secs: i64) -> Cookie<'static> {
    let mut c = Cookie::new(name.to_string(), value);
    c.set_http_only(true);
    c.set_secure(cfg.cookie_secure);
    c.set_same_site(SameSite::Lax);
    c.set_path("/");
    if let Some(domain) = cfg.cookie_domain.clone() {
        c.set_domain(domain);
    }
    c.set_max_age(time::Duration::seconds(max_age_secs));
    c
}

pub fn extract(jar: &CookieJar, name: &str) -> Option<String> {
    jar.get(name).map(|c| c.value().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> AuthConfig {
        AuthConfig {
            fusionauth_base_url: "http://x".into(),
            fusionauth_tenant_id: "t".into(),
            fusionauth_application_id: "a".into(),
            fusionauth_client_id: "a".into(),
            fusionauth_client_secret: "s".into(),
            fusionauth_api_key: "k".into(),
            cookie_domain: None,
            cookie_secure: true,
            disable_auth: false,
            is_development: false,
        }
    }

    #[test]
    fn round_trip_sets_and_reads() {
        let jar = CookieJar::new();
        let jar = set_auth_cookies(jar, &cfg(), "access-val", Some("refresh-val"));
        assert_eq!(extract(&jar, ACCESS_COOKIE).as_deref(), Some("access-val"));
        assert_eq!(extract(&jar, REFRESH_COOKIE).as_deref(), Some("refresh-val"));
    }

    #[test]
    fn clear_overwrites_with_zero_age() {
        let jar = CookieJar::new();
        let jar = set_auth_cookies(jar, &cfg(), "a", Some("r"));
        let jar = clear_auth_cookies(jar, &cfg());
        let access = jar.get(ACCESS_COOKIE).unwrap();
        assert_eq!(access.value(), "");
        assert_eq!(access.max_age(), Some(time::Duration::seconds(0)));
    }
}
