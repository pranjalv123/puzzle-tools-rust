use std::cell::{Cell, RefCell};
use std::fmt::{Debug, Formatter};
use std::ops::Deref;
use std::mem::replace;
use std::rc::Rc;
use delegate::delegate;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use typed_arena::Arena;
use crate::alphabet::{ALPHABET, get_idx};
use crate::wordlist::trie::haschildren::HasChildren;
use crate::wordlist::trie::trienode::TrieNode;
//
// #[derive(Ord, PartialOrd, Eq, PartialEq, Default, Debug)]
// pub struct TrieNodeRef<'a>(pub(crate) Cell<&'a MutableTrieNode>);


#[derive(Ord, PartialOrd, Eq, PartialEq, Serialize, Deserialize, Default)]
pub(crate) struct MutableTrieNode<'a> {
    #[serde(skip)]
    pub(crate) children: [Option<&'a Cell<TrieNodeRef>>; ALPHABET.len()],
    pub(crate) letter: char,
    pub(crate) is_terminal: bool,
    pub(crate) weight: usize,
    pub(crate) depth: usize,
    pub(crate) freq: usize,
    pub(crate) path: String,
}
//
// impl Clone for TrieNodeRef {
//     fn clone(&self) -> Self {
//         TrieNodeRef(self.0.clone())
//     }
// }
//
// impl Serialize for TrieNodeRef {
//     fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
//         self.0.deref().borrow().serialize(serializer)
//     }
// }
//
// impl<'de> Deserialize<'de> for TrieNodeRef {
//     fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: Deserializer<'de> {
//         Ok(TrieNodeRef::new(MutableTrieNode::deserialize(deserializer).unwrap()))
//     }
// }
//
//
//
// impl TrieNodeRef {
//     delegate! {
//
//         to self.0.deref().borrow_mut() {
//             pub fn create_child(&mut self, c: char);
//             pub fn set_child(&mut self, other: TrieNodeRef);
//             pub fn get_or_create_child(&mut self, c: char) -> TrieNodeRef;
//             pub fn insert(&mut self, word: &str);
//         }
//     }

impl MutableTrieNode {

    pub(crate) fn map_child<'a, T, F>(&'a self, f: &'a mut F) -> Vec<T>
        where F: FnMut(TrieNodeRef) -> T {
        self.0.deref().borrow().children.iter().filter(|x| !x.is_none())
            .map(|x| f(x.as_ref().unwrap().clone())).collect()
    }

    pub(crate) fn letter(&self) -> char {
        self.0.deref().borrow().letter
    }
    pub(crate) fn is_terminal(&self) -> bool {
        self.0.deref().borrow().is_terminal
    }
    pub(crate) fn set_is_terminal(&self, b: bool) {
        self.0.deref().borrow_mut().is_terminal = b;
    }
    pub(crate) fn weight(&self) -> usize {
        self.0.deref().borrow().weight
    }
    pub(crate) fn depth(&self) -> usize {
        self.0.deref().borrow().depth
    }
    pub(crate) fn freq(&self) -> usize {
        self.0.deref().borrow().freq
    }
    pub(crate) fn inc_freq(&self, v: usize) {
        self.0.deref().borrow_mut().freq += v;
    }

    pub(crate) fn path(&self) -> String {
        self.0.deref().borrow().path.to_string()
    }

    pub(crate) fn set_weight(&self, weight: usize) {
        self.0.deref().borrow_mut().weight = weight;
    }

    fn new(node: MutableTrieNode) -> TrieNodeRef {
        TrieNodeRef(Rc::new(RefCell::new(node)), None)
    }

    fn set_immutable(&mut self, node: TrieNode) {
        self.1 = Some(node);
    }
    fn take_immutable(&mut self) -> Option<TrieNode> {
        replace(&mut self.1, None)
    }

    pub(crate) fn decorate(&mut self) {
        self.map_child(
            &mut |mut x| x.decorate()
        );
        self.set_weight(
            self.map_child(&mut |x| x.weight())
                .iter()
                .fold(self.freq(), |x, y| x + y));
    }
}

impl Debug for MutableTrieNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MutableTrieNode")
            .field("path", &self.path)
            .field("letter", &self.letter)
            .field("weight", &self.weight)
            .field("freq", &self.freq)
            .field("is_terminal", &self.is_terminal)
            .field("children", &self.children.iter()
                .filter(|x| x.is_some())
                .map(|x| x.as_ref().unwrap().letter())
                .collect::<Vec<_>>(),
            )
            .finish()
    }
}

impl<'a> MutableTrieNode {
    fn get_child(&self, c: char) -> Option<TrieNodeRef> {
        self.children[get_idx(c)].as_ref().map(|x| x.clone()).or(None)
    }

    fn get_child_mut(&mut self, c: char) -> Option<TrieNodeRef> {
        self.children[get_idx(c)].as_ref().map(|x| x.clone()).or(None)
    }
    fn create_child(&mut self, c: char, arena: &Arena<MutableTrieNode>) {
        let mut path = self.path.clone();
        path.push(c);
        self.children[get_idx(c)] =

            Some(TrieNodeRef::new(
                arena::alloc(MutableTrieNode {
                    weight: 0,
                    children: Default::default(),
                    letter: c,
                    is_terminal: false,
                    freq: 0,
                    depth: self.depth + 1,
                    path,
                })));
    }

    fn set_child(&mut self, other: TrieNodeRef) {
        let c = other.letter();
        self.children[get_idx(c)] = Some(other);
    }

    fn get_or_create_child(&mut self, c: char, arena: &Arena<MutableTrieNode>) -> TrieNodeRef {
        if self.get_child(c).is_none() {
            self.create_child(c, arena);
        }
        return self.get_child_mut(c).unwrap();
    }


    fn insert(&mut self, word: &str) {
        match word.chars().nth(0) {
            None => {
                self.is_terminal = true;
                self.freq += 1;
            }
            Some(c) =>
                self.get_or_create_child(c).insert(&word[1..])
        }
    }
}


