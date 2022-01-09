use std::borrow::BorrowMut;
use std::cell::Cell;
use std::fmt::Formatter;
use std::thread::current;
use serde::de::{SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};
use typed_arena::Arena;

use crate::wordlist::trie::Trie;
use crate::wordlist::trie::mutablenode::{MutableTrieNode};

pub struct TrieBuilder<'a> {
    pub(crate) root: MutableTrieNode<'a>,
    arena: Arena<MutableTrieNode<'a>>,
    path_arena: Arena<String>,
}


impl TrieBuilder<'_> {
    pub(crate) fn new() -> Self {
        TrieBuilder {
            root: Default::default(),
            arena: Arena::new(),
            path_arena: Arena::new(),
        }
    }
}

impl<'a> TrieBuilder<'a> {
    pub fn add(&'a self, word: &str) -> &'a TrieBuilder<'a> {
        self.add_with_freq(word, 1)
    }

    pub fn add_all<'f, I>(&'a mut self, items: I) -> &'a mut TrieBuilder<'a>
        where I: IntoIterator<Item=&'f str> {
        items.into_iter().for_each(|x| ()); //&(self.add));
        self
    }

    pub fn add_with_freq<'f>(&'a self, word: &'f str, freq: usize) -> &'a TrieBuilder<'a> {
        let mut current = &self.root;
        {
            let arena = &self.arena;
            let path_arena = &self.path_arena;
            for c in word.chars() {
                current = current
                    .get_or_create_child(c, arena, &self.path_arena)
                    .get()
                    .unwrap()
            }
        }

        let end = current;
        end.is_terminal.set(true);
        end.freq.set(end.freq.get() + freq);
        self
    }
    pub fn build<'x>(&'a self) -> Trie<'x> {
        Trie::from(self)
    }


    pub(crate) fn decorate(&'a self) {
        self.root.decorate();
    }
}
//
// impl<'de> Deserialize<'de> for TrieBuilder {
//     fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: Deserializer<'de> {
//         deserializer.deserialize_seq(DeserializeTrieVisitor {})
//     }
// }


// struct DeserializeTrieVisitor {}
//
// impl<'de> Visitor<'de> for DeserializeTrieVisitor {
//     type Value = TrieBuilder;
//
//     fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
//         write!(formatter, "a TrieNode")
//     }
//
//     fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error> where A: SeqAccess<'de> {
//         let trie = TrieBuilder::new();
//         let mut stack: Vec<TrieNodeRef> = vec![trie.root.clone()];
//
//         seq.next_element::<TrieNodeRef>(); //discard root; we already have it
//
//         while let Some(thing) = seq.next_element()? {
//             {
//                 //println!("{:?}, {:?}", thing, stack);
//             }
//
//             let node: TrieNodeRef = thing;
//             {
//                 while node.depth() < stack.len() {
//                     stack.pop();
//                 }
//             }
//             let _letter = node.letter();
//
//             let mut parent = stack.pop().unwrap();
//             parent.set_child(node.clone());
//             stack.push(parent);
//             stack.push(node);
//         }
//         Ok(trie)
//     }
// }

