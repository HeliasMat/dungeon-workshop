# Add a Workshop panel or part

A checklist. The window is `workshop/src/app.rs`. The parts are the `workshop_*` crates; `workshop/README.md`, "The code", says which crate holds what.

## 1. Pick its crate

- **A panel of an existing space**: put it in that space's crate. For example, a Decor panel goes in `workshop_decor/src/panels/`, a Film panel in `workshop_film/src/`, a Resources panel in `workshop_resources/src/`.
- **A new part** (a space, or a large tool): make it a crate of its own, `workshop_<part>/`.
  - Its `Cargo.toml` takes the workspace's fields and `[lints] workspace = true` (copy `workshop_picture/Cargo.toml`).
  - Add it to `members` in the root `Cargo.toml` and as a dependency of `workshop/Cargo.toml`.
  - Add its rows to the two crate tables of `ARCHITECTURE.md`: what it holds, and what it uses. `sh tools/rule.sh` fails until those tables and the manifests agree.
  - A part uses `workshop_base` and the parts below it, never `workshop`.
  - Say what it holds in its `lib.rs` doc, and add it to the tree and the table of `workshop/README.md`, "The code".

## 2. Its state, and the session

A part keeps its own state in one struct, made once by `App::new` (`workshop/src/app.rs`) and kept in `Work`:

```rust
pub struct Things {
    /// What the Workshop works with (`session`).
    pub session: Arc<Session>,
    // ...its own state
}

impl Things {
    pub fn new(session: &Arc<Session>) -> Things { /* ... */ }
}
```

`Session` (`workshop_base/src/session.rs`) is everything the parts share:
- `context`: the library, the data and the film tables;
- `launch`: the launch options, read in `workshop_base/src/launch.rs`;
- things read once and kept (`Kept`, `kept: KeptByType`);
- `asks`: what the parts ask of the window.

Read the files through it (`session.library()`, `session.data()`), never through a static. `Session::for_tests()` gives a part's tests the same thing.

## 3. Asking the window for something

A part never changes another part's state. To open a file in its space, play the film from a line, go to a space, open a window or say something in the bar of state, it pushes an `Ask` (`workshop_base/src/ask.rs`):

```rust
use workshop_base::ask::{Ask, OpenDungeon};

things.session.ask(Ask::Film { path, line });   // a film at a line (0: where it was)
things.session.ask(Ask::Decor { path, line: 0 });
things.session.ask(Ask::Dungeon(OpenDungeon::NewFight));
things.session.ask(Ask::PlayFrom(line));
things.session.ask(Ask::Go(Space::Assets));
things.session.ask(Ask::Say("saved".to_string()));
```

The window does them all once a frame, in the order they were asked, in `App::answer` (`workshop/src/app.rs`). That function is the only place where requests between parts are handled.

**A new kind of request** takes two steps:
1. Add a variant to `Ask`, with its doc line.
2. Add its arm to `App::answer`.

Don't add an `Option<...>` mailbox field that the window `.take()`s.

## 4. Register the panel in the window

All in `workshop/src/app.rs`:

1. Add its variant to `Panel`, under its space's comment.
2. Give it a title in `Panel::title`.
3. Place it in its space's `layout`, with `split_left`, `split_below` or a tab beside another panel (`vec![Panel::Hierarchy, Panel::History]`).
4. Draw it in the `ui` match of `impl egui_dock::TabViewer for Panels`, by calling your crate: `Panel::Things => workshop_things::panel(ui, &mut self.work.things)`. That arm only calls the part; requests go through `Ask`.

**A new space**:
1. Add it to `Space` and `Space::ALL`, with its word and its accent, in `workshop_base/src/ask.rs`. Its accent colour goes in `workshop_base/src/theme.rs`.
2. Give it a `layout` (or `DockState::new(Vec::new())` for a page on the whole window, drawn in `App::update` as the Game and Dungeon spaces are).
3. Add its buttons to `App::tools`.
4. Add its `WORKSHOP_SPACE` value to `App::new`.

## 5. Its documentation

- What it does, for a user: its section of `workshop/README.md` (one per space; "Launch options" if it reads an environment variable, which also goes in `workshop_base/src/launch.rs`).
- The Wiki (`workshop_wiki/`) holds only the film language's words, so a panel has no Wiki page.

## 6. Run

```bash
cargo check -p workshop_things -p workshop       # after a small change
cargo clippy -p workshop_things -p workshop --tests
cargo nextest run -p workshop_things -p workshop
sh tools/verify.sh                                 # once, at the end
```

To see it without a mouse, launch the Workshop on it and take a picture: see `workshop/README.md`, "For a picture without a mouse".
