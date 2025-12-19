use yew::prelude::*;

#[function_component(AuroraBranchIcon)]
pub fn aurora_branch_icon() -> Html {
    html! {
        <svg 
            class="icon-aurora-stream" 
            viewBox="0 0 24 24" 
            fill="none" 
            stroke="currentColor" 
            stroke-width="2" 
            stroke-linecap="round" 
            stroke-linejoin="round"
            xmlns="http://www.w3.org/2000/svg"
        >
            // Main path fading straight ahead
            <path d="M 4 21 L 4 14" opacity="0.5" />
            
            // The branching decision point (Neon Highway curve)
            // Starts at bottom, curves smooth right
            <path d="M 4 14 C 4 8, 10 4, 20 4" class="aurora-path-active" />
            
            // Arrow head for the branch
            <path d="M 16 4 L 20 4 L 20 8" class="aurora-path-active" />
        </svg>
    }
}
