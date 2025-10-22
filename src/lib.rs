#![cfg_attr(docsrs, feature(doc_cfg))]

/// Extremely fast membership checks for small, static sets of byte sequences.
///
/// The `ByteSet` structure is created by the `byte_set!` macro, which compiles
/// a frequency-ordered trie at compile time. Lookup walks the trie without
/// allocation and short-circuits on the first missing edge.
#[derive(Clone, Copy, Debug)]
pub struct ByteSet {
    nodes: &'static [internal::Node],
    edges: &'static [internal::Edge],
    node_count: usize,
    #[allow(dead_code)]
    edge_count: usize,
    len: usize,
}

impl ByteSet {
    /// Construct a `ByteSet` from a compile-time trie emitted by the macro.
    pub const fn from_trie<const NODES: usize, const EDGES: usize>(
        trie: &'static internal::Trie<NODES, EDGES>,
    ) -> Self {
        Self {
            nodes: &trie.nodes,
            edges: &trie.edges,
            node_count: trie.node_count,
            edge_count: trie.edge_count,
            len: trie.sequence_count,
        }
    }

    /// Number of byte sequences in the set.
    #[inline]
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Returns `true` when the candidate sequence exists in the set.
    #[inline]
    pub fn contains(&self, candidate: &[u8]) -> bool {
        if self.node_count == 0 {
            return false;
        }

        let mut node_idx = 0usize;
        let mut pos = 0usize;

        while pos < candidate.len() {
            if node_idx >= self.node_count {
                return false;
            }

            let node = unsafe { self.nodes.get_unchecked(node_idx) };
            let mut found = false;
            let mut edge_i = node.child_start;
            let end = edge_i + node.child_len;

            while edge_i < end {
                let edge = unsafe { self.edges.get_unchecked(edge_i) };
                if edge.byte == candidate[pos] {
                    node_idx = edge.target;
                    found = true;
                    break;
                }
                edge_i += 1;
            }

            if !found {
                return false;
            }

            pos += 1;
        }

        node_idx < self.node_count && unsafe { self.nodes.get_unchecked(node_idx) }.terminal
    }

    /// Returns `true` when the set is empty.
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    #[cfg(test)]
    pub(crate) fn debug_nodes(&self) -> &[internal::Node] {
        &self.nodes[..self.node_count]
    }

    #[cfg(test)]
    pub(crate) fn debug_edges(&self) -> &[internal::Edge] {
        &self.edges[..self.edge_count]
    }
}

/// Counts the number of expressions passed to the `byte_set!` macro.
#[doc(hidden)]
#[macro_export]
macro_rules! __byte_set_count {
    () => {
        0usize
    };
    ($head:expr $(, $tail:expr)*) => {
        1usize + $crate::__byte_set_count!($($tail),*)
    };
}

/// Build a compile-time byte set from the provided byte string literals.
#[macro_export]
macro_rules! byte_set {
    ( $( $seq:expr ),* $(,)? ) => {{
        const COUNT: usize = $crate::__byte_set_count!($($seq),*);
        const SEQUENCES: [&[u8]; COUNT] = [ $( $seq ),* ];
        const TOTAL_BYTES: usize = $crate::__byte_set_total_bytes(&SEQUENCES);
        const TRIE: $crate::__ByteSetTrie<{TOTAL_BYTES + 1}, {TOTAL_BYTES}> =
            $crate::__byte_set_build_trie::<COUNT, {TOTAL_BYTES + 1}, {TOTAL_BYTES}>(&SEQUENCES);
        $crate::ByteSet::from_trie(&TRIE)
    }};
}

#[allow(dead_code)]
pub(crate) mod internal {
    const NO_EDGE: usize = usize::MAX;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct Node {
        pub terminal: bool,
        pub child_start: usize,
        pub child_len: usize,
    }

    impl Node {
        pub const fn empty() -> Self {
            Self {
                terminal: false,
                child_start: 0,
                child_len: 0,
            }
        }
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct Edge {
        pub byte: u8,
        pub target: usize,
        pub weight: usize,
    }

    impl Edge {
        pub const fn empty() -> Self {
            Self {
                byte: 0,
                target: 0,
                weight: 0,
            }
        }
    }

    #[derive(Clone, Debug)]
    pub struct Trie<const MAX_NODES: usize, const MAX_EDGES: usize> {
        pub nodes: [Node; MAX_NODES],
        pub edges: [Edge; MAX_EDGES],
        pub node_count: usize,
        pub edge_count: usize,
        pub sequence_count: usize,
    }

    impl<const MAX_NODES: usize, const MAX_EDGES: usize> Trie<MAX_NODES, MAX_EDGES> {
        const fn uninit() -> Self {
            Self {
                nodes: [Node::empty(); MAX_NODES],
                edges: [Edge::empty(); MAX_EDGES],
                node_count: 0,
                edge_count: 0,
                sequence_count: 0,
            }
        }
    }

    pub const fn total_bytes<const N: usize>(sequences: &[&[u8]; N]) -> usize {
        let mut sum = 0usize;
        let mut i = 0usize;
        while i < N {
            sum += sequences[i].len();
            i += 1;
        }
        sum
    }

    pub const fn build_trie<const N: usize, const MAX_NODES: usize, const MAX_EDGES: usize>(
        sequences: &[&[u8]; N],
    ) -> Trie<MAX_NODES, MAX_EDGES> {
        let mut trie = Trie::uninit();

        let mut node_terminal = [false; MAX_NODES];
        let mut node_first_child = [NO_EDGE; MAX_NODES];
        let mut node_child_count = [0usize; MAX_NODES];

        let mut edge_byte = [0u8; MAX_EDGES];
        let mut edge_target = [0usize; MAX_EDGES];
        let mut edge_weight = [0usize; MAX_EDGES];
        let mut edge_next = [NO_EDGE; MAX_EDGES];

        let mut node_count = 1usize;
        let mut edge_count = 0usize;

        node_terminal[0] = false;
        node_first_child[0] = NO_EDGE;
        node_child_count[0] = 0;

        let mut seq_idx = 0usize;
        while seq_idx < N {
            let seq = sequences[seq_idx];
            let mut node_idx = 0usize;
            let mut byte_pos = 0usize;
            while byte_pos < seq.len() {
                let byte = seq[byte_pos];
                let mut edge_idx = node_first_child[node_idx];
                let mut found_edge = NO_EDGE;

                while edge_idx != NO_EDGE {
                    if edge_byte[edge_idx] == byte {
                        found_edge = edge_idx;
                        break;
                    }
                    edge_idx = edge_next[edge_idx];
                }

                if found_edge == NO_EDGE {
                    let next_node = node_count;
                    node_count += 1;

                    edge_byte[edge_count] = byte;
                    edge_target[edge_count] = next_node;
                    edge_weight[edge_count] = 1;
                    edge_next[edge_count] = node_first_child[node_idx];
                    node_first_child[node_idx] = edge_count;
                    node_child_count[node_idx] += 1;

                    node_terminal[next_node] = false;
                    node_first_child[next_node] = NO_EDGE;
                    node_child_count[next_node] = 0;

                    node_idx = next_node;
                    edge_count += 1;
                } else {
                    edge_weight[found_edge] += 1;
                    node_idx = edge_target[found_edge];
                }

                byte_pos += 1;
            }

            node_terminal[node_idx] = true;
            seq_idx += 1;
        }

        let mut sorted_edge_bytes = [0u8; MAX_EDGES];
        let mut sorted_edge_targets = [0usize; MAX_EDGES];
        let mut sorted_edge_weights = [0usize; MAX_EDGES];

        let mut node_child_start = [0usize; MAX_NODES];
        let mut scratch_indexes = [0usize; MAX_EDGES];

        let mut current_node = 0usize;
        let mut sorted_edge_count = 0usize;
        while current_node < node_count {
            let start = sorted_edge_count;
            let child_total = node_child_count[current_node];

            let mut list_head = node_first_child[current_node];
            let mut idx = 0usize;
            while list_head != NO_EDGE {
                scratch_indexes[idx] = list_head;
                idx += 1;
                list_head = edge_next[list_head];
            }

            // Selection sort the local edge indexes by weight (desc), byte (asc)
            let mut i = 0usize;
            while i < child_total {
                let mut best = i;
                let mut j = i + 1;
                while j < child_total {
                    let left_idx = scratch_indexes[j];
                    let best_idx = scratch_indexes[best];
                    let left_weight = edge_weight[left_idx];
                    let best_weight = edge_weight[best_idx];
                    if left_weight > best_weight
                        || (left_weight == best_weight && edge_byte[left_idx] < edge_byte[best_idx])
                    {
                        best = j;
                    }
                    j += 1;
                }
                if best != i {
                    let tmp = scratch_indexes[i];
                    scratch_indexes[i] = scratch_indexes[best];
                    scratch_indexes[best] = tmp;
                }
                i += 1;
            }

            let mut k = 0usize;
            while k < child_total {
                let edge_idx = scratch_indexes[k];
                sorted_edge_bytes[sorted_edge_count] = edge_byte[edge_idx];
                sorted_edge_targets[sorted_edge_count] = edge_target[edge_idx];
                sorted_edge_weights[sorted_edge_count] = edge_weight[edge_idx];
                sorted_edge_count += 1;
                k += 1;
            }

            node_child_start[current_node] = start;
            current_node += 1;
        }

        let mut n = 0usize;
        while n < node_count {
            trie.nodes[n] = Node {
                terminal: node_terminal[n],
                child_start: node_child_start[n],
                child_len: node_child_count[n],
            };
            n += 1;
        }

        let mut e = 0usize;
        while e < sorted_edge_count {
            trie.edges[e] = Edge {
                byte: sorted_edge_bytes[e],
                target: sorted_edge_targets[e],
                weight: sorted_edge_weights[e],
            };
            e += 1;
        }

        trie.node_count = node_count;
        trie.edge_count = sorted_edge_count;
        trie.sequence_count = N;
        trie
    }
}

#[doc(hidden)]
pub use internal::{
    Trie as __ByteSetTrie, build_trie as __byte_set_build_trie,
    total_bytes as __byte_set_total_bytes,
};

#[cfg(test)]
mod tests {
    use super::*;

    const EXAMPLE_SET: ByteSet =
        byte_set![b"\xA0\xB1", b"\xA1\xB2", b"\xA1\xB2\xC3", b"\xA1\xB2\xC4",];

    #[test]
    fn finds_present_sequences() {
        assert!(EXAMPLE_SET.contains(b"\xA0\xB1"));
        assert!(EXAMPLE_SET.contains(b"\xA1\xB2"));
        assert!(EXAMPLE_SET.contains(b"\xA1\xB2\xC3"));
        assert!(EXAMPLE_SET.contains(b"\xA1\xB2\xC4"));
    }

    #[test]
    fn rejects_missing_sequences() {
        assert!(!EXAMPLE_SET.contains(b""));
        assert!(!EXAMPLE_SET.contains(b"\xA1"));
        assert!(!EXAMPLE_SET.contains(b"\xA1\xB2\x00"));
        assert!(!EXAMPLE_SET.contains(b"\xA1\xB3"));
        assert!(!EXAMPLE_SET.contains(b"\xA1\xB2\xC5"));
    }

    #[test]
    fn exposes_weighted_trie_structure() {
        let nodes = EXAMPLE_SET.debug_nodes();
        let edges = EXAMPLE_SET.debug_edges();

        assert!(
            nodes.len() >= 3,
            "expected at least three nodes in the trie"
        );
        let root = &nodes[0];
        assert!(
            root.child_len >= 2,
            "root should branch to shared A0 and A1 prefixes"
        );

        let first_child = &edges[root.child_start];
        let second_child = &edges[root.child_start + 1];

        assert_eq!(first_child.byte, 0xA1, "most common prefix should be A1");
        assert_eq!(
            first_child.weight, 3,
            "A1 branch should cover three sequences"
        );
        assert_eq!(
            second_child.byte, 0xA0,
            "less common branch should come second"
        );
        assert_eq!(
            second_child.weight, 1,
            "A0 branch should cover one sequence"
        );

        let shared_node = &nodes[first_child.target];
        assert!(
            shared_node.child_len >= 1,
            "branch A1 should have further children"
        );

        let mut found_b2 = false;
        let mut idx = shared_node.child_start;
        while idx < shared_node.child_start + shared_node.child_len {
            if edges[idx].byte == 0xB2 {
                assert_eq!(
                    edges[idx].weight, 3,
                    "A1B2 should be shared by all remaining sequences"
                );
                found_b2 = true;
                break;
            }
            idx += 1;
        }
        assert!(found_b2, "expected a child edge for byte B2 after A1");
    }
}
