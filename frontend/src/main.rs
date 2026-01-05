use dialect_coach_frontend::App;

fn main() {
    #[cfg(debug_assertions)]
    {
        wasm_logger::init(wasm_logger::Config::default());
        console_error_panic_hook::set_once();
    }
    yew::Renderer::<App>::new().render();
}
