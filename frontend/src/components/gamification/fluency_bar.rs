use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct FluencyBarProps {
    pub xp: u64,
}

#[function_component(FluencyBar)]
pub fn fluency_bar(props: &FluencyBarProps) -> Html {
    let level = (props.xp / 1000) + 1;
    let progress = (props.xp % 1000) as f64 / 10.0;

    html! {
        <div class="gamification-fluency-bar">
            { render_level_badge(level) }
            { render_progress_track(progress) }
        </div>
    }
}

fn render_level_badge(level: u64) -> Html {
    html! { <div class="level-badge">{ format!("Lvl {}", level) }</div> }
}

fn render_progress_track(progress: f64) -> Html {
    html! {
        <div class="progress-track" title={format!("{}%", progress)}>
            <div class="progress-fill" style={format!("width: {}%", progress)}></div>
        </div>
    }
}
