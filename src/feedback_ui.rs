use bevy::prelude::*;
use crate::state::AppState;

#[derive(Component)] pub struct BtnFeedback;
#[derive(Component)] pub struct BtnSuggestLesson;

#[cfg(target_arch = "wasm32")]
mod js {
    use wasm_bindgen::prelude::*;
    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_namespace = window, js_name = show_feedback_modal)]
        pub fn show_feedback_modal(context: &str);
    }
}

fn handle_feedback_btn(
    q: Query<&Interaction, (Changed<Interaction>, With<BtnFeedback>)>,
) {
    for i in &q {
        if *i == Interaction::Pressed {
            #[cfg(target_arch = "wasm32")]
            js::show_feedback_modal("home");
        }
    }
}

fn handle_suggest_lesson(
    q: Query<&Interaction, (Changed<Interaction>, With<BtnSuggestLesson>)>,
) {
    for i in &q {
        if *i == Interaction::Pressed {
            #[cfg(target_arch = "wasm32")]
            js::show_feedback_modal("lessons");
        }
    }
}

pub struct FeedbackUiPlugin;

impl Plugin for FeedbackUiPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Update,
                handle_feedback_btn.run_if(in_state(AppState::Home)))
            .add_systems(Update,
                handle_suggest_lesson.run_if(in_state(AppState::Lessons)));
    }
}
