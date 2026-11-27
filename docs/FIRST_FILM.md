# Your first film in 5 minutes

A short film with two Pokémon, made in the Workshop from nothing: a place, a cast, two lines of
your own, played, someone moved on the stage, and a video of it. No file to type by hand.

What you need: the Workshop, its first launch done (your own `.nds` read once into `data/`, see
[the first launch](TUTORIAL.md#1-the-first-launch-5-minutes-once)). To make a whole game of your
films (chapters, villages, dungeons), [Your first game in 30 minutes](TUTORIAL.md) comes after this.

---

## 1. New Film (1 minute)

The Workshop opens on the **Home**, with the welcome panel. Click **+ New Film** (in the panel, in
the Home's bar, or File › New Film…). A window of four steps opens.

1. **Place**: where it happens. Your own decors come first, then the places films use most (the
   beach, the town, the forest…), then every place of the game, by kind. Type in the search
   (`beach`, `cave`) and click a picture; a double-click goes on to the next step.
2. **Cast**: who is in it, two to six. Search by name or Pokédex number and click their portraits;
   the number on each says their order, and the first two lead. A click on a name in the cast
   takes them out. (Pokémon imported from SpriteCollab are in the list too; one not imported yet
   shows under the grid when searched, and a click opens SpriteCollab on it to import it.)
3. **Start**: how it begins. *A Conversation*, *An Arrival* (the last one walks in, late),
   *A Chase*, *Two Moments* (a morning, then the evening), or an *Empty Stage*. On the right, the
   lines it will write, with your cast.
4. **Title**: a title is offered; change it if you like. Click **Create the Film**.

**You see** the Film space: the Script on the left, the Scene in the middle, the Assets on the right, the film playing.
Everyone stands on the place's ground, turned to one another, framed, with the place's music.
The film's first lines say so:

```
scene: beach pikachu piplup
music hidden_highland
pikachu put 378 205 facing right
piplup put 422 211 facing left

moment: hello
frame pikachu piplup
pikachu (happy): Hey, Piplup! There you are.
```

## 2. Two lines of your own (2 minutes)

Click at the end of the last line of the Script and press Enter. Type:

```
piplup (joyous): Look! The sea is shining!
pikachu hop happy
```

While you type, the Script offers what may come next (Ctrl+Space shows it at any time): the names
of the scene, the faces (`happy`, `joyous`, `surprised`, `worried`…), the words that follow
`pikachu ` (what Pikachu can do). A word in red is unknown; the Problems under the Script say why,
often with a fix (Ctrl+.). F1 on a word opens its page in the Wiki, with an example that plays.

## 3. Play it (30 seconds)

**Ctrl+Enter** plays from the cursor's line; **Space** (in the Scene) plays or pauses; the
Timeline below shows each line in time, and a click there goes to that moment. Your two lines
play as soon as they are written.

## 4. Move someone on the stage, add someone (1 minute)

Put the cursor on a `put` line at the top: the Scene shows the stage as it stands there (no fade,
no title over it: *Staging*). Drag Piplup somewhere else: their `put` line is written again with
the new place. Click on someone and the dots around their feet turn them (`facing`). In
**Whole Place**, the camera's frame is a rectangle you can drag, and a corner of it zooms.
Ctrl+Z undoes each change.

**Someone new, a face, a song, without typing.** The **Assets** on the right hold what a film is
made with. Drag a Pokémon from its **Pokémon** tab onto the Scene: it joins the scene's cast and is
put there, facing the nearest (`eevee put 400 210 facing piplup`). Click Piplup on the Scene, then
the **Faces** tab: drag *surprised* onto Piplup's line in the Script, and it says it surprised; the
**Gestures** tab plays each of Piplup's gestures as the pointer rests on it: drag one onto Piplup.
In **Music**, rest the pointer on a song to hear it, and drag it onto a line (`music …`). A
double-click on any of them writes it at the Script's cursor. (View › Add Panel opens or closes the
panels; View › Default Layout puts them back.)

## 5. A video of it (30 seconds)

The film is saved as you type, in `library/films/<its title>.film`. To make a video of it, with
its sound, in a terminal in the Workshop's folder, by its file's name (ffmpeg needed):

```
scene_player video the_big_day
```

It writes `target/films/the_big_day/the_big_day.mp4`.

---

**What next?** All the words, family by family: [film/FILMS.md](../film/FILMS.md) and the Wiki in
the Script. A place of your own: **+ New Decor**, from one of the game's places (each house and tree
an element to move) or an empty page ([decor/DECORS.md](../decor/DECORS.md)). A game of your films:
the **Game** space, then [Your first game in 30 minutes](TUTORIAL.md).
