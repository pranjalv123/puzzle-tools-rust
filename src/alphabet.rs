
pub const ALPHABET : &[u8] = "ABCDEFGHIJKLMNOPQRSTUVWXYZ ".as_bytes();
pub fn get_idx(a: char) -> usize {
    if a == ' ' {
        return 26
    }
    (a.to_ascii_uppercase() as u8 - 'A' as u8) as usize
}
