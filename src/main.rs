#[cfg(feature = "yew")]
use konnekt_session::example::App;

#[cfg(feature = "yew")]
fn main() {
    wasm_logger::init(wasm_logger::Config::default());
    yew::Renderer::<App>::new().render();
}
#[cfg(not(feature = "yew"))]
fn main() {
    println!("Please enable the 'yew' feature to run the example.");
}
