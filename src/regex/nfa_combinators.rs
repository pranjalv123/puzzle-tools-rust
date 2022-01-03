// use std::io::SeekFrom::Start;
// use maplit::hashmap;
// use crate::regex::nfa::{Nfa, NfaGraph};
//
// /// Return an NFA that accepts if either NFA accepts
// fn either(first: &mut NfaGraph, second: &mut NfaGraph) -> impl Nfa {
//
//     let start = Box::new(Start);
//     let accept = Box::new(Accept);
//     NfaGraph {
//         states: vec![start, first, second, accept],
//         edges: hashmap! {
//                 start.as_ref() => vec![first.as_ref(), second.as_ref()],
//                 first.as_ref() => vec![&accept.as_ref()],
//                 second.as_ref() => vec![&accept.as_ref()]
//             },
//         start_state: start.as_ref()
//     }
// }
//
// /// Return an NFA that connects the accept state of this NFA to the start state of the other NFA
// fn sequence(first: Box<dyn Nfa>, second: Box<dyn Nfa>) -> impl Nfa {
//     let accept = Box::new(Accept);
//     Box::new(NfaGraph {
//         states: vec![first, other, accept],
//         edges: hashmap! {
//                 first.as_ref() => vec![second.as_ref()],
//                 second.as_ref() => vec![accept.as_ref()]
//             },
//         start_state: first.as_ref(),
//     })
// }
//
// /// Return an NFA that connects the accept state of this NFA to the start state of the other NFA
// fn repeat(nfa: Box<dyn Nfa>) -> impl Nfa {
//     let accept = Box::new(Accept);
//     Box::new(NfaGraph {
//         states: vec![nfa, accept],
//         edges: hashmap! {
//                 nfa.as_ref() => vec![nfa.as_ref(), accept.as_ref()],
//             },
//         start_state: nfa.as_ref(),
//     })
// }
//
