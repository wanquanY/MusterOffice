//! Causal clauses: structural prerequisites are AND, alternative triggers OR.
//! Resolving one trigger must not require all of its unchosen dependencies.
use super::*;

pub(super) struct Causality {
    unmet: Vec<usize>,
    listeners: Vec<Vec<usize>>,
    clauses: Vec<(usize, bool)>,
}
impl Causality {
    pub(super) fn new(vertices: usize) -> Self {
        Self {
            unmet: vec![0; vertices],
            listeners: vec![vec![]; vertices],
            clauses: vec![],
        }
    }
    pub(super) fn require(&mut self, target: usize, source: usize) {
        self.any(target, &[source]);
    }
    pub(super) fn any(&mut self, target: usize, alternatives: &[usize]) {
        if alternatives.is_empty() {
            return;
        }
        let clause = self.clauses.len();
        self.clauses.push((target, false));
        self.unmet[target] += 1;
        for &source in alternatives {
            self.listeners[source].push(clause);
        }
    }
    pub(super) fn validate(mut self, check: &dyn Fn() -> bool) -> Result<(), TimelineError> {
        let mut ready: VecDeque<_> = self
            .unmet
            .iter()
            .enumerate()
            .filter(|(_, n)| **n == 0)
            .map(|(i, _)| i)
            .collect();
        let mut visited = 0;
        while let Some(source) = ready.pop_front() {
            cancel(check)?;
            visited += 1;
            for &clause in &self.listeners[source] {
                cancel(check)?;
                let (target, resolved) = &mut self.clauses[clause];
                if !std::mem::replace(resolved, true) {
                    self.unmet[*target] -= 1;
                    if self.unmet[*target] == 0 {
                        ready.push_back(*target);
                    }
                }
            }
        }
        if visited != self.unmet.len() {
            return Err(invalid(None, "cyclic timing begin/end dependencies"));
        }
        Ok(())
    }
}
