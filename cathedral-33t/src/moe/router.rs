//! Hierarchical Router para 4096 experts

use crate::tensor::Tensor;
use std::collections::HashSet;

#[derive(Debug, Clone, Copy)]
pub struct RoutingIndex {
    pub expert_id: usize,
    pub weight: f32,
}

pub struct HierarchicalRouter {
    pub num_groups: usize,
    pub experts_per_group: usize,
    pub top_k: usize,
    pub hidden_size: usize,
    group_weights: Tensor,
    expert_weights: Tensor,
}

impl HierarchicalRouter {
    pub fn new(num_experts: usize, top_k: usize, hidden_size: usize) -> Self {
        let num_groups = 64;
        assert_eq!(num_experts % num_groups, 0);
        let experts_per_group = num_experts / num_groups;

        let group_weights = Tensor::randn(&[num_groups, hidden_size]);
        let expert_weights = Tensor::randn(&[num_groups, experts_per_group, hidden_size]);

        Self {
            num_groups,
            experts_per_group,
            top_k,
            hidden_size,
            group_weights,
            expert_weights,
        }
    }

    pub fn route(&self, x: &Tensor) -> Vec<Vec<RoutingIndex>> {
        let batch_size = x.shape()[0];
        let mut routing = Vec::with_capacity(batch_size);

        for i in 0..batch_size {
            let token = x.slice(i);
            let routing_entry = self.route_single(&token);
            routing.push(routing_entry);
        }

        routing
    }

    fn route_single(&self, token: &Tensor) -> Vec<RoutingIndex> {
        let group_logits = token.matmul(&self.group_weights.t());
        let top_groups = self.top_k_unique_indices(&group_logits, 2);

        let mut expert_indices = Vec::with_capacity(self.top_k);
        let mut seen_experts = HashSet::new();

        for (group_idx, _group_weight) in &top_groups {
            let expert_logits = self.compute_expert_logits(token, *group_idx);
            let experts_per_group = (self.top_k + top_groups.len() - 1) / top_groups.len();
            let top_experts = self.top_k_indices(&expert_logits, experts_per_group);

            for (idx, weight) in top_experts {
                let expert_id = group_idx * self.experts_per_group + idx;
                if seen_experts.insert(expert_id) {
                    expert_indices.push(RoutingIndex {
                        expert_id,
                        weight,
                    });
                }
            }
        }

        expert_indices.truncate(self.top_k);

        while expert_indices.len() < self.top_k {
            let random_expert = rand::random::<usize>() % (self.num_groups * self.experts_per_group);
            if seen_experts.insert(random_expert) {
                expert_indices.push(RoutingIndex {
                    expert_id: random_expert,
                    weight: 0.01,
                });
            }
        }

        expert_indices
    }

    fn compute_expert_logits(&self, token: &Tensor, group_idx: usize) -> Tensor {
        let group_weights = self.expert_weights.slice(group_idx);
        token.matmul(&group_weights.t())
    }

    fn top_k_indices(&self, logits: &Tensor, k: usize) -> Vec<(usize, f32)> {
        let vals: Vec<f32> = logits.to_vec();
        let mut indexed: Vec<(f32, usize)> = vals
            .iter()
            .enumerate()
            .map(|(i, &v)| (v, i))
            .collect();
        indexed.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        indexed
            .into_iter()
            .take(k)
            .map(|(v, i)| (i, v))
            .collect()
    }

    fn top_k_unique_indices(&self, logits: &Tensor, k: usize) -> Vec<(usize, f32)> {
        let vals: Vec<f32> = logits.to_vec();
        let mut indexed: Vec<(f32, usize)> = vals
            .iter()
            .enumerate()
            .map(|(i, &v)| (v, i))
            .collect();
        indexed.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        let mut result = Vec::new();
        let mut seen = HashSet::new();

        for (v, i) in indexed {
            if seen.insert(i) {
                result.push((i, v));
                if result.len() >= k {
                    break;
                }
            }
        }

        result
    }

    pub fn num_parameters(&self) -> usize {
        self.num_groups * self.hidden_size
            + self.num_groups * self.experts_per_group * self.hidden_size
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_router_creation() {
        let router = HierarchicalRouter::new(4096, 8, 8192);
        assert_eq!(router.num_groups, 64);
        assert_eq!(router.experts_per_group, 64);
    }

    #[test]
    fn test_route_single_returns_top_k() {
        let router = HierarchicalRouter::new(4096, 8, 16);
        let token = Tensor::randn(&[1, 16]);
        let routing = router.route_single(&token);
        assert_eq!(routing.len(), 8);
        let ids: Vec<usize> = routing.iter().map(|r| r.expert_id).collect();
        let unique_ids: HashSet<_> = ids.iter().copied().collect();
        assert_eq!(unique_ids.len(), 8);
    }

    #[test]
    fn test_route_returns_top_k() {
        let router = HierarchicalRouter::new(4096, 8, 16);
        let token = Tensor::randn(&[3, 16]);
        let routing = router.route(&token);
        assert_eq!(routing.len(), 3);
        assert_eq!(routing[0].len(), 8);
        assert_eq!(routing[1].len(), 8);
        assert_eq!(routing[2].len(), 8);
    }
}