use std::fmt::{Debug, Formatter};
use std::ops::Deref;
use std::rc::Rc;
use std::boxed::Box;

use serde::{Deserialize, Serialize};
use typed_arena::Arena;


use crate::alphabet::{ALPHABET};


use crate::wordlist::trie::haschildren::HasChildren;
use crate::wordlist::trie::mutablenode::{MutableTrieNode};
use crate::wordlist::trie::Trie;


#[derive(Ord, PartialOrd, Eq, PartialEq, Serialize, Deserialize, Default)]
pub(crate) struct TrieNode<'a> {
    #[serde(skip)]
    pub(crate) children: [Option<&'a TrieNode<'a>>; ALPHABET.len()],
    next_child: [Option<usize>; ALPHABET.len()],
    pub(crate) letter: char,
    pub(crate) is_terminal: bool,
    pub(crate) weight: usize,
    depth: usize,
    pub(crate) freq: usize,
    pub(crate) path: String,
}

impl<'a> TrieNode<'a> {
    pub(crate) fn order<T>(&'a self, f: fn(&'a TrieNode) -> T) -> OrderedTrieNode<T>
        where T: Ord, T: Debug {
        OrderedTrieNode { val: f(self), node: self }
    }


    fn build_next_child(children: &[Option<&TrieNode>]) -> [Option<usize>; ALPHABET.len()] {
        let mut next_child = [None; ALPHABET.len()];
        let mut idx: isize = (next_child.len() - 1) as isize;
        let mut next_idx = None;
        while idx >= 0 {
            next_child[idx as usize] = next_idx.clone();
            if children[idx as usize].is_some() {
                next_idx = Some(idx as usize)
            }
            idx -= 1;
        }
        next_child
    }

    //impl<'a, 'b> From<&'a MutableTrieNode<'a>> for &'b TrieNode<'b> {
    pub(crate) fn from_mutable<'b>(mnode: &'a MutableTrieNode<'a>, arena: &'b Arena<TrieNode<'b>>) -> &'b TrieNode<'b> {
        let mut children: [Option<&TrieNode>; ALPHABET.len()] = Default::default();

        (&mnode.children).iter().zip(&mut children.iter_mut())
            .for_each(|(old, new)|
                *new = old.get().map(|x| Self::from_mutable(x, arena))
            );

        let next_child = Self::build_next_child(&children);

        arena.alloc(TrieNode {
            children: children,
            next_child,
            letter: mnode.letter,
            is_terminal: mnode.is_terminal.get(),
            weight: mnode.weight.get(),
            depth: mnode.depth,
            freq: mnode.freq.get(),
            path: mnode.path.to_string(),
        })
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
                .filter(|x| x.is_some())
                .map(|x| x.as_ref().unwrap().letter)
                .collect::<Vec<_>>(),
            )
            .finish()
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug)]
pub(crate) struct OrderedTrieNode<'a, T>
    where T: Ord, T: Debug {
    val: T,
    pub(crate) node: &'a TrieNode<'a>,
}

impl<'a, T> From<&'a TrieNode<'a>> for OrderedTrieNode<'a, T>
    where T: Default + Ord + Debug {
    fn from(node: &'a TrieNode) ->
    Self {
        OrderedTrieNode {
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

impl<'a> IntoIterator for &'a TrieNode<'a> {
    type Item = &'a TrieNode<'a>;
    type IntoIter = TrieCursor<'a>;

    fn into_iter(self) -> Self::IntoIter {
        if self.children[0].is_some() {
            TrieCursor { idx: Some(0), node: self }
        } else {
            TrieCursor { idx: self.next_child[0], node: self }
        }
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
            rv = self.node.children[idx];
            self.idx = self.node.next_child[idx];
        }
        rv
    }
}


impl<'a> HasChildren for TrieNode<'a> {
    fn children(&self) -> &[Option<&Self>] {
        self.children.as_slice()
    }
}

