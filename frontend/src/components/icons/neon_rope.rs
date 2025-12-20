use yew::prelude::*;
use uuid::Uuid;

#[derive(Properties, PartialEq)]
pub struct NeonRopeProps {
    pub id: Uuid,
    #[prop_or(false)]
    pub has_children: bool,
    #[prop_or(false)]
    pub is_user_message: bool,
}

#[function_component(NeonRope)]
pub fn neon_rope(props: &NeonRopeProps) -> Html {
    // Path that hooks UPWARD toward message at start, then flows outward
    // Starts higher (y=30), curves down organically, ends with a hook for forking
    let path_d = "M 100,30 C 85,35 70,55 50,50 C 30,45 15,40 5,25"; 
    
    // Unique IDs to ensure this component's definitions don't clash with others
    let gradient_id = format!("neon-gradient-{}", props.id);
    let glow_filter_id = format!("neon-glow-{}", props.id);

    
    // Apply horizontal flip for user messages (rope points right instead of left)
    // Use translate to move content back into viewBox after flip
    let transform = if props.is_user_message {
        "scale(-1, 1) translate(-120, 0)"
    } else {
        ""
    };

    html! {
        <svg 
            xmlns="http://www.w3.org/2000/svg"
            viewBox="0 0 120 80" 
            class="neon-rope-svg neon-rope-forkable"
            preserveAspectRatio="xMidYMid meet"
            style="overflow: visible;" 
        >
            <defs>
                 // Local Gradient: Teal -> Purple -> Coral
                 // Note: SVG elements are case-sensitive. Using @{"tagName"} syntax to preserve casing.
                <@{"linearGradient"} id={gradient_id.clone()} x1="0%" y1="0%" x2="100%" y2="0%">
                    <stop offset="0%" style="stop-color:#4ECDC4;stop-opacity:1" />
                    <stop offset="50%" style="stop-color:#A78BFA;stop-opacity:1" />
                    <stop offset="100%" style="stop-color:#FF6B6B;stop-opacity:1" />
                </@>
                
                // Fork gradients: Both start from Teal (the rope end color)
                <@{"linearGradient"} id={format!("fork-up-gradient-{}", props.id)} x1="100%" y1="100%" x2="0%" y2="0%">
                    <stop offset="0%" style="stop-color:#4ECDC4;stop-opacity:1" />
                    <stop offset="100%" style="stop-color:#06D6A0;stop-opacity:1" />
                </@>
                <@{"linearGradient"} id={format!("fork-down-gradient-{}", props.id)} x1="100%" y1="0%" x2="0%" y2="100%">
                    <stop offset="0%" style="stop-color:#4ECDC4;stop-opacity:1" />
                    <stop offset="100%" style="stop-color:#A78BFA;stop-opacity:1" />
                </@>
                
                // Local Glow Filter: Blur + Source Merge for better intensity
                <filter id={glow_filter_id.clone()} x="-50%" y="-50%" width="200%" height="200%">
                        <@{"feGaussianBlur"} in="SourceGraphic" stdDeviation="0.02" result="coloredBlur" />
                        <@{"feMerge"}>
                            <@{"feMergeNode"} in="coloredBlur" />
                            <@{"feMergeNode"} in="SourceGraphic" />
                        </@>
                </filter>
            </defs>

            <g transform={transform}>
                // Layer 1: Ambient Glow (References Local Filter & Gradient)
                <path 
                    d={path_d} 
                    stroke={format!("url(#{})", gradient_id)} 
                    stroke-width="12" 
                    fill="none" 
                    stroke-linecap="round"
                    filter={format!("url(#{})", glow_filter_id)}
                    opacity="0.5"
                    class="rope-glow"
                />

                // Layer 2: The Physical Tube
                <path 
                    d={path_d} 
                    stroke={format!("url(#{})", gradient_id)} 
                    stroke-width="10" 
                    fill="none" 
                    stroke-linecap="round"
                    class="rope-core"
                />

                // Layer 3: Specular Highlight
                <path 
                    d={path_d} 
                    stroke="white" 
                    stroke-width="4" 
                    fill="none" 
                    stroke-linecap="round"
                    opacity="0.4" 
                    class="rope-highlight"
                    transform="translate(-1, -1)"
                />

                // Fork paths: Two diverging branches from the rope end
                
                // UPPER FORK
                // Layer 1: Glow (Width 8)
                <path 
                    d="M 5,25 C 0,15 -5,5 -15,0"
                    stroke={format!("url(#fork-up-gradient-{})", props.id)}
                    stroke-width="8" 
                    fill="none" 
                    stroke-linecap="round"
                    filter={format!("url(#{})", glow_filter_id)}
                    opacity="0.5"
                    class="rope-fork rope-fork-up rope-glow"
                />
                // Layer 2: Core (Width 6)
                <path 
                    d="M 5,25 C 0,15 -5,5 -15,0"
                    stroke={format!("url(#fork-up-gradient-{})", props.id)}
                    stroke-width="6" 
                    fill="none" 
                    stroke-linecap="round"
                    class="rope-fork rope-fork-up rope-core"
                />
                // Layer 3: Highlight (Width 2, White)
                <path 
                    d="M 5,25 C 0,15 -5,5 -15,0"
                    stroke="white" 
                    stroke-width="2" 
                    fill="none" 
                    stroke-linecap="round"
                    opacity="0.4"
                    transform="translate(-0.5, -0.5)" 
                    class="rope-fork rope-fork-up rope-highlight"
                />


                // LOWER FORK
                // Layer 1: Glow (Width 8)
                <path 
                    d="M 5,25 C 0,35 -5,45 -15,50"
                    stroke={format!("url(#fork-down-gradient-{})", props.id)}
                    stroke-width="8" 
                    fill="none" 
                    stroke-linecap="round"
                    filter={format!("url(#{})", glow_filter_id)}
                     opacity="0.5"
                    class="rope-fork rope-fork-down rope-glow"
                />
                // Layer 2: Core (Width 6)
                <path 
                    d="M 5,25 C 0,35 -5,45 -15,50"
                    stroke={format!("url(#fork-down-gradient-{})", props.id)}
                    stroke-width="6" 
                    fill="none" 
                    stroke-linecap="round"
                    class="rope-fork rope-fork-down rope-core"
                />
                // Layer 3: Highlight (Width 2, White)
                <path 
                    d="M 5,25 C 0,35 -5,45 -15,50"
                    stroke="white" 
                    stroke-width="2" 
                    fill="none" 
                    stroke-linecap="round"
                    opacity="0.4"
                    transform="translate(-0.5, -0.5)"
                    class="rope-fork rope-fork-down rope-highlight"
                />

            </g>
        </svg>
    }
}
