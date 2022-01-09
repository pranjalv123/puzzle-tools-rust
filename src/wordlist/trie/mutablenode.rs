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
    pub(crate) children: [Cell<Option<&'a MutableTrieNode<'a>>>; ALPHABET.len()],
    pub(crate) letter: char,
    pub(crate) is_terminal: Cell<bool>,
    pub(crate) weight: Cell<usize>,
    pub(crate) depth: usize,
    pub(crate) freq: Cell<usize>,
    pub(crate) path: &'a str,
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

impl<'n> MutableTrieNode<'n> {
    pub(crate) fn map_child<'a, T, F>(&'a self, f: &'a mut F) -> Vec<T>
        where F: FnMut(&MutableTrieNode) -> T {
        self.children.iter()
            .map(|x| {
                let optx = x.get();
                optx.map(|x| f(x))
            })
            .filter(|x| x.is_some())
            .map(|x| x.unwrap())
            .collect()
    }
    //
    // pub(crate) fn foreach_child_mut<'a, F>(&'a self, f: &'a mut F)
    //     where F: for <'r> FnMut(&MutableTrieNode) -> &'r MutableTrieNode<'r> {
    //     self.children.iter()
    //         .for_each(|mut child| {
    //             let opt_child = child.get();
    //             child.set(opt_child.map(|x| f(x)));
    //         })
    // }

    pub(crate) fn decorate(&self) -> &MutableTrieNode<'n> {
        for mut child_cell in &self.children {
            child_cell.set(
                match child_cell.get() {
                    None => None,
                    Some(child) => Some(child.decorate())
                })
        }

        self.weight.set(self.map_child(&mut |x| x.weight.get())
            .iter()
            .fold(self.freq.get(), |x, y| x + y));
        self
    }
}

impl Debug for MutableTrieNode<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MutableTrieNode")
            .field("path", &self.path)
            .field("letter", &self.letter)
            .field("weight", &self.weight)
            .field("freq", &self.freq)
            .field("is_terminal", &self.is_terminal)
            .field("children", &self.children.iter()
                .filter(|x| x.get().is_some())
                .map(|x| x.get().unwrap().letter)
                .collect::<Vec<_>>(),
            )
            .finish()
    }
}

impl<'a> MutableTrieNode<'a> {
    fn get_child(&'a self, c: char) -> Option<&'a MutableTrieNode> {
        self.children[get_idx(c)].get()
    }

    fn create_child(&self, c: char,
                    arena: &'a Arena<MutableTrieNode<'a>>,
                    path_arena: &'a Arena<String>) {
        let mut path = path_arena.alloc(self.path.to_string());
        path.push(c);
        self.children[get_idx(c)].set(
            Some(
                arena.alloc(MutableTrieNode {
                    weight: Cell::new(0),
                    children: Default::default(),
                    letter: c,
                    is_terminal: Cell::new(false),
                    freq: Cell::new(0),
                    depth: self.depth + 1,
                    path,
                })));
    }

    pub(crate) fn get_or_create_child(&'a self, c: char,
                                      arena: &'a Arena<MutableTrieNode<'a>>,
                                      path_arena: &'a Arena<String>)
        -> &Cell<Option<&'a MutableTrieNode<'a>>> {
        if self.get_child(c).is_none() {
            self.create_child(c, arena, path_arena);
        }
        return &self.children[get_idx(c)];
    }
    //
    // fn insert(&mut self, word: &str) {
    //     match word.chars().nth(0) {
    //         None => {
    //             self.is_terminal = true;
    //             self.freq += 1;
    //         }
    //         Some(c) =>
    //             self.get_or_create_child(c).insert(&word[1..])
    //     }
    // }
}


