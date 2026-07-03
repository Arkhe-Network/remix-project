use std::collections::HashMap;

/// Cache isolado por Tenant para evitar KV-Cache Side Channels lógicos
pub struct TenantAwareCache<T> {
    partitions: HashMap<String, HashMap<String, T>>,
}

impl<T> Default for TenantAwareCache<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> TenantAwareCache<T> {
    pub fn new() -> Self {
        Self {
            partitions: HashMap::new(),
        }
    }

    pub fn get(&self, tenant_id: &str, key: &str) -> Option<&T> {
        self.partitions.get(tenant_id).and_then(|map| map.get(key))
    }

    pub fn insert(&mut self, tenant_id: &str, key: String, value: T) {
        self.partitions
            .entry(tenant_id.to_string())
            .or_default()
            .insert(key, value);
    }

    /// Invalida todo o cache de um tenant específico (útil em Logout/BAU)
    pub fn invalidate_tenant(&mut self, tenant_id: &str) {
        self.partitions.remove(tenant_id);
    }
}
