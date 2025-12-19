use crate::components::icons::AuroraBranchIcon;

#[derive(Properties, PartialEq)]
pub struct BranchSwitcherPillProps {
    pub branch_name: AttrValue,
    pub on_click: Callback<()>,
}

#[function_component(BranchSwitcherPill)]
pub fn branch_switcher_pill(props: &BranchSwitcherPillProps) -> Html {
    let onclick = props.on_click.clone();

    html! {
        <button
            class="branch-switcher-pill"
            onclick={move |_| onclick.emit(())}
            title="Switch Branch"
        >
            <span class="branch-icon"><AuroraBranchIcon /></span>
            <span class="branch-name">{&props.branch_name}</span>
            <span class="chevron">{"›"}</span>
        </button>
    }
}
