
use std::cell::Cell;
use std::fmt::{Debug, Formatter};



use typed_arena::Arena;

use crate::wordlist::trie::node::{ImmutableTrieNode, TrieNode};

pub struct Trie<'a> {
    pub(crate) root: TrieNode<'a>,
    arena: Arena<TrieNode<'a>>,
    //path_arena: Arena<String>,
    pub built: Cell<bool>,
}
pub struct ImmutableTrie<'a> {
    pub(crate) root: Cell<Option<&'a ImmutableTrieNode<'a>>>,
    arena: Arena<ImmutableTrieNode<'a>>
}

impl<'a> ImmutableTrie<'a> {
    pub fn new() -> ImmutableTrie<'a> {
        ImmutableTrie {
            root: Cell::new(None),
            arena: Arena::new()
        }
    }
}

impl Trie<'_> {
    fn make_immutable<'a>(&self, immutable: &'a ImmutableTrie<'a>) {
        immutable.arena.reserve_extend(self.arena.len());
        let root = self.root.make_immutable(&immutable.arena);
        immutable.root.set(Some(root));
    }
}

impl Trie<'_> {
    pub(crate) fn new() -> Self {
        Trie {
            root: Default::default(),
            built: Cell::new(false),
            arena: Arena::new(),
            //path_arena: Arena::new(),
        }
    }
}


impl<'a> Trie<'a> {
    pub fn add<'f>(&'a self, word: &str) {
        self.add_with_freq(word, 1)
    }

    pub fn add_all<'f, I>(&'a self, items: I)
        where I: IntoIterator<Item=&'f str> {
        items.into_iter().for_each(|x| { self.add(x); });
    }

    pub fn add_with_freq<'f>(&'a self, word: &'f str, freq: usize) {
        assert!(!self.built.get());
        let mut current = &self.root;
        {
            for c in word.chars() {
                current = current
                    .get_or_create_child(c, &self.arena)//, &self.path_arena)
                    .get()
                    .unwrap()
            }
        }

        let end = current;
        end.is_terminal.set(true);
        end.freq.set(end.freq.get() + freq);
    }
    pub fn build<'f>(&self, immutable: &'f ImmutableTrie<'f>){
        self.built.set(true);
        println!("Decorating...");
        self.decorate();
        println!("Converting...");
        self.make_immutable(immutable);
    }

    fn decorate(&self) {
        self.root.decorate();
    }
}


impl<'a> Debug for Trie<'a> {
    fn fmt<'f>(&'f self, f: &'f mut Formatter<'_>) -> std::fmt::Result {
        let mut l = f.debug_list();
        let mut stack = vec![&self.root];
        while !stack.is_empty() {
            let x = stack.pop().unwrap();
            l.entry(&x);
            x.children.iter().for_each(|x|
                                           x.get().map(|c|
                                                           stack.push(c)
                                           ).unwrap_or(()));
        }
        // self.root.traverse_prefix(|x| { ml.entry(x); });
        l.finish()
    }
}


#[cfg(test)]
mod tests {
    use crate::wordlist::index::Index;
    use crate::wordlist::trie::trie::{ImmutableTrie, Trie};

    #[test]
    fn finds_words_in_trie() {
        let words = vec!["HELLO", "HELP", "GOODBYE", "GOOD"];
        let trie = Trie::new();
        let items = (&words).iter().map(|x| *x);
        trie.add_all(items);
        let immut = ImmutableTrie::new();
        trie.build(&immut);
        (&words).iter().for_each(|word| assert!(trie.contains(&word)));
    }

    #[test]
    fn doesnt_finds_words_not_in_trie() {
        let words = vec!["HELLO", "HELP", "GOODBYE", "GOOD"];
        let bad_words = vec!["HE", "H", "LOL", "BANANA"];
        let trie = Trie::new();
        trie.add_all((&words).iter().map(|x| *x));
        let immut = ImmutableTrie::new();
        trie.build(&immut);
        (&bad_words).iter().for_each(|word| assert!(!trie.contains(&word)));
    }


    #[test]
    fn query_words_in_trie() {
        let words = vec!["HELLO", "HELP", "GOODBYE", "GOOD"];
        let trie = Trie::new();
        trie.add_all((&words).iter().map(|x| *x));
        let immut = ImmutableTrie::new();
        trie.build(&immut);

        let mut result = trie.query_regex("H.L*(O|P)");
        result.sort();

        assert_eq!(result, vec!["HELLO", "HELP"])
    }

    // #[test]
    // fn test_serialize_deserialize() {
    //     let words = vec!["HELLO", "HELP", "GOODBYE", "GOOD"];
    //     let trie = Trie::builder().add_all(words.clone()).build();
    //     let serialized = serde_json::to_string(&trie).unwrap();
    //     let new_trie = serde_json::from_str::<Trie>(&serialized).unwrap();
    //
    //     (&words).iter().for_each(|word| assert!(new_trie.contains(&word)));
    // }

    #[test]
    fn test_anagram() {
        let words = vec!["HELLO", "HELP", "GOODBYE", "GOOD"];
        let trie = Trie::new();
        trie.add_all((&words).iter().map(|x| *x));
        let immut = ImmutableTrie::new();
        trie.build(&immut);

        assert_eq!(trie.query_anagram("OLEHL"), vec!["HELLO"]);
        assert!(trie.query_anagram("LEHL").is_empty());
        assert!(trie.query_anagram("LELO").is_empty());
        assert!(trie.query_anagram("DOG").is_empty());
        assert_eq!(trie.query_anagram("OOGD"), vec!["GOOD"]);
    }

}


