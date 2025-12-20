use yew::prelude::*;
use uuid::Uuid;

#[derive(Properties, PartialEq)]
pub struct NeonRopeProps {
    pub id: Uuid,
    #[prop_or(false)]
    pub has_children: bool,
}

#[function_component(NeonRope)]
pub fn neon_rope(props: &NeonRopeProps) -> Html {
    let path_d = "M 110,20 C 90,20 80,70 40,60 S 10,20 40,10"; 
    
    // Unique IDs to ensure this component's definitions don't clash with others
    let gradient_id = format!("neon-gradient-{}", props.id);
    let glow_filter_id = format!("neon-glow-{}", props.id);

    html! {
        <svg 
            xmlns="http://www.w3.org/2000/svg"
            viewBox="0 0 120 80" 
            class="neon-rope-svg"
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
                
                // Local Glow Filter: Blur + Source Merge for better intensity
                <filter id={glow_filter_id.clone()} x="-50%" y="-50%" width="200%" height="200%">
                        <@{"feGaussianBlur"} in="SourceGraphic" stdDeviation="4" result="coloredBlur" />
                        <@{"feMerge"}>
                            <@{"feMergeNode"} in="coloredBlur" />
                            <@{"feMergeNode"} in="SourceGraphic" />
                        </@>
                </filter>
            </defs>

            // Layer 1: Ambient Glow (References Local Filter & Gradient)
            <path 
                d={path_d} 
                stroke={format!("url(#{})", gradient_id)} 
                stroke-width="20" 
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
        </svg>
    }
}
