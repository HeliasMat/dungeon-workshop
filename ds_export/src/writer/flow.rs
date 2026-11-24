//! The waits: for someone, the camera, a sound, a fade; a beat a while after.

use engine::{Beat, FlowBeat, SoundOf};

use super::Writer;

impl Writer<'_> {
    /// A flow beat the course left (`course_flow`): a wait, a beat after another.
    pub(super) fn flow(&mut self, number: i32, word: Option<&str>, beat: &FlowBeat) {
        match beat {
            FlowBeat::WaitCam => self.line("WaitExecutePerformer(0);"),
            FlowBeat::WaitSound { of: SoundOf::Game(sound), .. } => self.line(&format!("WaitSe({});", sound.0)),
            FlowBeat::WaitFor(who) | FlowBeat::WaitMoves(who) => {
                if let Some(actor) = self.who(number, *who) {
                    self.wait_for(&actor);
                }
            }
            FlowBeat::WaitAll => {
                for actor in self.busy.clone() {
                    self.wait_for(&actor);
                }
                self.line("WaitExecutePerformer(0);");
            }
            // What comes so long after: at once, the script going on as it plays.
            FlowBeat::After { beat, .. } => self.plain(number, word, beat),
            FlowBeat::Wait(frames) => {
                // Before the scene shows: the title's time, left out with it;
                // the engine's pauses after a line said: the box closing;
                // in a thread of `together`: its moves go on alone.
                if std::mem::take(&mut self.said_over) {
                    // (The time a caption shows: the game waits for its key.)
                } else if self.started && (!self.thread || self.capture.is_some()) && !(self.talking && frames.0 < 15.0) {
                    self.line(&format!("Wait({});", frames.0.round() as i32));
                    self.shown_for += frames.0;
                }
            }
            FlowBeat::WaitFade => self.line("WaitScreenFade();"),
            other => self.unexported(number, word, &Beat::Flow(other.clone())),
        }
    }
}
