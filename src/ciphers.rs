mod morse;

pub fn caesar(word: &str, shift: i32) -> String {
    word.as_bytes().iter().map(|c| shift_letter(c,shift)).map(|x| x as char).collect()
}

fn shift_letter(c: &u8, shift: i32) -> u8 {
    (match c {
        97..=122 => ((*c as i32 - 97 + shift) % 26 + 97),
        65..=90 => ((*c as i32 - 65 + shift) % 26 + 65),
        _ => *c as i32
    }) as u8
}
