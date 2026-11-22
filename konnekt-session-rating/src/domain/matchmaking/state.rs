use super::Offer;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Searching,
    Offering(Offer),
    Reserved(Offer),
    Committing(Offer),
    Joining(Offer),
    Matched(Offer),
    Cancelled,
}
