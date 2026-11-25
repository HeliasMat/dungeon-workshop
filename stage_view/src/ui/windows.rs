//! The game's windows and frames.

use super::*;

impl Canvas {
    /// A window whose inside starts at tile `(x, y)` of the canvas and is
    /// `width` by `height` tiles: the game's `window_params` (add
    /// [`BOTTOM`] to `y` for the touch screen). The frame goes around it.
    pub fn window(&mut self, art: &UiArt, x: usize, y: usize, width: usize, height: usize) {
        // Over the game the grey is half seen through.
        let frame = self.frame_art(art);
        self.window_with(frame, y < BOTTOM, 0, x, y, width, height);
    }

    /// The frame texture in the colour windows are drawn with.
    fn frame_art<'a>(&self, art: &'a UiArt) -> &'a [u8] {
        let color = match self.frame {
            Frame::Blue => 0,
            Frame::Pink => 1,
            Frame::Green => 2,
        };
        if let Some(picture) = self.frame_type.checked_sub(1).and_then(|kind| art.frame_types.get(kind * 3 + color)).filter(|picture| !picture.is_empty()) {
            return picture;
        }
        match self.frame {
            Frame::Blue => &art.frame,
            Frame::Pink => &art.frame_pink,
            Frame::Green => &art.frame_green,
        }
    }

    /// The window of dialogue outside dungeons: the same frame, plain grey
    /// (measured on the original).
    pub fn dialogue_window(&mut self, art: &UiArt, x: usize, y: usize, width: usize, height: usize) {
        let frame = self.frame_art(art);
        self.window_with(frame, true, 0, x, y, width, height);
    }

    /// The thin frame around a portrait of 5 by 5 tiles at tile `(x, y)`:
    /// the frame texture's second set of tiles.
    pub fn portrait_frame(&mut self, art: &UiArt, x: usize, y: usize) {
        let frame = self.frame_art(art);
        self.window_with(frame, true, THIN_FRAME, x, y, 5, 5);
    }

    /// The arrow under a message waiting for a key, on the bottom of the
    /// window whose inside ends at pixel row `bottom`, centred on `x`.
    pub fn more_arrow(&mut self, art: &UiArt, x: i32, bottom: i32) {
        let (width, height, rgba) = &art.more;
        self.blit(x - *width as i32 / 2, bottom, *width, *height, rgba);
    }

    /// A picture covering one screen (`top` is its first pixel row).
    pub fn picture(&mut self, top: usize, rgba: &[u8]) {
        self.blit(0, top as i32, SCREEN_WIDTH, SCREEN_HEIGHT, rgba);
    }

    /// `set`: the column of the frame texture its nine tiles start at.
    fn window_with(&mut self, frame: &[u8], plain: bool, set: usize, x: usize, y: usize, width: usize, height: usize) {
        self.window_at(frame, plain, set, (x * TILE) as i32, (y * TILE) as i32, width, height);
    }

    /// A small window of the dialogue's look anywhere on the screens (its
    /// inside's top-left pixel, its size in tiles): the portraits' thin
    /// frame, in the frames' colour and type; and that frame's colour, for
    /// what goes with it (a bubble's tail).
    pub(crate) fn small_window(&mut self, art: &UiArt, left: i32, top: i32, width: usize, height: usize) -> [u8; 4] {
        let frame = self.frame_art(art);
        self.window_at(frame, true, THIN_FRAME, left, top, width, height);
        // (The ring's colour: the first pixel seen down the middle of its
        // top tile.)
        (0..TILE)
            .find_map(|dy| pixel_at(frame, (dy * FRAME_WIDTH + (THIN_FRAME + 1) * TILE + TILE / 2) * 4).filter(|color| color[3] != 0))
            .unwrap_or([248, 248, 248, 255])
    }

    fn window_at(&mut self, frame: &[u8], plain: bool, set: usize, left: i32, top: i32, width: usize, height: usize) {
        let (inner_w, inner_h) = (width * TILE, height * TILE);
        // The frame's ring leaves a pixel of its tiles to the inside.
        let alpha = if plain { 255 } else { 128 };
        let [r, g, b] = WINDOW_GREY;
        self.fill(left - 1, top - 1, inner_w + 2, inner_h + 2, [r, g, b, alpha]);
        let tile = |canvas: &mut Self, tx: usize, ty: usize, at_x: i32, at_y: i32| {
            for dy in 0..TILE {
                for dx in 0..TILE {
                    let at = ((ty * TILE + dy) * FRAME_WIDTH + (set + tx) * TILE + dx) * 4;
                    if let Some(color) = pixel_at(frame, at)
                        && color[3] != 0
                    {
                        canvas.put(at_x + dx as i32, at_y + dy as i32, color);
                    }
                }
            }
        };
        let (right, bottom) = (left + inner_w as i32, top + inner_h as i32);
        for column in 0..width {
            let at_x = left + (column * TILE) as i32;
            tile(self, 1, 0, at_x, top - TILE as i32);
            tile(self, 1, 2, at_x, bottom);
        }
        for row in 0..height {
            let at_y = top + (row * TILE) as i32;
            tile(self, 0, 1, left - TILE as i32, at_y);
            tile(self, 2, 1, right, at_y);
        }
        tile(self, 0, 0, left - TILE as i32, top - TILE as i32);
        tile(self, 2, 0, right, top - TILE as i32);
        tile(self, 0, 2, left - TILE as i32, bottom);
        tile(self, 2, 2, right, bottom);
    }

    /// The line the game draws under a title or between entries: white
    /// over black.
    pub fn separator(&mut self, x: i32, y: i32, width: usize) {
        self.fill(x, y, width, 1, [248, 248, 248, 255]);
        self.fill(x, y + 1, width, 1, [0, 0, 0, 255]);
    }
}
