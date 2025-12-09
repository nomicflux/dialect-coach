use dialect_coach_shared::models::gamification::Quest;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct QuestListProps {
    pub quests: Vec<Quest>,
}

#[function_component(QuestList)]
pub fn quest_list(props: &QuestListProps) -> Html {
    html! {
        <div class="gamification-quest-list">
            <h3>{ "Daily Focus" }</h3>
            { for props.quests.iter().map(render_quest_item) }
        </div>
    }
}

fn render_quest_item(quest: &Quest) -> Html {
    let status_class = if quest.completed {
        "completed"
    } else {
        "pending"
    };
    html! {
        <div class={classes!("quest-item", status_class)}>
            <div class="quest-checkbox">{ if quest.completed { "☑" } else { "☐" } }</div>
            <div class="quest-desc">{ &quest.description }</div>
        </div>
    }
}
