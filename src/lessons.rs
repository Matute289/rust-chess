use bevy::prelude::*;
use std::sync::{Arc, Mutex};
use crate::{
    auth::UserSession,
    board::GameHistory,
    state::{AppState, GameConfig, GameMode, LessonMode, LessonSetup},
};

// ─── Lesson Data ─────────────────────────────────────────────────────────────

pub struct LessonExercise {
    pub fen:        &'static str,
    pub answer_uci: &'static str,
    pub hint_text:  &'static str,
}

pub struct LessonData {
    pub title:       &'static str,
    pub description: &'static str,
    pub exercises:   [LessonExercise; 3],
}

pub const LESSONS: [LessonData; 5] = [
    LessonData {
        title: "Mate en 1: Torre",
        description: "La torre controla filas y columnas enteras. Buscá una línea directa hacia el rey enemigo sin piezas en el camino.",
        exercises: [
            LessonExercise { fen: "6k1/5ppp/8/8/8/8/5PPP/4R1K1 w - - 0 1", answer_uci: "e1e8", hint_text: "La torre en e1 puede alcanzar el rey en g8 por la columna e... pero el rey está en g8. ¿Hay una casilla en la misma fila?" },
            LessonExercise { fen: "k7/pp6/1R6/8/8/8/8/1R5K w - - 0 1",     answer_uci: "b6b8", hint_text: "La torre en b6 puede subir por la columna b. El rey negro está en a8." },
            LessonExercise { fen: "6pk/7p/8/8/8/8/6Q1/6RK w - - 0 1",      answer_uci: "g2g8", hint_text: "La dama controla diagonales y rectas. ¿Cuál pieza puede dar jaque mate en g8?" },
        ],
    },
    LessonData {
        title: "Mate en 1: Dama",
        description: "La dama es la pieza más poderosa: combina los movimientos de torre y alfil. Encontrá la casilla desde donde ataca al rey sin escapatoria.",
        exercises: [
            LessonExercise { fen: "6k1/5ppp/8/8/8/8/5PPP/5QK1 w - - 0 1",  answer_uci: "f1f8", hint_text: "La dama en f1 controla la columna f. El rey negro está en g8 sin salida por la fila 8." },
            LessonExercise { fen: "6k1/5p1p/6pQ/5N2/8/8/8/6K1 w - - 0 1", answer_uci: "h6g7", hint_text: "La dama en h6 puede ir a g7. El caballo en f5 controla e7 y h6. ¿Hay escapatoria para el rey?" },
            LessonExercise { fen: "k7/8/1K1Q4/8/8/8/8/8 w - - 0 1",        answer_uci: "d6d8", hint_text: "La dama en d6 controla toda la fila 8. El rey blanco en b6 limita las fugas del rey negro en a8." },
        ],
    },
    LessonData {
        title: "Horquilla de Caballo",
        description: "El caballo es la única pieza que salta sobre otras. Una horquilla ataca dos piezas enemigas al mismo tiempo desde un punto central.",
        exercises: [
            LessonExercise { fen: "4k3/3q4/8/3N4/8/8/8/4K3 w - - 0 1", answer_uci: "d5f6", hint_text: "El caballo en d5 puede ir a f6 o e7. ¿Desde cuál casilla ataca tanto al rey en e8 como a la dama en d7?" },
            LessonExercise { fen: "4k3/7r/8/8/6N1/8/8/4K3 w - - 0 1", answer_uci: "g4f6", hint_text: "El caballo en g4 necesita atacar al rey en e8 y a la torre en h7 simultáneamente." },
            LessonExercise { fen: "r3k3/8/8/3N4/8/8/8/4K3 w - - 0 1", answer_uci: "d5c7", hint_text: "El caballo en d5 puede ir a c7, atacando al rey en e8 y a la torre en a8." },
        ],
    },
    LessonData {
        title: "La Clavada",
        description: "Una clavada inmoviliza una pieza porque si se mueve, expone una pieza más valiosa (o al rey) a captura.",
        exercises: [
            LessonExercise { fen: "4k3/4r3/8/8/8/8/4Q3/4K3 w - - 0 1", answer_uci: "e2e7", hint_text: "La dama en e2 puede clavar la torre en e7 contra el rey en e8. Si la torre se mueve, el rey queda expuesto." },
            LessonExercise { fen: "7k/8/8/4r3/8/8/1B6/4K3 w - - 0 1",  answer_uci: "b2e5", hint_text: "El alfil en b2 puede clavar la torre en e5 contra el rey en h8 por la diagonal." },
            LessonExercise { fen: "4k3/4r3/8/4R3/8/8/8/4K3 w - - 0 1", answer_uci: "e5e7", hint_text: "La torre blanca en e5 puede clavar la torre negra en e7 contra el rey en e8." },
        ],
    },
    LessonData {
        title: "Ensartada",
        description: "La ensartada ataca una pieza valiosa que, al moverse, expone la pieza detrás de ella a captura.",
        exercises: [
            LessonExercise { fen: "8/8/8/8/8/8/1K6/R3kq2 w - - 0 1", answer_uci: "a1e1", hint_text: "La torre en a1 puede ensartar al rey negro en e1 contra la dama en f1. El rey debe moverse y la dama queda capturada." },
            LessonExercise { fen: "8/8/8/8/1k5q/8/8/R3K3 w - - 0 1", answer_uci: "a1a4", hint_text: "La torre en a1 puede subir a a4, atacando al rey en b4. Cuando el rey se mueva, la dama en h4 queda expuesta." },
            LessonExercise { fen: "1q6/k7/8/8/8/8/8/R3K3 w - - 0 1", answer_uci: "a1a8", hint_text: "La torre en a1 sube a a8, ensartando al rey en a7 contra la dama en b8. El rey debe moverse." },
        ],
    },
];

// ─── Progress loading (WASM) ──────────────────────────────────────────────────

#[derive(Clone, Default)]
pub struct LoadedProgress {
    pub stars: [u8; 5],
}

#[derive(Resource, Clone)]
pub struct ProgressFetchState(pub Arc<Mutex<Option<LoadedProgress>>>);
impl Default for ProgressFetchState {
    fn default() -> Self { Self(Arc::new(Mutex::new(None))) }
}

#[derive(Resource, Default)]
struct CachedProgress(Option<LoadedProgress>);

#[derive(Resource, Default)]
pub struct SelectedLessonMode(pub LessonMode);

// ─── Components (Lessons screen) ─────────────────────────────────────────────

#[derive(Component)] struct LessonsRoot;
#[derive(Component)] struct BtnLessonStart(usize);
#[derive(Component)] struct BtnLessonsBack;
#[derive(Component)] struct BtnModeInteractive;
#[derive(Component)] struct BtnModeGuided;
#[derive(Component)] struct ModeInteractiveMarker;
#[derive(Component)] struct ModeGuidedMarker;

// ─── Components (Playing overlay) ────────────────────────────────────────────

#[derive(Component)] pub struct LessonOverlayRoot;
#[derive(Component)] pub struct BtnLessonHint;
#[derive(Component)] pub struct BtnLessonRetry;
#[derive(Component)] pub struct BtnLessonExit;
#[derive(Component)] pub struct LessonSuccessOverlay;
#[derive(Component)] pub struct BtnLessonNext;
#[derive(Component)] pub struct BtnLessonFinish;

// ─── UI Helpers ──────────────────────────────────────────────────────────────

fn small_btn(parent: &mut ChildBuilder, font: Handle<Font>, text: &str, marker: impl Bundle, bg: Color) {
    parent.spawn((
        ButtonBundle {
            style: Style {
                padding: UiRect { left: Val::Px(18.0), right: Val::Px(18.0), top: Val::Px(10.0), bottom: Val::Px(10.0) },
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            background_color: BackgroundColor(bg),
            border_color: BorderColor(Color::rgba(1., 1., 1., 0.15)),
            ..default()
        },
        marker,
    ))
    .with_children(|p| {
        p.spawn(TextBundle::from_section(text, TextStyle { font, font_size: 18.0, color: Color::rgb(0.92, 0.92, 0.92) }));
    });
}

fn star_string(stars: u8) -> &'static str {
    match stars {
        0 => "☆☆",
        1 => "★☆",
        _ => "★★",
    }
}

// ─── Lessons Screen Builder ───────────────────────────────────────────────────

fn build_lessons_root(
    commands:    &mut Commands,
    asset_server: &AssetServer,
    progress:    &LoadedProgress,
    mode:        LessonMode,
) {
    let font: Handle<Font> = asset_server.load("fonts/FiraSans-Bold.ttf");

    let mode_interactive_bg = if mode == LessonMode::Interactive {
        Color::rgba(0.20, 0.55, 0.20, 0.95)
    } else {
        Color::rgba(0.10, 0.20, 0.10, 0.80)
    };
    let mode_guided_bg = if mode == LessonMode::Guided {
        Color::rgba(0.20, 0.35, 0.65, 0.95)
    } else {
        Color::rgba(0.08, 0.12, 0.28, 0.80)
    };

    commands.spawn((
        NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                row_gap: Val::Px(12.0),
                ..default()
            },
            background_color: BackgroundColor(Color::rgba(0.04, 0.04, 0.10, 0.97)),
            z_index: ZIndex::Global(5),
            ..default()
        },
        LessonsRoot,
    ))
    .with_children(|root| {
        root.spawn(TextBundle::from_section(
            "CURRÍCULO DE LECCIONES",
            TextStyle { font: font.clone(), font_size: 44.0, color: Color::rgb(0.95, 0.92, 0.80) },
        ));
        root.spawn(TextBundle::from_section(
            "TÁCTICA",
            TextStyle { font: font.clone(), font_size: 22.0, color: Color::rgb(0.65, 0.65, 0.80) },
        ));

        // Mode toggle row
        root.spawn(NodeBundle {
            style: Style { flex_direction: FlexDirection::Row, column_gap: Val::Px(12.0), margin: UiRect { top: Val::Px(4.0), bottom: Val::Px(8.0), ..default() }, ..default() },
            ..default()
        })
        .with_children(|row| {
            row.spawn((
                ButtonBundle {
                    style: Style {
                        padding: UiRect { left: Val::Px(24.0), right: Val::Px(24.0), top: Val::Px(10.0), bottom: Val::Px(10.0) },
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    background_color: BackgroundColor(mode_interactive_bg),
                    border_color: BorderColor(Color::rgba(0.30, 0.70, 0.30, 0.60)),
                    ..default()
                },
                BtnModeInteractive,
            ))
            .with_children(|p| {
                p.spawn(TextBundle::from_section(
                    "◉ Interactivo",
                    TextStyle { font: font.clone(), font_size: 20.0, color: Color::rgb(0.90, 0.95, 0.90) },
                ));
            });

            row.spawn((
                ButtonBundle {
                    style: Style {
                        padding: UiRect { left: Val::Px(24.0), right: Val::Px(24.0), top: Val::Px(10.0), bottom: Val::Px(10.0) },
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    background_color: BackgroundColor(mode_guided_bg),
                    border_color: BorderColor(Color::rgba(0.30, 0.50, 0.90, 0.60)),
                    ..default()
                },
                BtnModeGuided,
            ))
            .with_children(|p| {
                p.spawn(TextBundle::from_section(
                    "◎ Guiado",
                    TextStyle { font: font.clone(), font_size: 20.0, color: Color::rgb(0.85, 0.88, 0.98) },
                ));
            });
        });

        // Lesson list
        for (idx, lesson) in LESSONS.iter().enumerate() {
            let stars = progress.stars.get(idx).copied().unwrap_or(0);
            root.spawn((
                ButtonBundle {
                    style: Style {
                        width: Val::Px(520.0),
                        flex_direction: FlexDirection::Row,
                        justify_content: JustifyContent::SpaceBetween,
                        align_items: AlignItems::Center,
                        padding: UiRect { left: Val::Px(24.0), right: Val::Px(20.0), top: Val::Px(14.0), bottom: Val::Px(14.0) },
                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    },
                    background_color: BackgroundColor(Color::rgba(0.10, 0.10, 0.22, 0.92)),
                    border_color: BorderColor(Color::rgba(0.40, 0.40, 0.65, 0.45)),
                    ..default()
                },
                BtnLessonStart(idx),
            ))
            .with_children(|row| {
                row.spawn(TextBundle::from_section(
                    format!("{}. {}", idx + 1, lesson.title),
                    TextStyle { font: font.clone(), font_size: 22.0, color: Color::rgb(0.88, 0.88, 0.95) },
                ));
                row.spawn(TextBundle::from_section(
                    star_string(stars),
                    TextStyle { font: font.clone(), font_size: 24.0, color: Color::rgb(1.0, 0.85, 0.20) },
                ));
            });
        }

        // Back button
        root.spawn(NodeBundle { style: Style { height: Val::Px(12.0), ..default() }, ..default() });
        root.spawn((
            ButtonBundle {
                style: Style {
                    width: Val::Px(520.0),
                    height: Val::Px(52.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                background_color: BackgroundColor(Color::rgba(0.12, 0.12, 0.22, 0.85)),
                border_color: BorderColor(Color::rgba(0.35, 0.35, 0.55, 0.40)),
                ..default()
            },
            BtnLessonsBack,
        ))
        .with_children(|p| {
            p.spawn(TextBundle::from_section(
                "← Menú Learning",
                TextStyle { font, font_size: 20.0, color: Color::rgb(0.75, 0.75, 0.88) },
            ));
        });
    });
}

// ─── Lifecycle systems (Lessons screen) ──────────────────────────────────────

fn setup_lessons(
    mut commands:    Commands,
    asset_server:    Res<AssetServer>,
    cached:          Res<CachedProgress>,
    mode:            Res<SelectedLessonMode>,
    fetch_state:     Res<ProgressFetchState>,
    session:         Res<UserSession>,
) {
    // Kick off backend fetch
    if let Ok(mut g) = fetch_state.0.try_lock() { *g = None; }
    #[cfg(target_arch = "wasm32")]
    if let Some(jwt) = session.jwt.clone() {
        let arc = fetch_state.0.clone();
        wasm_bindgen_futures::spawn_local(async move {
            if let Some(p) = fetch_progress_async(jwt).await {
                *arc.lock().unwrap() = Some(p);
            }
        });
    }

    let progress = cached.0.clone().unwrap_or_default();
    build_lessons_root(&mut commands, &asset_server, &progress, mode.0);
}

fn despawn_lessons(
    mut commands: Commands,
    q: Query<Entity, With<LessonsRoot>>,
) {
    for e in &q { commands.entity(e).despawn_recursive(); }
}

fn poll_progress_result(
    fetch_state:  Res<ProgressFetchState>,
    mut cached:   ResMut<CachedProgress>,
    mode:         Res<SelectedLessonMode>,
    root_q:       Query<Entity, With<LessonsRoot>>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    if let Ok(mut guard) = fetch_state.0.try_lock() {
        if let Some(p) = guard.take() {
            cached.0 = Some(p.clone());
            for e in &root_q { commands.entity(e).despawn_recursive(); }
            build_lessons_root(&mut commands, &asset_server, &p, mode.0);
        }
    }
}

// ─── Button handlers (Lessons screen) ────────────────────────────────────────

fn handle_lesson_start(
    q:                Query<(&Interaction, &BtnLessonStart), Changed<Interaction>>,
    mut lesson_setup: ResMut<LessonSetup>,
    mut game_config:  ResMut<GameConfig>,
    mode:             Res<SelectedLessonMode>,
    mut next_state:   ResMut<NextState<AppState>>,
) {
    for (i, btn) in &q {
        if *i != Interaction::Pressed { continue; }
        let idx = btn.0;
        let ex  = &LESSONS[idx].exercises[0];
        *lesson_setup = LessonSetup {
            lesson_idx:   idx,
            exercise_idx: 0,
            fen:          ex.fen.to_string(),
            answer_uci:   ex.answer_uci.to_string(),
            lesson_mode:  mode.0,
        };
        game_config.mode        = GameMode::Lesson;
        game_config.player_side = crate::pieces::PieceColor::White;
        next_state.set(AppState::Playing);
    }
}

fn handle_lessons_back(
    q: Query<&Interaction, (Changed<Interaction>, With<BtnLessonsBack>)>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    for i in &q {
        if *i == Interaction::Pressed {
            next_state.set(AppState::PvLHub);
        }
    }
}

fn handle_mode_interactive(
    q:        Query<&Interaction, (Changed<Interaction>, With<BtnModeInteractive>)>,
    mut mode: ResMut<SelectedLessonMode>,
    root_q:   Query<Entity, With<LessonsRoot>>,
    cached:   Res<CachedProgress>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    for i in &q {
        if *i != Interaction::Pressed { continue; }
        mode.0 = LessonMode::Interactive;
        let p = cached.0.clone().unwrap_or_default();
        for e in &root_q { commands.entity(e).despawn_recursive(); }
        build_lessons_root(&mut commands, &asset_server, &p, mode.0);
    }
}

fn handle_mode_guided(
    q:        Query<&Interaction, (Changed<Interaction>, With<BtnModeGuided>)>,
    mut mode: ResMut<SelectedLessonMode>,
    root_q:   Query<Entity, With<LessonsRoot>>,
    cached:   Res<CachedProgress>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    for i in &q {
        if *i != Interaction::Pressed { continue; }
        mode.0 = LessonMode::Guided;
        let p = cached.0.clone().unwrap_or_default();
        for e in &root_q { commands.entity(e).despawn_recursive(); }
        build_lessons_root(&mut commands, &asset_server, &p, mode.0);
    }
}

fn highlight_lesson_btns(
    mut q: Query<(&Interaction, &mut BackgroundColor), (Changed<Interaction>, With<BtnLessonStart>)>,
) {
    for (i, mut c) in &mut q {
        *c = match i {
            Interaction::Pressed => BackgroundColor(Color::rgba(0.24, 0.24, 0.50, 0.97)),
            Interaction::Hovered => BackgroundColor(Color::rgba(0.16, 0.16, 0.36, 0.95)),
            Interaction::None    => BackgroundColor(Color::rgba(0.10, 0.10, 0.22, 0.92)),
        };
    }
}

fn highlight_nav_btns(
    mut q: Query<(&Interaction, &mut BackgroundColor), (Changed<Interaction>, With<BtnLessonsBack>)>,
) {
    for (i, mut c) in &mut q {
        *c = match i {
            Interaction::Pressed => BackgroundColor(Color::rgba(0.22, 0.22, 0.42, 0.97)),
            Interaction::Hovered => BackgroundColor(Color::rgba(0.16, 0.16, 0.34, 0.95)),
            Interaction::None    => BackgroundColor(Color::rgba(0.12, 0.12, 0.22, 0.85)),
        };
    }
}

// ─── WASM progress fetch ──────────────────────────────────────────────────────

#[cfg(target_arch = "wasm32")]
async fn fetch_progress_async(jwt: String) -> Option<LoadedProgress> {
    #[derive(serde::Deserialize)]
    struct Row { lesson_idx: i32, stars: i16 }

    let resp = gloo_net::http::Request::get(
        "https://rustchess.greenmountain.dev/api/lesson_progress",
    )
    .header("Authorization", &format!("Bearer {}", jwt))
    .send()
    .await
    .ok()?;

    let rows: Vec<Row> = resp.json().await.ok()?;
    let mut p = LoadedProgress::default();
    for row in rows {
        if let Some(s) = p.stars.get_mut(row.lesson_idx as usize) {
            *s = row.stars as u8;
        }
    }
    Some(p)
}

#[cfg(target_arch = "wasm32")]
pub async fn post_progress_async(jwt: String, lesson_idx: usize, mode: LessonMode) {
    let mode_str = if mode == LessonMode::Interactive { "interactive" } else { "guided" };
    let body = serde_json::json!({ "lesson_idx": lesson_idx, "mode": mode_str });
    let _ = gloo_net::http::Request::post(
        "https://rustchess.greenmountain.dev/api/lesson_progress",
    )
    .header("Authorization", &format!("Bearer {}", jwt))
    .header("Content-Type", "application/json")
    .body(body.to_string())
    .unwrap()
    .send()
    .await;
}

// ─── In-game lesson overlay ───────────────────────────────────────────────────

fn setup_lesson_overlay(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    lesson_setup: Res<LessonSetup>,
    game_config:  Res<GameConfig>,
) {
    if game_config.mode != GameMode::Lesson { return; }

    let lesson = &LESSONS[lesson_setup.lesson_idx];
    let ex     = &lesson.exercises[lesson_setup.exercise_idx];
    let font: Handle<Font> = asset_server.load("fonts/FiraSans-Bold.ttf");

    let is_guided = lesson_setup.lesson_mode == LessonMode::Guided;
    let exercise_label = format!(
        "Lección {} · Ejercicio {}/3: {}",
        lesson_setup.lesson_idx + 1,
        lesson_setup.exercise_idx + 1,
        lesson.title
    );

    commands.spawn((
        NodeBundle {
            style: Style {
                position_type:   PositionType::Absolute,
                top:             Val::Px(12.0),
                left:            Val::Px(0.0),
                right:           Val::Px(0.0),
                flex_direction:  FlexDirection::Column,
                align_items:     AlignItems::Center,
                row_gap:         Val::Px(6.0),
                ..default()
            },
            z_index: ZIndex::Global(15),
            ..default()
        },
        LessonOverlayRoot,
    ))
    .with_children(|root| {
        // Header bar
        root.spawn(NodeBundle {
            style: Style {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(10.0),
                padding: UiRect { left: Val::Px(16.0), right: Val::Px(16.0), top: Val::Px(8.0), bottom: Val::Px(8.0) },
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            background_color: BackgroundColor(Color::rgba(0.04, 0.06, 0.18, 0.92)),
            border_color: BorderColor(Color::rgba(0.35, 0.35, 0.65, 0.55)),
            ..default()
        })
        .with_children(|bar| {
            bar.spawn(TextBundle::from_section(
                exercise_label.as_str(),
                TextStyle { font: font.clone(), font_size: 19.0, color: Color::rgb(0.90, 0.88, 0.75) },
            ));
            if is_guided {
                bar.spawn((
                    ButtonBundle {
                        style: Style {
                            padding: UiRect { left: Val::Px(14.0), right: Val::Px(14.0), top: Val::Px(6.0), bottom: Val::Px(6.0) },
                            ..default()
                        },
                        background_color: BackgroundColor(Color::rgba(0.12, 0.30, 0.60, 0.90)),
                        ..default()
                    },
                    BtnLessonHint,
                ))
                .with_children(|p| {
                    p.spawn(TextBundle::from_section("Pista", TextStyle { font: font.clone(), font_size: 16.0, color: Color::rgb(0.80, 0.88, 1.00) }));
                });
            }
            bar.spawn((
                ButtonBundle {
                    style: Style { padding: UiRect { left: Val::Px(14.0), right: Val::Px(14.0), top: Val::Px(6.0), bottom: Val::Px(6.0) }, ..default() },
                    background_color: BackgroundColor(Color::rgba(0.35, 0.20, 0.08, 0.90)),
                    ..default()
                },
                BtnLessonRetry,
            ))
            .with_children(|p| {
                p.spawn(TextBundle::from_section("Reintentar", TextStyle { font: font.clone(), font_size: 16.0, color: Color::rgb(1.00, 0.82, 0.65) }));
            });
            bar.spawn((
                ButtonBundle {
                    style: Style { padding: UiRect { left: Val::Px(14.0), right: Val::Px(14.0), top: Val::Px(6.0), bottom: Val::Px(6.0) }, ..default() },
                    background_color: BackgroundColor(Color::rgba(0.30, 0.10, 0.10, 0.90)),
                    ..default()
                },
                BtnLessonExit,
            ))
            .with_children(|p| {
                p.spawn(TextBundle::from_section("Salir", TextStyle { font: font.clone(), font_size: 16.0, color: Color::rgb(1.00, 0.72, 0.72) }));
            });
        });

        // Guided hint text panel
        if is_guided {
            root.spawn(NodeBundle {
                style: Style {
                    max_width: Val::Px(680.0),
                    padding: UiRect { left: Val::Px(18.0), right: Val::Px(18.0), top: Val::Px(10.0), bottom: Val::Px(10.0) },
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                background_color: BackgroundColor(Color::rgba(0.04, 0.12, 0.28, 0.90)),
                border_color: BorderColor(Color::rgba(0.28, 0.50, 0.90, 0.50)),
                ..default()
            })
            .with_children(|p| {
                p.spawn(TextBundle::from_section(
                    lesson.description,
                    TextStyle { font: font.clone(), font_size: 17.0, color: Color::rgb(0.80, 0.86, 1.00) },
                ));
            });
        }
    });
}

fn despawn_lesson_overlay(
    mut commands: Commands,
    q:            Query<Entity, With<LessonOverlayRoot>>,
    sq:           Query<Entity, With<LessonSuccessOverlay>>,
) {
    for e in &q  { commands.entity(e).despawn_recursive(); }
    for e in &sq { commands.entity(e).despawn_recursive(); }
}

fn validate_lesson_move(
    history:      Res<GameHistory>,
    lesson_setup: Res<LessonSetup>,
    game_config:  Res<GameConfig>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    overlay_q:    Query<Entity, With<LessonSuccessOverlay>>,
    session:      Res<UserSession>,
) {
    if game_config.mode != GameMode::Lesson { return; }
    if !history.is_changed() { return; }
    if history.moves.is_empty() { return; }

    // A move was made — it must be the answer (wrong moves are intercepted in move_piece)
    for e in &overlay_q { commands.entity(e).despawn_recursive(); }

    // Post progress if last exercise
    #[cfg(target_arch = "wasm32")]
    if lesson_setup.is_last_exercise() {
        if let Some(jwt) = session.jwt.clone() {
            let lesson_idx  = lesson_setup.lesson_idx;
            let lesson_mode = lesson_setup.lesson_mode;
            wasm_bindgen_futures::spawn_local(async move {
                post_progress_async(jwt, lesson_idx, lesson_mode).await;
            });
        }
    }
    let _ = session;

    spawn_success_overlay(&mut commands, &asset_server, &lesson_setup);
}

fn spawn_success_overlay(commands: &mut Commands, asset_server: &AssetServer, setup: &LessonSetup) {
    let font: Handle<Font> = asset_server.load("fonts/FiraSans-Bold.ttf");
    let is_last = setup.is_last_exercise();

    commands.spawn((
        NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                width:  Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                row_gap: Val::Px(24.0),
                ..default()
            },
            background_color: BackgroundColor(Color::rgba(0.0, 0.0, 0.0, 0.68)),
            z_index: ZIndex::Global(40),
            ..default()
        },
        LessonSuccessOverlay,
    ))
    .with_children(|root| {
        let title = if is_last { "¡Lección completada! ★★" } else { "¡Correcto!" };
        root.spawn(TextBundle::from_section(
            title,
            TextStyle { font: font.clone(), font_size: 52.0, color: Color::rgb(0.35, 1.00, 0.45) },
        ));

        if is_last {
            root.spawn((
                ButtonBundle {
                    style: Style {
                        padding: UiRect { left: Val::Px(36.0), right: Val::Px(36.0), top: Val::Px(14.0), bottom: Val::Px(14.0) },
                        justify_content: JustifyContent::Center, align_items: AlignItems::Center,
                        ..default()
                    },
                    background_color: BackgroundColor(Color::rgba(0.18, 0.18, 0.42, 0.95)),
                    ..default()
                },
                BtnLessonFinish,
            ))
            .with_children(|p| {
                p.spawn(TextBundle::from_section("Volver al Currículo", TextStyle { font, font_size: 26.0, color: Color::rgb(0.90, 0.90, 1.00) }));
            });
        } else {
            let label = format!("Siguiente ({}/3)", setup.exercise_idx + 2);
            root.spawn((
                ButtonBundle {
                    style: Style {
                        padding: UiRect { left: Val::Px(36.0), right: Val::Px(36.0), top: Val::Px(14.0), bottom: Val::Px(14.0) },
                        justify_content: JustifyContent::Center, align_items: AlignItems::Center,
                        ..default()
                    },
                    background_color: BackgroundColor(Color::rgba(0.12, 0.38, 0.12, 0.95)),
                    ..default()
                },
                BtnLessonNext,
            ))
            .with_children(|p| {
                p.spawn(TextBundle::from_section(label.as_str(), TextStyle { font, font_size: 26.0, color: Color::rgb(0.85, 1.00, 0.85) }));
            });
        }
    });
}

// ─── In-game button handlers ──────────────────────────────────────────────────

fn handle_lesson_hint(
    q:            Query<&Interaction, (Changed<Interaction>, With<BtnLessonHint>)>,
    lesson_setup: Res<LessonSetup>,
    mut hint:     ResMut<crate::state::Suggestion>,
) {
    for i in &q {
        if *i != Interaction::Pressed { continue; }
        let b = lesson_setup.answer_uci.as_bytes();
        if b.len() < 4 { continue; }
        hint.from_sq = Some((b[1] - b'1', b[0] - b'a'));
        hint.to_sq   = Some((b[3] - b'1', b[2] - b'a'));
        hint.text    = Some("Pista: mové la pieza resaltada a la casilla indicada.".to_string());
    }
}

fn handle_lesson_retry(
    q:         Query<&Interaction, (Changed<Interaction>, With<BtnLessonRetry>)>,
    mut next:  ResMut<NextState<AppState>>,
) {
    for i in &q {
        if *i == Interaction::Pressed { next.set(AppState::LessonRetry); }
    }
}

fn handle_lesson_exit(
    q:        Query<&Interaction, (Changed<Interaction>, With<BtnLessonExit>)>,
    mut next: ResMut<NextState<AppState>>,
) {
    for i in &q {
        if *i == Interaction::Pressed { next.set(AppState::Lessons); }
    }
}

fn handle_lesson_next(
    q:                Query<&Interaction, (Changed<Interaction>, With<BtnLessonNext>)>,
    mut lesson_setup: ResMut<LessonSetup>,
    mut next:         ResMut<NextState<AppState>>,
) {
    for i in &q {
        if *i != Interaction::Pressed { continue; }
        lesson_setup.exercise_idx += 1;
        let ex = &LESSONS[lesson_setup.lesson_idx].exercises[lesson_setup.exercise_idx];
        lesson_setup.fen         = ex.fen.to_string();
        lesson_setup.answer_uci  = ex.answer_uci.to_string();
        next.set(AppState::LessonRetry);
    }
}

fn handle_lesson_finish(
    q:        Query<&Interaction, (Changed<Interaction>, With<BtnLessonFinish>)>,
    mut next: ResMut<NextState<AppState>>,
) {
    for i in &q {
        if *i == Interaction::Pressed { next.set(AppState::Lessons); }
    }
}

fn highlight_overlay_btns(
    mut q: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, Or<(With<BtnLessonNext>, With<BtnLessonFinish>, With<BtnLessonRetry>, With<BtnLessonExit>, With<BtnLessonHint>)>),
    >,
) {
    for (i, mut c) in &mut q {
        if *i == Interaction::Hovered {
            c.0 = Color::rgba(c.0.r() + 0.05, c.0.g() + 0.05, c.0.b() + 0.05, c.0.a());
        }
    }
}

fn lesson_retry_enter(mut next: ResMut<NextState<AppState>>) {
    next.set(AppState::Playing);
}

// ─── Plugin ───────────────────────────────────────────────────────────────────

pub struct LessonsPlugin;

impl Plugin for LessonsPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<ProgressFetchState>()
            .init_resource::<CachedProgress>()
            .init_resource::<SelectedLessonMode>()
            // Lessons screen
            .add_systems(OnEnter(AppState::Lessons),  setup_lessons)
            .add_systems(OnExit(AppState::Lessons),   despawn_lessons)
            .add_systems(Update, (
                handle_lesson_start,
                handle_lessons_back,
                handle_mode_interactive,
                handle_mode_guided,
                highlight_lesson_btns,
                highlight_nav_btns,
                poll_progress_result,
            ).run_if(in_state(AppState::Lessons)))
            // Playing overlay (runs whenever mode == Lesson)
            .add_systems(OnEnter(AppState::Playing),  setup_lesson_overlay)
            .add_systems(OnExit(AppState::Playing),   despawn_lesson_overlay)
            .add_systems(Update, (
                validate_lesson_move,
                handle_lesson_hint,
                handle_lesson_retry,
                handle_lesson_exit,
                handle_lesson_next,
                handle_lesson_finish,
                highlight_overlay_btns,
            ).run_if(in_state(AppState::Playing)))
            // LessonRetry bounce: immediately re-enters Playing
            .add_systems(OnEnter(AppState::LessonRetry), lesson_retry_enter);
    }
}
