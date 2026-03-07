//! MCTS tree node.
//!
//! [`MctsNode`] holds the visit statistics for one player-decision position in
//! the search tree.  A node is *expanded* the first time the policy-value
//! network is evaluated there; before that it is a leaf.

/// One node in the MCTS tree, representing a player-decision position.
///
/// `w` stores the sum of values backed up into this node, always from the
/// perspective of **the player who acts here**.  `q()` therefore also returns
/// a value in `(-1, 1)` from that same perspective.
#[derive(Debug)]
pub struct MctsNode {
    /// Visit count `N(s, a)`.
    pub n: u32,
    /// Sum of backed-up values `W(s, a)` — from **this node's player's** perspective.
    pub w: f32,
    /// Prior probability `P(s, a)` assigned by the policy head (after masked softmax).
    pub p: f32,
    /// Children: `(action_index, child_node)`, populated on first expansion.
    pub children: Vec<(usize, MctsNode)>,
    /// `true` after the network has been evaluated and children have been set up.
    pub expanded: bool,
}

impl MctsNode {
    /// Create a fresh, unexpanded leaf with the given prior probability.
    pub fn new(prior: f32) -> Self {
        Self {
            n: 0,
            w: 0.0,
            p: prior,
            children: Vec::new(),
            expanded: false,
        }
    }

    /// `Q(s, a) = W / N`, or `0.0` if this node has never been visited.
    #[inline]
    pub fn q(&self) -> f32 {
        if self.n == 0 { 0.0 } else { self.w / self.n as f32 }
    }

    /// PUCT selection score:
    ///
    /// ```text
    /// Q(s,a) + c_puct · P(s,a) · √N_parent / (1 + N(s,a))
    /// ```
    #[inline]
    pub fn puct(&self, parent_n: u32, c_puct: f32) -> f32 {
        self.q() + c_puct * self.p * (parent_n as f32).sqrt() / (1.0 + self.n as f32)
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn q_zero_when_unvisited() {
        let node = MctsNode::new(0.5);
        assert_eq!(node.q(), 0.0);
    }

    #[test]
    fn q_reflects_w_over_n() {
        let mut node = MctsNode::new(0.5);
        node.n = 4;
        node.w = 2.0;
        assert!((node.q() - 0.5).abs() < 1e-6);
    }

    #[test]
    fn puct_exploration_dominates_unvisited() {
        // Unvisited child should outscore a visited child with negative Q.
        let mut visited = MctsNode::new(0.5);
        visited.n = 10;
        visited.w = -5.0; // Q = -0.5

        let unvisited = MctsNode::new(0.5);

        let parent_n = 10;
        let c = 1.5;
        assert!(
            unvisited.puct(parent_n, c) > visited.puct(parent_n, c),
            "unvisited child should have higher PUCT than a negatively-valued visited child"
        );
    }
}
