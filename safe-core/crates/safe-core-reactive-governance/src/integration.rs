//! Integration traits for UED Teacher and Sparse Router.

use crate::ReactiveLog;
use hash::Hasher;
use std::sync::Arc;

#[async_trait::async_trait]
pub trait UedGovernance<H: Hasher> {
    async fn is_frozen(&self) -> bool;
    async fn get_reward_adjustment(&self, teacher_id: &str) -> f64;
    async fn get_rollback_sth(&self) -> Option<Vec<u8>>;
}

#[async_trait::async_trait]
pub trait SparseRouterGovernance<H: Hasher> {
    async fn is_route_banned(&self, router_id: &str, from_module: &str, to_module: &str) -> bool;
    async fn is_frozen(&self) -> bool;
}

#[async_trait::async_trait]
impl<H: Hasher + Send + Sync> UedGovernance<H> for ReactiveLog<H> {
    async fn is_frozen(&self) -> bool {
        self.is_frozen().await
    }

    async fn get_reward_adjustment(&self, teacher_id: &str) -> f64 {
        self.get_teacher_reward_delta(teacher_id).await
    }

    async fn get_rollback_sth(&self) -> Option<Vec<u8>> {
        self.get_last_rollback_sth().await
    }
}

#[async_trait::async_trait]
impl<H: Hasher + Send + Sync> SparseRouterGovernance<H> for ReactiveLog<H> {
    async fn is_route_banned(&self, router_id: &str, from_module: &str, to_module: &str) -> bool {
        self.is_route_banned(router_id, from_module, to_module).await
    }

    async fn is_frozen(&self) -> bool {
        self.is_frozen().await
    }
}
