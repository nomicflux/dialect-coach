use yew::prelude::*;

#[function_component(LoadingScreen)]
pub fn loading_screen() -> Html {
    html! {
        <div class="loading-screen" style="
            display: flex; 
            justify_content: center; 
            align_items: center; 
            height: 100%; 
            width: 100%; 
            position: absolute; 
            top: 0; 
            left: 0; 
            background: rgba(0,0,0,0.5); 
            backdrop-filter: blur(5px); 
            z-index: 9999;
            color: white;
            flex-direction: column;
        ">
            <div class="spinner" style="
                border: 4px solid rgba(255, 255, 255, 0.3);
                border-radius: 50%;
                border-top: 4px solid var(--teal, #00d2ff);
                width: 40px;
                height: 40px;
                animation: spin 1s linear infinite;
                margin-bottom: 20px;
            "></div>
            <h2>{"Signing in..."}</h2>
            <style>
                {"
                @keyframes spin {
                    0% { transform: rotate(0deg); }
                    100% { transform: rotate(360deg); }
                }
                "}
            </style>
        </div>
    }
}
