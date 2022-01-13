use std::collections::{BinaryHeap, HashSet};

use std::ops::{Deref};
use std::sync::{Arc, TryLockResult};
use std::sync::{Mutex, MutexGuard};
use std::sync::atomic::AtomicIsize;
use std::sync::atomic::Ordering::Relaxed;
use rand::{Rng, thread_rng};


// register_counter!("pq_ops", "ops");
// register_gauge!("pq_size", "size");
// register_histogram!("tries_to_find_queue", "tries");
// register_histogram!("tries_to_find_empty_queue", "tries");

pub struct ConcurrentPQ<T: QItem> {
    pq: Arc<Vec<MutexQ<T>>>,
    len: Arc<AtomicIsize>,
    num_queues: usize,
    available: Arc<Mutex<HashSet<usize>>>,
}

pub trait QItem: Send + Ord + Eq + PartialOrd + PartialEq {}

type SingleQ<T> = BinaryHeap<T>;

struct MutexQ<T: QItem> {
    queue: Mutex<SingleQ<T>>,
}

struct HeldQ<'a, T: QItem> {
    queue: MutexGuard<'a, SingleQ<T>>,
}

impl<'a, T: QItem> HeldQ<'a, T> {
    fn from_result(result: TryLockResult<MutexGuard<'a, SingleQ<T>>>) -> Option<HeldQ<'a, T>> {
        match result {
            Ok(x) => Some(HeldQ { queue: x }),
            Err(_) => None
        }
    }
    fn from_guard(guard: MutexGuard<'a, SingleQ<T>>) -> HeldQ<'a, T> {
        HeldQ { queue: guard }
    }
}

impl<T: QItem> MutexQ<T> {
    fn new() -> MutexQ<T> {
        MutexQ { queue: Mutex::new(SingleQ::new()) }
    }
    fn get(&self) -> HeldQ<T> {
        HeldQ::from_guard(self.queue.lock().unwrap())
    }
    fn try_get(&self) -> Option<HeldQ<T>> {
        HeldQ::from_result(self.queue.try_lock())
    }
}


impl<T: QItem> Clone for ConcurrentPQ<T> {
    fn clone(&self) -> Self {
        ConcurrentPQ { pq: self.pq.clone(), len: self.len.clone(), num_queues: self.num_queues, available: self.available.clone() }
    }
}

impl<T: QItem> ConcurrentPQ<T> {
    pub fn new() -> ConcurrentPQ<T> {
        let n = rayon::current_num_threads() * 2;
        let mut v = Vec::with_capacity(n);
        let mut available = HashSet::new();
        for i in 0..n {
            v.push(MutexQ::new());
            available.insert(i);
        }
        let pq = ConcurrentPQ {
            pq: Arc::new(v),
            len: Arc::new(AtomicIsize::new(0)),
            num_queues: n,
            available: Arc::new(Mutex::new(available)),
        };
        pq
    }

    fn rand_index(&self) -> usize {
        thread_rng().gen_range(0..self.pq.len())
    }
    fn try_get_queue(&self) -> Option<HeldQ<T>> {
        let idx = self.rand_index();
        let m: &MutexQ<T> = self.pq.get(idx).unwrap();
        let held = m.try_get();
        held
    }

    fn get_queue(&self) -> HeldQ<T> {
        //let tries = 1;
        let mut result = self.try_get_queue();
        while result.is_none() {
            //tries += 1;
            result = self.try_get_queue();
        }
        result.unwrap()
    }

    fn get_nonempty_queue(&self, claimed: usize) -> Option<HeldQ<T>> {
        let mut q = self.get_queue();
        while q.queue.is_empty() {
            if self.len.load(Relaxed) <= claimed as isize {
                return None;
            }
            q = self.get_queue()
        }
        Some(q)
    }

    pub fn push(&self, item: T) {
        self.get_queue().queue.push(item);
        self.len.fetch_add(1, Relaxed);
    }

    pub fn clear(&mut self) {
        for q in self.pq.deref() {
            q.queue.lock().unwrap().clear();
        }
    }

    pub fn try_pop(&mut self) -> Option<T> {
        if self.len.load(Relaxed) <= 0 { return None; }
        let mut q1 = self.get_nonempty_queue(0);
        let q1_sz = q1.as_ref().map(|q| q.queue.len()).unwrap_or(0);
        let mut q2 = self.get_queue();
        if q2.queue.is_empty() && q1.is_none() {
            return None;
        }
        self.len.fetch_add(-1, Relaxed);
        if q1.is_none() {
            return q2.queue.pop();
        }
        if q2.queue.is_empty() {
            return q1.unwrap().queue.pop();
        }

        if q1.as_ref().unwrap().queue.peek() > q2.queue.peek() {
            q1.unwrap().queue.pop()
        } else { q2.queue.pop() }
    }
}


#[cfg(test)]
mod tests {
    use std::sync::atomic::Ordering::Relaxed;
    use crate::wordlist::trie::concurrent_pq::{ConcurrentPQ, QItem};

    impl QItem for i32 {}

    #[test]
    fn test_pq() {
        let mut pq = ConcurrentPQ::new();
        pq.push(5);
        pq.push(2);
        pq.push(7);
        pq.push(3);
        pq.push(1);

        assert_eq!(pq.len.load(Relaxed), 5);

        let mut popped = vec![];
        while let Some(x) = pq.try_pop() {
            popped.push(x);
        }

        popped.sort(); // popping is slightly non-deterministic so we're not guaranteed an order
        assert_eq!(popped, vec![1, 2, 3, 5, 7]);
    }
}
