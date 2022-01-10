use crate::wordlist::trie::node::{ImmutableTrieNode, TrieNode};

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
        if self.next_child.get().is_none() {
            self.build_next_child();
        }

        if self.children[0].get().is_some() {
            TrieCursor { idx: Some(0), node: self }
        } else {
            TrieCursor { idx: self.next_child.get().unwrap()[0], node: self }
        }
    }
}



pub(crate) struct ImmutableTrieCursor<'a> {
    idx: Option<usize>,
    node: &'a ImmutableTrieNode<'a>,
}

impl<'a> Iterator for ImmutableTrieCursor<'a> {
    type Item = &'a ImmutableTrieNode<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut rv = None;
        if let Some(idx) = self.idx {
            rv = self.node.children[idx];
            self.idx = self.node.next_child[idx];
        }
        rv
    }
}

impl<'a> IntoIterator for &'a ImmutableTrieNode<'a> {
    type Item = &'a ImmutableTrieNode<'a>;
    type IntoIter = ImmutableTrieCursor<'a>;

    fn into_iter(self) -> Self::IntoIter {

        if self.children[0].is_some() {
            ImmutableTrieCursor { idx: Some(0), node: self }
        } else {
            ImmutableTrieCursor { idx: self.next_child[0], node: self }
        }
    }
}
