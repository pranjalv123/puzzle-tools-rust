use std::cell::Cell;
use crate::alphabet::get_idx;
use crate::wordlist::trie::node::TrieNode;

impl<'a> TrieNode<'a> {
    fn children(&self) -> &[Cell<Option<&'a Self>>] where Self : Sized {
        &self.children
    }
    pub(crate) fn get_child(&self, c: char) -> Option<&'a Self> where Self : Sized {
        self.children[get_idx(c)].get()
    }

    pub(crate) fn traverse_prefix<'f, T, F>(&'f self, f: &mut F)
        where F: Fn(&'f Self) -> T, Self: Sized {
        self.traverse_prefix_mut(f)
    }
    pub(crate) fn traverse_prefix_mut<'f, T, F>(&'f self, f:&mut  F)
        where F: FnMut(&'f Self) -> T, Self: Sized {
        f(self);
        self.children().iter()
            .for_each(|node| {
                node.get().map(|x| x.traverse_prefix_mut(f))
                    .unwrap_or(())
            });
    }
    fn traverse_postfix<T, F>(&self, f:&mut  F)
        where F: Fn(&Self) -> T, Self: Sized {
        self.traverse_postfix_mut(f)
    }
    fn traverse_postfix_mut<T, F>(&self, f:&mut  F)
        where F: FnMut(&Self) -> T, Self: Sized {
        self.children().iter()
            .for_each(|node| {
                node.get().map(|x| x.traverse_postfix_mut(f))
                    .unwrap_or(())
            });
        f(self);
    }
}
