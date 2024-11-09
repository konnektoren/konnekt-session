use super::Challenge;
use crate::components::ActivityProps;
use crate::model::Named;
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
        let props = ctx.props();
        html! {
            <div class="konnekt-session-challenge">
                <h1 class="konnekt-session-challenge__title">{props.activity.name()}</h1>
                <p>{"Complete the challenge to earn points!"}</p>
            </div>
        }
    }
}
