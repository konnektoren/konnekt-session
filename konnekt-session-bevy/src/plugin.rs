use bevy_app::{App, Plugin, Update};
use bevy_ecs::message::{Message, MessageReader, MessageWriter};
use bevy_ecs::prelude::Resource;
use bevy_ecs::system::ResMut;
use konnekt_session_core::{DomainCommand, DomainEvent, DomainEventLoop};

#[derive(Debug, Clone, Message)]
pub struct SessionCommand(pub DomainCommand);

#[derive(Debug, Clone, Message)]
pub struct SessionDomainEvent {
    pub sequence: u64,
    pub event: DomainEvent,
}

#[derive(Debug, Resource, Default)]
pub struct SessionDomain {
    pub event_loop: DomainEventLoop,
}

#[derive(Debug, Resource, Default)]
pub struct SessionSequence(pub u64);

#[derive(Debug, Clone, Resource, Default)]
pub struct SessionEventLog(pub Vec<SessionDomainEvent>);

#[derive(Default)]
pub struct SessionPlugin;

impl Plugin for SessionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SessionDomain>()
            .init_resource::<SessionSequence>()
            .init_resource::<SessionEventLog>()
            .add_message::<SessionCommand>()
            .add_message::<SessionDomainEvent>()
            .add_systems(Update, apply_commands);
    }
}

fn apply_commands(
    mut commands: MessageReader<SessionCommand>,
    mut emitted: MessageWriter<SessionDomainEvent>,
    mut session: ResMut<SessionDomain>,
    mut sequence: ResMut<SessionSequence>,
    mut event_log: ResMut<SessionEventLog>,
) {
    for SessionCommand(command) in commands.read() {
        let event = session.event_loop.handle_command(command.clone());
        sequence.0 += 1;

        let wrapped = SessionDomainEvent {
            sequence: sequence.0,
            event,
        };

        emitted.write(wrapped.clone());
        event_log.0.push(wrapped);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use konnekt_session_core::DomainEvent;
    use uuid::Uuid;

    #[test]
    fn create_lobby_emits_sequenced_event() {
        let mut app = App::new();
        app.add_plugins(SessionPlugin);

        let lobby_id = Uuid::new_v4();
        app.world_mut().write_message(SessionCommand(DomainCommand::CreateLobby {
            lobby_id: Some(lobby_id),
            lobby_name: "Friday Session".to_string(),
            host_name: "Alice".to_string(),
        }));

        app.update();

        let log = app.world().resource::<SessionEventLog>();
        assert_eq!(log.0.len(), 1);
        assert_eq!(log.0[0].sequence, 1);

        match &log.0[0].event {
            DomainEvent::LobbyCreated { lobby } => {
                assert_eq!(lobby.id(), lobby_id);
                assert_eq!(lobby.name(), "Friday Session");
            }
            other => panic!("expected LobbyCreated, got {other:?}"),
        }

        let session = app.world().resource::<SessionDomain>();
        assert!(session.event_loop.get_lobby(&lobby_id).is_some());
    }

    #[test]
    fn commands_are_processed_in_order_with_monotonic_sequence() {
        let mut app = App::new();
        app.add_plugins(SessionPlugin);

        let lobby_id = Uuid::new_v4();
        app.world_mut().write_message(SessionCommand(DomainCommand::CreateLobby {
            lobby_id: Some(lobby_id),
            lobby_name: "Stable Lobby".to_string(),
            host_name: "Host".to_string(),
        }));
        app.world_mut().write_message(SessionCommand(DomainCommand::JoinLobby {
            lobby_id,
            guest_name: "Guest".to_string(),
        }));

        app.update();

        let log = app.world().resource::<SessionEventLog>();
        assert_eq!(log.0.len(), 2);
        assert_eq!(log.0[0].sequence, 1);
        assert_eq!(log.0[1].sequence, 2);

        assert!(matches!(log.0[0].event, DomainEvent::LobbyCreated { .. }));
        assert!(matches!(log.0[1].event, DomainEvent::GuestJoined { .. }));
    }
}
