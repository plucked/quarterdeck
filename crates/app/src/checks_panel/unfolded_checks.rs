use std::collections::HashSet;

/// The workflows and groups the reader unfolded, by name, so they stay
/// open while the checks refresh; everything starts folded.
#[derive(Debug, Default)]
pub struct UnfoldedChecks {
    nodes: HashSet<FoldKey>,
}

/// A workflow, or a group in one, by name.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct FoldKey {
    workflow: String,
    group: Option<String>,
}

impl UnfoldedChecks {
    /// Whether the workflow `workflow`, or its group `group`, is unfolded.
    pub fn contains(&self, workflow: &str, group: Option<&str>) -> bool {
        self.nodes.contains(&FoldKey {
            workflow: workflow.to_owned(),
            group: group.map(str::to_owned),
        })
    }

    /// Folds an unfolded workflow or group, or unfolds a folded one.
    pub fn toggle(&mut self, workflow: &str, group: Option<&str>) {
        let key = FoldKey {
            workflow: workflow.to_owned(),
            group: group.map(str::to_owned),
        };
        if !self.nodes.remove(&key) {
            self.nodes.insert(key);
        }
    }

    /// Folds everything (another pull request).
    pub fn clear(&mut self) {
        self.nodes.clear();
    }
}
