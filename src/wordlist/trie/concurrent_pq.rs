use std::collections::BinaryHeap;
use std::fmt::Debug;
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, Mutex, MutexGuard, TryLockResult};
use std::sync::atomic::AtomicIsize;
use std::sync::atomic::Ordering::Relaxed;
use rand::{Rng, thread_rng};
use metrics::{histogram, counter, register_histogram, register_gauge, register_counter};

// register_counter!("pq_ops", "ops");
// register_gauge!("pq_size", "size");
// register_histogram!("tries_to_find_queue", "tries");
// register_histogram!("tries_to_find_empty_queue", "tries");

pub struct ConcurrentPQ<T: QItem> {
    pq: Arc<Vec<MutexQ<T>>>,
    len: Arc<AtomicIsize>,
    num_queues: usize,
}

pub trait QItem: Send + Ord + Eq + PartialOrd + PartialEq {}

type SingleQ<T: QItem> = BinaryHeap<T>;

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


impl< T: QItem> Clone for ConcurrentPQ<T> {
    fn clone(&self) -> Self {
        ConcurrentPQ { pq: self.pq.clone(), len: self.len.clone(), num_queues: self.num_queues }
    }
}

impl<T: QItem> ConcurrentPQ<T> {
    pub fn new() -> ConcurrentPQ<T> {
        let n = rayon::current_num_threads() * 2;
        println!("Creating PQ with {} queues", n);
        let mut v = Vec::with_capacity(n);
        for _ in 0..n {
            v.push(MutexQ::new());
        }
        let mut pq = ConcurrentPQ {
            pq: Arc::new(v),
            len: Arc::new(AtomicIsize::new(0)),
            num_queues: n,
        };
        pq
    }

    fn rand_index(&self) -> usize {
        thread_rng().gen_range(0..self.pq.len())
    }
    fn try_get_queue(&self) -> Option<HeldQ<T>> {
        let m: &MutexQ<T> = self.pq.get(self.rand_index()).unwrap();
        m.try_get()
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
        let mut q2 = self.get_nonempty_queue(q1_sz);

        let q: Option<HeldQ<T>> = match (q1, q2) {
            (None, None) => None,
            (Some(mut x), None) => Some(x),
            (None, Some(mut x)) => Some(x),
            (Some(x1), Some(x2)) =>
                if x1.queue.peek() > x2.queue.peek() { Some(x1) } else { Some(x2) }
        };

        if q.is_none() {
            return None
        }

        self.len.fetch_add(-1, Relaxed);
        q.unwrap().queue.pop()
    }
}


#[cfg(test)]
mod tests {
    use std::sync::atomic::Ordering::Relaxed;
    use crate::wordlist::trie::concurrent_pq::{ConcurrentPQ, QItem};

    impl QItem for i32{}

    #[test]
    fn test_pq() {
        let mut pq = ConcurrentPQ::new();
        println!("Starting...");
        pq.push(5);
        pq.push(2);
        pq.push(7);
        pq.push(3);
        pq.push(1);

        println!("Pushed {}", pq.len.load(Relaxed));
        assert_eq!(pq.len.load(Relaxed), 5);

        let mut popped = vec![];
        while let Some(x) = pq.try_pop() {
            println!("{}", x);
            popped.push(x);
        }

        popped.sort(); // popping is slightly non-deterministic so we're not guaranteed an order
        assert_eq!(popped, vec![1,2,3,5,7]);
    }
}