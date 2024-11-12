use super::Challenge;
use crate::components::ActivityProps;
use crate::example::ChallengeResult;
use crate::model::{LobbyCommand, Named};
use yew::prelude::*;

#[derive(PartialEq)]
pub struct ChallengeComp;

impl Component for ChallengeComp {
    type Message = ();
    type Properties = ActivityProps<Challenge>;

    fn create(_ctx: &Context<Self>) -> Self {
        Self
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let props = ctx.props().clone();

        let on_end = {
            let challenge_result = ChallengeResult {
                id: props.activity.id.clone(),
                performance: 0,
            };
            let data = serde_json::to_string(&challenge_result).unwrap();

            let command = LobbyCommand::AddActivityResult {
                activity_id: props.activity.id.clone(),
                player_id: props.player_id,
                data,
            };
            let on_command = props.on_command.clone();
            Callback::from(move |_| {
                on_command.emit(command.clone());
            })
        };

        html! {
            <div class="konnekt-session-challenge">
                <h1 class="konnekt-session-challenge__title">{props.activity.name()}</h1>
                <p>{"Complete the challenge to earn points!"}</p>
                <button class="konnekt-session-challenge__end" onclick={on_end}>{"End"}</button>
            </div>
        }
    }
}
