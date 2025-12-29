use std::env;

fn main() {
    let websocket_url =
        env::var("WEBSOCKET_URL").unwrap_or_else(|_| String::from("wss://echo.websocket.events"));

    println!("cargo:rerun-if-env-changed=WEBSOCKET_URL");
    println!("cargo:rustc-env=WEBSOCKET_URL={}", websocket_url);
}
