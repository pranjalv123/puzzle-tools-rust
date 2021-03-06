pub fn morse_spaces(morse: &str) -> String {
    morse.split(" ").map(
        morse_char
    ).collect()
}


pub fn morse_char(morse: &str) -> char {
    match morse {
        "._" => 'A',
        "_..." => 'B',
        "_._." => 'C',
        "_.." => 'D',
        "." => 'E',
        "..-." => 'F',
        "__." => 'G',
        "...." => 'H',
        ".." => 'I',
        ".___" => 'J',
        "_._" => 'K',
        "._.." => 'L',
        "__" => 'M',
        "_." => 'N',
        "___" => 'O',
        ".__." => 'P',
        "__._" => 'Q',
        "._." => 'R',
        "..." => 'S',
        "_" => 'T',
        ".._" => 'U',
        "..._" => 'V',
        ".__" => 'W',
        "_.._" => 'X',
        "_.__" => 'Y',
        "__.." => 'Z',
        _ => '?'
    }
}

const MORSE_CHARS: [&str; 26] = ["._",
    "_...",
    "_._.",
    "_..",
    ".",
    "..-.",
    "__.",
    "....",
    "..",
    ".___",
    "_._",
    "._..",
    "__",
    "_.",
    "___",
    ".__.",
    "__._",
    "._.",
    "...",
    "_",
    ".._",
    "..._",
    ".__",
    "_.._",
    "_.__",
    "__.."];