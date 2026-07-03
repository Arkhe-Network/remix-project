----------------- MODULE ArkheZeroTrustCredentials -----------------
EXTENDS Integers, Sequences, FiniteSets, TLC

(* Constantes do Modelo *)
CONSTANTS
    Namespaces,    \* Conjunto finito de Tenant Namespaces (ex: {NS_A, NS_B})
    Sessions,      \* Conjunto finito de Sessões (ex: {S_A, S_B})
    Tools,         \* Conjunto finito de Ferramentas (ex: {"github", "slack"})
    Secrets,       \* Conjunto finito de Tokens/Segredos reais (ex: {T1, T2})
    MaxIterations  \* Limite numérico da política BAU

(* Variáveis de Estado *)
VARIABLES
    vault,             \* Estado do cofre proxy: [Namespaces \X Tools -> Secrets \cup {None}]
    active_sessions,   \* Conjunto de sessões válidas no Kernel
    context_session,   \* Mapeamento de contextos ativos para sua sessão dona
    context_ns,        \* Mapeamento de contextos ativos para seu namespace dono
    context_payload,   \* Representação abstrata do conteúdo do contexto (contém segredos ou não)
    context_iter,      \* Contador de iterações BAU de cada contexto
    outbound_requests  \* Registro de requisições de rede montadas pelo proxy

vars == <<vault, active_sessions, context_session, context_ns, context_payload, context_iter, outbound_requests>>

None == "None"
NoSecret == "NoSecret"

(* Estado Inicial *)
Init ==
    && vault \in [Namespaces \X Tools -> Secrets \cup {None}]
    && active_sessions = {}
    && context_session = [s \in Sessions |-> None]
    && context_ns = [s \in Sessions |-> None]
    && context_payload = [s \in Sessions |-> NoSecret]
    && context_iter = [s \in Sessions |-> 0]
    && outbound_requests = {}

(* Transições de Estado *)

\* Registra uma nova sessão legítima vinculada a um Namespace
RegisterSession(s, ns) ==
    && s \notin active_sessions
    && active_sessions' = active_sessions \cup {s}
    && context_session' = [context_session EXCEPT ![s] = s]
    && context_ns' = [context_ns EXCEPT ![s] = ns]
    && context_iter' = [context_iter EXCEPT ![s] = 0]
    && context_payload' = [context_payload EXCEPT ![s] = NoSecret]
    && UNCHANGED <<vault, outbound_requests>>

\* Simula um ataque onde o atacante injeta instruções maliciosas tentando evocar um segredo
InjectContextPayload(s, malicious_payload) ==
    && s \in active_sessions
    && context_payload' = [context_payload EXCEPT ![s] = malicious_payload]
    && UNCHANGED <<vault, active_sessions, context_session, context_ns, context_iter, outbound_requests>>

\* Ação Central do Kernel: Processa e monta a requisição de rede de forma isolada
EnforceAndPrepareOutbound(s, tool) ==
    LET ns == context_ns[s]
        secret_to_inject == vault[ns, tool]
    IN
    && s \in active_sessions
    && context_session[s] = s  \* Invariante de Fronteira: O contexto precisa pertencer à sessão ativa
    && context_iter[s] < MaxIterations
    && secret_to_inject /= None
    \* Execução do Proxy: Constrói a requisição injetando o segredo fora do payload do contexto
    && outbound_requests' = outbound_requests \cup {[target_ns |-> ns, used_secret |-> secret_to_inject]}
    \* Efeito BAU (Burn-After-Use) pós-execução
    && context_iter' = [context_iter EXCEPT ![s] = context_iter[s] + 1]
    && IF context_iter[s] + 1 >= MaxIterations THEN
            \* Expulsa a sessão e limpa/zeroiza os dados imediatamente
            && active_sessions' = active_sessions \ {s}
            && context_session' = [context_session EXCEPT ![s] = None]
            && context_ns' = [context_ns EXCEPT ![s] = None]
            && context_payload' = [context_payload EXCEPT ![s] = NoSecret]
       ELSE
            && UNCHANGED <<active_sessions, context_session, context_ns, context_payload>>
    && UNCHANGED <<vault>>

Next ==
    \/ \E s \in Sessions, ns \in Namespaces : RegisterSession(s, ns)
    \/ \E s \in Sessions, p \in Secrets \cup {NoSecret} : InjectContextPayload(s, p)
    \/ \E s \in Sessions, t \in Tools : EnforceAndPrepareOutbound(s, t)

Spec == Init /\ [][Next]_vars

---------------------------------------------------------------------

(* Invariantes de Segurança Verificáveis *)

\* Tipo Correto (Type OK)
TypeOK ==
    && active_sessions \subseteq Sessions
    && outbound_requests \subseteq [target_ns: Namespaces \cup {None}, used_secret: Secrets \cup {None}]

\* INVARIANTE CRÍTICA 1: O payload do contexto do LLM nunca pode conter o token secreto real bruto
InvContextIsSecretFree ==
    \A s \in Sessions : context_payload[s] \notin Secrets

\* INVARIANTE CRÍTICA 2: Segurança Multi-Tenant Estrita (Sem vazamento cruzado no Proxy)
InvStrictIsolation ==
    \A req \in outbound_requests :
        req.used_secret /= None =>
            \E tool \in Tools : vault[req.target_ns, tool] = req.used_secret

=====================================================================
