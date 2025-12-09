use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct StreakDisplayProps {
    pub current_streak: u32,
}

#[function_component(StreakDisplay)]
pub fn streak_display(props: &StreakDisplayProps) -> Html {
    let active_class = if props.current_streak > 0 { "active" } else { "inactive" };
    
    html! {
        <div class={classes!("gamification-streak", active_class)}>
            <span class="streak-icon">{"🔥"}</span>
            <span class="streak-count">{ props.current_streak }</span>
            <span class="streak-label">{ "Day Streak" }</span>
        </div>
    }
}
