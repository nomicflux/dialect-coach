use dialect_coach_frontend::App;

fn main() {
    // The panic hook is installed unconditionally. Without it a panic in a
    // release build unwinds into an unreachable trap with no message: the Yew
    // render dies, #app keeps the index.html placeholder, and the console stays
    // empty - which is indistinguishable from the app never having started.
    console_error_panic_hook::set_once();

    #[cfg(debug_assertions)]
    wasm_logger::init(wasm_logger::Config::default());

    yew::Renderer::<App>::new().render();
}
