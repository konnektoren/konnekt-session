#[cfg(feature = "yew")]
mod app;

#[cfg(feature = "yew")]
fn main() {
    yew::Renderer::<app::App>::new().render();
}
#[cfg(not(feature = "yew"))]
fn main() {
    println!("Please enable the 'yew' feature to run the example.");
}
