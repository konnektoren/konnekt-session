//! Pair confirmation for a matchmaking search. Transport and timeouts are supplied by the caller.
use state_machines::state_machine;
use uuid::Uuid;

pub use crate::domain::{Hello, Message, Offer, State, search_window};

state_machine! {
    name: SearchMachine,
    context: (),
    dynamic: true,
    initial: Searching,
    states: [Searching, Offering, Reserved, Committing, Joining, Matched, Cancelled],
    final_states: [Matched, Cancelled],
    events {
        offer_sent { transition: { from: Searching, to: Offering } }
        offer_received { transition: { from: Searching, to: Reserved } }
        offer_accepted { transition: { from: Offering, to: Committing } }
        commit_received { transition: { from: Reserved, to: Joining } }
        commit_acknowledged { transition: { from: Committing, to: Joining } }
        attempt_aborted {
            transition: { from: Offering, to: Searching }
            transition: { from: Reserved, to: Searching }
            transition: { from: Committing, to: Searching }
            transition: { from: Joining, to: Searching }
        }
        room_ready { transition: { from: Joining, to: Matched } }
        cancel {
            transition: { from: Searching, to: Cancelled }
            transition: { from: Offering, to: Cancelled }
            transition: { from: Reserved, to: Cancelled }
            transition: { from: Committing, to: Cancelled }
            transition: { from: Joining, to: Cancelled }
        }
    }
}

/// One local search. The caller checks eligibility, peer identity and room readiness.
pub struct Matchmaking {
    search: Uuid,
    machine: DynamicSearchMachine,
    offer: Option<Offer>,
}

impl Matchmaking {
    pub fn new(search: Uuid) -> Self {
        Self {
            search,
            machine: DynamicSearchMachine::new(()),
            offer: None,
        }
    }

    pub fn state(&self) -> State {
        match self.machine.current_state() {
            SearchMachineState::Searching => State::Searching,
            SearchMachineState::Offering => {
                State::Offering(self.offer.expect("offering has an offer"))
            }
            SearchMachineState::Reserved => {
                State::Reserved(self.offer.expect("reservation has an offer"))
            }
            SearchMachineState::Committing => {
                State::Committing(self.offer.expect("commit has an offer"))
            }
            SearchMachineState::Joining => {
                State::Joining(self.offer.expect("joining has an offer"))
            }
            SearchMachineState::Matched => State::Matched(self.offer.expect("match has an offer")),
            SearchMachineState::Cancelled => State::Cancelled,
        }
    }

    /// Reserve an eligible candidate. Only the lower search ID can propose.
    pub fn offer(&mut self, candidate: Uuid, id: Uuid, room: Uuid) -> Option<Message> {
        if self.search >= candidate || self.machine.handle(SearchMachineEvent::OfferSent).is_err() {
            return None;
        }
        let offer = Offer {
            proposer: self.search,
            responder: candidate,
            id,
            room,
        };
        self.offer = Some(offer);
        Some(Message::Offer(offer))
    }

    /// Process a message after the caller verifies its sender and eligibility.
    pub fn receive(&mut self, peer: Uuid, message: Message) -> Option<Message> {
        let offer = match message {
            Message::Offer(o)
            | Message::Accept(o)
            | Message::Commit(o)
            | Message::CommitAck(o)
            | Message::Busy(o)
            | Message::Cancel(o) => o,
        };
        if offer.proposer >= offer.responder
            || !((offer.proposer == self.search && offer.responder == peer)
                || (offer.responder == self.search && offer.proposer == peer))
        {
            return None;
        }
        match (self.state(), message) {
            (State::Searching, Message::Offer(o)) if o.responder == self.search => {
                self.machine
                    .handle(SearchMachineEvent::OfferReceived)
                    .ok()?;
                self.offer = Some(o);
                Some(Message::Accept(o))
            }
            (State::Reserved(current), Message::Offer(o)) if current == o => {
                Some(Message::Accept(o))
            }
            (State::Offering(current), Message::Accept(o)) if current == o => {
                self.machine
                    .handle(SearchMachineEvent::OfferAccepted)
                    .ok()?;
                Some(Message::Commit(o))
            }
            (State::Committing(current), Message::Accept(o)) if current == o => {
                Some(Message::Commit(o))
            }
            (State::Reserved(current), Message::Commit(o)) if current == o => {
                self.machine
                    .handle(SearchMachineEvent::CommitReceived)
                    .ok()?;
                Some(Message::CommitAck(o))
            }
            (State::Joining(current), Message::Commit(o))
                if current == o && self.search == o.responder =>
            {
                Some(Message::CommitAck(o))
            }
            (State::Committing(current), Message::CommitAck(o)) if current == o => {
                self.machine
                    .handle(SearchMachineEvent::CommitAcknowledged)
                    .ok()?;
                None
            }
            (State::Offering(current), Message::Busy(o) | Message::Cancel(o))
            | (State::Reserved(current), Message::Cancel(o))
            | (State::Committing(current), Message::Cancel(o))
            | (State::Joining(current), Message::Cancel(o))
                if current == o =>
            {
                self.machine
                    .handle(SearchMachineEvent::AttemptAborted)
                    .ok()?;
                self.offer = None;
                None
            }
            (_, Message::Offer(o))
                if o.responder == self.search
                    && !matches!(self.state(), State::Cancelled | State::Matched(_)) =>
            {
                Some(Message::Busy(o))
            }
            _ => None,
        }
    }

    /// Abort a pending attempt, for example after a timeout or disconnect.
    pub fn abort_attempt(&mut self) -> Option<Message> {
        self.machine
            .handle(SearchMachineEvent::AttemptAborted)
            .ok()?;
        self.offer.take().map(Message::Cancel)
    }

    /// Call after both expected players acknowledge readiness in the game room.
    pub fn room_ready(&mut self, offer: Offer) -> bool {
        self.offer == Some(offer) && self.machine.handle(SearchMachineEvent::RoomReady).is_ok()
    }

    pub fn cancel(&mut self) -> Option<Message> {
        self.machine.handle(SearchMachineEvent::Cancel).ok()?;
        self.offer.take().map(Message::Cancel)
    }
}
