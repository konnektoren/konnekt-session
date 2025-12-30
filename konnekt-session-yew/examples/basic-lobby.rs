use konnekt_session_yew::{LobbyView, SessionProvider};
use yew::prelude::*;

#[function_component(App)]
fn app() -> Html {
    html! {
        <div class="app">
            <SessionProvider signalling_server="wss://match.konnektoren.help">
                <LobbyView />
            </SessionProvider>
        </div>
    }
}

fn main() {
    // Initialize tracing
    tracing_wasm::set_as_global_default();

    yew::Renderer::<App>::new().render();
}
