use konnekt_session_yew::App;

fn main() {
    // Initialize tracing for WASM
    tracing_wasm::set_as_global_default();

    tracing::info!("Starting Konnekt Session Yew Example");

    yew::Renderer::<App>::new().render();
}
