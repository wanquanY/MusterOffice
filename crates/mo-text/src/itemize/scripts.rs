use super::*;
use mo_unicode::{
    bidi::bracket,
    script::{Script, ScriptSet},
};
use std::collections::BTreeMap;
/// Resolve each isolate scope separately. A pair retains the opening run's
/// script, including when its enclosed text uses another script or direction.
pub(super) fn resolve(
    clusters: &mut [Cluster],
    scopes: &[Vec<usize>],
    check: &dyn Fn() -> bool,
) -> Result<(), TextError> {
    for scope in scopes {
        let pairs = matching_pairs(clusters, scope, check)?;
        let mut pending = Vec::new();
        let mut candidates: Option<ScriptSet> = None;
        let mut preferred = None;
        for &i in scope {
            cancelled(check)?;
            if clusters[i].kind != TextItemKind::Text {
                continue;
            }
            let own = clusters[i].scripts;
            let paired = pairs.get(&i).and_then(|&at| clusters[at].resolved);
            let set = match paired {
                Some(s) if own.is_none_or(|v| v.contains(s)) => Some(ScriptSet::singleton(s)),
                _ => own,
            };
            let intersection = match (candidates, set) {
                (Some(a), Some(b)) => Some(a.intersection(b)),
                (a, b) => a.or(b),
            };
            if intersection.is_some_and(|s| s.is_empty()) {
                finish(clusters, &mut pending, candidates, preferred, check)?;
                candidates = set;
                preferred = None;
            } else {
                candidates = intersection;
            }
            if preferred.is_none_or(|s| !candidates.is_none_or(|set| set.contains(s))) {
                preferred = clusters[i]
                    .preferred
                    .filter(|&s| candidates.is_none_or(|set| set.contains(s)));
            }
            pending.push(i);
        }
        finish(clusters, &mut pending, candidates, preferred, check)?;
    }
    Ok(())
}
fn finish(
    clusters: &mut [Cluster],
    pending: &mut Vec<usize>,
    set: Option<ScriptSet>,
    preferred: Option<Script>,
    check: &dyn Fn() -> bool,
) -> Result<(), TextError> {
    let script = preferred
        .filter(|&s| set.is_none_or(|set| set.contains(s)))
        .or_else(|| set.and_then(|s| s.iter().next()))
        .unwrap_or_else(Script::common);
    let ambiguous = preferred.is_none() && set.is_some_and(|s| s.iter().nth(1).is_some());
    for i in pending.drain(..) {
        cancelled(check)?;
        let hint = clusters[i]
            .language_script
            .filter(|&hint| ambiguous && set.is_some_and(|set| set.contains(hint)));
        clusters[i].resolved = Some(hint.unwrap_or(script));
        clusters[i].ambiguous = ambiguous && hint.is_none();
    }
    Ok(())
}
fn matching_pairs(
    clusters: &[Cluster],
    scope: &[usize],
    check: &dyn Fn() -> bool,
) -> Result<BTreeMap<usize, usize>, TextError> {
    // Per-type linked stack heads avoid backwards rescans for many unmatched
    // closing brackets. Each opening is popped at most once within this scope.
    let mut stack: Vec<(u32, usize, Option<usize>)> = Vec::new();
    let mut heads = BTreeMap::new();
    let mut pairs = BTreeMap::new();
    for &i in scope {
        cancelled(check)?;
        let Some(b) = bracket(clusters[i].first) else {
            continue;
        };
        if b.is_open {
            let previous = heads.insert(b.normalized_opening, stack.len());
            stack.push((b.normalized_opening, i, previous));
        } else if let Some(&position) = heads.get(&b.normalized_opening) {
            pairs.insert(i, stack[position].1);
            while stack.len() > position {
                cancelled(check)?;
                let (key, _, previous) = stack.pop().unwrap();
                if let Some(at) = previous {
                    heads.insert(key, at);
                } else {
                    heads.remove(&key);
                }
            }
        }
    }
    Ok(pairs)
}
