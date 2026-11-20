//! The project's own files: numbers, bytes and texts written one after the
//! other, read back in the same order. What the game needs is written
//! there once, from the ROM (`extractor`); the game then plays from those
//! files alone.

/// Writes.
#[derive(Default)]
pub struct Packer(pub Vec<u8>);

impl Packer {
    pub fn number(&mut self, value: usize) {
        self.0.extend((value as u32).to_le_bytes());
    }

    pub fn bytes(&mut self, bytes: &[u8]) {
        self.number(bytes.len());
        self.0.extend(bytes);
    }

    pub fn text(&mut self, text: &str) {
        self.bytes(text.as_bytes());
    }

    pub fn texts(&mut self, texts: &[String]) {
        self.number(texts.len());
        texts.iter().for_each(|text| self.text(text));
    }
}

/// Reads; `None` once the file gives out.
pub struct Unpacker<'a>(pub &'a [u8]);

impl Unpacker<'_> {
    pub fn number(&mut self) -> Option<usize> {
        let (value, rest) = self.0.split_at_checked(4)?;
        self.0 = rest;
        Some(u32::from_le_bytes(value.try_into().ok()?) as usize)
    }

    pub fn bytes(&mut self) -> Option<Vec<u8>> {
        let length = self.number()?;
        let (bytes, rest) = self.0.split_at_checked(length)?;
        self.0 = rest;
        Some(bytes.to_vec())
    }

    pub fn text(&mut self) -> Option<String> {
        String::from_utf8(self.bytes()?).ok()
    }

    pub fn texts(&mut self) -> Option<Vec<String>> {
        (0..self.number()?).map(|_| self.text()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn what_is_written_is_read_back() {
        let mut packer = Packer::default();
        packer.number(70_000);
        packer.bytes(&[1, 2, 3]);
        packer.texts(&["Pokémon".to_string(), String::new()]);
        let mut unpacker = Unpacker(&packer.0);
        assert_eq!(unpacker.number(), Some(70_000));
        assert_eq!(unpacker.bytes(), Some(vec![1, 2, 3]));
        assert_eq!(unpacker.texts(), Some(vec!["Pokémon".to_string(), String::new()]));
        assert_eq!(unpacker.number(), None);
    }
}
