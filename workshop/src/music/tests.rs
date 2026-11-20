//! The tests of the music editor: chords arpeggiated.

use super::notes::arpeggiated;
use super::*;

#[test]
fn chords_are_arpeggiated_along_their_length() {
    let chord: Vec<Note> = [0.0, 4.0, 7.0].iter().map(|tone| Note { start: 0.0, length: 2.0, tone: *tone, velocity: 1.0 }).collect();
    let up = arpeggiated(&chord, 0, 0.5);
    assert_eq!(up.iter().map(|note| note.tone).collect::<Vec<_>>(), vec![0.0, 4.0, 7.0, 0.0]);
    assert_eq!(up.iter().map(|note| note.start).collect::<Vec<_>>(), vec![0.0, 0.5, 1.0, 1.5]);
    let both = arpeggiated(&chord, 2, 0.25);
    assert_eq!(both.iter().take(5).map(|note| note.tone).collect::<Vec<_>>(), vec![0.0, 4.0, 7.0, 4.0, 0.0]);
}
