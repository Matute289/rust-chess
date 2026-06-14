use bevy::prelude::*;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::ButtonState;
use crate::{auth::UserSession, state::AppState};

// ─── Resource ────────────────────────────────────────────────────────────────

#[derive(Resource, Default)]
pub struct FeedbackUi {
    pub open: bool,
    pub text: String,
    pub sent: bool,
}

// ─── Components ───────────────────────────────────────────────────────────────

#[derive(Component)] pub struct BtnFeedback;
#[derive(Component)] pub struct BtnSuggestLesson;
#[derive(Component)] struct FeedbackModalRoot;
#[derive(Component)] struct BtnFeedbackSend;
#[derive(Component)] struct BtnFeedbackClose;

// ─── Modal builder ────────────────────────────────────────────────────────────

fn spawn_modal(commands: &mut Commands, asset_server: &AssetServer, text: &str, sent: bool) {
    let font: Handle<Font> = asset_server.load("fonts/DejaVuSans-Bold.ttf");

    commands.spawn((
        NodeBundle {
            style: Style {
                position_type:   PositionType::Absolute,
                width:           Val::Percent(100.0),
                height:          Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items:     AlignItems::Center,
                ..default()
            },
            background_color: BackgroundColor(Color::rgba(0.0, 0.0, 0.0, 0.78)),
            z_index: ZIndex::Global(60),
            ..default()
        },
        FeedbackModalRoot,
    ))
    .with_children(|root| {
        root.spawn(NodeBundle {
            style: Style {
                flex_direction: FlexDirection::Column,
                padding:        UiRect::all(Val::Px(28.0)),
                row_gap:        Val::Px(16.0),
                min_width:      Val::Px(420.0),
                max_width:      Val::Px(520.0),
                border:         UiRect::all(Val::Px(1.0)),
                ..default()
            },
            background_color: BackgroundColor(Color::rgba(0.07, 0.07, 0.18, 0.97)),
            border_color:     BorderColor(Color::rgba(0.40, 0.40, 0.70, 0.60)),
            ..default()
        })
        .with_children(|dlg| {
            dlg.spawn(TextBundle::from_section(
                "Reportar problema / Sugerencia",
                TextStyle { font: font.clone(), font_size: 21.0, color: Color::rgb(0.90, 0.90, 1.00) },
            ));

            if sent {
                dlg.spawn(TextBundle::from_section(
                    "¡Gracias! Recibimos tu mensaje.",
                    TextStyle { font: font.clone(), font_size: 18.0, color: Color::rgb(0.38, 0.95, 0.52) },
                ));
                dlg.spawn((
                    ButtonBundle {
                        style: Style {
                            padding: UiRect { left: Val::Px(24.0), right: Val::Px(24.0), top: Val::Px(8.0), bottom: Val::Px(8.0) },
                            align_self: AlignSelf::Center,
                            border: UiRect::all(Val::Px(1.0)),
                            ..default()
                        },
                        background_color: BackgroundColor(Color::rgba(0.14, 0.14, 0.30, 0.90)),
                        border_color:     BorderColor(Color::rgba(0.38, 0.38, 0.65, 0.55)),
                        ..default()
                    },
                    BtnFeedbackClose,
                ))
                .with_children(|p| {
                    p.spawn(TextBundle::from_section("Cerrar",
                        TextStyle { font: font.clone(), font_size: 16.0, color: Color::rgb(0.80, 0.80, 1.00) }));
                });
            } else {
                // Text input area
                dlg.spawn(NodeBundle {
                    style: Style {
                        padding:    UiRect::all(Val::Px(12.0)),
                        min_height: Val::Px(90.0),
                        border:     UiRect::all(Val::Px(1.0)),
                        ..default()
                    },
                    background_color: BackgroundColor(Color::rgba(0.04, 0.04, 0.12, 0.95)),
                    border_color:     BorderColor(Color::rgba(0.38, 0.38, 0.65, 0.70)),
                    ..default()
                })
                .with_children(|p| {
                    let (display, color) = if text.is_empty() {
                        ("Escribí tu problema o sugerencia...".to_string(), Color::rgba(0.45, 0.45, 0.60, 0.80))
                    } else {
                        (format!("{}_", text), Color::rgb(0.90, 0.90, 1.00))
                    };
                    p.spawn(TextBundle::from_section(display,
                        TextStyle { font: font.clone(), font_size: 15.0, color }));
                });

                // Button row
                dlg.spawn(NodeBundle {
                    style: Style {
                        flex_direction:  FlexDirection::Row,
                        justify_content: JustifyContent::FlexEnd,
                        column_gap:      Val::Px(10.0),
                        ..default()
                    },
                    ..default()
                })
                .with_children(|row| {
                    row.spawn((
                        ButtonBundle {
                            style: Style {
                                padding: UiRect { left: Val::Px(18.0), right: Val::Px(18.0), top: Val::Px(7.0), bottom: Val::Px(7.0) },
                                border: UiRect::all(Val::Px(1.0)),
                                ..default()
                            },
                            background_color: BackgroundColor(Color::rgba(0.18, 0.08, 0.08, 0.90)),
                            border_color:     BorderColor(Color::rgba(0.55, 0.25, 0.25, 0.55)),
                            ..default()
                        },
                        BtnFeedbackClose,
                    ))
                    .with_children(|p| {
                        p.spawn(TextBundle::from_section("Cancelar",
                            TextStyle { font: font.clone(), font_size: 15.0, color: Color::rgb(0.88, 0.68, 0.68) }));
                    });

                    row.spawn((
                        ButtonBundle {
                            style: Style {
                                padding: UiRect { left: Val::Px(18.0), right: Val::Px(18.0), top: Val::Px(7.0), bottom: Val::Px(7.0) },
                                border: UiRect::all(Val::Px(1.0)),
                                ..default()
                            },
                            background_color: BackgroundColor(Color::rgba(0.08, 0.24, 0.08, 0.90)),
                            border_color:     BorderColor(Color::rgba(0.25, 0.60, 0.25, 0.55)),
                            ..default()
                        },
                        BtnFeedbackSend,
                    ))
                    .with_children(|p| {
                        p.spawn(TextBundle::from_section("Enviar",
                            TextStyle { font: font.clone(), font_size: 15.0, color: Color::rgb(0.68, 0.95, 0.68) }));
                    });
                });
            }
        });
    });
}

// ─── Systems ──────────────────────────────────────────────────────────────────

fn handle_feedback_btn(
    q:            Query<&Interaction, (Changed<Interaction>, With<BtnFeedback>)>,
    mut ui:       ResMut<FeedbackUi>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    for i in &q {
        if *i == Interaction::Pressed {
            ui.open = true;
            ui.text.clear();
            ui.sent = false;
            spawn_modal(&mut commands, &asset_server, "", false);
        }
    }
}

fn handle_feedback_input(
    mut ui:       ResMut<FeedbackUi>,
    mut key_ev:   EventReader<KeyboardInput>,
    modal_q:      Query<Entity, With<FeedbackModalRoot>>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    if !ui.open || ui.sent { return; }
    let mut changed = false;
    let mut close   = false;

    for ev in key_ev.read() {
        if ev.state != ButtonState::Pressed { continue; }
        match &ev.logical_key {
            Key::Character(s) => {
                for ch in s.chars() {
                    if !ch.is_control() && ui.text.len() < 1500 {
                        ui.text.push(ch);
                        changed = true;
                    }
                }
            }
            Key::Backspace => { ui.text.pop(); changed = true; }
            Key::Escape    => { close = true; }
            _ => {}
        }
    }

    if close {
        ui.open = false;
        for e in &modal_q { commands.entity(e).despawn_recursive(); }
        return;
    }
    if changed {
        for e in &modal_q { commands.entity(e).despawn_recursive(); }
        let text = ui.text.clone();
        spawn_modal(&mut commands, &asset_server, &text, false);
    }
}

fn handle_feedback_send(
    q:            Query<&Interaction, (Changed<Interaction>, With<BtnFeedbackSend>)>,
    mut ui:       ResMut<FeedbackUi>,
    modal_q:      Query<Entity, With<FeedbackModalRoot>>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    session:      Res<UserSession>,
) {
    for i in &q {
        if *i != Interaction::Pressed { continue; }
        if ui.text.trim().is_empty() { continue; }

        #[cfg(target_arch = "wasm32")]
        {
            let text = ui.text.clone();
            let _ = session.jwt.clone(); // suppress unused warning
            wasm_bindgen_futures::spawn_local(async move {
                post_feedback_async(text, "home".to_string()).await;
            });
        }
        let _ = &session;

        ui.sent = true;
        for e in &modal_q { commands.entity(e).despawn_recursive(); }
        spawn_modal(&mut commands, &asset_server, &ui.text, true);
    }
}

fn handle_feedback_close(
    q:            Query<&Interaction, (Changed<Interaction>, With<BtnFeedbackClose>)>,
    mut ui:       ResMut<FeedbackUi>,
    modal_q:      Query<Entity, With<FeedbackModalRoot>>,
    mut commands: Commands,
) {
    for i in &q {
        if *i == Interaction::Pressed {
            ui.open = false;
            ui.text.clear();
            ui.sent = false;
            for e in &modal_q { commands.entity(e).despawn_recursive(); }
        }
    }
}

fn handle_suggest_lesson(
    q:       Query<&Interaction, (Changed<Interaction>, With<BtnSuggestLesson>)>,
    mut sent: Local<bool>,
    session: Res<UserSession>,
) {
    for i in &q {
        if *i != Interaction::Pressed { continue; }
        if *sent { continue; }
        *sent = true;

        #[cfg(target_arch = "wasm32")]
        {
            let _ = session.jwt.clone();
            wasm_bindgen_futures::spawn_local(async move {
                post_feedback_async("Solicitud de nueva lección".to_string(), "lessons".to_string()).await;
            });
        }
        let _ = &session;
    }
}

fn despawn_on_home_exit(
    mut commands: Commands,
    modal_q:      Query<Entity, With<FeedbackModalRoot>>,
    mut ui:       ResMut<FeedbackUi>,
) {
    for e in &modal_q { commands.entity(e).despawn_recursive(); }
    ui.open = false;
    ui.text.clear();
    ui.sent = false;
}

// ─── WASM async POST ──────────────────────────────────────────────────────────

#[cfg(target_arch = "wasm32")]
pub async fn post_feedback_async(message: String, context: String) {
    #[derive(serde::Serialize)]
    struct Body { message: String, context: String }
    let Ok(body) = serde_json::to_string(&Body { message, context }) else { return };
    let _ = gloo_net::http::Request::post(
        "https://rustchess.greenmountain.dev/api/feedback",
    )
    .header("Content-Type", "application/json")
    .body(body)
    .unwrap()
    .send()
    .await;
}

// ─── Plugin ───────────────────────────────────────────────────────────────────

pub struct FeedbackUiPlugin;

impl Plugin for FeedbackUiPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<FeedbackUi>()
            .add_systems(OnExit(AppState::Home), despawn_on_home_exit)
            .add_systems(Update, (
                handle_feedback_btn,
                handle_feedback_input,
                handle_feedback_send,
                handle_feedback_close,
            ).run_if(in_state(AppState::Home)))
            .add_systems(Update,
                handle_suggest_lesson.run_if(in_state(AppState::Lessons)),
            );
    }
}
