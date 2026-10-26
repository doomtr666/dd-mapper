use crate::model::NodeId;

/// Hands out node ids from a single map-wide, increasing counter.
#[derive(Debug, Default)]
pub struct NodeIdAllocator {
    next: u32,
}

impl NodeIdAllocator {
    pub fn allocate(&mut self) -> NodeId {
        let id = NodeId(self.next);
        self.next += 1;
        id
    }

    /// The value Dungeondraft expects in `next_node_id`.
    pub fn next_free(&self) -> NodeId {
        NodeId(self.next)
    }
}

#[cfg(test)]
mod tests {
    use super::NodeIdAllocator;
    use crate::model::NodeId;

    #[test]
    fn allocates_increasing_ids_and_reports_the_next_free_one() {
        let mut ids = NodeIdAllocator::default();
        assert_eq!(ids.allocate(), NodeId(0));
        assert_eq!(ids.allocate(), NodeId(1));
        assert_eq!(ids.next_free(), NodeId(2));
    }
}
