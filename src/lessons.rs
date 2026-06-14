use bevy::prelude::*;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::ButtonState;
use bevy::reflect::TypePath;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use crate::{
    auth::UserSession,
    board::GameHistory,
    pieces::{Piece, PieceColor, PieceType},
    state::{AppState, GameConfig, GameMode, LessonMode, LessonSetup},
};

// ─── JSON Lesson Types ────────────────────────────────────────────────────────

#[derive(serde::Deserialize, Clone)]
pub struct LessonJson {
    pub title:       String,
    pub description: String,
    pub theme:       String,
    pub fen:         String,
    pub answer_uci:  String,
}

#[derive(Asset, TypePath, serde::Deserialize, Clone)]
pub struct LessonsAsset {
    pub lessons: Vec<LessonJson>,
}

#[derive(Default)]
pub struct LessonsJsonLoader;

impl bevy::asset::AssetLoader for LessonsJsonLoader {
    type Asset    = LessonsAsset;
    type Settings = ();
    type Error    = std::io::Error;

    fn load<'a>(
        &'a self,
        reader:       &'a mut bevy::asset::io::Reader,
        _settings:    &'a (),
        _load_context: &'a mut bevy::asset::LoadContext,
    ) -> bevy::utils::BoxedFuture<'a, Result<Self::Asset, Self::Error>> {
        Box::pin(async move {
            use bevy::asset::AsyncReadExt;
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes).await?;
            serde_json::from_slice(&bytes)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
        })
    }

    fn extensions(&self) -> &[&str] { &["json"] }
}

#[derive(Resource, Default)]
pub struct LessonsHandle(pub Option<Handle<LessonsAsset>>);

#[derive(Resource, Default)]
pub struct LoadedLessons(pub Vec<LessonJson>);

// ─── Progress loading (WASM) ──────────────────────────────────────────────────

#[derive(Clone, Default)]
pub struct LoadedProgress {
    pub stars: HashMap<usize, u8>,
}

impl LoadedProgress {
    pub fn get(&self, idx: usize) -> u8 { self.stars.get(&idx).copied().unwrap_or(0) }
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

#[derive(Resource, Default)]
pub struct SearchQuery(pub String);

#[derive(Resource, Default, PartialEq, Clone, Copy)]
pub enum LessonsTab { #[default] List, Table }

// ─── Components (Lessons screen) ─────────────────────────────────────────────

#[derive(Component)] struct LessonsRoot;
#[derive(Component)] struct BtnLessonStart(usize);
#[derive(Component)] struct OriginalBg(Color);
#[derive(Component)] struct BtnLessonsBack;
#[derive(Component)] struct BtnModeInteractive;
#[derive(Component)] struct BtnModeGuided;
#[derive(Component)] struct BtnTabList;
#[derive(Component)] struct BtnTabTable;
#[derive(Component)] struct LessonScrollInner;
#[derive(Component)] struct LessonConfirmPanel;
#[derive(Component)] struct BtnConfirmStart(usize);
#[derive(Component)] struct BtnConfirmCancel;

#[derive(Resource, Default)]
struct SelectedLesson(Option<(usize, String)>);

#[derive(Resource, Default)]
pub struct LessonScrollOffset(pub f32);

// ─── Components (Playing overlay) ────────────────────────────────────────────

#[derive(Component)] pub struct LessonOverlayRoot;
#[derive(Component)] pub struct BtnLessonHint;
#[derive(Component)] pub struct BtnLessonRetry;
#[derive(Component)] pub struct BtnLessonExit;
#[derive(Component)] pub struct LessonSuccessOverlay;
#[derive(Component)] pub struct BtnLessonFinish;

// ─── UI Helpers ──────────────────────────────────────────────────────────────

fn star_string(stars: u8) -> &'static str {
    match stars { 0 => "○○", 1 => "●○", _ => "●●" }
}

fn lesson_row_colors(stars: u8) -> (Color, Color, Color) {
    match stars {
        0 => (Color::rgba(0.10, 0.10, 0.22, 0.92), Color::rgba(0.35, 0.35, 0.60, 0.45), Color::rgb(0.82, 0.82, 0.95)),
        1 => (Color::rgba(0.06, 0.20, 0.10, 0.92), Color::rgba(0.25, 0.58, 0.30, 0.60), Color::rgb(0.75, 0.95, 0.80)),
        _ => (Color::rgba(0.08, 0.28, 0.12, 0.92), Color::rgba(0.28, 0.72, 0.34, 0.80), Color::rgb(0.82, 1.00, 0.86)),
    }
}

fn icon_btn(
    parent: &mut ChildBuilder,
    font:   Handle<Font>,
    label:  &str,
    marker: impl Bundle,
    bg:     Color,
    border: Color,
) {
    parent.spawn((
        ButtonBundle {
            style: Style {
                padding: UiRect { left: Val::Px(18.0), right: Val::Px(18.0), top: Val::Px(8.0), bottom: Val::Px(8.0) },
                border: UiRect::all(Val::Px(2.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            background_color: BackgroundColor(bg),
            border_color: BorderColor(border),
            ..default()
        },
        marker,
    ))
    .with_children(|p| {
        p.spawn(TextBundle::from_section(label, TextStyle { font, font_size: 18.0, color: Color::rgb(0.92, 0.92, 0.96) }));
    });
}

fn spawn_table_row(
    parent: &mut ChildBuilder, font: &Handle<Font>,
    num: &str, title: &str, status: &str, header: bool,
) {
    let bg        = if header { Color::rgba(0.16, 0.16, 0.35, 0.95) } else { Color::rgba(0.08, 0.08, 0.20, 0.85) };
    let font_size = if header { 16.0f32 } else { 15.0 };
    let color     = if header { Color::rgb(0.75, 0.75, 1.00) } else { Color::rgb(0.82, 0.82, 0.95) };

    parent.spawn(NodeBundle {
        style: Style {
            width: Val::Px(580.0),
            flex_direction: FlexDirection::Row,
            padding: UiRect { left: Val::Px(10.0), right: Val::Px(10.0), top: Val::Px(6.0), bottom: Val::Px(6.0) },
            border: UiRect::all(Val::Px(1.0)),
            ..default()
        },
        background_color: BackgroundColor(bg),
        border_color:     BorderColor(Color::rgba(0.30, 0.30, 0.55, 0.35)),
        ..default()
    })
    .with_children(|r| {
        let mk = || TextStyle { font: font.clone(), font_size, color };
        r.spawn(NodeBundle { style: Style { width: Val::Px(36.0), ..default() }, ..default() })
         .with_children(|p| { p.spawn(TextBundle::from_section(num, mk())); });
        r.spawn(NodeBundle { style: Style { flex_grow: 1.0, ..default() }, ..default() })
         .with_children(|p| { p.spawn(TextBundle::from_section(title, mk())); });
        r.spawn(NodeBundle { style: Style { width: Val::Px(120.0), ..default() }, ..default() })
         .with_children(|p| { p.spawn(TextBundle::from_section(status, mk())); });
    });
}

// ─── Lessons Screen Builder ───────────────────────────────────────────────────

fn build_lessons_root(
    commands:     &mut Commands,
    asset_server: &AssetServer,
    progress:     &LoadedProgress,
    mode:         LessonMode,
    search:       &str,
    tab:          LessonsTab,
    lessons:      &[LessonJson],
) {
    let font: Handle<Font> = asset_server.load("fonts/DejaVuSans-Bold.ttf");

    let completed   = progress.stars.values().filter(|&&s| s > 0).count();
    let double_star = progress.stars.values().filter(|&&s| s >= 2).count();
    let single_star = progress.stars.values().filter(|&&s| s == 1).count();
    let total = lessons.len();

    const PAGE_SIZE: usize = 200;

    let sq = search.to_lowercase();
    let all_filtered: Vec<(usize, &LessonJson)> = lessons.iter().enumerate()
        .filter(|(_, l)| sq.is_empty()
            || l.title.to_lowercase().contains(&sq)
            || l.description.to_lowercase().contains(&sq)
            || l.theme.to_lowercase().contains(&sq))
        .collect();
    let overflow = sq.is_empty() && all_filtered.len() > PAGE_SIZE;
    let filtered: &[(usize, &LessonJson)] = if overflow { &all_filtered[..PAGE_SIZE] } else { &all_filtered };

    let mode_i_bg = if mode == LessonMode::Interactive { Color::rgba(0.18, 0.50, 0.18, 0.95) } else { Color::rgba(0.08, 0.18, 0.08, 0.80) };
    let mode_g_bg = if mode == LessonMode::Guided      { Color::rgba(0.18, 0.32, 0.60, 0.95) } else { Color::rgba(0.06, 0.10, 0.24, 0.80) };
    let tab_l_bg  = if tab == LessonsTab::List  { Color::rgba(0.22, 0.22, 0.50, 0.92) } else { Color::rgba(0.10, 0.10, 0.26, 0.80) };
    let tab_t_bg  = if tab == LessonsTab::Table { Color::rgba(0.22, 0.22, 0.50, 0.92) } else { Color::rgba(0.10, 0.10, 0.26, 0.80) };

    commands.spawn((
        NodeBundle {
            style: Style {
                width:          Val::Percent(100.0),
                height:         Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items:    AlignItems::Center,
                padding:        UiRect { top: Val::Px(20.0), bottom: Val::Px(12.0), ..default() },
                row_gap:        Val::Px(8.0),
                ..default()
            },
            background_color: BackgroundColor(Color::rgba(0.04, 0.04, 0.10, 0.97)),
            z_index: ZIndex::Global(5),
            ..default()
        },
        LessonsRoot,
    ))
    .with_children(|root| {
        // ── Title ──
        root.spawn(TextBundle::from_section(
            "CURRÍCULO DE LECCIONES",
            TextStyle { font: font.clone(), font_size: 38.0, color: Color::rgb(0.95, 0.92, 0.80) },
        ));

        if lessons.is_empty() {
            root.spawn(TextBundle::from_section(
                "Cargando lecciones...",
                TextStyle { font: font.clone(), font_size: 22.0, color: Color::rgba(0.55, 0.55, 0.72, 1.0) },
            ));
            // Back button still shown
            root.spawn(NodeBundle { style: Style { flex_grow: 1.0, ..default() }, ..default() });
            root.spawn((
                ButtonBundle {
                    style: Style {
                        width: Val::Px(580.0), height: Val::Px(46.0),
                        justify_content: JustifyContent::Center, align_items: AlignItems::Center,
                        border: UiRect::all(Val::Px(1.0)), ..default()
                    },
                    background_color: BackgroundColor(Color::rgba(0.10, 0.10, 0.22, 0.85)),
                    border_color:     BorderColor(Color::rgba(0.32, 0.32, 0.52, 0.40)),
                    ..default()
                },
                BtnLessonsBack,
            ))
            .with_children(|p| {
                p.spawn(TextBundle::from_section("← Menú Learning",
                    TextStyle { font, font_size: 18.0, color: Color::rgb(0.72, 0.72, 0.88) }));
            });
            return;
        }

        // ── Stats bar ──
        root.spawn(TextBundle::from_section(
            format!("Completadas: {}/{} · ●● {} · ●○ {}", completed, total, double_star, single_star),
            TextStyle { font: font.clone(), font_size: 16.0, color: Color::rgb(0.55, 0.82, 0.60) },
        ));

        // ── Controls row ──
        root.spawn(NodeBundle {
            style: Style { flex_direction: FlexDirection::Row, column_gap: Val::Px(18.0), align_items: AlignItems::Center, ..default() },
            ..default()
        })
        .with_children(|row| {
            row.spawn(NodeBundle {
                style: Style { flex_direction: FlexDirection::Row, column_gap: Val::Px(6.0), ..default() },
                ..default()
            })
            .with_children(|g| {
                icon_btn(g, font.clone(), "◉ Interactivo", BtnModeInteractive, mode_i_bg, Color::rgba(0.28, 0.68, 0.28, 0.65));
                icon_btn(g, font.clone(), "◎ Guiado",      BtnModeGuided,      mode_g_bg, Color::rgba(0.28, 0.48, 0.88, 0.65));
            });
            row.spawn(NodeBundle {
                style: Style { flex_direction: FlexDirection::Row, column_gap: Val::Px(6.0), ..default() },
                ..default()
            })
            .with_children(|g| {
                icon_btn(g, font.clone(), "≡ Lista",  BtnTabList,  tab_l_bg, Color::rgba(0.45, 0.45, 0.75, 0.55));
                icon_btn(g, font.clone(), "⊞ Tabla", BtnTabTable, tab_t_bg, Color::rgba(0.45, 0.45, 0.75, 0.55));
            });
        });

        // ── Search box ──
        root.spawn(NodeBundle {
            style: Style {
                width: Val::Px(580.0),
                padding: UiRect { left: Val::Px(14.0), right: Val::Px(14.0), top: Val::Px(8.0), bottom: Val::Px(8.0) },
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            background_color: BackgroundColor(Color::rgba(0.07, 0.07, 0.18, 0.92)),
            border_color:     BorderColor(Color::rgba(0.45, 0.45, 0.72, 0.65)),
            ..default()
        })
        .with_children(|p| {
            let (text, color) = if search.is_empty() {
                ("▸ Buscar lección...".to_string(), Color::rgba(0.50, 0.50, 0.68, 1.0))
            } else {
                (format!("▸ {}_", search), Color::rgb(0.92, 0.92, 1.00))
            };
            p.spawn(TextBundle::from_section(text, TextStyle { font: font.clone(), font_size: 17.0, color }));
        });

        // ── Content area (clips overflow, inner node scrolls via top offset) ──
        // min_height:0 + flex_basis:0 prevent children from inflating the flex item's
        // base size, so flex_grow:1.0 fills exactly the remaining space without
        // pushing the back button off screen.
        root.spawn(NodeBundle {
            style: Style {
                width:      Val::Px(600.0),
                flex_direction: FlexDirection::Column,
                flex_grow:  1.0,
                flex_shrink: 1.0,
                min_height: Val::Px(0.0),
                overflow:   Overflow::clip_y(),
                ..default()
            },
            ..default()
        })
        .with_children(|content| {
            content.spawn((
                NodeBundle {
                    style: Style {
                        flex_direction: FlexDirection::Column,
                        row_gap:        Val::Px(5.0),
                        width:          Val::Percent(100.0),
                        position_type:  PositionType::Relative,
                        top:            Val::Px(0.0),
                        ..default()
                    },
                    ..default()
                },
                LessonScrollInner,
            ))
            .with_children(|inner| {
                if tab == LessonsTab::List {
                    for (idx, lesson) in filtered.iter() {
                        let stars = progress.get(*idx);
                        let (bg, border, text_color) = lesson_row_colors(stars);
                        inner.spawn((
                            ButtonBundle {
                                style: Style {
                                    width: Val::Px(580.0), flex_direction: FlexDirection::Row,
                                    justify_content: JustifyContent::SpaceBetween,
                                    align_items: AlignItems::Center,
                                    padding: UiRect { left: Val::Px(18.0), right: Val::Px(14.0), top: Val::Px(11.0), bottom: Val::Px(11.0) },
                                    border: UiRect::all(Val::Px(1.0)), ..default()
                                },
                                background_color: BackgroundColor(bg),
                                border_color:     BorderColor(border),
                                ..default()
                            },
                            BtnLessonStart(*idx),
                            OriginalBg(bg),
                        ))
                        .with_children(|row| {
                            row.spawn(TextBundle::from_section(
                                format!("{}. {}", idx + 1, lesson.title),
                                TextStyle { font: font.clone(), font_size: 19.0, color: text_color },
                            ));
                            row.spawn(TextBundle::from_section(
                                star_string(stars),
                                TextStyle { font: font.clone(), font_size: 20.0,
                                    color: if stars > 0 { Color::rgb(0.38, 0.95, 0.48) } else { Color::rgba(0.45, 0.45, 0.62, 0.80) }
                                },
                            ));
                        });
                    }
                    if filtered.is_empty() {
                        inner.spawn(TextBundle::from_section(
                            "No se encontraron lecciones.",
                            TextStyle { font: font.clone(), font_size: 17.0, color: Color::rgba(0.55, 0.55, 0.70, 1.0) },
                        ));
                    } else if overflow {
                        inner.spawn(TextBundle::from_section(
                            format!("Mostrando {} de {} — buscá para filtrar", PAGE_SIZE, all_filtered.len()),
                            TextStyle { font: font.clone(), font_size: 15.0, color: Color::rgba(0.55, 0.55, 0.75, 0.85) },
                        ));
                    }
                } else {
                    spawn_table_row(inner, &font, "#", "Lección", "Estado", true);
                    for (idx, lesson) in filtered.iter() {
                        let stars = progress.get(*idx);
                        let status = match stars { 0 => "Pendiente", 1 => "●○  Guiado", _ => "●●  Hecha" };
                        spawn_table_row(inner, &font, &format!("{}", idx + 1), &lesson.title, status, false);
                    }
                    if filtered.is_empty() {
                        inner.spawn(TextBundle::from_section(
                            "No se encontraron lecciones.",
                            TextStyle { font: font.clone(), font_size: 17.0, color: Color::rgba(0.55, 0.55, 0.70, 1.0) },
                        ));
                    }
                }
            });
        });

        // ── Back button ──
        root.spawn((
            ButtonBundle {
                style: Style {
                    width: Val::Px(580.0), height: Val::Px(46.0),
                    justify_content: JustifyContent::Center, align_items: AlignItems::Center,
                    border: UiRect::all(Val::Px(1.0)), ..default()
                },
                background_color: BackgroundColor(Color::rgba(0.10, 0.10, 0.22, 0.85)),
                border_color:     BorderColor(Color::rgba(0.32, 0.32, 0.52, 0.40)),
                ..default()
            },
            BtnLessonsBack,
        ))
        .with_children(|p| {
            p.spawn(TextBundle::from_section("← Menú Learning",
                TextStyle { font: font.clone(), font_size: 18.0, color: Color::rgb(0.72, 0.72, 0.88) }));
        });

        // ── Suggest a lesson ──
        root.spawn((
            ButtonBundle {
                style: Style {
                    padding: UiRect { left: Val::Px(8.0), right: Val::Px(8.0), top: Val::Px(4.0), bottom: Val::Px(4.0) },
                    ..default()
                },
                background_color: BackgroundColor(Color::NONE),
                ..default()
            },
            crate::feedback_ui::BtnSuggestLesson,
        ))
        .with_children(|p| {
            p.spawn(TextBundle::from_section(
                "¿Falta una lección? Avisanos →",
                TextStyle { font, font_size: 12.0, color: Color::rgba(0.40, 0.40, 0.58, 0.72) },
            ));
        });
    });
}

// ─── Asset init / poll ────────────────────────────────────────────────────────

fn init_lessons_asset(
    asset_server: Res<AssetServer>,
    mut handle:   ResMut<LessonsHandle>,
) {
    if handle.0.is_none() {
        handle.0 = Some(asset_server.load("lessons.json"));
    }
}

fn poll_lessons_asset(
    handle:       Res<LessonsHandle>,
    assets:       Res<Assets<LessonsAsset>>,
    mut loaded:   ResMut<LoadedLessons>,
    root_q:       Query<Entity, With<LessonsRoot>>,
    confirm_q:    Query<Entity, With<LessonConfirmPanel>>,
    cached:       Res<CachedProgress>,
    mode:         Res<SelectedLessonMode>,
    sq:           Res<SearchQuery>,
    tab:          Res<LessonsTab>,
    mut selected: ResMut<SelectedLesson>,
    mut scroll:   ResMut<LessonScrollOffset>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    if !loaded.0.is_empty() { return; }
    let Some(h) = &handle.0 else { return };
    let Some(asset) = assets.get(h) else { return };
    loaded.0 = asset.lessons.clone();
    if !root_q.is_empty() {
        let p = cached.0.clone().unwrap_or_default();
        clear_selection_and_rebuild(&mut commands, &asset_server, &root_q, &confirm_q, &mut selected, &mut scroll, &p, mode.0, &sq.0, *tab, &loaded.0);
    }
}

// ─── Setup / Despawn ─────────────────────────────────────────────────────────

fn setup_lessons(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    cached:       Res<CachedProgress>,
    mode:         Res<SelectedLessonMode>,
    fetch_state:  Res<ProgressFetchState>,
    session:      Res<UserSession>,
    mut sq:       ResMut<SearchQuery>,
    tab:          Res<LessonsTab>,
    loaded:       Res<LoadedLessons>,
    mut scroll:   ResMut<LessonScrollOffset>,
) {
    sq.0.clear();
    scroll.0 = 0.0;
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
    let _ = session;
    let progress = cached.0.clone().unwrap_or_default();
    build_lessons_root(&mut commands, &asset_server, &progress, mode.0, &sq.0, *tab, &loaded.0);
}

fn despawn_lessons(
    mut commands: Commands,
    q:            Query<Entity, With<LessonsRoot>>,
    confirm_q:    Query<Entity, With<LessonConfirmPanel>>,
    mut selected: ResMut<SelectedLesson>,
    mut scroll:   ResMut<LessonScrollOffset>,
) {
    for e in &q         { commands.entity(e).despawn_recursive(); }
    for e in &confirm_q { commands.entity(e).despawn_recursive(); }
    *selected = SelectedLesson(None);
    scroll.0   = 0.0;
}

fn poll_progress_result(
    fetch_state:  Res<ProgressFetchState>,
    mut cached:   ResMut<CachedProgress>,
    mode:         Res<SelectedLessonMode>,
    sq:           Res<SearchQuery>,
    tab:          Res<LessonsTab>,
    loaded:       Res<LoadedLessons>,
    root_q:       Query<Entity, With<LessonsRoot>>,
    confirm_q:    Query<Entity, With<LessonConfirmPanel>>,
    mut selected: ResMut<SelectedLesson>,
    mut scroll:   ResMut<LessonScrollOffset>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    if let Ok(mut guard) = fetch_state.0.try_lock() {
        if let Some(p) = guard.take() {
            cached.0 = Some(p.clone());
            clear_selection_and_rebuild(&mut commands, &asset_server, &root_q, &confirm_q, &mut selected, &mut scroll, &p, mode.0, &sq.0, *tab, &loaded.0);
        }
    }
}

// ─── Button handlers (Lessons screen) ────────────────────────────────────────

fn handle_lesson_select(
    q:            Query<(&Interaction, &BtnLessonStart), Changed<Interaction>>,
    loaded:       Res<LoadedLessons>,
    confirm_q:    Query<Entity, With<LessonConfirmPanel>>,
    mut selected: ResMut<SelectedLesson>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    for (i, btn) in &q {
        if *i != Interaction::Pressed { continue; }
        let idx = btn.0;
        let Some(lesson) = loaded.0.get(idx) else { continue };
        for e in &confirm_q { commands.entity(e).despawn_recursive(); }
        *selected = SelectedLesson(Some((idx, lesson.title.clone())));
        spawn_confirm_panel(&mut commands, &asset_server, idx, &lesson.title);
    }
}

fn handle_confirm_start(
    q:                Query<(&Interaction, &BtnConfirmStart), Changed<Interaction>>,
    mut lesson_setup: ResMut<LessonSetup>,
    mut game_config:  ResMut<GameConfig>,
    mode:             Res<SelectedLessonMode>,
    mut next_state:   ResMut<NextState<AppState>>,
    loaded:           Res<LoadedLessons>,
) {
    for (i, btn) in &q {
        if *i != Interaction::Pressed { continue; }
        let idx = btn.0;
        let Some(lesson) = loaded.0.get(idx) else { continue };
        *lesson_setup = LessonSetup {
            lesson_idx:  idx,
            fen:         lesson.fen.clone(),
            answer_uci:  lesson.answer_uci.clone(),
            lesson_mode: mode.0,
            title:       lesson.title.clone(),
            description: lesson.description.clone(),
        };
        game_config.mode        = GameMode::Lesson;
        game_config.player_side = match lesson.fen.split_whitespace().nth(1) {
            Some("b") => PieceColor::Black,
            _ => PieceColor::White,
        };
        next_state.set(AppState::Playing);
    }
}

fn handle_confirm_cancel(
    q:            Query<&Interaction, (Changed<Interaction>, With<BtnConfirmCancel>)>,
    confirm_q:    Query<Entity, With<LessonConfirmPanel>>,
    mut selected: ResMut<SelectedLesson>,
    mut commands: Commands,
) {
    for i in &q {
        if *i == Interaction::Pressed {
            for e in &confirm_q { commands.entity(e).despawn_recursive(); }
            *selected = SelectedLesson(None);
        }
    }
}

fn handle_lessons_back(
    q: Query<&Interaction, (Changed<Interaction>, With<BtnLessonsBack>)>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    for i in &q {
        if *i == Interaction::Pressed { next_state.set(AppState::PvLHub); }
    }
}

fn clear_selection_and_rebuild(
    commands:     &mut Commands,
    asset_server: &AssetServer,
    root_q:       &Query<Entity, With<LessonsRoot>>,
    confirm_q:    &Query<Entity, With<LessonConfirmPanel>>,
    selected:     &mut SelectedLesson,
    scroll:       &mut LessonScrollOffset,
    progress:     &LoadedProgress,
    mode:         LessonMode,
    sq:           &str,
    tab:          LessonsTab,
    lessons:      &[LessonJson],
) {
    for e in root_q    { commands.entity(e).despawn_recursive(); }
    for e in confirm_q { commands.entity(e).despawn_recursive(); }
    *selected = SelectedLesson(None);
    scroll.0   = 0.0;
    build_lessons_root(commands, asset_server, progress, mode, sq, tab, lessons);
}

fn handle_mode_interactive(
    q:            Query<&Interaction, (Changed<Interaction>, With<BtnModeInteractive>)>,
    mut mode:     ResMut<SelectedLessonMode>,
    sq:           Res<SearchQuery>,
    tab:          Res<LessonsTab>,
    loaded:       Res<LoadedLessons>,
    root_q:       Query<Entity, With<LessonsRoot>>,
    confirm_q:    Query<Entity, With<LessonConfirmPanel>>,
    cached:       Res<CachedProgress>,
    mut selected: ResMut<SelectedLesson>,
    mut scroll:   ResMut<LessonScrollOffset>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    for i in &q {
        if *i != Interaction::Pressed { continue; }
        mode.0 = LessonMode::Interactive;
        let p = cached.0.clone().unwrap_or_default();
        clear_selection_and_rebuild(&mut commands, &asset_server, &root_q, &confirm_q, &mut selected, &mut scroll, &p, mode.0, &sq.0, *tab, &loaded.0);
    }
}

fn handle_mode_guided(
    q:            Query<&Interaction, (Changed<Interaction>, With<BtnModeGuided>)>,
    mut mode:     ResMut<SelectedLessonMode>,
    sq:           Res<SearchQuery>,
    tab:          Res<LessonsTab>,
    loaded:       Res<LoadedLessons>,
    root_q:       Query<Entity, With<LessonsRoot>>,
    confirm_q:    Query<Entity, With<LessonConfirmPanel>>,
    cached:       Res<CachedProgress>,
    mut selected: ResMut<SelectedLesson>,
    mut scroll:   ResMut<LessonScrollOffset>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    for i in &q {
        if *i != Interaction::Pressed { continue; }
        mode.0 = LessonMode::Guided;
        let p = cached.0.clone().unwrap_or_default();
        clear_selection_and_rebuild(&mut commands, &asset_server, &root_q, &confirm_q, &mut selected, &mut scroll, &p, mode.0, &sq.0, *tab, &loaded.0);
    }
}

fn handle_tab_list(
    q:            Query<&Interaction, (Changed<Interaction>, With<BtnTabList>)>,
    mut tab:      ResMut<LessonsTab>,
    sq:           Res<SearchQuery>,
    mode:         Res<SelectedLessonMode>,
    loaded:       Res<LoadedLessons>,
    root_q:       Query<Entity, With<LessonsRoot>>,
    confirm_q:    Query<Entity, With<LessonConfirmPanel>>,
    cached:       Res<CachedProgress>,
    mut selected: ResMut<SelectedLesson>,
    mut scroll:   ResMut<LessonScrollOffset>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    for i in &q {
        if *i != Interaction::Pressed { continue; }
        *tab = LessonsTab::List;
        let p = cached.0.clone().unwrap_or_default();
        clear_selection_and_rebuild(&mut commands, &asset_server, &root_q, &confirm_q, &mut selected, &mut scroll, &p, mode.0, &sq.0, *tab, &loaded.0);
    }
}

fn handle_tab_table(
    q:            Query<&Interaction, (Changed<Interaction>, With<BtnTabTable>)>,
    mut tab:      ResMut<LessonsTab>,
    sq:           Res<SearchQuery>,
    mode:         Res<SelectedLessonMode>,
    loaded:       Res<LoadedLessons>,
    root_q:       Query<Entity, With<LessonsRoot>>,
    confirm_q:    Query<Entity, With<LessonConfirmPanel>>,
    cached:       Res<CachedProgress>,
    mut selected: ResMut<SelectedLesson>,
    mut scroll:   ResMut<LessonScrollOffset>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    for i in &q {
        if *i != Interaction::Pressed { continue; }
        *tab = LessonsTab::Table;
        let p = cached.0.clone().unwrap_or_default();
        clear_selection_and_rebuild(&mut commands, &asset_server, &root_q, &confirm_q, &mut selected, &mut scroll, &p, mode.0, &sq.0, *tab, &loaded.0);
    }
}

fn handle_search_input(
    mut key_ev:   EventReader<KeyboardInput>,
    mut sq:       ResMut<SearchQuery>,
    tab:          Res<LessonsTab>,
    mode:         Res<SelectedLessonMode>,
    loaded:       Res<LoadedLessons>,
    root_q:       Query<Entity, With<LessonsRoot>>,
    confirm_q:    Query<Entity, With<LessonConfirmPanel>>,
    cached:       Res<CachedProgress>,
    mut selected: ResMut<SelectedLesson>,
    mut scroll:   ResMut<LessonScrollOffset>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    let mut changed = false;
    for ev in key_ev.read() {
        if ev.state != ButtonState::Pressed { continue; }
        match &ev.logical_key {
            Key::Character(s) => {
                for ch in s.chars() {
                    if !ch.is_control() { sq.0.push(ch); changed = true; }
                }
            }
            Key::Backspace => { sq.0.pop(); changed = true; }
            _ => {}
        }
    }
    if changed {
        let p = cached.0.clone().unwrap_or_default();
        clear_selection_and_rebuild(&mut commands, &asset_server, &root_q, &confirm_q, &mut selected, &mut scroll, &p, mode.0, &sq.0, *tab, &loaded.0);
    }
}

fn highlight_lesson_btns(
    mut q: Query<(&Interaction, &mut BackgroundColor, &OriginalBg), (Changed<Interaction>, With<BtnLessonStart>)>,
) {
    for (i, mut c, orig) in &mut q {
        c.0 = match i {
            Interaction::Hovered => {
                let s = Srgba::from(orig.0);
                Color::srgba((s.red + 0.08).min(1.0), (s.green + 0.08).min(1.0), (s.blue + 0.08).min(1.0), s.alpha)
            }
            _ => orig.0,
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
    .send().await.ok()?;

    let rows: Vec<Row> = resp.json().await.ok()?;
    let mut p = LoadedProgress::default();
    for row in rows {
        p.stars.insert(row.lesson_idx as usize, row.stars as u8);
    }
    Some(p)
}

#[cfg(target_arch = "wasm32")]
pub async fn post_progress_async(jwt: String, lesson_idx: usize, mode: LessonMode) {
    let mode_str = if mode == LessonMode::Interactive { "interactive" } else { "guided" };
    let body = format!(r#"{{"lesson_idx": {}, "mode": "{}"}}"#, lesson_idx, mode_str);
    let _ = gloo_net::http::Request::post(
        "https://rustchess.greenmountain.dev/api/lesson_progress",
    )
    .header("Authorization", &format!("Bearer {}", jwt))
    .header("Content-Type", "application/json")
    .body(body).unwrap()
    .send().await;
}

// ─── In-game lesson overlay ───────────────────────────────────────────────────

fn setup_lesson_overlay(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    lesson_setup: Res<LessonSetup>,
    game_config:  Res<GameConfig>,
) {
    if game_config.mode != GameMode::Lesson { return; }

    let font: Handle<Font> = asset_server.load("fonts/DejaVuSans-Bold.ttf");
    let is_guided = lesson_setup.lesson_mode == LessonMode::Guided;

    commands.spawn((
        NodeBundle {
            style: Style {
                position_type:  PositionType::Absolute,
                top:            Val::Px(12.0),
                left:           Val::Px(0.0),
                right:          Val::Px(0.0),
                flex_direction: FlexDirection::Column,
                align_items:    AlignItems::Center,
                row_gap:        Val::Px(6.0),
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
            border_color:     BorderColor(Color::rgba(0.35, 0.35, 0.65, 0.55)),
            ..default()
        })
        .with_children(|bar| {
            let label = format!("#{} — {}", lesson_setup.lesson_idx + 1, lesson_setup.title);
            bar.spawn(TextBundle::from_section(
                label.as_str(),
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
                    OriginalBg(Color::rgba(0.12, 0.30, 0.60, 0.90)),
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
                OriginalBg(Color::rgba(0.35, 0.20, 0.08, 0.90)),
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
                OriginalBg(Color::rgba(0.30, 0.10, 0.10, 0.90)),
            ))
            .with_children(|p| {
                p.spawn(TextBundle::from_section("Salir", TextStyle { font: font.clone(), font_size: 16.0, color: Color::rgb(1.00, 0.72, 0.72) }));
            });
        });

        // Description panel (guided mode)
        if is_guided && !lesson_setup.description.is_empty() {
            root.spawn(NodeBundle {
                style: Style {
                    max_width: Val::Px(680.0),
                    padding: UiRect { left: Val::Px(18.0), right: Val::Px(18.0), top: Val::Px(10.0), bottom: Val::Px(10.0) },
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                background_color: BackgroundColor(Color::rgba(0.04, 0.12, 0.28, 0.90)),
                border_color:     BorderColor(Color::rgba(0.28, 0.50, 0.90, 0.50)),
                ..default()
            })
            .with_children(|p| {
                p.spawn(TextBundle::from_section(
                    lesson_setup.description.as_str(),
                    TextStyle { font, font_size: 17.0, color: Color::rgb(0.80, 0.86, 1.00) },
                ));
            });
        }
    });
}

fn despawn_lesson_overlay(
    mut commands: Commands,
    q:  Query<Entity, With<LessonOverlayRoot>>,
    sq: Query<Entity, With<LessonSuccessOverlay>>,
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

    for e in &overlay_q { commands.entity(e).despawn_recursive(); }

    // Single exercise per lesson — always post progress
    #[cfg(target_arch = "wasm32")]
    if let Some(jwt) = session.jwt.clone() {
        let lesson_idx  = lesson_setup.lesson_idx;
        let lesson_mode = lesson_setup.lesson_mode;
        wasm_bindgen_futures::spawn_local(async move {
            post_progress_async(jwt, lesson_idx, lesson_mode).await;
        });
    }
    let _ = session;

    spawn_success_overlay(&mut commands, &asset_server);
}

fn spawn_success_overlay(commands: &mut Commands, asset_server: &AssetServer) {
    let font: Handle<Font> = asset_server.load("fonts/DejaVuSans-Bold.ttf");
    commands.spawn((
        NodeBundle {
            style: Style {
                position_type:   PositionType::Absolute,
                width:           Val::Percent(100.0),
                height:          Val::Percent(100.0),
                flex_direction:  FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items:     AlignItems::Center,
                row_gap:         Val::Px(24.0),
                ..default()
            },
            background_color: BackgroundColor(Color::rgba(0.0, 0.0, 0.0, 0.68)),
            z_index: ZIndex::Global(40),
            ..default()
        },
        LessonSuccessOverlay,
    ))
    .with_children(|root| {
        root.spawn(TextBundle::from_section(
            "¡Lección completada! ●●",
            TextStyle { font: font.clone(), font_size: 52.0, color: Color::rgb(0.35, 1.00, 0.45) },
        ));
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
            OriginalBg(Color::rgba(0.18, 0.18, 0.42, 0.95)),
        ))
        .with_children(|p| {
            p.spawn(TextBundle::from_section("Volver al Currículo",
                TextStyle { font, font_size: 26.0, color: Color::rgb(0.90, 0.90, 1.00) }));
        });
    });
}

// ─── In-game button handlers ──────────────────────────────────────────────────

fn handle_lesson_hint(
    q:            Query<&Interaction, (Changed<Interaction>, With<BtnLessonHint>)>,
    lesson_setup: Res<LessonSetup>,
    pieces_q:     Query<&Piece>,
    mut hint:     ResMut<crate::state::Suggestion>,
) {
    for i in &q {
        if *i != Interaction::Pressed { continue; }
        let b = lesson_setup.answer_uci.as_bytes();
        if b.len() < 4 { continue; }

        let from_file = b[0] - b'a';
        let from_rank = b[1] - b'1';
        let to_file   = b[2] - b'a';
        let to_rank   = b[3] - b'1';

        hint.from_sq = Some((from_rank, from_file));
        hint.to_sq   = Some((to_rank, to_file));

        let piece_name = pieces_q.iter()
            .find(|p| p.x == from_rank && p.y == from_file)
            .map(|p| match p.piece_type {
                PieceType::King   => "Rey",
                PieceType::Queen  => "Dama",
                PieceType::Bishop => "Alfil",
                PieceType::Knight => "Caballo",
                PieceType::Rook   => "Torre",
                PieceType::Pawn   => "Peón",
            })
            .unwrap_or("pieza");

        let from_label = format!("{}{}", b[0] as char, b[1] as char);
        let to_label   = format!("{}{}", b[2] as char, b[3] as char);

        hint.text = Some(format!(
            "Mové la {} de {} a {}  —  {}",
            piece_name, from_label, to_label, lesson_setup.description
        ));
    }
}

fn handle_lesson_retry(
    q:        Query<&Interaction, (Changed<Interaction>, With<BtnLessonRetry>)>,
    mut next: ResMut<NextState<AppState>>,
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
        (&Interaction, &mut BackgroundColor, &OriginalBg),
        (Changed<Interaction>, Or<(With<BtnLessonFinish>, With<BtnLessonRetry>, With<BtnLessonExit>, With<BtnLessonHint>)>),
    >,
) {
    for (i, mut c, orig) in &mut q {
        c.0 = match i {
            Interaction::Hovered => {
                let s = Srgba::from(orig.0);
                Color::srgba((s.red + 0.08).min(1.0), (s.green + 0.08).min(1.0), (s.blue + 0.08).min(1.0), s.alpha)
            }
            _ => orig.0,
        };
    }
}

fn lesson_retry_enter(mut next: ResMut<NextState<AppState>>) {
    next.set(AppState::Playing);
}

// ─── Lesson scroll ────────────────────────────────────────────────────────────

fn handle_lesson_scroll(
    mut wheel:  EventReader<bevy::input::mouse::MouseWheel>,
    mut scroll: ResMut<LessonScrollOffset>,
) {
    for ev in wheel.read() {
        use bevy::input::mouse::MouseScrollUnit;
        let speed = match ev.unit { MouseScrollUnit::Line => 45.0, MouseScrollUnit::Pixel => 1.0 };
        scroll.0 = (scroll.0 - ev.y * speed).max(0.0);
    }
    #[cfg(target_arch = "wasm32")]
    {
        let js = crate::LESSON_SCROLL_DELTA.swap(0, std::sync::atomic::Ordering::Relaxed) as f32;
        if js.abs() > 0.5 {
            scroll.0 = (scroll.0 + js).max(0.0);
        }
    }
}

fn apply_lesson_scroll(
    scroll: Res<LessonScrollOffset>,
    mut q:  Query<&mut Style, With<LessonScrollInner>>,
) {
    if !scroll.is_changed() { return; }
    for mut style in &mut q {
        style.top = Val::Px(-scroll.0);
    }
}

// ─── Confirm panel ────────────────────────────────────────────────────────────

fn spawn_confirm_panel(commands: &mut Commands, asset_server: &AssetServer, idx: usize, title: &str) {
    let font: Handle<Font> = asset_server.load("fonts/DejaVuSans-Bold.ttf");
    commands.spawn((
        NodeBundle {
            style: Style {
                position_type:   PositionType::Absolute,
                bottom:          Val::Px(0.0),
                left:            Val::Px(0.0),
                right:           Val::Px(0.0),
                flex_direction:  FlexDirection::Column,
                align_items:     AlignItems::Center,
                padding:         UiRect { top: Val::Px(16.0), bottom: Val::Px(18.0), left: Val::Px(12.0), right: Val::Px(12.0) },
                row_gap:         Val::Px(10.0),
                border:          UiRect { top: Val::Px(1.0), ..default() },
                ..default()
            },
            background_color: BackgroundColor(Color::rgba(0.04, 0.06, 0.16, 0.97)),
            border_color:     BorderColor(Color::rgba(0.40, 0.65, 0.40, 0.50)),
            z_index: ZIndex::Global(20),
            ..default()
        },
        LessonConfirmPanel,
    ))
    .with_children(|p| {
        p.spawn(TextBundle::from_section(
            format!("#{} — {}", idx + 1, title),
            TextStyle { font: font.clone(), font_size: 20.0, color: Color::rgb(0.92, 0.92, 1.00) },
        ));
        p.spawn(TextBundle::from_section(
            "¿Querés arrancar esta lección?",
            TextStyle { font: font.clone(), font_size: 15.0, color: Color::rgba(0.68, 0.68, 0.85, 0.90) },
        ));
        p.spawn(NodeBundle {
            style: Style { flex_direction: FlexDirection::Row, column_gap: Val::Px(16.0), ..default() },
            ..default()
        })
        .with_children(|row| {
            row.spawn((
                ButtonBundle {
                    style: Style {
                        padding: UiRect { left: Val::Px(28.0), right: Val::Px(28.0), top: Val::Px(11.0), bottom: Val::Px(11.0) },
                        border:  UiRect::all(Val::Px(1.0)), ..default()
                    },
                    background_color: BackgroundColor(Color::rgba(0.20, 0.06, 0.06, 0.92)),
                    border_color:     BorderColor(Color::rgba(0.60, 0.25, 0.25, 0.55)),
                    ..default()
                },
                BtnConfirmCancel,
            ))
            .with_children(|p| {
                p.spawn(TextBundle::from_section("Cancelar",
                    TextStyle { font: font.clone(), font_size: 18.0, color: Color::rgb(0.90, 0.68, 0.68) }));
            });
            row.spawn((
                ButtonBundle {
                    style: Style {
                        padding: UiRect { left: Val::Px(28.0), right: Val::Px(28.0), top: Val::Px(11.0), bottom: Val::Px(11.0) },
                        border:  UiRect::all(Val::Px(1.0)), ..default()
                    },
                    background_color: BackgroundColor(Color::rgba(0.06, 0.26, 0.06, 0.92)),
                    border_color:     BorderColor(Color::rgba(0.25, 0.68, 0.25, 0.60)),
                    ..default()
                },
                BtnConfirmStart(idx),
            ))
            .with_children(|p| {
                p.spawn(TextBundle::from_section("▶ Arrancar",
                    TextStyle { font: font.clone(), font_size: 18.0, color: Color::rgb(0.68, 0.95, 0.68) }));
            });
        });
    });
}

// ─── Plugin ───────────────────────────────────────────────────────────────────

pub struct LessonsPlugin;

impl Plugin for LessonsPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_asset::<LessonsAsset>()
            .init_asset_loader::<LessonsJsonLoader>()
            .init_resource::<LessonsHandle>()
            .init_resource::<LoadedLessons>()
            .init_resource::<ProgressFetchState>()
            .init_resource::<CachedProgress>()
            .init_resource::<SelectedLessonMode>()
            .init_resource::<SearchQuery>()
            .init_resource::<LessonsTab>()
            .init_resource::<SelectedLesson>()
            .init_resource::<LessonScrollOffset>()
            // Load the JSON file at startup
            .add_systems(Startup, init_lessons_asset)
            // Poll asset loading every frame (cheap — no-op once loaded)
            .add_systems(Update, poll_lessons_asset)
            // Lessons screen
            .add_systems(OnEnter(AppState::Lessons), setup_lessons)
            .add_systems(OnExit(AppState::Lessons),  despawn_lessons)
            .add_systems(Update, (
                handle_lesson_select,
                handle_confirm_start,
                handle_confirm_cancel,
                handle_lessons_back,
                handle_mode_interactive,
                handle_mode_guided,
                handle_tab_list,
                handle_tab_table,
                handle_search_input,
                handle_lesson_scroll,
                apply_lesson_scroll,
                highlight_lesson_btns,
                highlight_nav_btns,
                poll_progress_result,
            ).run_if(in_state(AppState::Lessons)))
            // Playing overlay
            .add_systems(OnEnter(AppState::Playing), setup_lesson_overlay)
            .add_systems(OnExit(AppState::Playing),  despawn_lesson_overlay)
            .add_systems(Update, (
                validate_lesson_move,
                handle_lesson_hint,
                handle_lesson_retry,
                handle_lesson_exit,
                handle_lesson_finish,
                highlight_overlay_btns,
            ).run_if(in_state(AppState::Playing)))
            // LessonRetry bounce
            .add_systems(OnEnter(AppState::LessonRetry), lesson_retry_enter);
    }
}
