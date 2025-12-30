use cucumber::{given, then, when};
use konnekt_session_p2p::{
    PeerId,
    domain::{MatchboxPeerId, PeerParticipantMap},
};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Default, cucumber::World)]
pub struct MappingWorld {
    map: PeerParticipantMap,
    peer_ids: HashMap<String, PeerId>,
    participant_ids: HashMap<String, Uuid>,
    last_removed_peer: Option<Option<Uuid>>,
    last_removed_participant: Option<Option<PeerId>>,
    last_query_result_peer: Option<Option<PeerId>>,
    last_query_result_participant: Option<Option<Uuid>>,
}

impl MappingWorld {
    fn get_or_create_peer(&mut self, name: &str) -> PeerId {
        if let Some(peer) = self.peer_ids.get(name) {
            *peer
        } else {
            let peer = PeerId::new(MatchboxPeerId(Uuid::new_v4()));
            self.peer_ids.insert(name.to_string(), peer);
            peer
        }
    }

    fn get_or_create_participant(&mut self, name: &str) -> Uuid {
        if let Some(participant) = self.participant_ids.get(name) {
            *participant
        } else {
            let participant = Uuid::new_v4();
            self.participant_ids.insert(name.to_string(), participant);
            participant
        }
    }
}

// ===== Given Steps =====

#[given("an empty peer-participant mapping")]
async fn empty_mapping(world: &mut MappingWorld) {
    world.map = PeerParticipantMap::new();
    assert!(world.map.is_empty());
}

#[given(expr = "a peer with ID {string}")]
async fn create_peer(world: &mut MappingWorld, peer_name: String) {
    world.get_or_create_peer(&peer_name);
}

#[given(expr = "a participant with ID {string}")]
async fn create_participant(world: &mut MappingWorld, participant_name: String) {
    world.get_or_create_participant(&participant_name);
}

#[given(expr = "peer {string} is mapped to participant {string}")]
async fn peer_mapped_to_participant(
    world: &mut MappingWorld,
    peer_name: String,
    participant_name: String,
) {
    let peer = world.get_or_create_peer(&peer_name);
    let participant = world.get_or_create_participant(&participant_name);
    world.map.register(peer, participant);
}

// ===== When Steps =====

#[when(expr = "I register peer {string} to participant {string}")]
async fn register_mapping(world: &mut MappingWorld, peer_name: String, participant_name: String) {
    let peer = world.get_or_create_peer(&peer_name);
    let participant = world.get_or_create_participant(&participant_name);
    world.map.register(peer, participant);
}

#[when(expr = "I remove the mapping for peer {string}")]
async fn remove_by_peer(world: &mut MappingWorld, peer_name: String) {
    let peer = world.get_or_create_peer(&peer_name);
    let result = world.map.remove_by_peer(&peer);
    world.last_removed_peer = Some(result);
}

#[when(expr = "I remove the mapping for participant {string}")]
async fn remove_by_participant(world: &mut MappingWorld, participant_name: String) {
    let participant = world.get_or_create_participant(&participant_name);
    let result = world.map.remove_by_participant(&participant);
    world.last_removed_participant = Some(result);
}

#[when(expr = "I query participant for peer {string}")]
async fn query_participant(world: &mut MappingWorld, peer_name: String) {
    let peer = world.get_or_create_peer(&peer_name);
    let result = world.map.get_participant(&peer);
    world.last_query_result_participant = Some(result);
}

#[when(expr = "I query peer for participant {string}")]
async fn query_peer(world: &mut MappingWorld, participant_name: String) {
    let participant = world.get_or_create_participant(&participant_name);
    let result = world.map.get_peer(&participant);
    world.last_query_result_peer = Some(result);
}

#[when("I clear all mappings")]
async fn clear_mappings(world: &mut MappingWorld) {
    world.map.clear();
}

// ===== Then Steps =====

#[then(expr = "the mapping should contain {int} entry")]
#[then(expr = "the mapping should contain {int} entries")]
async fn mapping_size(world: &mut MappingWorld, expected: usize) {
    assert_eq!(world.map.len(), expected, "Mapping size mismatch");
}

#[then("the mapping should be empty")]
async fn mapping_empty(world: &mut MappingWorld) {
    assert!(world.map.is_empty(), "Mapping should be empty");
}

#[then(expr = "peer {string} should map to participant {string}")]
async fn peer_maps_to_participant(
    world: &mut MappingWorld,
    peer_name: String,
    participant_name: String,
) {
    let peer = world.peer_ids.get(&peer_name).expect("Peer not found");
    let expected_participant = world
        .participant_ids
        .get(&participant_name)
        .expect("Participant not found");

    let actual = world.map.get_participant(peer);
    assert_eq!(
        actual,
        Some(*expected_participant),
        "Peer {} should map to participant {}, but got {:?}",
        peer_name,
        participant_name,
        actual
    );
}

#[then(expr = "participant {string} should map to peer {string}")]
async fn participant_maps_to_peer(
    world: &mut MappingWorld,
    participant_name: String,
    peer_name: String,
) {
    let participant = world
        .participant_ids
        .get(&participant_name)
        .expect("Participant not found");
    let expected_peer = world.peer_ids.get(&peer_name).expect("Peer not found");

    let actual = world.map.get_peer(participant);
    assert_eq!(
        actual,
        Some(*expected_peer),
        "Participant {} should map to peer {}, but got {:?}",
        participant_name,
        peer_name,
        actual
    );
}

#[then(expr = "peer {string} should not be mapped")]
async fn peer_not_mapped(world: &mut MappingWorld, peer_name: String) {
    let peer = world.peer_ids.get(&peer_name).expect("Peer not found");
    let result = world.map.get_participant(peer);
    assert!(
        result.is_none(),
        "Peer {} should not be mapped, but is mapped to {:?}",
        peer_name,
        result
    );
}

#[then(expr = "participant {string} should not be mapped")]
async fn participant_not_mapped(world: &mut MappingWorld, participant_name: String) {
    let participant = world
        .participant_ids
        .get(&participant_name)
        .expect("Participant not found");
    let result = world.map.get_peer(participant);
    assert!(
        result.is_none(),
        "Participant {} should not be mapped, but is mapped to {:?}",
        participant_name,
        result
    );
}

#[then("the removal should return None")]
async fn removal_returns_none(world: &mut MappingWorld) {
    if let Some(result) = &world.last_removed_peer {
        assert!(result.is_none(), "Expected None, got {:?}", result);
    } else if let Some(result) = &world.last_removed_participant {
        assert!(result.is_none(), "Expected None, got {:?}", result);
    } else {
        panic!("No removal operation was performed");
    }
}

#[then("the query should return None")]
async fn query_returns_none(world: &mut MappingWorld) {
    if let Some(result) = &world.last_query_result_participant {
        assert!(result.is_none(), "Expected None, got {:?}", result);
    } else if let Some(result) = &world.last_query_result_peer {
        assert!(result.is_none(), "Expected None, got {:?}", result);
    } else {
        panic!("No query operation was performed");
    }
}
