use std::borrow::BorrowMut;
use std::fmt::Formatter;
use std::thread::current;
use serde::de::{SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};
use typed_arena::Arena;

use crate::wordlist::trie::Trie;
use crate::wordlist::trie::mutablenode::{MutableTrieNode, TrieNodeRef};

pub struct TrieBuilder {
    pub(crate) root: TrieNodeRef,
    arena: Arena<MutableTrieNode>
}


impl TrieBuilder {
    pub fn add(&mut self, word: &str) -> &mut TrieBuilder {
        self.add_with_freq(word, 1)
    }
    pub fn add_with_freq(&mut self, word: &str, freq: usize) -> &mut TrieBuilder{
        let mut current = self.root.clone();
        for c in word.chars() {
            current = current.borrow_mut().get_or_create_child(c)
        }
        current.borrow_mut().set_is_terminal(true);
        current.borrow_mut().inc_freq(freq);
        //self.root.insert(word);
        self
    }
    pub fn add_all<'f, I>(&mut self, items: I) -> &mut TrieBuilder
        where I: IntoIterator<Item=&'f str> {
        items.into_iter().for_each(|x| { self.add(x); } );
        self
    }
    pub fn new() -> TrieBuilder {
        TrieBuilder {
            root: Default::default(),
            arena: Arena::new()
        }
    }

    pub fn build(&mut self) -> Trie {
        Trie::from(self)
    }


    pub(crate) fn decorate(&mut self) {
        self.root.decorate();
    }


}

impl<'de> Deserialize<'de> for TrieBuilder {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: Deserializer<'de> {
        deserializer.deserialize_seq(DeserializeTrieVisitor {})
    }
}


struct DeserializeTrieVisitor {}

impl<'de> Visitor<'de> for DeserializeTrieVisitor {
    type Value = TrieBuilder;

    fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
        write!(formatter, "a TrieNode")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error> where A: SeqAccess<'de> {
        let trie = TrieBuilder::new();
        let mut stack: Vec<TrieNodeRef> = vec![trie.root.clone()];

        seq.next_element::<TrieNodeRef>(); //discard root; we already have it

        while let Some(thing) = seq.next_element()? {
            {
                //println!("{:?}, {:?}", thing, stack);
            }

            let node: TrieNodeRef = thing;
            {
                while node.depth() < stack.len() {
                    stack.pop();
                }
            }
            let _letter = node.letter();

            let mut parent = stack.pop().unwrap();
            parent.set_child(node.clone());
            stack.push(parent);
            stack.push(node);
        }
        Ok(trie)
    }
}

