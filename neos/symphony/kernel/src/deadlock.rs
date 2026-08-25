//! Deadlock detection over the wait-for graph.
//!
//! Contract §8. Required precisely **because** the field model does not provide
//! it: load equilibrium eliminates thrashing and bottlenecks, but deadlock is a
//! circular wait in resource *acquisition*, orthogonal to how work is
//! distributed. Four perfectly balanced cores still deadlock on two locks taken
//! in opposite orders — asserted directly in the test suite.
//!
//! A kernel built believing deadlock impossible hangs with no diagnostic.

use crate::scheduler::TaskId;
use std::collections::HashMap;

/// Who is waiting on whom.
///
/// An edge `a -> b` means task `a` is blocked on a resource currently held by
/// task `b`. A cycle in this graph is a deadlock.
#[derive(Debug, Clone, Default)]
pub struct WaitForGraph {
    edges: HashMap<TaskId, Vec<TaskId>>,
}

#[derive(Clone, Copy, PartialEq)]
enum Mark {
    Unvisited,
    InProgress,
    Done,
}

impl WaitForGraph {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record that `waiter` is blocked on a resource held by `holder`.
    pub fn add_wait(&mut self, waiter: TaskId, holder: TaskId) {
        self.edges.entry(waiter).or_default().push(holder);
        self.edges.entry(holder).or_default();
    }

    /// Release everything `task` was waiting on.
    pub fn clear_waits(&mut self, task: TaskId) {
        if let Some(v) = self.edges.get_mut(&task) {
            v.clear();
        }
    }

    /// Remove one `waiter -> holder` edge, leaving any other edges `waiter`
    /// holds untouched.
    ///
    /// Distinct from [`clear_waits`](Self::clear_waits), which drops every
    /// edge for a task regardless of which resource each one came from. A
    /// feeder that tracks resource handoffs (a waiter's wait ending because
    /// its specific holder released, not because the waiter itself gave up
    /// on everything) needs the finer-grained removal to keep the graph
    /// exact rather than merely non-stale.
    pub fn remove_wait(&mut self, waiter: TaskId, holder: TaskId) {
        if let Some(v) = self.edges.get_mut(&waiter) {
            v.retain(|&h| h != holder);
        }
    }

    pub fn is_empty(&self) -> bool {
        self.edges.values().all(Vec::is_empty)
    }

    pub fn task_count(&self) -> usize {
        self.edges.len()
    }

    /// Find a wait-for cycle, if one exists.
    ///
    /// Returns the tasks forming the cycle, in order. `None` means no deadlock
    /// — which is a genuine result, not merely "none found": the search is
    /// exhaustive over the graph.
    ///
    /// Depth-first search with three-colour marking. A back edge to a node
    /// still `InProgress` closes a cycle.
    pub fn detect_cycle(&self) -> Option<Vec<TaskId>> {
        let mut mark: HashMap<TaskId, Mark> =
            self.edges.keys().map(|k| (*k, Mark::Unvisited)).collect();

        let mut roots: Vec<TaskId> = self.edges.keys().copied().collect();
        roots.sort(); // deterministic output across runs

        for root in roots {
            if mark[&root] != Mark::Unvisited {
                continue;
            }
            let mut path = Vec::new();
            if let Some(cycle) = self.visit(root, &mut mark, &mut path) {
                return Some(cycle);
            }
        }
        None
    }

    fn visit(
        &self,
        node: TaskId,
        mark: &mut HashMap<TaskId, Mark>,
        path: &mut Vec<TaskId>,
    ) -> Option<Vec<TaskId>> {
        mark.insert(node, Mark::InProgress);
        path.push(node);

        if let Some(next) = self.edges.get(&node) {
            for &n in next {
                match mark.get(&n).copied().unwrap_or(Mark::Unvisited) {
                    Mark::InProgress => {
                        // Back edge: the cycle is the path from n onward.
                        let start = path.iter().position(|&p| p == n).unwrap_or(0);
                        return Some(path[start..].to_vec());
                    }
                    Mark::Unvisited => {
                        if let Some(c) = self.visit(n, mark, path) {
                            return Some(c);
                        }
                    }
                    Mark::Done => {}
                }
            }
        }

        path.pop();
        mark.insert(node, Mark::Done);
        None
    }

    /// Whether any deadlock exists.
    pub fn has_deadlock(&self) -> bool {
        self.detect_cycle().is_some()
    }
}
