//! Chimecco Assembly and the team's name, as the original has them: who of
//! the team goes exploring (four at most: the leader, the partner, the
//! recruits along), each member seen (level, HP, moves, tactic), its tactic
//! changed, a recruit sent away for good once asked twice; the partner
//! may stay home once the team has graduated (`set graduated`), or lead
//! the team (the leader becoming its partner), as the original lets it
//! once its story is over; a recruit cannot lead (the rules keep it as a
//! recruit, not as a member that grows). And the team named, letter by
//! letter. What they do is the rules' (`rules::run::Run`).

use rules::run::Run;
use rules::run::town::{DEFAULT_TEAM_NAME, TEAM_NAME_LETTERS, Whom};
use rules::services::Input;
use shell::Keys;
use stage_view::ui::{ListCursor, Picked};

/// The most who go exploring, as in the original.
const TEAM_MOST: usize = 4;

/// What the assembly shows: a message, choices under it, the one the cursor is on.
pub type Shown = (String, Vec<String>, usize);

/// The assembly open.
#[derive(Default)]
pub struct Assembly {
    cursor: ListCursor,
    page: Page,
    /// The partner may stay home: the team has graduated.
    pub free: bool,
    /// What it did, shown till a key.
    said: Option<String>,
}

#[derive(Default)]
enum Page {
    /// The team, each member going or not.
    #[default]
    Team,
    /// One member: what it is, what may be done with it.
    Member { whom: Whom, cursor: ListCursor },
    /// "Release …?" asked of a recruit.
    Release { at: usize, cursor: ListCursor },
}

/// What may be done with a member.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Choice {
    /// It goes, or stays home.
    Along,
    /// Its next tactic.
    Tactic,
    /// The partner leads, the leader becomes its partner.
    Lead,
    Release,
    Back,
}

/// What a member's page offers: the leader always goes; the partner stays
/// home only once the team is free to; a recruit goes or not, and may be
/// released.
fn choices(whom: Whom, free: bool) -> Vec<Choice> {
    match whom {
        Whom::Leader => vec![Choice::Back],
        Whom::Partner if free => vec![Choice::Along, Choice::Lead, Choice::Tactic, Choice::Back],
        Whom::Partner => vec![Choice::Tactic, Choice::Back],
        Whom::Recruit(_) => vec![Choice::Along, Choice::Tactic, Choice::Release, Choice::Back],
    }
}

impl Assembly {
    /// A key, and what is shown; whether it is left.
    pub fn play(&mut self, pressed: Option<Input>, run: &mut Run) -> (bool, Shown) {
        if self.said.is_some() {
            if matches!(pressed, Some(Input::Confirm | Input::Cancel)) {
                self.said = None;
            }
            return (false, (self.said.clone().unwrap_or_default(), Vec::new(), 0));
        }
        let left = self.press(pressed, run);
        if let Some(said) = &self.said {
            return (left, (said.clone(), Vec::new(), 0));
        }
        (left, self.shown(run))
    }

    fn press(&mut self, pressed: Option<Input>, run: &mut Run) -> bool {
        let members = run.members();
        match &mut self.page {
            Page::Team => {
                // (Space: the member under the cursor goes, or not, at once.)
                if pressed == Some(Input::Mark) {
                    if let Some(&whom) = members.get(self.cursor.at) {
                        self.toggle(whom, run);
                    }
                    return false;
                }
                match self.cursor.press(pressed, members.len() + 1) {
                    Picked::Chosen(row) if row < members.len() => self.page = Page::Member { whom: members[row], cursor: ListCursor::default() },
                    Picked::Chosen(_) | Picked::Left => return true,
                    _ => {}
                }
            }
            Page::Member { whom, cursor } => {
                let (whom, offered) = (*whom, choices(*whom, self.free));
                match cursor.press(pressed, offered.len()).chosen_or_left(offered.len()) {
                    Some(Some(row)) => match offered[row] {
                        Choice::Along => self.toggle(whom, run),
                        Choice::Tactic => {
                            let who = match whom {
                                Whom::Partner => 1,
                                Whom::Recruit(at) => at + 2,
                                Whom::Leader => 0,
                            };
                            run.next_tactic(who);
                        }
                        Choice::Lead => {
                            if run.swap_leader() {
                                self.said = Some(format!("{} leads the team now!", run.member_name(Whom::Leader)));
                                (self.page, self.cursor) = (Page::Team, ListCursor::default());
                            }
                        }
                        Choice::Release => {
                            if let Whom::Recruit(at) = whom {
                                self.page = Page::Release { at, cursor: ListCursor::at(1) };
                            }
                        }
                        Choice::Back => self.page = Page::Team,
                    },
                    Some(None) => self.page = Page::Team,
                    None => {}
                }
            }
            Page::Release { at, cursor } => {
                let at = *at;
                match cursor.press(pressed, 2).chosen_or_left(2) {
                    Some(Some(0)) => {
                        let name = run.member_name(Whom::Recruit(at));
                        run.release(at);
                        self.said = Some(format!("{name} left the team. Take care, {name}!"));
                        self.page = Page::Team;
                        self.cursor = ListCursor::default();
                    }
                    Some(_) => self.page = Page::Member { whom: Whom::Recruit(at), cursor: ListCursor::default() },
                    None => {}
                }
            }
        }
        false
    }

    /// A member goes, or stays home, if it may: the leader always goes, the
    /// partner stays only once the team is free to, the recruits as there is room.
    fn toggle(&mut self, whom: Whom, run: &mut Run) {
        match whom {
            Whom::Leader => {}
            Whom::Partner if self.free => {
                let home = !run.team().progress.partner_home;
                run.keep_partner_home(home);
            }
            Whom::Partner => self.said = Some("Chimecco: Your partner goes with you till your training at the guild is over!".to_string()),
            Whom::Recruit(at) => {
                let along = run.along().contains(&at);
                run.pick_along(at);
                if !along && !run.along().contains(&at) {
                    self.said = Some(format!("Chimecco: The team is full! {TEAM_MOST} go exploring at most."));
                }
            }
        }
    }

    fn shown(&self, run: &Run) -> Shown {
        match &self.page {
            Page::Team => {
                let along = run.along();
                let partner_home = run.team().progress.partner_home;
                let row = |whom: Whom| {
                    let (goes, note) = match whom {
                        Whom::Leader => (true, "  Leader"),
                        Whom::Partner => (!partner_home, if partner_home { "  at home" } else { "" }),
                        Whom::Recruit(at) => (along.contains(&at), ""),
                    };
                    let mark = if goes { "[CS:G]+[CR]" } else { "  " };
                    format!("{mark} {}  Lv. {}{note}", run.member_name(whom), run.member_level(whom))
                };
                let options = run.members().into_iter().map(row).chain(["Done".to_string()]).collect();
                let message = format!(
                    "Chimecco: Welcome to Chimecco Assembly! Who goes exploring? {} / {TEAM_MOST}\nEnter: see a member   Space: goes or stays",
                    run.team_size()
                );
                (message, options, self.cursor.at)
            }
            Page::Member { whom, cursor } => {
                let whom = *whom;
                let tactic = tactic_name(run, run.member_tactic(whom));
                let along = match whom {
                    Whom::Leader => true,
                    Whom::Partner => !run.team().progress.partner_home,
                    Whom::Recruit(at) => run.along().contains(&at),
                };
                let option = |choice: &Choice| match choice {
                    Choice::Along if along => "Stay home".to_string(),
                    Choice::Along => "Come along".to_string(),
                    Choice::Tactic => format!("Tactic: {tactic}"),
                    Choice::Lead => "Lead the team".to_string(),
                    Choice::Release => "Release".to_string(),
                    Choice::Back => "Back".to_string(),
                };
                let options = choices(whom, self.free).iter().map(option).collect();
                (summary(run, whom), options, cursor.at)
            }
            Page::Release { at, cursor } => {
                let name = run.member_name(Whom::Recruit(*at));
                (format!("Release {name}? It will leave the team for good."), vec!["Yes".to_string(), "No".to_string()], cursor.at)
            }
        }
    }
}

/// A member as its page tells it: its name, species and level, its HP, its
/// moves, its tactic (the leader: none, it leads).
fn summary(run: &Run, whom: Whom) -> String {
    let book = &run.book;
    let (name, species) = (run.member_name(whom), book.species_name(run.member_species(whom)));
    let called = if name == species { name } else { format!("{name} ({species})") };
    let moves: Vec<String> = run.member_moves(whom).into_iter().map(|id| book.move_name(id)).collect();
    let moves = if moves.is_empty() { "none".to_string() } else { moves.join(", ") };
    let role = match whom {
        Whom::Leader => "The leader: always goes.".to_string(),
        whom => format!("Tactic: {}", tactic_name(run, run.member_tactic(whom))),
    };
    format!("{called}  Lv. {}  HP {}\nMoves: {moves}\n{role}", run.member_level(whom), run.member_hp(whom))
}

fn tactic_name(run: &Run, tactic: u8) -> String {
    run.book.tables.tactics.get(tactic as usize).cloned().filter(|name| !name.is_empty()).unwrap_or_else(|| format!("Tactic {tactic}"))
}

/// What a key did to a list: the row chosen (`Some(Some(row))`), left
/// (`Some(None)`), or nothing yet.
trait ChosenOrLeft {
    fn chosen_or_left(self, rows: usize) -> Option<Option<usize>>;
}

impl ChosenOrLeft for Picked {
    fn chosen_or_left(self, rows: usize) -> Option<Option<usize>> {
        match self {
            Picked::Chosen(row) if row < rows => Some(Some(row)),
            Picked::Chosen(_) | Picked::Left => Some(None),
            _ => None,
        }
    }
}

/// The team's name typed (`shop name`, `name_team`), from the one it has.
pub struct Naming {
    pub typed: String,
}

impl Naming {
    pub fn new(run: &Run) -> Self {
        Naming { typed: run.team().progress.names.2.clone() }
    }

    /// The letters of a frame, Backspace, Enter (the name taken, if there is
    /// one), Escape (left as it was); what is shown; whether it is left.
    pub fn play(&mut self, keys: &Keys, run: &mut Run) -> (bool, String) {
        let backspace = keys.named.iter().any(|name| name == "backspace");
        self.typed = typed_into(&self.typed, &keys.typed, backspace);
        let left = match () {
            _ if keys.enter => {
                if !self.typed.trim().is_empty() {
                    run.rename(rules::run::team::Whose::Team, &self.typed);
                }
                true
            }
            _ => keys.escape,
        };
        let shown = if self.typed.is_empty() { format!("[CS:E]{DEFAULT_TEAM_NAME}[CR]") } else { format!("[CS:G]{}_[CR]", self.typed) };
        (left, format!("What is your team's name?\n{shown}\nType it ({TEAM_NAME_LETTERS} letters at most), then Enter."))
    }
}

/// A name with the letters of a frame written in, one taken off first if
/// asked: no more than the original lets.
fn typed_into(name: &str, letters: &[char], backspace: bool) -> String {
    let mut name = name.to_string();
    if backspace {
        name.pop();
    }
    for &letter in letters.iter().filter(|letter| letter.is_alphanumeric() || matches!(letter, ' ' | '-' | '\'' | '!' | '.')) {
        if name.chars().count() < TEAM_NAME_LETTERS {
            name.push(letter);
        }
    }
    name
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_team_s_name_takes_letters_to_ten_and_loses_them_to_backspace() {
        assert_eq!(typed_into("", &['T', 'h', 'o', 'r', 'n', 's'], false), "Thorns");
        assert_eq!(typed_into("Thorns", &[], true), "Thorn");
        assert_eq!(typed_into("Thornsbrig", &['x'], false), "Thornsbrig", "ten letters at most");
        assert_eq!(typed_into("A", &['\t', 'b'], false), "Ab");
    }

    #[test]
    fn the_leader_always_goes_and_the_partner_stays_home_once_free() {
        assert_eq!(choices(Whom::Leader, true), [Choice::Back]);
        assert!(!choices(Whom::Partner, false).contains(&Choice::Along));
        assert!(choices(Whom::Partner, true).contains(&Choice::Along));
        assert!(choices(Whom::Partner, true).contains(&Choice::Lead) && !choices(Whom::Partner, false).contains(&Choice::Lead), "leads once free");
        assert!(!choices(Whom::Recruit(0), true).contains(&Choice::Lead), "a recruit does not lead");
        assert!(choices(Whom::Recruit(0), false).contains(&Choice::Release));
    }
}
