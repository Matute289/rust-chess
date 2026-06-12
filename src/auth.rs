use bevy::prelude::*;
use std::sync::{Arc, Mutex};

#[derive(Resource, Default, Clone)]
pub struct UserSession {
    pub user_id:      Option<String>,
    pub jwt:          Option<String>,
    pub display_name: Option<String>,
    pub elo:          Option<i32>,
}

impl UserSession {
    pub fn is_logged_in(&self) -> bool { self.jwt.is_some() }
}

pub struct MeData {
    pub user_id:      String,
    pub display_name: Option<String>,
    pub elo:          i32,
}

#[derive(Resource, Clone)]
pub struct MeFetchState(pub Arc<Mutex<Option<MeData>>>);

impl Default for MeFetchState {
    fn default() -> Self { Self(Arc::new(Mutex::new(None))) }
}

pub struct AuthPlugin;

impl Plugin for AuthPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UserSession>()
           .init_resource::<MeFetchState>()
           .add_systems(Startup, load_session_from_storage)
           .add_systems(Update, poll_me_result);
    }
}

fn load_session_from_storage(
    mut session: ResMut<UserSession>,
    fetch_state: Res<MeFetchState>,
) {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(jwt) = read_jwt_from_url_or_storage() {
            let jwt_clone = jwt.clone();
            session.jwt = Some(jwt);
            let state = fetch_state.0.clone();
            wasm_bindgen_futures::spawn_local(async move {
                let result = fetch_me(jwt_clone).await;
                *state.lock().unwrap() = result;
            });
        }
    }
    let _ = fetch_state; // silence unused warning on non-WASM
}

fn poll_me_result(
    fetch_state: Res<MeFetchState>,
    mut session: ResMut<UserSession>,
) {
    if let Ok(mut guard) = fetch_state.0.try_lock() {
        if let Some(data) = guard.take() {
            session.user_id = Some(data.user_id);
            session.display_name = data.display_name;
            session.elo = Some(data.elo);
        }
    }
}

#[cfg(target_arch = "wasm32")]
async fn fetch_me(jwt: String) -> Option<MeData> {
    #[derive(serde::Deserialize)]
    struct MeResponse {
        id: String,
        display_name: Option<String>,
        elo: i32,
    }

    let resp = gloo_net::http::Request::get("https://rustchess.greenmountain.dev/api/me")
        .header("Authorization", &format!("Bearer {}", jwt))
        .send()
        .await
        .ok()?;

    let data: MeResponse = resp.json().await.ok()?;
    Some(MeData {
        user_id:      data.id,
        display_name: data.display_name,
        elo:          data.elo,
    })
}

#[cfg(target_arch = "wasm32")]
fn read_jwt_from_url_or_storage() -> Option<String> {
    use web_sys::window;

    let win = window()?;
    let location = win.location();

    if let Ok(search) = location.search() {
        if let Some(jwt) = parse_jwt_from_query(&search) {
            if let Ok(Some(storage)) = win.local_storage() {
                let _ = storage.set_item("chess_jwt", &jwt);
            }
            let _ = location.set_search("");
            return Some(jwt);
        }
    }

    if let Ok(Some(storage)) = win.local_storage() {
        if let Ok(Some(jwt)) = storage.get_item("chess_jwt") {
            return Some(jwt);
        }
    }

    None
}

#[cfg(target_arch = "wasm32")]
fn parse_jwt_from_query(search: &str) -> Option<String> {
    let query = search.trim_start_matches('?');
    for part in query.split('&') {
        if let Some(val) = part.strip_prefix("jwt=") {
            if !val.is_empty() {
                return Some(val.to_string());
            }
        }
    }
    None
}
