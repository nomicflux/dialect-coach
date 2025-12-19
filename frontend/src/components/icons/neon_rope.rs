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
    let gradient_id = format!("neon-gradient-{}", props.id);
    let glow_filter_id = format!("glow-blur-{}", props.id);
    
    // Path Definition:
    // Starts from center-right (near message), winds out to the left (negative x), and curves back.
    // SVG Viewbox: 0 0 120 80
    // Path Logic:
    // Start: 110, 20 (Near the message bubble attachment point)
    // Bend 1: Curve OUT to the left. Control points at 80,20 and 40,60 to 10,40
    // Bend 2: Curve BACK up/in. Control points at 0,30 and 30,0 to 50,10 (Tail)
    // This creates a complex "S" or "Loop" shape.
    
    // We strive for a "hanging wire" feel that defies the box.
    // The "d" attribute:
    // M 110,10  (Start near top-right/message)
    // C 80,10 50,70 20,50 (Big loop down and left)
    // S 10,10 40,10 (Curve back up)
    
    let path_d = "M 110,20 C 90,20 80,70 40,60 S 10,20 40,10"; 

    html! {
        <svg 
            viewBox="0 0 120 80" 
            class="neon-rope-svg"
            preserveAspectRatio="xMidYMid meet"
            style="overflow: visible;" // Ensure the glow isn't clipped
        >
            <defs>
                <linearGradient id={gradient_id.clone()} x1="0%" y1="0%" x2="100%" y2="0%">
                    <stop offset="0%" stop-color="var(--teal)" stop-opacity="1" />
                    <stop offset="50%" stop-color="var(--purple)" stop-opacity="1" />
                    <stop offset="100%" stop-color="var(--coral)" stop-opacity="1" />
                </linearGradient>
                
                <filter id={glow_filter_id.clone()} x="-50%" y="-50%" width="200%" height="200%">
                        <feGaussianBlur in="SourceGraphic" stdDeviation="4" />
                </filter>
            </defs>

            // Layer 1: Ambient Glow (The gas ionizing around the tube)
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

            // Layer 2: The Physical Tube (The glass containing the gas)
            <path 
                d={path_d} 
                stroke={format!("url(#{})", gradient_id)} 
                stroke-width="6" 
                fill="none" 
                stroke-linecap="round"
                class="rope-core"
            />

            // Layer 3: Specular Highlight (The reflection on the glass tube)
            // Offset slightly to simulate a light source from top-left
            <path 
                d={path_d} 
                stroke="white" 
                stroke-width="2" 
                fill="none" 
                stroke-linecap="round"
                opacity="0.4" 
                class="rope-highlight"
                transform="translate(-1, -1)"
            />
        </svg>
    }
}
