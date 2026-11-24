//! What a making asks of the Resources space that holds it.

/// What making asks of the Resources: a file written (to be read again and chosen), or done.
pub enum Done {
    Picture(String),
    Sound(String),
    Closed,
    /// A boss fight to open in its editor (none named: a new one).
    Fight(String),
}
