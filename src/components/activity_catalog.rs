use crate::model::{Activity, ActivityCatalog, ActivityData, Named};
use yew::prelude::*;

#[derive(Properties, PartialEq, Clone)]
pub struct ActivityCatalogProps<T>
where
    T: ActivityData + 'static,
{
    pub catalog: ActivityCatalog<T>,
    pub on_select: Callback<Activity<T>>,
}

#[function_component(ActivityCatalogComp)]
pub fn activity_catalog_comp<T>(props: &ActivityCatalogProps<T>) -> Html
where
    T: ActivityData + 'static,
{
    html! {
        <div class="konnekt-session-activity-catalog">
            <h2 class="konnekt-session-activity-catalog__title">{"Available Activities"}</h2>
            <div class="konnekt-session-activity-catalog__list">
                {for props.catalog.get_activities().iter().map(|activity| {
                    let activity = (*activity).clone();
                    let on_select = props.on_select.clone();
                    let onclick = {
                        let activity = activity.clone();
                        Callback::from(move |_| {
                        on_select.emit(activity.clone());
                    })
                    };

                    html! {
                        <button
                            class="konnekt-session-activity-catalog__item"
                            {onclick}
                        >
                            {activity.name()}
                        </button>
                    }
                })}
            </div>
        </div>
    }
}
