use bevy::prelude::*;

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

pub struct AuthPlugin;

impl Plugin for AuthPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UserSession>()
           .add_systems(Startup, load_session_from_storage);
    }
}

fn load_session_from_storage(mut session: ResMut<UserSession>) {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(jwt) = read_jwt_from_url_or_storage() {
            session.jwt = Some(jwt);
        }
    }
    // On native (non-WASM), session stays empty — no localStorage
}

#[cfg(target_arch = "wasm32")]
fn read_jwt_from_url_or_storage() -> Option<String> {
    use web_sys::window;

    let win = window()?;
    let location = win.location();

    // Check URL query param first (?jwt=...)
    if let Ok(search) = location.search() {
        if let Some(jwt) = parse_jwt_from_query(&search) {
            // Store in localStorage and clean the URL
            if let Ok(Some(storage)) = win.local_storage() {
                let _ = storage.set_item("chess_jwt", &jwt);
            }
            let _ = location.set_search("");
            return Some(jwt);
        }
    }

    // Fall back to localStorage
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
