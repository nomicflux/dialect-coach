use yew::prelude::*;

#[function_component(AuroraBranchIcon)]
pub fn aurora_branch_icon() -> Html {
    html! {
        <svg 
            class="icon-aurora-stream" 
            viewBox="0 0 60 40" 
            width="60"
            height="40"
            fill="none" 
            xmlns="http://www.w3.org/2000/svg"
        >
            <defs>
                <linearGradient id="aurora-icon-gradient" x1="0%" y1="0%" x2="100%" y2="0%">
                    <stop offset="0%" style="stop-color:#4ECDC4;stop-opacity:1" />
                    <stop offset="50%" style="stop-color:#A78BFA;stop-opacity:1" />
                    <stop offset="100%" style="stop-color:#FF6B6B;stop-opacity:1" />
                </linearGradient>
            </defs>
            
            // faint guide line (optional, kept very subtle)
            <path d="M 55 35 L 55 25" stroke="#4ECDC4" stroke-width="2" opacity="0.2" stroke-dasharray="2 2" />
            
            // THE NEON ROPE
            // Smooth Quadratic Curve for physical flexibility look
            // Starts bottom-right (55, 35), Control Point (25, 35), Ends top-left (15, 5)
            <path 
                d="M 55 35 Q 25 35 15 5" 
                class="aurora-path-active" 
                stroke="url(#aurora-icon-gradient)"
                stroke-width="8"
                stroke-linecap="round"
                stroke-linejoin="round"
            />
        </svg>
    }
}
