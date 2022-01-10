
use std::collections::HashSet;
use std::fmt::{Debug, Formatter};

use std::hash::{Hash, Hasher};
use std::ops::Deref;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock, RwLockWriteGuard};
use maplit::hashset;
use crate::regex::nfa::state::NfaStateKind::Dummy;
use crate::regex::nfa::state::NfaStatePtr::{Strong, Weak};


#[derive(Debug)]
pub struct NfaState {
    kind: NfaStateKind,
    successors: Vec<NfaStatePtr>,
}


// #[derive(Clone)]
// pub enum NfaStatePtr {
//     Strong(Rc<RefCell<NfaState>>, u64, NfaStateKind),
//     // reference, uid, state kind of reference
//     Weak(std::rc::Weak<RefCell<NfaState>>, u64, NfaStateKind),
// }

#[derive(Clone)]
pub enum NfaStatePtr {
    Strong(Arc<RwLock<NfaState>>, u64, NfaStateKind),
    // reference, uid, state kind of reference
    Weak(std::sync::Weak<RwLock<NfaState>>, u64, NfaStateKind),
}


impl PartialOrd for NfaStatePtr {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.get_id().partial_cmp(&other.get_id())
    }
}
impl Ord for NfaStatePtr{
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.get_id().cmp(&other.get_id())
    }
}

#[derive(PartialEq, Eq, Debug, Clone, Hash)]
pub enum NfaStateKind {
    Literal(char),
    Set(Vec<char>),
    Wildcard,
    Start,
    Accept,
    Dummy,
}

impl PartialEq for NfaStatePtr {
    fn eq(&self, other: &Self) -> bool {
        self.get_id() == other.get_id()
    }
}

impl Eq for NfaStatePtr {}

impl Hash for NfaStatePtr {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.get_id().hash(state);
    }
}

impl Debug for NfaStatePtr {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Ptr")
            .field("id", &self.get_id())
            .field("kind", &self.get_kind())
            .field("from", &self.get_kind())
            .field("edges", &self.get_successors().len())
            .field("to", &self.get_successors().iter().map(|x| x.get_id()).collect::<Vec<u64>>()).finish()
    }
}

impl NfaStatePtr {
    fn next_id() -> u64 {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        COUNTER.fetch_add(1, Ordering::Relaxed)
    }

    pub(crate) fn get_id(&self) -> u64 {
        match self {
            Strong(_, id, _) => *id,
            Weak(_, id, _) => *id
        }
    }

    fn internal_get<T, F>(&self, mut f: F) -> T
        where F: FnMut(&NfaState) -> T {
        match self {
            Strong(x, _, _) => f(&x.deref().read().unwrap()),
            Weak(x, _, _) => f(&x.upgrade().unwrap().deref().read().unwrap())
            // Strong(x, _, _) => f(&x.deref().borrow()),
            // Weak(x, _, _) => f(&x.upgrade().unwrap().deref().borrow())
        }
    }

    fn internal_get_mut<T, F>(&self, mut f: F) -> T
        where F: FnMut(RwLockWriteGuard<NfaState>) -> T {
        match self {
            Strong(x, _, _) => f(x.deref().write().unwrap()),
            Weak(x, _, _) => f(x.upgrade().unwrap().deref().write().unwrap())
            // Strong(x, _, _) => f(x.deref().borrow_mut()),
            // Weak(x, _, _) => f(x.upgrade().unwrap().deref().borrow_mut())
        }
    }

    fn strong(state: NfaState) -> NfaStatePtr {
        let kind = state.kind.clone();
        Strong(Arc::new(RwLock::new(state)),
               NfaStatePtr::next_id(), kind)
    }
    fn clone(&self) -> NfaStatePtr {
        match self {
            Strong(x, id, kind) =>
                Strong(x.clone(), *id, kind.clone()),
            Weak(x, id, kind) =>
                Weak(x.clone(), *id, kind.clone())
        }
    }
    fn weak_clone(&self) -> NfaStatePtr {
        match self {
            Strong(x, id, kind) =>
                Weak(Arc::downgrade(&x), *id, kind.clone()),
            Weak(x, id, kind) =>
                Weak(x.clone(), *id, kind.clone())
        }
    }

    pub(crate) fn kind_is(&self, kind: &NfaStateKind) -> bool {
        &self.get_kind() == kind
    }

    fn get_kind(&self) -> NfaStateKind {
        match self {
            Strong(_, _, kind) => kind.clone(),
            Weak(_, _, kind) => kind.clone()
        }
    }

    pub(crate) fn non_dummy_successors(&self) -> Vec<NfaStatePtr> {
        let strong_successors = {
            let mut seen = hashset! {};
            self.non_dummy_successors_strong(&mut seen)
        };
        let weak_successors : Vec<NfaStatePtr> = {
            let mut seen = hashset!{};
            self.non_dummy_successors_all(&mut seen)
                .drain().filter(|x| !strong_successors.contains(x)).collect()
        };

        strong_successors.iter().map(|x| x.to_strong()).chain(
            weak_successors.into_iter().map(|x| x.to_weak())).collect()
    }


    fn non_dummy_successors_all(&self, seen: &mut HashSet<NfaStatePtr>) -> HashSet<NfaStatePtr> {
        self.internal_get(|x| x.non_dummy_successors_all(seen))
    }

    fn non_dummy_successors_strong(&self, seen: &mut HashSet<NfaStatePtr>) -> HashSet<NfaStatePtr> {
        self.internal_get(|x| x.non_dummy_successors_strong(seen))
    }

    pub(crate) fn get_successors(&self) -> Vec<NfaStatePtr> {
        self.internal_get(|x| x.successors.clone())
    }

    pub(crate) fn set_successors(&mut self, other: Vec<NfaStatePtr>) {
        self.internal_get_mut(|mut x| x.set_successors(other.clone()))
    }

    pub(crate) fn accepts(&self, next_char: char) -> bool {
        self.internal_get(|x| x.accepts(next_char))
    }

    fn to_strong(&self) -> NfaStatePtr {
        match self {
            Strong(..) => self.clone(),
            Weak(x, id, kind) =>
                Strong(x.upgrade().unwrap(), *id, kind.clone())
        }
    }
    fn to_weak(&self) -> NfaStatePtr {
        match self {
            Strong(x, id, kind) =>
                Weak(Arc::downgrade(x), *id, kind.clone()),
            Weak(..) => self.clone()

        }
    }
    fn is_strong(&self) -> bool {
        match self {
            Strong(..) => true,
            Weak(..) => false
        }
    }

    fn is_weak(&self) -> bool {
        match self {
            Strong(..) => false,
            Weak(..) => true
        }
    }

    pub(crate) fn add_strong_successors_to(&self, other: &mut Vec<NfaStatePtr>) {
        self.internal_get(|x| x.successors.iter()
            .filter(|x| x.is_strong())
            .for_each(|x| other.push(x.clone())))
    }


    pub(crate) fn add_successors_to(&self, other: &mut Vec<NfaStatePtr>) {
        self.internal_get(|x| x.successors.iter()
            .for_each(|x| other.push(x.clone())))
    }

    pub(crate) fn add_successor(&self, other: &NfaStatePtr) {
        self.internal_get_mut(|mut x| x.add_successor(other))
    }

    pub(crate) fn add_weak_successor(&self, other: &NfaStatePtr) {
        self.internal_get_mut(|mut x| x.add_weak_successor(other))
    }
}


impl NfaState {
    pub(crate) fn strong_ptr(state: NfaStateKind) -> NfaStatePtr {
        NfaStatePtr::strong(NfaState { kind: state, successors: vec![] })
    }

    fn add_successor(&mut self, other: &NfaStatePtr) {
        self.successors.push(other.clone());
    }

    fn add_weak_successor(&mut self, other: &NfaStatePtr) {
        self.successors.push(other.weak_clone());
    }

    fn set_successors(&mut self, other: Vec<NfaStatePtr>) {
        self.successors = other;
    }

    fn non_dummy_successors() {

    }

    fn non_dummy_successors_all(&self, seen: &mut HashSet<NfaStatePtr>) -> HashSet<NfaStatePtr> {
        let succ_dummy: Vec<NfaStatePtr> = self.successors.iter()
            .filter(|x| !seen.contains(x))
            .filter(|x| x.kind_is(&Dummy))
            .map(|x| x.clone())
            .collect();

        let succ_nondummy: Vec<NfaStatePtr> = self.successors.iter()
            .filter(|x| !x.kind_is(&Dummy))
            .map(|x| x.clone())
            .collect();

        succ_dummy.iter().for_each(|x| { seen.insert(x.clone()); });

        let succ_dummy_succ: Vec<NfaStatePtr> = succ_dummy.iter()
            .map(|x| x.non_dummy_successors_all(seen))
            .flatten()
            .collect();

        let set: HashSet<NfaStatePtr> = succ_dummy_succ.into_iter()
            .chain(succ_nondummy.into_iter())
            .collect();

        HashSet::from_iter(set.into_iter())
    }
    fn non_dummy_successors_strong(&self, seen: &mut HashSet<NfaStatePtr>) -> HashSet<NfaStatePtr> {
        let succ_dummy: Vec<NfaStatePtr> = self.successors.iter()
            .filter(|x| x.is_strong())
            .filter(|x| !seen.contains(x))
            .filter(|x| x.kind_is(&Dummy))
            .map(|x| x.clone())
            .collect();

        let succ_nondummy: Vec<NfaStatePtr> = self.successors.iter()
            .filter(|x|x.is_strong())
            .filter(|x| !x.kind_is(&Dummy))
            .map(|x| x.clone())
            .collect();

        succ_dummy.iter().for_each(|x| { seen.insert(x.clone()); });

        let succ_dummy_succ: Vec<NfaStatePtr> = succ_dummy.iter()
            .map(|x| x.non_dummy_successors_strong(seen))
            .flatten()
            .collect();

        let set: HashSet<NfaStatePtr> = succ_dummy_succ.into_iter()
            .chain(succ_nondummy.into_iter())
            .collect();

        HashSet::from_iter(set.into_iter())
    }

    fn accepts(&self, next_char: char) -> bool {
        use NfaStateKind::*;

        match &self.kind {
            Literal(c) => *c == next_char,
            Set(set) => set.contains(&next_char),
            Wildcard => true,
            Start => true,
            Accept => true,
            Dummy => true
        }
    }
}


#[cfg(test)]
mod tests {
    use crate::regex::nfa::state::*;
    use crate::regex::nfa::state::NfaStateKind::*;

    #[test]
    fn test_non_dummy_successors_without_dummy() {
        let state1 = NfaState::strong_ptr(Start);
        let state2 = NfaState::strong_ptr(Literal('a'));
        let state3 = NfaState::strong_ptr(Literal('b'));
        let state4 = NfaState::strong_ptr(Accept);

        state1.add_successor(&state2);
        state2.add_successor(&state3);
        state3.add_successor(&state4);


        assert_eq!(state1.clone().non_dummy_successors(), vec![state2.clone()]);
        assert_eq!(state2.clone().non_dummy_successors(), vec![state3.clone()]);
        assert_eq!(state3.clone().non_dummy_successors(), vec![state4.clone()]);
        assert_eq!(state4.clone().non_dummy_successors(), vec![]);
    }

    #[test]
    fn test_non_dummy_successors_linear_dummy() {
        let state1 = NfaState::strong_ptr(Start);
        let state2 = NfaState::strong_ptr(Literal('a'));
        let state3 = NfaState::strong_ptr(Dummy);
        let state4 = NfaState::strong_ptr(Accept);

        state1.add_successor(&state2);
        state2.add_successor(&state3);
        state3.add_successor(&state4);


        assert_eq!(state1.clone().non_dummy_successors(), vec![state2.clone()]);
        assert_eq!(state2.clone().non_dummy_successors(), vec![state4.clone()]);
        assert_eq!(state4.clone().non_dummy_successors(), vec![]);
    }


    #[test]
    fn test_non_dummy_successors_loop_dummy() {
        let state0 = NfaState::strong_ptr(Start);
        let state1 = NfaState::strong_ptr(Literal('a'));
        let state2 = NfaState::strong_ptr(Dummy);
        let state3 = NfaState::strong_ptr(Dummy);
        let state4 = NfaState::strong_ptr(Accept);

        state0.add_successor(&state1);
        state1.add_successor(&state2);
        state2.add_successor(&state3);
        state3.add_successor(&state4);

        state3.add_weak_successor(&state2);

        assert_eq!(state0.clone().get_successors(), vec![state1.clone()]);
        assert_eq!(state0.clone().non_dummy_successors(), vec![state1.clone()]);
        assert_eq!(state1.clone().non_dummy_successors(), vec![state4.clone()]);
        assert_eq!(state4.clone().non_dummy_successors(), vec![]);
    }

    #[test]
    fn test_non_dummy_successors_forking_dummy() {
        let state1 = NfaState::strong_ptr(Start);
        let state2 = NfaState::strong_ptr(Dummy);
        let state3 = NfaState::strong_ptr(Literal('a'));
        let state4 = NfaState::strong_ptr(Literal('b'));
        let state5 = NfaState::strong_ptr(Dummy);
        let state6 = NfaState::strong_ptr(Accept);

        state1.add_successor(&state2);
        state2.add_successor(&state3);
        state2.add_successor(&state4);
        state3.add_successor(&state5);
        state4.add_successor(&state5);
        state5.add_successor(&state6);

        assert_eq!(state1.clone().non_dummy_successors().iter().collect::<HashSet<_>>(),
                   vec![state3.clone(), state4.clone()].iter().collect::<HashSet<_>>());
        assert_eq!(state3.clone().non_dummy_successors(), vec![state6.clone()]);
        assert_eq!(state3.clone().non_dummy_successors(), vec![state6.clone()]);
    }

    #[test]
    fn test_non_dummy_successors_backward_edge() {
        let state1 = NfaState::strong_ptr(Start);
        let state2 = NfaState::strong_ptr(Dummy);
        let state3 = NfaState::strong_ptr(Literal('a'));
        let state4 = NfaState::strong_ptr(Literal('b'));
        let state5 = NfaState::strong_ptr(Dummy);
        let state6 = NfaState::strong_ptr(Accept);

        state1.add_successor(&state2);
        state2.add_successor(&state3);
        state3.add_successor(&state4);
        state4.add_successor(&state5);
        state5.add_successor(&state6);

        state4.add_weak_successor(&state2);

        assert_eq!(state1.clone().non_dummy_successors(), vec![state3.clone()]);
        assert_eq!(state3.clone().non_dummy_successors(), vec![state4.clone()]);
        assert_eq!(state4.clone().non_dummy_successors().iter().collect::<HashSet<_>>(),
                   vec![state3.clone(), state6.clone()].iter().collect::<HashSet<_>>());
    }
}