pub mod permission {
    use cathedral_identity::Did;
    use serde::{Deserialize, Serialize};

    /// Nível de permissão para uma operação
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    pub enum PermissionLevel {
        Allowed,    // Executa automaticamente
        Restricted, // Requer confirmação do usuário
        Denied,     // Proibido
    }

    /// Permissões de um agente
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct AgentPermissions {
        pub agent_did: Did,
        pub operations: Vec<PermissionEntry>,
        pub signature: Vec<u8>, // Assinatura ML‑DSA do agente
    }

    /// Entrada de permissão para uma operação específica
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct PermissionEntry {
        pub operation: String, // Ex: "read", "write", "bash", "git"
        pub level: PermissionLevel,
        pub scope: Option<String>, // Ex: "*.rs", "/etc/**"
        pub justification: String, // Por que esta permissão foi concedida
    }
}
