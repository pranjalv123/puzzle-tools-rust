use std::borrow::Borrow;
use std::cell::{RefCell};
use std::fmt::{Debug, Formatter};
use std::ops::Deref;
use std::rc::Rc;
use delegate::delegate;
use serde::{Serialize, Deserialize, Serializer, Deserializer};
use crate::alphabet::{ALPHABET, get_idx};

#[derive(Ord, PartialOrd, Eq, PartialEq, Default, Clone, Debug)]
pub struct TrieNodeRef(Rc<RefCell<TrieNode>>);

impl Serialize for TrieNodeRef {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        self.0.deref().borrow().serialize(serializer)
    }
}
impl<'de> Deserialize<'de> for TrieNodeRef {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: Deserializer<'de> {
        Ok(TrieNodeRef::new(TrieNode::deserialize(deserializer).unwrap()))
    }
}

#[derive(Ord, PartialOrd, Eq, PartialEq, Serialize, Deserialize, Default)]
struct TrieNode {
    #[serde(skip)]
    children: [Option<TrieNodeRef>; ALPHABET.len()],
    letter: char,
    is_terminal: bool,
    weight: usize,
    depth: usize,
    freq: usize,
    path: String,
}


impl TrieNodeRef {
    delegate! {
        to self.0.deref().borrow() {
            pub fn get_child(&self, c: char) -> Option<TrieNodeRef>;
        }

        to self.0.deref().borrow_mut() {
            pub fn create_child(&mut self, c: char);
            pub fn set_child(&mut self, other: TrieNodeRef);
            pub fn get_or_create_child(&mut self, c: char) -> TrieNodeRef;
            pub fn insert(&mut self, word: &str);

        }
    }


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
    pub(crate) fn weight(&self) -> usize {
        self.0.deref().borrow().weight
    }
    pub(crate) fn depth(&self) -> usize {
        self.0.deref().borrow().depth
    }
    pub(crate) fn freq(&self) -> usize {
        self.0.deref().borrow().freq
    }

    pub(crate) fn path(&self) -> String {
        self.0.deref().borrow().path.to_string()
    }

    pub(crate) fn set_weight(&self, weight: usize) {
        self.0.deref().borrow_mut().weight = weight;
    }

    fn new(node: TrieNode) -> TrieNodeRef {
        TrieNodeRef(Rc::new(RefCell::new(node)))
    }


    pub(crate) fn traverse_prefix<T, F>(&self, f: &mut F)
        where F: FnMut(TrieNodeRef) -> T {
        f(self.clone());
        self.0.deref().borrow().children.iter()
            .for_each(|node| {
                node.as_ref().map(|x| x.traverse_prefix(f))
                    .unwrap_or(())
            });
    }

    pub(crate) fn traverse_postfix<T, F>(&self, f: &mut F)
        where F: FnMut(TrieNodeRef) -> T {
        self.0.deref().borrow().children.iter()
            .for_each(|node| {
                node.as_ref().map(|x| x.traverse_postfix(f))
                    .unwrap_or(())
            });
        f(self.clone());
    }

    pub(crate) fn order<T>(&self, f: fn(TrieNodeRef) -> T) -> OrderedTrieNode<T>
        where T: Ord, T: Debug {
        OrderedTrieNode { val: f(self.clone()), node: self.clone() }
    }
}

impl Debug for TrieNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TrieNode")
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

impl<'a> TrieNode {
    fn get_child(&self, c: char) -> Option<TrieNodeRef> {
        self.children[get_idx(c)].as_ref().map(|x|x.clone()).or(None)
    }

    fn get_child_mut(&mut self, c: char) -> Option<TrieNodeRef> {
        self.children[get_idx(c)].as_mut().map(|x|x.clone()).or(None)
    }
    fn create_child(&mut self, c: char) {
        let mut path = self.path.clone();
        path.push(c);
        self.children[get_idx(c)] =
            Some(TrieNodeRef::new(
                TrieNode {
                    weight: 0,
                    children: Default::default(),
                    letter: c,
                    is_terminal: false,
                    freq: 0,
                    depth: self.depth + 1,
                    path,
                }));
    }

    fn set_child(&mut self, other: TrieNodeRef) {
        let c = other.letter();
        self.children[get_idx(c)] = Some(other);
    }

    fn get_or_create_child(&mut self, c: char) -> TrieNodeRef {
        if self.get_child(c).is_none() {
            self.create_child(c);
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

    //
    // fn traverse_prefix_mut<T, F>(&mut self, f: &mut F)
    //     where F: FnMut(TrieNodeRef) -> T {
    //     f(self);
    //     self.children.iter_mut()
    //         .for_each(|node| {
    //             node.as_mut().map(|x| x.traverse_prefix_mut(f))
    //                 .unwrap_or(())
    //         });
    // }
    // fn traverse_prefix<T, F>(&self, f: &mut F)
    //     where F: FnMut(TrieNodeRef) -> T {
    //     f(self);
    //     self.children.iter()
    //         .for_each(|node| {
    //             node.as_ref().map(|x| x.traverse_prefix(f))
    //                 .unwrap_or(())
    //         });
    // }
    //
    // fn traverse_postfix_mut<T, F>(&mut self, f: &mut F)
    //     where F: FnMut(&mut TrieNode) -> T {
    //     self.children.iter_mut()
    //         .for_each(|node| {
    //             node.as_mut().map(|x| x.traverse_postfix_mut(f))
    //                 .unwrap_or(())
    //         });
    //     f(self);
    // }
    // fn traverse_postfix<T, F>(&self, f: &mut F)
    //     where F: FnMut(&TrieNode) -> T {
    //     self.children.iter()
    //         .for_each(|node| {
    //             node.as_ref().map(|x| x.traverse_postfix(f))
    //                 .unwrap_or(())
    //         });
    //     f(self);
    // }

}


#[derive(PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct OrderedTrieNode<T>
    where T: Ord, T: Debug {
    val: T,
    pub(crate) node: TrieNodeRef,
}

impl<'a, T> Deref for OrderedTrieNode<T>
    where T: Ord, T: Debug {
    type Target = TrieNodeRef;

    fn deref(&self) -> &Self::Target {
        &self.node
    }
}

impl IntoIterator for TrieNodeRef {
    type Item = TrieNodeRef;
    type IntoIter = TrieCursor;

    fn into_iter(self) -> Self::IntoIter {
        TrieCursor { idx: -1, node: self.clone() }
    }
}

pub struct TrieCursor {
    idx: isize,
    node: TrieNodeRef,
}

impl Iterator for TrieCursor {
    type Item = TrieNodeRef;


    fn next(&mut self) -> Option<Self::Item> {
        self.idx += 1;
        while self.node.0.deref().borrow().children[self.idx as usize].is_none() {
            self.idx += 1;
            if self.idx as usize >= self.node.0.deref().borrow().children.len() {
                return None;
            }
        }
        self.node.0.deref().borrow().children[self.idx as usize].clone()
    }
}
