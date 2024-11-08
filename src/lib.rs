#[cfg(feature = "yew")]
pub mod components;
pub mod model;

pub mod prelude {
    #[cfg(feature = "yew")]
    pub use crate::components::*;
    pub use crate::model::Activity;
    pub use crate::model::Identifiable;
    pub use crate::model::Lobby;
    pub use crate::model::Named;
    pub use crate::model::Player;
    pub use crate::model::PlayerData;
    pub use crate::model::Role;
}
