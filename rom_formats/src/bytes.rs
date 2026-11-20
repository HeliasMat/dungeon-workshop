//! Little-endian numbers in the ROM's files, read at an offset: where one may
//! be missing (`u16_at`, `u32_at`), where a file cut short is an error
//! (`u16_le`, `u32_le`), and four bytes saying where something is or how
//! long it is (`offset_at`).

use crate::nds::RomError;

/// Two bytes there; none past the end.
pub(crate) fn u16_at(data: &[u8], off: usize) -> Option<u16> {
    Some(u16::from_le_bytes(data.get(off..off + 2)?.try_into().ok()?))
}

/// Four bytes there; none past the end.
pub(crate) fn u32_at(data: &[u8], off: usize) -> Option<u32> {
    Some(u32::from_le_bytes(data.get(off..off + 4)?.try_into().ok()?))
}

/// Two bytes there; the file cut short, an error.
pub(crate) fn u16_le(data: &[u8], off: usize) -> Result<u16, RomError> {
    u16_at(data, off).ok_or(RomError::Truncated)
}

/// Four bytes there; the file cut short, an error.
pub(crate) fn u32_le(data: &[u8], off: usize) -> Result<u32, RomError> {
    u32_at(data, off).ok_or(RomError::Truncated)
}

/// Four bytes there, an offset or a size; the file cut short, an error.
pub(crate) fn offset_at(data: &[u8], off: usize) -> Result<usize, RomError> {
    u32_le(data, off).map(|value| value as usize)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_are_read_little_end_first_and_none_past_the_end() {
        let data = [0x34, 0x12, 0x78, 0x56];
        assert_eq!((u16_at(&data, 0), u32_at(&data, 0)), (Some(0x1234), Some(0x5678_1234)));
        assert_eq!((u16_at(&data, 3), u32_at(&data, 1)), (None, None));
        assert!(matches!(offset_at(&data, 2), Err(RomError::Truncated)));
        assert_eq!(offset_at(&data, 0).ok(), Some(0x5678_1234));
    }
}
