//! How someone is shown, beyond where it stands and how it moves (its
//! [`Actor`]): the one model of a body's looks, kept through the scenes even
//! when it goes off the stage. A variant is painted as such (`vues`): the
//! ways it is painted in; once it stands it faces the first again, unless a
//! line turned it. What it wears is in its pictures (a variant, painted by hand).

use super::*;

/// What someone is shown as.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Figure {
    /// The ways it is painted in, if only some (see `BodyBeat::Views`).
    pub views: Vec<u16>,
    /// A line turned it a way it is not painted in, until it walks or
    /// stands as usual again.
    pub turned: bool,
}

impl Stage {
    /// The ways someone is painted in, if only some.
    pub fn painted_ways(&self, who: Who) -> Option<&[u16]> {
        self.figures.get(&who).map(|figure| figure.views.as_slice()).filter(|ways| !ways.is_empty())
    }

    /// Those painted in some ways only, and those ways.
    pub fn painted(&self) -> impl Iterator<Item = (Who, &[u16])> {
        self.figures.iter().filter(|(_, figure)| !figure.views.is_empty()).map(|(who, figure)| (*who, figure.views.as_slice()))
    }

    /// The ways someone is painted in (none: all of them); turned by no line
    /// since.
    pub(in crate::stage) fn paint(&mut self, who: Who, ways: Vec<u16>) {
        let figure = self.figures.entry(who).or_default();
        (figure.views, figure.turned) = (ways, false);
    }

    /// Whether a line turned someone a way it is not painted in (only one
    /// painted in some ways is ever turned).
    pub(crate) fn turned_by_a_line(&mut self, who: Who, turned: bool) {
        if let Some(figure) = self.figures.get_mut(&who).filter(|figure| !turned || !figure.views.is_empty()) {
            figure.turned = turned;
        }
    }

    /// Who is painted facing some ways only faces them again once it stands,
    /// unless a line turned it; walking, it is turned no more.
    pub(in crate::stage) fn keep_painted_ways(&mut self) {
        for (who, figure) in self.figures.iter_mut().filter(|(_, figure)| !figure.views.is_empty()) {
            let Some(actor) = self.cast.get_mut(who) else {
                continue;
            };
            if actor.walks() {
                figure.turned = false;
            } else if !figure.turned
                && !figure.views.contains(&actor.direction)
                && actor.air.spin.is_none()
                && actor.habits.orbit.is_none()
                && let Some(first) = figure.views.first()
            {
                actor.direction = *first;
            }
        }
    }
}
