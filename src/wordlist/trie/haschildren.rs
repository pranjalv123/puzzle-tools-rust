use crate::alphabet::get_idx;

pub(crate) trait HasChildren {
    fn children(&self) -> &[Option<Self>] where Self : Sized;
    fn get_child(&self, c: char) -> Option<&Self> where Self : Sized {
        self.children()[get_idx(c)].as_ref()
    }

    fn traverse_prefix<T, F>(&self, f: &mut F)
        where F: FnMut(&Self) -> T, Self: Sized {
        f(self);
        self.children().iter()
            .for_each(|node| {
                node.as_ref().map(|x| x.traverse_prefix(f))
                    .unwrap_or(())
            });
    }
    fn traverse_postfix<T, F>(&self, f: &mut F)
        where F: FnMut(&Self) -> T, Self: Sized {
        self.children().iter()
            .for_each(|node| {
                node.as_ref().map(|x| x.traverse_postfix(f))
                    .unwrap_or(())
            });
        f(self);
    }
}
