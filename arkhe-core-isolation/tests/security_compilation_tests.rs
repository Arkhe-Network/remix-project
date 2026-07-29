use arkhe_core_isolation::credential_proxy::CredentialInjectionProxy;
use arkhe_core_isolation::isolation_barrier::IsolationBarrier;
use arkhe_core_isolation::secure_context::ContextState;
use arkhe_core_isolation::*;
use secrecy::ExposeSecret;

#[tokio::test]
async fn test_kernel_isolation_and_mitigations() {
    let proxy = CredentialInjectionProxy::new();
    let ns_legit = NamespaceId("TENANT_LE_GRAND".to_string());

    // Armazena credencial sensível de forma encriptada e isolada
    proxy
        .store_credential(
            ns_legit.clone(),
            "github".to_string(),
            "PROD_OAUTH_TOKEN_A_123",
        )
        .await
        .unwrap();

    let barrier = IsolationBarrier::new(proxy, b"key".to_vec(), 100, 2);

    let session_legit = SessionId("SESSION_LEGIT_USER".to_string());
    let session_attacker = SessionId("SESSION_MALICIOUS_USER".to_string());

    let context_legit = ContextState::new(ns_legit.clone(), session_legit.clone(), "List repos");
    let context_attacker = ContextState::new(
        NamespaceId("TENANT_EVIL".to_string()),
        session_attacker.clone(),
        "Evil",
    );

    barrier
        .register_session(session_legit.clone(), context_legit)
        .await;
    barrier
        .register_session(session_attacker.clone(), context_attacker)
        .await;

    // CENÁRIO 1: Usuário Legítimo evoca ferramenta via barreira isolada do Kernel
    let invocation_a = credential_proxy::ToolInvocation {
        namespace: ns_legit.clone(),
        tool_name: "github".to_string(),
        parameters: std::collections::HashMap::new(),
    };
    let request_ok = barrier.execute_intent(&session_legit, invocation_a).await;

    assert!(request_ok.is_ok());
    let req_payload = request_ok.unwrap();
    assert_eq!(
        req_payload.authorization_header.expose_secret(),
        "Bearer PROD_OAUTH_TOKEN_A_123"
    );

    // CENÁRIO 2: Sessão Atacante tenta injetar/passar o contexto de outra sessão na barreira
    let invocation_attack = credential_proxy::ToolInvocation {
        namespace: ns_legit.clone(),
        tool_name: "github".to_string(),
        parameters: std::collections::HashMap::new(),
    };
    let request_breach = barrier
        .execute_intent(&session_attacker, invocation_attack)
        .await;

    // O Kernel do ARKHE OS intercepta a colisão e retorna IsolationBreach imediatamente
    assert!(request_breach.is_err());
    match request_breach {
        Err(IsolationError::CrossSessionBreach { .. }) => {} // Capturado com sucesso
        _ => panic!("Security validation bypassed. Invariant compromised!"),
    }
}
