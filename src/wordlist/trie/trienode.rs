use std::borrow::Borrow;
use std::cell::RefCell;
use std::fmt::{Debug, Formatter};
use std::ops::Deref;
use std::rc::Rc;
use std::boxed::Box;
use delegate::delegate;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::{BinaryHeap, HashMap};
use maplit::hashmap;
use crate::alphabet::{ALPHABET, get_idx};
use crate::regex::nfa::graph::{NfaGraph, NfaResult};
use crate::regex::nfa::state::NfaStateKind::Accept;
use crate::regex::nfa::state::NfaStatePtr;
use crate::wordlist::trie::haschildren::HasChildren;
use crate::wordlist::trie::mutablenode::{MutableTrieNode, TrieNodeRef};
use crate::wordlist::trie::Trie;
use crate::wordlist::trie::trie_builder::TrieBuilder;


#[derive(Ord, PartialOrd, Eq, PartialEq, Serialize, Deserialize, Default)]
pub(crate) struct TrieNode {
    #[serde(skip)]
    children: Box<[Option<TrieNode>; ALPHABET.len()]>,
    next_child: [Option<usize>; ALPHABET.len()],
    pub(crate) letter: char,
    pub(crate) is_terminal: bool,
    pub(crate) weight: usize,
    depth: usize,
    pub(crate) freq: usize,
    pub(crate) path: String,
}

impl TrieNode {
    pub(crate) fn order<T>(&self, f: fn(&TrieNode) -> T) -> OrderedTrieNode<T>
        where T: Ord, T: Debug {
        OrderedTrieNode { val: f(self), node: self }
    }
}

impl From<MutableTrieNode> for TrieNode {
    fn from(mnode: MutableTrieNode) -> Self {
        let children = mnode.children
            .map(|maybe_ref| maybe_ref.map(
                |child_ref| {
                    Rc::try_unwrap(child_ref.0).unwrap().into_inner()
                }
            ).map(|mutable_child|
                TrieNode::from(mutable_child))
            );

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

        TrieNode {
            children: Box::new(children),
            next_child,
            letter: mnode.letter,
            is_terminal: mnode.is_terminal,
            weight: mnode.weight,
            depth: mnode.depth,
            freq: mnode.freq,
            path: mnode.path,
        }
    }
}


impl Debug for TrieNode {
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
    pub(crate) node: &'a TrieNode,
}

impl<'a, T> From<&'a TrieNode> for OrderedTrieNode<'a, T>
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
    type Target = TrieNode;

    fn deref(&self) -> &TrieNode { self.node }
}

impl<'a> IntoIterator for &'a TrieNode {
    type Item = &'a TrieNode;
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
    node: &'a TrieNode,
}

impl<'a> Iterator for TrieCursor<'a> {
    type Item = &'a TrieNode;

    fn next(&mut self) -> Option<Self::Item> {
        let mut rv = None;
        if let Some(idx) = self.idx {
            rv = self.node.children[idx].as_ref();
            self.idx = self.node.next_child[idx];
        }
        rv
    }
}

impl HasChildren for TrieNode {
    fn children(&self) -> &[Option<Self>] {
        self.children.as_slice()
    }
}

