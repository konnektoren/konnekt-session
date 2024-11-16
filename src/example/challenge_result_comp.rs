use crate::components::{ActivityResultProps, AvatarComp};
use crate::example::{ChallengeResult, PlayerProfile};
use crate::model::{Named, Scorable, Timable};
use yew::prelude::*;

#[derive(PartialEq, Clone)]
pub struct ChallengeResultComp;

impl Component for ChallengeResultComp {
    type Message = ();
    type Properties = ActivityResultProps<PlayerProfile, ChallengeResult>;

    fn create(_ctx: &Context<Self>) -> Self {
        Self
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let props = ctx.props().clone();

        html! {
            <div class="konnekt-session-activity-result">
                <div class="konnekt-session-activity-result__player">
                    <AvatarComp player_id={props.player.id.clone()} />
                    {props.player.name()}
                </div>
                <div class="konnekt-session-activity-result__score">
                    <span><i class="fas fa-trophy konnekt-session-result__score-icon"></i>{"Score"}</span>
                    <span>{props.result.data.score()}</span>
                </div>
                <div class="konnekt-session-activity-result__time">
                    <span><i class="fas fa-clock konnekt-session-result__time-icon"></i>{"Time"}</span>
                    <span>{format!("{}:{:02}", props.result.data.time_taken() / 60, props.result.data.time_taken() % 60)}</span>
                </div>
            </div>
        }
    }
}
