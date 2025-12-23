use uuid::Uuid;
use yew::prelude::*;

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
    // ANGULAR PIPE DESIGN
    // ViewBox: 0 0 160 100
    // Start (under message): Right side, approx (150, 20)
    // Flow: Down -> Diagonal -> Left

    // Main Trunk Path:
    // M 150,20 (Start under bubble)
    // L 150,40 (Vertical drop)
    // L 140,50 (Chamfer turn)
    // L 50,50  (Long horizontal run to hub)
    let path_d = "M 150,20 L 150,40 L 140,50 L 50,50";

    // Fork Hub Node Center: 50,50

    // Upper Fork: 50,50 -> 40,40 -> 10,10
    let fork_up_d = "M 50,50 L 40,40 L 10,10";

    // Lower Fork: 50,50 -> 40,60 -> 10,90
    let fork_down_d = "M 50,50 L 40,60 L 10,90";

    // Unique IDs
    let gradient_id = format!("neon-circuit-grad-{}", props.id);
    let glow_filter_id = format!("neon-circuit-glow-{}", props.id);

    // Flip for user messages (User is on right sides)
    let transform = if props.is_user_message {
        "scale(-1, 1) translate(-160, 0)" // Flip across X, shift back by viewBox width
    } else {
        ""
    };

    html! {
        <svg
            xmlns="http://www.w3.org/2000/svg"
            viewBox="0 0 160 100"
            class="neon-rope-svg neon-rope-forkable"
            preserveAspectRatio="xMidYMid meet"
            style="overflow: visible;"
        >
            <defs>
                 // Local Gradient: Cyan -> Purple -> Magenta (Cyberpunk)
                <@{"linearGradient"} id={gradient_id.clone()} x1="100%" y1="0%" x2="0%" y2="100%">
                    <stop offset="0%" style="stop-color:#00FFFF;stop-opacity:1" /> // Cyan (Start/Message)
                    <stop offset="50%" style="stop-color:#bf00ff;stop-opacity:1" /> // Purple
                    <stop offset="100%" style="stop-color:#FF0080;stop-opacity:1" /> // Magenta (End/Fork)
                </@>

                // Fork Gradients
                <@{"linearGradient"} id={format!("fork-up-grad-{}", props.id)} x1="100%" y1="100%" x2="0%" y2="0%">
                    <stop offset="0%" style="stop-color:#FF0080;stop-opacity:1" /> // Match Trunk End
                    <stop offset="100%" style="stop-color:#00FFFF;stop-opacity:1" /> // Fade to Cyan
                </@>
                <@{"linearGradient"} id={format!("fork-down-grad-{}", props.id)} x1="0%" y1="0%" x2="0%" y2="100%">
                    <stop offset="0%" style="stop-color:#FF0080;stop-opacity:1" />
                    <stop offset="100%" style="stop-color:#bf00ff;stop-opacity:1" /> // Fade to Purple
                </@>

                // Tighter Glow Filter for "Pipe" look
                <filter id={glow_filter_id.clone()} x="-50%" y="-50%" width="200%" height="200%">
                        // Less blur for focused "plasma" look
                        <@{"feGaussianBlur"} in="SourceGraphic" stdDeviation="1.5" result="coloredBlur" />
                        <@{"feColorMatrix"} in="coloredBlur" type="matrix" values="
                            1 0 0 0 0
                            0 1 0 0 0
                            0 0 1 0 0
                            0 0 0 2.5 0" result="boostedGlow" />
                        <@{"feMerge"}>
                            <@{"feMergeNode"} in="boostedGlow" />
                            <@{"feMergeNode"} in="SourceGraphic" />
                        </@>
                </filter>
            </defs>

            <g transform={transform}>
                // --- MAIN TRUNK ---

                // Layer 1: Glow / Atmosphere
                <path
                    d={path_d}
                    stroke={format!("url(#{})", gradient_id)}
                    stroke-width="6"
                    fill="none"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    filter={format!("url(#{})", glow_filter_id)}
                    opacity="0.8"
                    class="rope-glow"
                />

                // Layer 2: Core Pipe
                <path
                    d={path_d}
                    stroke={format!("url(#{})", gradient_id)}
                    stroke-width="3"
                    fill="none"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    class="rope-core"
                />

                // Nodes: Start, Turn, Junction
                <circle cx="150" cy="20" r="3" fill="#00FFFF" class="rope-node" />
                <circle cx="150" cy="40" r="2" fill="#bf00ff" class="rope-node" /> // Small joint
                <circle cx="50" cy="50" r="4" fill="#FF0080" class="rope-node" /> // Main Hub

                // --- FORKS ---

                // UPPER FORK
                <path
                    d={fork_up_d}
                    stroke={format!("url(#fork-up-grad-{})", props.id)}
                    stroke-width="5"
                    fill="none"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    filter={format!("url(#{})", glow_filter_id)}
                    opacity="0" // Hidden by default
                    class="rope-fork rope-fork-up rope-glow"
                />
                <path
                    d={fork_up_d}
                    stroke={format!("url(#fork-up-grad-{})", props.id)}
                    stroke-width="2"
                    fill="none"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    opacity="0" // Hidden by default
                    class="rope-fork rope-fork-up rope-core"
                />
                 <circle cx="10" cy="10" r="3" fill="#00FFFF" class="rope-fork rope-fork-up rope-node" opacity="0" />


                // LOWER FORK
                <path
                    d={fork_down_d}
                    stroke={format!("url(#fork-down-grad-{})", props.id)}
                    stroke-width="5"
                    fill="none"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                     filter={format!("url(#{})", glow_filter_id)}
                    opacity="0"
                    class="rope-fork rope-fork-down rope-glow"
                />
                 <path
                    d={fork_down_d}
                    stroke={format!("url(#fork-down-grad-{})", props.id)}
                    stroke-width="2"
                    fill="none"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    opacity="0"
                    class="rope-fork rope-fork-down rope-core"
                />
                <circle cx="10" cy="90" r="3" fill="#bf00ff" class="rope-fork rope-fork-down rope-node" opacity="0" />

            </g>
        </svg>
    }
}
