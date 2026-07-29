use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct RetentionPolicy {
    pub max_iterations: u32,
    pub max_ttl_seconds: u64,
}

pub struct IsolationBarrier {
    sessions: Arc<RwLock<HashMap<String, crate::secure_context::ContextState>>>,
    proxy: Arc<crate::credential_proxy::CredentialInjectionProxy>,
    audit: Arc<RwLock<crate::audit::AuditTrail>>,
    pub master_key: Vec<u8>,
    pub max_memory: usize,
    max_iterations: u32,
}

impl IsolationBarrier {
    pub fn new(
        proxy: crate::credential_proxy::CredentialInjectionProxy,
        master_key: Vec<u8>,
        max_memory: usize,
        max_iterations: u32,
    ) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            proxy: Arc::new(proxy),
            audit: Arc::new(RwLock::new(crate::audit::AuditTrail::new(10000))),
            master_key,
            max_memory,
            max_iterations,
        }
    }

    pub async fn register_session(
        &self,
        session_id: crate::SessionId,
        context: crate::secure_context::ContextState,
    ) {
        let mut lock = self.sessions.write().await;
        lock.insert(session_id.0.clone(), context);
    }

    pub async fn execute_intent(
        &self,
        session_id: &crate::SessionId,
        invocation: crate::credential_proxy::ToolInvocation,
    ) -> Result<crate::credential_proxy::SecuredRequest, crate::IsolationError> {
        // ESCOPO DE LOCK CURTO: Apenas para validação de estado
        {
            let mut sessions = self.sessions.write().await;

            // Verifica a sessão
            if let Some(session) = sessions.get(&session_id.0) {
                if session.iterations >= self.max_iterations {
                    let iter = session.iterations;
                    sessions.remove(&session_id.0); // Burn
                    return Err(crate::IsolationError::IterationLimitExceeded {
                        iterations: iter,
                        max: self.max_iterations,
                    });
                }

                // INVARIANT CHECK: Isolamento de Namespace
                if invocation.namespace != session.namespace {
                    sessions.remove(&session_id.0); // Burn
                    drop(sessions); // Libera o lock antes de logar
                    self.log_breach(session_id, &invocation.namespace).await;
                    return Err(crate::IsolationError::CrossSessionBreach {
                        session_id: session_id.0.clone(),
                        target_session: format!("{:?}", invocation.namespace),
                    });
                }
            } else {
                return Err(crate::IsolationError::SessionNotFound(session_id.0.clone()));
            }

            // Se chegamos aqui, a sessão é válida
            if let Some(session) = sessions.get_mut(&session_id.0) {
                session.increment_iteration();
            }
        } // <--- O LOCK DO HASHMAP É LIBERADO AQUI ---

        // ESCOPO SEM LOCK: Preparação do request (Pode envolver I/O no futuro)
        let request = self.proxy.prepare_secure_request(&invocation).await?;

        // ESCOPO DE LOCK CURTO: Apenas para auditoria
        self.log_action(
            session_id,
            crate::audit::AuditAction::Inject,
            &format!("Credential injected for tool: {}", invocation.tool_name),
        )
        .await;

        Ok(request)
    }

    async fn log_breach(&self, session_id: &crate::SessionId, _target: &crate::NamespaceId) {
        let mut audit = self.audit.write().await;
        audit.log(crate::audit::AuditRecord {
            timestamp: chrono::Utc::now(),
            session_id: session_id.0.clone(),
            action: crate::audit::AuditAction::Destroy,
            detail: "CROSS-SESSION BREACH: Foreign namespace access".to_string(),
        });
    }

    async fn log_action(
        &self,
        session_id: &crate::SessionId,
        action: crate::audit::AuditAction,
        detail: &str,
    ) {
        let mut audit = self.audit.write().await;
        audit.log(crate::audit::AuditRecord {
            timestamp: chrono::Utc::now(),
            session_id: session_id.0.clone(),
            action,
            detail: detail.to_string(),
        });
    }
}
