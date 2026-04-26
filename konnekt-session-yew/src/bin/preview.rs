use konnekt_session_yew::preview::preview_groups;
use yew::prelude::*;
use yew_preview::prelude::*;

#[function_component(App)]
fn app() -> Html {
    html! {
        <div style="font-family: Arial, sans-serif; height: 100vh; width: 100vw; display: flex; flex-direction: column; overflow: hidden; margin: 0; padding: 0;">
            <div style="padding: 10px 20px; background: #1a1a2e; flex-shrink: 0; display: flex; align-items: center; gap: 16px; border-bottom: 1px solid #2d2d44; box-sizing: border-box; z-index: 100;">
                <span style="color: #fff; font-weight: 700; font-size: 1.1rem; letter-spacing: 0.5px;">{"Konnekt Session"}</span>
                <span style="color: #8b949e; font-size: 0.85rem; border-left: 1px solid #3d3d5c; padding-left: 16px;">{"Component Preview"}</span>
            </div>
            <div style="flex: 1; min-height: 0; position: relative; width: 100%;">
                <PreviewPage groups={preview_groups()} />
            </div>
        </div>
    }
}

fn main() {
    tracing_wasm::set_as_global_default();
    yew::Renderer::<App>::new().render();
}
