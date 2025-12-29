use super::Challenge;
use crate::components::ActivityProps;
use crate::example::ChallengeResult;
use crate::model::{LobbyCommand, Named};
use yew::prelude::*;

#[derive(PartialEq, Clone)]
pub struct ChallengeComp {
    selection: String,
    result: Option<ChallengeResult>,
}

pub enum Msg {
    SelectionChanged(String),
    EndChallenge,
}

impl Component for ChallengeComp {
    type Message = Msg;
    type Properties = ActivityProps<Challenge>;

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            selection: "Correct".to_string(),
            result: None,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::SelectionChanged(new_selection) => {
                self.selection = new_selection;
                true
            }
            Msg::EndChallenge => {
                let activity_id = ctx.props().activity.id.clone();
                let challenge_result = ChallengeResult {
                    id: activity_id.clone(),
                    performance: 0,
                    selection: self.selection.clone(),
                };

                self.result = Some(challenge_result.clone());

                let data = serde_json::to_string(&challenge_result).unwrap();

                let command = LobbyCommand::AddActivityResult {
                    activity_id: activity_id.clone(),
                    player_id: ctx.props().player_id,
                    data,
                };
                ctx.props().on_command.emit(command);
                false
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let on_selection_change = ctx.link().callback(|e: Event| {
            let target = e.target_unchecked_into::<web_sys::HtmlSelectElement>();
            Msg::SelectionChanged(target.value())
        });

        let on_end = ctx.link().callback(|_| Msg::EndChallenge);

        if let Some(result) = &self.result {
            return html! {
                <div class="konnekt-session-challenge">
                    <h1 class="konnekt-session-challenge__title">{ctx.props().activity.name()}</h1>
                    <p>{"Challenge completed!"}</p>
                    <p>{format!("You earned {} points!", result.performance)}</p>
                    <p>{format!("You selected {} as your answer!", result.selection)}</p>
                </div>
            };
        }

        html! {
            <div class="konnekt-session-challenge">
                <h1 class="konnekt-session-challenge__title">{ctx.props().activity.name()}</h1>
                <p>{"Complete the challenge to earn points!"}</p>
                <p>{"Select the CORRECT answer!"}</p>
                <select onchange={on_selection_change} value={self.selection.clone()}>
                    <option value="Correct">{"Correct"}</option>
                    <option value="Incorrect">{"Incorrect"}</option>
                    <option value="Skipped">{"Skipped"}</option>
                </select>
                <button class="konnekt-session-challenge__end" onclick={on_end}>{"End"}</button>
            </div>
        }
    }
}
