use serde::{Deserialize, Serialize};

#[cfg(debug_assertions)]
pub const HTTP_BASE: &str = "http://localhost:8080";
#[cfg(not(debug_assertions))]
pub const HTTP_BASE: &str = "";

fn url(path: &str) -> String {
    format!("{HTTP_BASE}{path}")
}

// ── Response types ────────────────────────────────────────────────────────────

#[derive(Clone, Debug, Deserialize)]
pub struct MeResponse {
    pub id: i64,
    pub username: String,
    #[serde(default)]
    pub email_verified: bool,
}

#[derive(Clone, Debug, Deserialize)]
pub struct UserProfile {
    pub id: i64,
    pub username: String,
    pub created_at: i64,
    pub total_games: i64,
    pub wins: i64,
    pub losses: i64,
    pub draws: i64,
}

#[derive(Clone, Debug, Deserialize)]
pub struct GameSummary {
    pub id: i64,
    pub game_id: String,
    pub room_code: String,
    pub started_at: i64,
    pub ended_at: Option<i64>,
    pub result: Option<String>,
    pub outcome: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct GamesResponse {
    pub games: Vec<GameSummary>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Participant {
    pub player_id: i64,
    pub outcome: Option<String>,
    pub username: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct GameDetail {
    pub id: i64,
    pub game_id: String,
    pub room_code: String,
    pub started_at: i64,
    pub ended_at: Option<i64>,
    pub result: Option<String>,
    pub participants: Vec<Participant>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct PageContent {
    pub title: String,
    pub content: String,
}

// ── Request bodies ────────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct RegisterBody<'a> {
    pub username: &'a str,
    pub email: &'a str,
    pub password: &'a str,
}

#[derive(Serialize)]
pub struct LoginBody<'a> {
    pub username: &'a str,
    pub password: &'a str,
}

// ── Fetch helpers ─────────────────────────────────────────────────────────────

pub async fn get_me() -> Result<MeResponse, String> {
    let resp = gloo_net::http::Request::get(&url("/auth/me"))
        .credentials(web_sys::RequestCredentials::Include)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if resp.status() == 200 {
        resp.json::<MeResponse>().await.map_err(|e| e.to_string())
    } else {
        Err(format!("status {}", resp.status()))
    }
}

pub async fn post_login(username: &str, password: &str) -> Result<MeResponse, String> {
    let body = LoginBody { username, password };
    let resp = gloo_net::http::Request::post(&url("/auth/login"))
        .credentials(web_sys::RequestCredentials::Include)
        .json(&body)
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if resp.status() == 200 {
        resp.json::<MeResponse>().await.map_err(|e| e.to_string())
    } else {
        let text = resp.text().await.unwrap_or_default();
        Err(text)
    }
}

pub async fn post_register(username: &str, email: &str, password: &str) -> Result<MeResponse, String> {
    let body = RegisterBody { username, email, password };
    let resp = gloo_net::http::Request::post(&url("/auth/register"))
        .credentials(web_sys::RequestCredentials::Include)
        .json(&body)
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if resp.status() == 201 {
        resp.json::<MeResponse>().await.map_err(|e| e.to_string())
    } else {
        let text = resp.text().await.unwrap_or_default();
        Err(text)
    }
}

pub async fn post_logout() -> Result<(), String> {
    let resp = gloo_net::http::Request::post(&url("/auth/logout"))
        .credentials(web_sys::RequestCredentials::Include)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if resp.status() == 204 {
        Ok(())
    } else {
        Err(format!("status {}", resp.status()))
    }
}

pub async fn get_user_profile(username: &str) -> Result<UserProfile, String> {
    let resp = gloo_net::http::Request::get(&url(&format!("/users/{username}")))
        .credentials(web_sys::RequestCredentials::Include)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if resp.status() == 200 {
        resp.json::<UserProfile>().await.map_err(|e| e.to_string())
    } else {
        Err(format!("status {}", resp.status()))
    }
}

pub async fn get_user_games(username: &str, page: i64) -> Result<GamesResponse, String> {
    let resp = gloo_net::http::Request::get(&url(&format!(
        "/users/{username}/games?page={page}&per_page=20"
    )))
    .credentials(web_sys::RequestCredentials::Include)
    .send()
    .await
    .map_err(|e| e.to_string())?;
    if resp.status() == 200 {
        resp.json::<GamesResponse>().await.map_err(|e| e.to_string())
    } else {
        Err(format!("status {}", resp.status()))
    }
}

pub async fn get_game_detail(id: i64) -> Result<GameDetail, String> {
    let resp = gloo_net::http::Request::get(&url(&format!("/games/{id}")))
        .credentials(web_sys::RequestCredentials::Include)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if resp.status() == 200 {
        resp.json::<GameDetail>().await.map_err(|e| e.to_string())
    } else {
        Err(format!("status {}", resp.status()))
    }
}

pub async fn get_verify_email(token: &str) -> Result<(), String> {
    let resp = gloo_net::http::Request::get(&url(&format!("/auth/verify-email?token={token}")))
        .credentials(web_sys::RequestCredentials::Include)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if resp.status() == 200 {
        Ok(())
    } else {
        let text = resp.text().await.unwrap_or_default();
        Err(text)
    }
}

pub async fn post_resend_verification() -> Result<(), String> {
    let resp = gloo_net::http::Request::post(&url("/auth/resend-verification"))
        .credentials(web_sys::RequestCredentials::Include)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if resp.status() == 200 {
        Ok(())
    } else {
        Err(format!("status {}", resp.status()))
    }
}

pub async fn post_forgot_password(email: &str) -> Result<(), String> {
    let body = serde_json::json!({ "email": email });
    let resp = gloo_net::http::Request::post(&url("/auth/forgot-password"))
        .credentials(web_sys::RequestCredentials::Include)
        .json(&body)
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if resp.status() == 200 {
        Ok(())
    } else {
        Err(format!("status {}", resp.status()))
    }
}

pub async fn post_reset_password(token: &str, new_password: &str) -> Result<(), String> {
    let body = serde_json::json!({ "token": token, "new_password": new_password });
    let resp = gloo_net::http::Request::post(&url("/auth/reset-password"))
        .credentials(web_sys::RequestCredentials::Include)
        .json(&body)
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if resp.status() == 200 {
        Ok(())
    } else {
        let text = resp.text().await.unwrap_or_default();
        Err(text)
    }
}

pub async fn get_page(slug: &str, lang: &str) -> Result<PageContent, String> {
    let resp = gloo_net::http::Request::get(&url(&format!("/pages/{slug}?lang={lang}")))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if resp.status() == 200 {
        resp.json::<PageContent>().await.map_err(|e| e.to_string())
    } else {
        Err(format!("status {}", resp.status()))
    }
}

// ── Utilities ─────────────────────────────────────────────────────────────────

/// Maps to the `Intl.DateTimeFormat` options object accepted by `Date.toLocaleString`.
/// `Default` passes no options (browser default: full date + time).
pub struct DateFormatOptions {
    /// "full" | "long" | "medium" | "short" — omit to suppress date part
    pub date_style: Option<&'static str>,
    /// "full" | "long" | "medium" | "short" — omit to suppress time part
    pub time_style: Option<&'static str>,
}

impl Default for DateFormatOptions {
    fn default() -> Self {
        Self { date_style: None, time_style: None }
    }
}

impl DateFormatOptions {
    pub fn date_only() -> Self {
        Self { date_style: Some("short"), time_style: None }
    }

    pub fn time_only() -> Self {
        Self { date_style: None, time_style: Some("short") }
    }

    pub fn date_time() -> Self {
        Self { date_style: Some("short"), time_style: Some("short") }
    }

    fn to_js_value(&self) -> wasm_bindgen::JsValue {
        if self.date_style.is_none() && self.time_style.is_none() {
            return wasm_bindgen::JsValue::UNDEFINED;
        }
        let obj = js_sys::Object::new();
        if let Some(v) = self.date_style {
            let _ = js_sys::Reflect::set(&obj, &"dateStyle".into(), &v.into());
        }
        if let Some(v) = self.time_style {
            let _ = js_sys::Reflect::set(&obj, &"timeStyle".into(), &v.into());
        }
        obj.into()
    }
}

pub fn format_ts(ts: i64, locale: &str, opts: &DateFormatOptions) -> String {
    let ms = (ts * 1000) as f64;
    let date = js_sys::Date::new(&wasm_bindgen::JsValue::from_f64(ms));
    date.to_locale_string(locale, &opts.to_js_value())
        .as_string()
        .unwrap_or_default()
}
