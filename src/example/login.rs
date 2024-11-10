use crate::example::PlayerProfile;
use crate::model::{Player, Role};
use uuid::Uuid;
use yew::prelude::*;

type LobbyId = String;
type Password = Option<String>;

pub type LoginCallback = (Player<PlayerProfile>, Role, LobbyId, Password);

#[derive(Properties, PartialEq)]
pub struct LoginCompProps {
    pub on_login: Callback<LoginCallback>,
}

#[function_component(LoginComp)]
pub fn login_comp(props: &LoginCompProps) -> Html {
    let username = use_state(|| "".to_string());
    let password = use_state(|| Password::default());
    let role = use_state(|| Role::Participant);
    let lobby_id = use_state(|| "".to_string());

    let on_username_change = {
        let username = username.clone();
        Callback::from(move |e: InputEvent| {
            let input = e.target_unchecked_into::<web_sys::HtmlInputElement>();
            username.set(input.value());
        })
    };

    let on_password_change = {
        let password = password.clone();
        Callback::from(move |e: InputEvent| {
            let input = e.target_unchecked_into::<web_sys::HtmlInputElement>();
            if input.value().is_empty() {
                password.set(None);
                return;
            }
            password.set(Some(input.value()));
        })
    };

    let on_role_change = {
        let role = role.clone();
        Callback::from(move |e: Event| {
            let select = e.target_unchecked_into::<web_sys::HtmlSelectElement>();
            let selected_role = match select.value().as_str() {
                "Admin" => Role::Admin,
                "Participant" => Role::Participant,
                "Observer" => Role::Observer,
                _ => Role::Participant,
            };
            role.set(selected_role);
        })
    };

    let on_lobby_id_change = {
        let lobby_id = lobby_id.clone();
        Callback::from(move |e: InputEvent| {
            let input = e.target_unchecked_into::<web_sys::HtmlInputElement>();
            lobby_id.set(input.value());
        })
    };

    let on_submit = {
        let username = username.clone();
        let password = password.clone();
        let role = role.clone();
        let lobby_id = lobby_id.clone();
        let on_login = props.on_login.clone();

        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();

            let profile = PlayerProfile {
                id: Uuid::new_v4().to_string(),
                name: (*username).clone(),
            };

            let player = Player::new(*role, profile);

            let lobby_id = if lobby_id.is_empty() {
                Uuid::new_v4().to_string()
            } else {
                (*lobby_id).clone()
            };

            on_login.emit((player, *role, lobby_id, (*password).clone()));
        })
    };

    html! {
        <div class="konnekt-session-login">
            <h1 class="konnekt-session-login__title">{"Login"}</h1>
            <form class="konnekt-session-login__form" onsubmit={on_submit}>
                <label class="konnekt-session-login__label" for="username">{"Username"}</label>
                <input class="konnekt-session-login__input" type="text" id="username" name="username" value={(*username).clone()} oninput={on_username_change} />
                <label class="konnekt-session-login__label" for="password">{"Password"}</label>
                <input class="konnekt-session-login__input" type="password" id="password" name="password" value={(*password).clone()} oninput={on_password_change} />
                <label class="konnekt-session-login__label" for="role">{"Role"}</label>
                <select class="konnekt-session-login__select" id="role" name="role" onchange={on_role_change}>
                    <option value="Admin">{"Admin"}</option>
                    <option value="Participant">{"Participant"}</option>
                    <option value="Observer">{"Observer"}</option>
                </select>
                <label class="konnekt-session-login__label" for="lobby_id">{"Lobby ID (optional)"}</label>
                <input class="konnekt-session-login__input" type="text" id="lobby_id" name="lobby_id" value={(*lobby_id).clone()} oninput={on_lobby_id_change} />
                <button class="konnekt-session-login__button" type="submit">{"Login"}</button>
            </form>
        </div>
    }
}
