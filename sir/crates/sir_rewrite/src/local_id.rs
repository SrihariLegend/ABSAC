use std::fmt;

/// A node identifier scoped to a single `DetachedArena`.
///
/// Unlike `NodeId` (which is global within a `Function`), `LocalNodeId`
/// only has meaning inside the `DetachedArena` that created it.
/// `RewriteBuilder` maps `LocalNodeId` → `NodeId` during import.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LocalNodeId(pub u64);

impl LocalNodeId {
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    pub fn as_u64(self) -> u64 {
        self.0
    }
}

impl fmt::Display for LocalNodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "local#{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_node_id_copy_and_eq() {
        let a = LocalNodeId::new(1);
        let b = a;
        assert_eq!(a, b);
    }

    #[test]
    fn local_node_id_display() {
        assert_eq!(format!("{}", LocalNodeId::new(3)), "local#3");
    }

    #[test]
    fn local_node_id_ordering() {
        let ids: Vec<LocalNodeId> = vec![
            LocalNodeId::new(3),
            LocalNodeId::new(1),
            LocalNodeId::new(2),
        ];
        let mut sorted = ids.clone();
        sorted.sort();
        assert_eq!(
            sorted,
            vec![
                LocalNodeId::new(1),
                LocalNodeId::new(2),
                LocalNodeId::new(3),
            ]
        );
    }
}
