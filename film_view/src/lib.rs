//! What the films show, a group each: the view (`view`: `view`, `frame`,
//! `shot`, `camera`, one family), the sky and its light (`sky`: the weather,
//! the hour), what is shown of no one's doing (`effects`: particles, a flash,
//! the screen trembling), the game's pictures (`pictures`: looks at them,
//! visions, a scene's transitions) and the nodes' properties (`nodes`).
//! Each plays its lines by the core's helpers (`film_core::emit`,
//! `film_core::spots`); `film` puts their `GROUP`s in its order.

pub mod effects;
pub mod nodes;
pub mod pictures;
pub mod sky;
pub mod view;
