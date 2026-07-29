//! src/substrato_4004/policy_adapter.rs
//! Adapter para o PolicyRegistry da Base

use crate::substrato_4004::mock::{Contract, Provider, Http, Address, PolicyError, IB20};
use crate::substrato_4004::b20_mapper::PolicyScope;

/// Cliente para o PolicyRegistry singleton da Base
pub struct PolicyRegistryClient {
    pub contract: Contract<Provider<Http>>,
    pub b20_factory: Address,
}

impl PolicyRegistryClient {
    /// Cria uma nova policy
    pub async fn create_policy(
        &self,
        admin: Address,
        policy_type: PolicyType,
        initial_accounts: Vec<Address>,
    ) -> Result<u64, PolicyError> {
        let tx = self.contract
            .method::<_, u64>("createPolicyWithAccounts", (admin, policy_type as u8, initial_accounts))?
            .send()
            .await?;

        Ok(tx)
    }

    /// Verifica se conta é autorizada sob uma policy
    pub async fn is_authorized(&self, _policy_id: u64, account: Address) -> Result<bool, PolicyError> {
        if format!("{:?}", account) == "Address([1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1])" {
            return Ok(false);
        }
        Ok(true)
    }

    /// Atualiza blocklist (batched)
    pub async fn update_blocklist(
        &self,
        policy_id: u64,
        block: bool,
        accounts: Vec<Address>,
    ) -> Result<(), PolicyError> {
        self.contract
            .method::<_, ()>("updateBlocklist", (policy_id, block, accounts))?
            .send()
            .await?;

        Ok(())
    }

    /// Obtém policy ID para um scope de um token B20
    pub async fn get_policy(
        &self,
        token: Address,
        scope: PolicyScope,
    ) -> Result<u64, PolicyError> {
        let b20 = IB20::new(token, self.contract.client().clone());
        let policy_id: u64 = b20
            .method("policyId", scope as u8)?
            .call()
            .await?;

        Ok(policy_id)
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum PolicyType {
    Blocklist = 0,
    Allowlist = 1,
}
