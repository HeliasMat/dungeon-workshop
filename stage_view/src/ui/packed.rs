//! The interface kept in a file: `interface.bin`.

use super::*;

impl UiArt {
    /// The interface of the game's folder.
    pub fn load(folder: &str) -> Option<Self> {
        Self::unpack(&shell::data::read(folder, "interface.bin")?)
    }

    /// The interface's pictures, font and texts as a file of the
    /// project's own, and back.
    pub fn pack(&self) -> Vec<u8> {
        let mut packer = media::pack::Packer::default();
        self.font.pack(&mut packer);
        let pictures = [
            &self.frame,
            &self.frame_green,
            &self.frame_pink,
            &self.title_logo,
            &self.menu_art,
            &self.menu_backdrop,
            &self.title_art,
            &self.backdrop,
            &self.hud,
        ];
        pictures.iter().for_each(|picture| packer.bytes(picture));
        packer.number(self.marks.len());
        for (wide, high, pixels) in self.marks.iter().chain([&self.cursor, &self.more]) {
            packer.number(*wide);
            packer.number(*high);
            packer.bytes(pixels);
        }
        packer.bytes(&self.text_colors.concat());
        let texts = &self.strings;
        packer.texts(&texts.menu);
        packer.texts(&texts.weather);
        for text in [
            &texts.bag_title,
            &texts.moves_title,
            &texts.controls,
            &texts.moves_hint,
            &texts.nothing_here,
            &texts.leader,
            &texts.team_name,
            &texts.menu_controls,
            &texts.town_controls,
        ] {
            packer.text(text);
        }
        packer.texts(&texts.log);
        packer.texts(&texts.hints);
        packer.number(self.frame_types.len());
        self.frame_types.iter().for_each(|picture| packer.bytes(picture));
        packer.number(self.title_sprites.len());
        for (wide, high, pixels) in &self.title_sprites {
            packer.number(*wide);
            packer.number(*high);
            packer.bytes(pixels);
        }
        packer.0
    }

    pub fn unpack(data: &[u8]) -> Option<Self> {
        let mut unpacker = media::pack::Unpacker(data);
        let font = Font::unpack(&mut unpacker)?;
        let mut pictures = Vec::new();
        for _ in 0..9 {
            pictures.push(unpacker.bytes()?);
        }
        let mut pictures = pictures.into_iter();
        let mut picture = move || pictures.next().unwrap_or_default();
        let (frame, frame_green, frame_pink, title_logo) = (picture(), picture(), picture(), picture());
        let (menu_art, menu_backdrop, title_art, backdrop, hud) = (picture(), picture(), picture(), picture(), picture());
        let count = unpacker.number()?;
        let mut small = Vec::new();
        for _ in 0..count + 2 {
            small.push((unpacker.number()?, unpacker.number()?, unpacker.bytes()?));
        }
        let (more, cursor) = (small.pop()?, small.pop()?);
        let text_colors = unpacker.bytes()?.as_chunks::<3>().0.iter().map(|color| [color[0], color[1], color[2]]).collect();
        let (menu, weather) = (unpacker.texts()?, unpacker.texts()?);
        let strings = UiStrings {
            menu,
            weather,
            bag_title: unpacker.text()?,
            moves_title: unpacker.text()?,
            controls: unpacker.text()?,
            moves_hint: unpacker.text()?,
            nothing_here: unpacker.text()?,
            leader: unpacker.text()?,
            team_name: unpacker.text()?,
            menu_controls: unpacker.text()?,
            town_controls: unpacker.text()?,
            // (Older files have none.)
            log: unpacker.texts().unwrap_or_default(),
            hints: unpacker.texts().unwrap_or_default(),
        };
        // (Older files have none.)
        let frame_types = unpacker.number().map_or(Vec::new(), |count| (0..count).map_while(|_| unpacker.bytes()).collect());
        let title_sprites =
            unpacker.number().map_or(Vec::new(), |count| (0..count).map_while(|_| Some((unpacker.number()?, unpacker.number()?, unpacker.bytes()?))).collect());
        Some(Self {
            font,
            frame,
            frame_green,
            frame_pink,
            frame_types,
            title_logo,
            menu_art,
            menu_backdrop,
            title_art,
            title_sprites,
            marks: small,
            backdrop,
            hud,
            text_colors,
            cursor,
            more,
            strings: Arc::new(strings),
        })
    }
}
