use yew::prelude::*;

#[function_component(NeonAssets)]
pub fn neon_assets() -> Html {
    html! {
        <svg xmlns="http://www.w3.org/2000/svg" style="width: 0; height: 0; position: absolute; pointer-events: none; overflow: hidden;">
            <defs>
                // Global Gradient: Teal -> Purple -> Coral
                // Hardcoded IDs referenced by NeonRope. Using @{"tagName"} to preserve casing.
                <@{"linearGradient"} id="global-neon-gradient" x1="0%" y1="0%" x2="100%" y2="0%">
                    <stop offset="0%" style="stop-color:#4ECDC4;stop-opacity:1" />
                    <stop offset="50%" style="stop-color:#A78BFA;stop-opacity:1" />
                    <stop offset="100%" style="stop-color:#FF6B6B;stop-opacity:1" />
                </@>

                // Global Glow Filter
                <filter id="global-neon-glow" x="-50%" y="-50%" width="200%" height="200%">
                     <@{"feGaussianBlur"} in="SourceGraphic" stdDeviation="4" />
                </filter>
            </defs>
        </svg>
    }
}
