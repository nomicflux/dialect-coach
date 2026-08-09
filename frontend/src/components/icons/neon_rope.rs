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
    // ViewBox: 0 0 108 44, matching the .rope-anchor box under the bubble.
    // The rope branches OUT into the margin, away from the transcript: it
    // descends from the bubble's inner side and runs outward to the fork hub,
    // where the two forks continue outward. The trunk starts at y=0, flush with
    // the bubble's bottom edge, so it grows out of the message it belongs to.
    let path_d = "M 96,0 L 96,14 Q 96,24 86,24 L 34,24";

    // Fork Hub Node Center: 34,24. The forks continue outward past the hub,
    // opening away from the conversation.
    let fork_up_d = "M 34,24 Q 20,24 14,10";
    let fork_down_d = "M 34,24 Q 20,24 14,38";

    // Unique IDs
    let gradient_id = format!("neon-circuit-grad-{}", props.id);

    // The trunk is drawn descending from the right edge and running left, which
    // is the agent (left-aligned) case. A user message sits against the right of
    // the transcript, so its rope mirrors to descend from the left edge and run
    // right, keeping the drop against the bubble and the forks pointing into the
    // open column rather than back under the message.
    let transform = if props.is_user_message {
        "scale(-1, 1) translate(-108, 0)" // Flip across X, shift back by viewBox width
    } else {
        ""
    };

    html! {
        <svg
            xmlns="http://www.w3.org/2000/svg"
            viewBox="0 0 108 44"
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

            </defs>

            <g transform={transform}>
                // --- MAIN TRUNK ---

                // Layer 1: Glow / Atmosphere
                <path
                    d={path_d}
                    stroke={format!("url(#{})", gradient_id)}
                    stroke-width="3"
                    fill="none"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    opacity="0.5"
                    class="rope-glow"
                />

                // Layer 2: Core Pipe
                <path
                    d={path_d}
                    stroke={format!("url(#{})", gradient_id)}
                    stroke-width="1.5"
                    fill="none"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    class="rope-core"
                />

                // Nodes: Start, Turn, Junction
                <circle cx="96" cy="0" r="2.5" fill="#00FFFF" class="rope-node" /> // Anchor at the bubble edge
                <circle cx="34" cy="24" r="3.5" fill="#FF0080" class="rope-node" /> // Main Hub

                // --- FORKS ---

                // UPPER FORK
                <path
                    d={fork_up_d}
                    stroke={format!("url(#fork-up-grad-{})", props.id)}
                    stroke-width="2.5"
                    fill="none"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    opacity="0" // Hidden by default
                    class="rope-fork rope-fork-up rope-glow"
                />
                <path
                    d={fork_up_d}
                    stroke={format!("url(#fork-up-grad-{})", props.id)}
                    stroke-width="1"
                    fill="none"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    opacity="0" // Hidden by default
                    class="rope-fork rope-fork-up rope-core"
                />
                 <circle cx="14" cy="10" r="2.5" fill="#00FFFF" class="rope-fork rope-fork-up rope-node" opacity="0" />


                // LOWER FORK
                <path
                    d={fork_down_d}
                    stroke={format!("url(#fork-down-grad-{})", props.id)}
                    stroke-width="2.5"
                    fill="none"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    opacity="0"
                    class="rope-fork rope-fork-down rope-glow"
                />
                 <path
                    d={fork_down_d}
                    stroke={format!("url(#fork-down-grad-{})", props.id)}
                    stroke-width="1"
                    fill="none"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    opacity="0"
                    class="rope-fork rope-fork-down rope-core"
                />
                <circle cx="14" cy="38" r="2.5" fill="#bf00ff" class="rope-fork rope-fork-down rope-node" opacity="0" />

            </g>
        </svg>
    }
}
