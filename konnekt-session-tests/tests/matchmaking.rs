use konnekt_session_rating::{
    RatingKey,
    protocol::matchmaking::{Hello, Matchmaking, Message, Offer, State, search_window},
};
use uuid::Uuid;

fn id(value: u128) -> Uuid {
    Uuid::from_u128(value)
}

fn offer(proposer: u128, responder: u128, offer_id: u128) -> Offer {
    Offer {
        proposer: id(proposer),
        responder: id(responder),
        id: id(offer_id),
        room: id(100),
    }
}

#[test]
fn two_players_confirm_a_match_and_retry_lost_messages() {
    let mut alice = Matchmaking::new(id(1));
    let mut bob = Matchmaking::new(id(2));
    let expected = offer(1, 2, 3);
    let request = alice.offer(id(2), id(3), id(100)).unwrap();
    assert_eq!(request, Message::Offer(expected));
    let accept = bob.receive(id(1), request).unwrap();
    assert_eq!(bob.receive(id(1), request), Some(accept));
    let commit = alice.receive(id(2), accept).unwrap();
    assert_eq!(alice.receive(id(2), accept), Some(commit));
    let ack = bob.receive(id(1), commit).unwrap();
    assert_eq!(bob.receive(id(1), commit), Some(ack));
    assert_eq!(alice.receive(id(2), ack), None);
    assert_eq!(alice.state(), State::Joining(expected));
    assert_eq!(bob.state(), State::Joining(expected));
    assert!(!alice.room_ready(offer(1, 2, 4)));
    assert!(alice.room_ready(expected));
    assert!(bob.room_ready(expected));
    assert_eq!(alice.state(), State::Matched(expected));
    assert_eq!(bob.state(), State::Matched(expected));
}

#[test]
fn competing_offers_and_stale_messages_do_not_steal_a_reservation() {
    let mut responder = Matchmaking::new(id(3));
    let first = offer(1, 3, 10);
    let competing = offer(2, 3, 11);
    assert_eq!(
        responder.receive(id(1), Message::Offer(first)),
        Some(Message::Accept(first))
    );
    assert_eq!(
        responder.receive(id(2), Message::Offer(competing)),
        Some(Message::Busy(competing))
    );
    assert_eq!(responder.receive(id(2), Message::Cancel(competing)), None);
    assert_eq!(responder.receive(id(2), Message::Commit(first)), None);
    assert_eq!(responder.state(), State::Reserved(first));
    assert_eq!(responder.abort_attempt(), Some(Message::Cancel(first)));
    assert_eq!(responder.receive(id(1), Message::Commit(first)), None);
    assert_eq!(
        responder.receive(id(2), Message::Offer(competing)),
        Some(Message::Accept(competing))
    );
    assert_eq!(responder.state(), State::Reserved(competing));
    assert_eq!(responder.cancel(), Some(Message::Cancel(competing)));
    assert_eq!(responder.receive(id(2), Message::Commit(competing)), None);
    assert_eq!(responder.state(), State::Cancelled);
}

#[test]
fn discovery_requires_mutual_rating_window_and_matching_identity() {
    let hello = Hello {
        version: 1,
        search: id(1),
        rating_key: RatingKey::new("duel").unwrap(),
        rating: 1000.0,
        window: 100.0,
    };
    let mut candidate = Hello {
        search: id(2),
        rating: 1100.0,
        ..hello.clone()
    };
    assert!(hello.accepts(&candidate));
    candidate.window = 99.0;
    assert!(!hello.accepts(&candidate));
    candidate.window = 100.0;
    candidate.rating = f64::NAN;
    assert!(!hello.accepts(&candidate));
    candidate.rating = 1000.0;
    candidate.search = hello.search;
    assert!(!hello.accepts(&candidate));
    assert_eq!(
        (
            search_window(0),
            search_window(4),
            search_window(5),
            search_window(30),
            search_window(u64::MAX)
        ),
        (100, 100, 150, 400, 400)
    );
}
