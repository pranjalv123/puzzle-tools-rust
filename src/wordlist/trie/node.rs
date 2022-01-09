use std::cell::{Cell, RefCell};
use std::fmt::{Debug, Formatter};
use std::ops::Deref;
use std::mem::replace;
use std::rc::Rc;
use delegate::delegate;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use typed_arena::Arena;
use crate::alphabet::{ALPHABET, get_idx};
//
// #[derive(Ord, PartialOrd, Eq, PartialEq, Default, Debug)]
// pub struct TrieNodeRef<'a>(pub(crate) Cell<&'a MutableTrieNode>);


#[derive(Ord, PartialOrd, Eq, PartialEq, Serialize, Deserialize, Default)]
pub(crate) struct TrieNode<'a> {
    #[serde(skip)]
    pub(crate) children: [Cell<Option<&'a TrieNode<'a>>>; ALPHABET.len()],
    next_child: Cell<Option<[Option<usize>; ALPHABET.len()]>>,
    pub(crate) letter: char,
    pub(crate) is_terminal: Cell<bool>,
    pub(crate) weight: Cell<usize>,
    pub(crate) depth: usize,
    pub(crate) freq: Cell<usize>,
    pub(crate) path: String,
}

impl<'n> TrieNode<'n> {
    pub(crate) fn map_child<T, F>(&self, f: &mut F) -> Vec<T>
        where F: FnMut(&TrieNode) -> T {
        self.children.iter()
            .map(|x| {
                let optx = x.get();
                optx.map(|x| f(x))
            })
            .filter(|x| x.is_some())
            .map(|x| x.unwrap())
            .collect()
    }

    pub(crate) fn decorate(&self) -> &TrieNode<'n> {
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

impl Debug for TrieNode<'_> {
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

impl<'a> TrieNode<'a> {

    fn create_child<'f>(&'f self, c: char,
                    arena: &'a Arena<TrieNode<'a>>) {
        let mut path = self.path.clone();
        path.push(c);
        self.children[get_idx(c)].set(
            Some(
                arena.alloc(TrieNode {
                    weight: Cell::new(0),
                    next_child: Default::default(),
                    children: Default::default(),
                    letter: c,
                    is_terminal: Cell::new(false),
                    freq: Cell::new(0),
                    depth: self.depth + 1,
                    path,
                })));
    }

    pub(crate) fn get_or_create_child<'f>(&'f self, c: char,
                                      arena: &'a Arena<TrieNode<'a>>)
                                      //path_arena: &'f Arena<String>)
                                      -> &'f Cell<Option<&'a TrieNode<'a>>> {
        if self.get_child(c).is_none() {
            self.create_child(c, arena);
        }
        return &self.children[get_idx(c)];
    }
}

#[derive(Debug)]
pub(crate) struct TrieCursor<'a> {
    idx: Option<usize>,
    node: &'a TrieNode<'a>,
}

impl<'a> Iterator for TrieCursor<'a> {
    type Item = &'a TrieNode<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut rv = None;
        if let Some(idx) = self.idx {
            rv = self.node.children[idx].get();
            self.idx = self.node.next_child.get().unwrap()[idx];
        }
        rv
    }
}

impl<'a> IntoIterator for &'a TrieNode<'a> {
    type Item = &'a TrieNode<'a>;
    type IntoIter = TrieCursor<'a>;

    fn into_iter(self) -> Self::IntoIter {
        if(self.next_child.get().is_none()) {
            self.build_next_child();
        }

        if self.children[0].get().is_some() {
            TrieCursor { idx: Some(0), node: self }
        } else {
            TrieCursor { idx: self.next_child.get().unwrap()[0], node: self }
        }
    }
}

impl<'a> TrieNode<'a> {
    fn build_next_child(&self) {
        let mut next_child = [None; ALPHABET.len()];
        let mut idx: isize = (next_child.len() - 1) as isize;
        let mut next_idx = None;
        while idx >= 0 {
            next_child[idx as usize] = next_idx.clone();
            if self.children[idx as usize].get().is_some() {
                next_idx = Some(idx as usize)
            }
            idx -= 1;
        }
        self.next_child.set(Some(next_child));
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug)]
pub(crate) struct OrderedTrieNode<'a, T>
    where T: Ord, T: Debug {
    val: T,
    pub(crate) node: &'a TrieNode<'a>,
}

impl<'a> TrieNode<'a> {
    pub(crate) fn order<T>(&'a self, f: fn(&'a TrieNode) -> T) -> OrderedTrieNode<T>
        where T: Ord, T: Debug {
        OrderedTrieNode { val: f(self), node: self }
    }
}

impl<'a, T> From<&'a TrieNode<'a>> for OrderedTrieNode<'a, T>
    where T: Default + Ord + Debug {
    fn from(node: &'a TrieNode<'a>) ->
    Self {
        OrderedTrieNode::<'a, T> {
            val: Default::default(),
            node,
        }
    }
}

impl<'a, T> Deref for OrderedTrieNode<'a, T>
    where T: Ord, T: Debug {
    type Target = TrieNode<'a>;

    fn deref(&self) -> &TrieNode<'a> {
        self.node
    }
}
