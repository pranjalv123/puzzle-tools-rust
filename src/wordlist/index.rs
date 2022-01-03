pub trait Index {
    fn add(&mut self, word : &str);
    fn contains(&self, word : &str) -> bool;

    fn add_all<'a, I>(&mut self, items : I)
        where I: Iterator<Item = &'a str>{
        self.add_all_(items)
    }
    fn add_all_<'a,I>(&mut self, items : I)
    where I: Iterator<Item = &'a str>{
        items.for_each(|x| self.add(x.clone()));
    }
}