pub trait Scorable {
    fn score(&self) -> u32 {
        0
    }
}
