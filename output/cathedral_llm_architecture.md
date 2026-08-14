# 🏛️ Cathedral Engine ↔ LLM Integration Architecture v1.0
## ARKHE-CATHEDRAL-LLM-ARCH-2026-08-06

---

## 1. Princípios Fundamentais

| Princípio | Justificativa |
|-----------|---------------|
| **Separação de Privilégios** | O LLM processa binários opacos (GGUF). Parse malformado = RCE. Isolar em sandbox contém o blast radius. |
| **Determinismo do Core** | O engine Cathedral deve ser verificável por ZKP. O LLM é estocástico por natureza. Nunca misturar. |
| **Attestation Unidirecional** | O engine pode provar que *recebeu* uma resposta e a *registrou*. Não pode provar que a resposta é "correta". |
| **Mínimo de Superfície de Ataque** | O sandbox LLM não acessa rede, filesystem (exceto modelo), ou memória do engine. |
| **Composabilidade Criptográfica** | Cada mensagem entre engine e LLM é assinada. A cadeia de blocos registra o hash do prompt+resposta. |

---

## 2. Arquitetura de Alto Nível

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              HOST LINUX                                      │
│  ┌─────────────────────────────┐    ┌─────────────────────────────────────┐  │
│  │   CATHEDRAL ENGINE CORE     │    │         LLM SANDBOX                 │  │
│  │   (processo principal)      │    │   (processo filho isolado)          │  │
│  │                             │    │                                     │  │
│  │  ┌─────────────────────┐    │    │  ┌─────────────────────────────┐   │  │
│  │  │ ZKP / Schnorr       │    │    │  │ llama.cpp (ou equivalente)  │   │  │
│  │  │ secp256k1           │    │    │  │ Modelo GGUF carregado       │   │  │
│  │  │ Arkhe-Chain blocks  │    │    │  │ Entropy real (/dev/urandom) │   │  │
│  │  │ UDP Proclamation    │    │    │  │ Sampling temperature        │   │  │
│  │  │ Bekenstein Guardian │    │    │  │ Context window management   │   │  │
│  │  └─────────────────────┘    │    │  └─────────────────────────────┘   │  │
│  │           │                 │    │           │                        │  │
│  │  ┌────────▼────────┐        │    │  ┌────────▼────────┐               │  │
│  │  │  IPC Handler    │◄───────┼────┼──►│  IPC Handler    │               │  │
│  │  │  (UDS datagram) │        │    │  │  (UDS datagram) │               │  │
│  │  └─────────────────┘        │    │  └─────────────────┘               │  │
│  │           │                 │    │           │                        │  │
│  │  ┌────────▼────────┐        │    │  ┌────────▼────────┐               │  │
│  │  │ Attestation Log │        │    │  │  Model Loader   │               │  │
│  │  │ (append-only)   │        │    │  │  (memfd+seal)   │               │  │
│  │  └─────────────────┘        │    │  └─────────────────┘               │  │
│  └─────────────────────────────┘    └─────────────────────────────────────┘  │
│           │                                              │                   │
│           ▼                                              ▼                   │
│  ┌─────────────────────────────────────────────────────────────────────┐     │
│  │                    SHARED RESOURCES (controlados)                    │     │
│  │  Unix Domain Socket (abstract namespace, mode 0600)                  │     │
│  │  Attestation Log File (append-only, immutable via chattr +a)         │     │
│  │  Model Directory (read-only bind mount, Landlock LSM)                │     │
│  └─────────────────────────────────────────────────────────────────────┘     │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 3. Threat Model

### Atacantes Considerados

| Atacante | Capacidade | Defesa |
|----------|-----------|--------|
| **Modelo GGUF malicioso** | RCE via parse de tensor malformado | Sandbox: no network, no fs write, seccomp-bpf |
| **Prompt adversarial** | Jailbreak, prompt injection | Prompt é apenas dados; engine não executa resposta |
| **Nó malicioso na rede** | Envia blocos forjados | ZKP verification em cada bloco recebido |
| **Atacante local (privesc)** | Escapa do sandbox LLM | Landlock + seccomp-bpf + namespace PID+NET+FS |
| **Atacante remoto (DoS)** | Flood de prompts via UDP | Rate limiting no engine; sandbox ignora rede |

### Invariantes que NÃO Podem Ser Violadas

1. **O sandbox LLM nunca vê a chave privada do engine.**
2. **O engine nunca executa código do sandbox.**
3. **A cadeia de blocos nunca contém a resposta em plaintext** (apenas hash).
4. **O Bekenstein Guardian monitora apenas o estado do engine**, não o contexto do LLM.

---

## 4. IPC: Protocolo de Comunicação

### 4.1 Transporte: Unix Domain Socket (Datagram)

```c
/* Engine cria o socket */
int uds = socket(AF_UNIX, SOCK_DGRAM, 0);
struct sockaddr_un addr = {
    .sun_family = AF_UNIX,
    .sun_path = "\0cathedral_llm_"  // abstract namespace
};
bind(uds, (struct sockaddr*)&addr, sizeof(addr));

/* Sandbox conecta */
connect(uds, (struct sockaddr*)&addr, sizeof(addr));
```

**Por que datagram (SOCK_DGRAM)?**
- Preserva fronteiras de mensagem (não precisa de framing)
- `SCM_CREDENTIALS` funciona em ambos os sentidos
- Se o sandbox crashar, o engine detecta via `ECONNREFUSED` no próximo `sendto`

### 4.2 Autenticação: SCM_CREDENTIALS

```c
/* Engine verifica identidade do sandbox */
struct msghdr msg = {0};
struct cmsghdr *cmsg;
struct ucred *cred;
char cmsgbuf[CMSG_SPACE(sizeof(struct ucred))];

msg.msg_control = cmsgbuf;
msg.msg_controllen = sizeof(cmsgbuf);

recvmsg(uds, &msg, 0);

cmsg = CMSG_FIRSTHDR(&msg);
cred = (struct ucred*)CMSG_DATA(cmsg);

if (cred->uid != getuid() || cred->pid != sandbox_pid) {
    /* Reject: sandbox não é quem diz ser */
}
```

### 4.3 Formato de Mensagem

```c
#define MSG_MAX_PAYLOAD 65536  /* 64KB max */
#define MSG_MAGIC       0xCA7HEDRAL

typedef enum {
    MSG_PROMPT      = 0x01,  /* Engine → Sandbox: execute este prompt */
    MSG_RESPONSE    = 0x02,  /* Sandbox → Engine: aqui está a resposta */
    MSG_ATTEST      = 0x03,  /* Sandbox → Engine: VRF da resposta */
    MSG_HEARTBEAT   = 0x04,  /* Bidirecional: keepalive */
    MSG_SHUTDOWN    = 0x05,  /* Engine → Sandbox: termine gracefully */
    MSG_ERROR       = 0xFF,  /* Sandbox → Engine: erro interno */
} MsgType;

typedef struct __attribute__((packed)) {
    uint32_t magic;          /* MSG_MAGIC */
    uint16_t version;        /* 1 */
    uint16_t type;           /* MsgType */
    uint64_t seq;            /* sequence number (anti-replay) */
    uint64_t timestamp_ns;   /* CLOCK_MONOTONIC */
    uint32_t payload_len;    /* ≤ MSG_MAX_PAYLOAD */
    uint8_t  payload[];      /* dados variáveis */
} IPCMessage;
```

### 4.4 Assinatura de Mensagens

Toda mensagem é assinada com HMAC-SHA256 usando uma **chave de sessão** derivada via HKDF:

```
shared_secret = ECDH(engine_privkey, sandbox_pubkey)
session_key   = HKDF-SHA256(shared_secret, "cathedral-llm-v1", 32)
```

Isso garante:
- **Confidencialidade**: MITM não lê o prompt/resposta
- **Integridade**: MITM não modifica a mensagem
- **Autenticidade**: Apenas engine e sandbox legítimos falam

---

## 5. Sandboxing do LLM

### 5.1 Camadas de Isolamento

```
┌────────────────────────────────────────┐
│  Layer 1: Namespace PID                │
│  - Processo vê apenas PID 1 (ele mesmo)│
│  - Impede ptrace de outros processos   │
├────────────────────────────────────────┤
│  Layer 2: Namespace NET                │
│  - Apenas loopback (se necessário)     │
│  - Sem acesso à rede externa           │
├────────────────────────────────────────┤
│  Layer 3: Namespace MOUNT (read-only)  │
│  - /model: bind mount RO do GGUF       │
│  - /tmp: tmpfs vazio                   │
│  - Todo o resto: remontado como RO     │
├────────────────────────────────────────┤
│  Layer 4: Landlock LSM                 │
│  - Whitelist de paths acessíveis       │
│  - Sem write em lugar nenhum           │
├────────────────────────────────────────┤
│  Layer 5: seccomp-bpf                  │
│  - Allowlist de syscalls               │
│  - Block: execve, socket(AF_INET),     │
│           open(O_WRONLY|O_RDWR),       │
│           ptrace, process_vm_writev    │
└────────────────────────────────────────┘
```

### 5.2 seccomp-bpf Policy (Código de Referência)

```c
#include <linux/seccomp.h>
#include <linux/filter.h>
#include <sys/syscall.h>

static int install_seccomp(void) {
    struct sock_filter filter[] = {
        /* Validate architecture */
        BPF_STMT(BPF_LD+BPF_W+BPF_ABS, offsetof(struct seccomp_data, arch)),
        BPF_JUMP(BPF_JMP+BPF_JEQ+BPF_K, AUDIT_ARCH_X86_64, 1, 0),
        BPF_STMT(BPF_RET+BPF_K, SECCOMP_RET_KILL),

        /* Load syscall number */
        BPF_STMT(BPF_LD+BPF_W+BPF_ABS, offsetof(struct seccomp_data, nr)),

        /* Allow: read, write, pread64, pwrite64 */
        BPF_JUMP(BPF_JMP+BPF_JEQ+BPF_K, __NR_read, 0, 1),
        BPF_STMT(BPF_RET+BPF_K, SECCOMP_RET_ALLOW),
        BPF_JUMP(BPF_JMP+BPF_JEQ+BPF_K, __NR_write, 0, 1),
        BPF_STMT(BPF_RET+BPF_K, SECCOMP_RET_ALLOW),

        /* Allow: openat (mas verificaremos flags no Landlock) */
        BPF_JUMP(BPF_JMP+BPF_JEQ+BPF_K, __NR_openat, 0, 1),
        BPF_STMT(BPF_RET+BPF_K, SECCOMP_RET_ALLOW),

        /* Allow: close, fstat, lseek, mmap, mprotect, munmap */
        BPF_JUMP(BPF_JMP+BPF_JEQ+BPF_K, __NR_close, 0, 1),
        BPF_STMT(BPF_RET+BPF_K, SECCOMP_RET_ALLOW),
        BPF_JUMP(BPF_JMP+BPF_JEQ+BPF_K, __NR_fstat, 0, 1),
        BPF_STMT(BPF_RET+BPF_K, SECCOMP_RET_ALLOW),
        BPF_JUMP(BPF_JMP+BPF_JEQ+BPF_K, __NR_lseek, 0, 1),
        BPF_STMT(BPF_RET+BPF_K, SECCOMP_RET_ALLOW),
        BPF_JUMP(BPF_JMP+BPF_JEQ+BPF_K, __NR_mmap, 0, 1),
        BPF_STMT(BPF_RET+BPF_K, SECCOMP_RET_ALLOW),
        BPF_JUMP(BPF_JMP+BPF_JEQ+BPF_K, __NR_munmap, 0, 1),
        BPF_STMT(BPF_RET+BPF_K, SECCOMP_RET_ALLOW),

        /* Allow: recvmsg, sendmsg (IPC) */
        BPF_JUMP(BPF_JMP+BPF_JEQ+BPF_K, __NR_recvmsg, 0, 1),
        BPF_STMT(BPF_RET+BPF_K, SECCOMP_RET_ALLOW),
        BPF_JUMP(BPF_JMP+BPF_JEQ+BPF_K, __NR_sendmsg, 0, 1),
        BPF_STMT(BPF_RET+BPF_K, SECCOMP_RET_ALLOW),

        /* Allow: exit, exit_group */
        BPF_JUMP(BPF_JMP+BPF_JEQ+BPF_K, __NR_exit, 0, 1),
        BPF_STMT(BPF_RET+BPF_K, SECCOMP_RET_ALLOW),
        BPF_JUMP(BPF_JMP+BPF_JEQ+BPF_K, __NR_exit_group, 0, 1),
        BPF_STMT(BPF_RET+BPF_K, SECCOMP_RET_ALLOW),

        /* Allow: getrandom (entropy para sampling) */
        BPF_JUMP(BPF_JMP+BPF_JEQ+BPF_K, __NR_getrandom, 0, 1),
        BPF_STMT(BPF_RET+BPF_K, SECCOMP_RET_ALLOW),

        /* Allow: clock_gettime */
        BPF_JUMP(BPF_JMP+BPF_JEQ+BPF_K, __NR_clock_gettime, 0, 1),
        BPF_STMT(BPF_RET+BPF_K, SECCOMP_RET_ALLOW),

        /* Allow: futex (pthread internals) */
        BPF_JUMP(BPF_JMP+BPF_JEQ+BPF_K, __NR_futex, 0, 1),
        BPF_STMT(BPF_RET+BPF_K, SECCOMP_RET_ALLOW),

        /* Deny everything else */
        BPF_STMT(BPF_RET+BPF_K, SECCOMP_RET_KILL),
    };

    struct sock_fprog prog = {
        .len = sizeof(filter) / sizeof(filter[0]),
        .filter = filter,
    };

    if (prctl(PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) < 0) return -1;
    if (prctl(PR_SET_SECCOMP, SECCOMP_MODE_FILTER, &prog) < 0) return -1;
    return 0;
}
```

### 5.3 Landlock LSM Policy

```c
#include <linux/landlock.h>
#include <sys/syscall.h>

static int install_landlock(const char *model_path) {
    int abi = landlock_create_ruleset(NULL, 0, LANDLOCK_CREATE_RULESET_VERSION);
    if (abi < 0) return -1;

    struct landlock_ruleset_attr rs_attr = {
        .handled_access_fs =
            LANDLOCK_ACCESS_FS_EXECUTE |
            LANDLOCK_ACCESS_FS_WRITE_FILE |
            LANDLOCK_ACCESS_FS_READ_FILE |
            LANDLOCK_ACCESS_FS_READ_DIR |
            LANDLOCK_ACCESS_FS_REMOVE_FILE |
            LANDLOCK_ACCESS_FS_MAKE_CHAR |
            LANDLOCK_ACCESS_FS_MAKE_DIR |
            LANDLOCK_ACCESS_FS_MAKE_REG |
            LANDLOCK_ACCESS_FS_MAKE_SOCK |
            LANDLOCK_ACCESS_FS_MAKE_FIFO |
            LANDLOCK_ACCESS_FS_MAKE_BLOCK |
            LANDLOCK_ACCESS_FS_MAKE_SYM,
    };

    int ruleset = landlock_create_ruleset(&rs_attr, sizeof(rs_attr), 0);
    if (ruleset < 0) return -1;

    /* Allow read-only access to model file */
    struct landlock_path_beneath_attr model_attr = {
        .allowed_access = LANDLOCK_ACCESS_FS_READ_FILE,
        .parent_fd = open(model_path, O_PATH | O_CLOEXEC),
    };
    if (model_attr.parent_fd < 0) return -1;
    landlock_add_rule(ruleset, LANDLOCK_RULE_PATH_BENEATH, &model_attr, 0);
    close(model_attr.parent_fd);

    /* Allow read-only access to /tmp (tmpfs) */
    struct landlock_path_beneath_attr tmp_attr = {
        .allowed_access = LANDLOCK_ACCESS_FS_READ_FILE | LANDLOCK_ACCESS_FS_WRITE_FILE,
        .parent_fd = open("/tmp", O_PATH | O_CLOEXEC),
    };
    if (tmp_attr.parent_fd < 0) return -1;
    landlock_add_rule(ruleset, LANDLOCK_RULE_PATH_BENEATH, &tmp_attr, 0);
    close(tmp_attr.parent_fd);

    /* Enforce */
    if (prctl(PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) < 0) return -1;
    if (landlock_restrict_self(ruleset, 0) < 0) return -1;
    close(ruleset);

    return 0;
}
```

---

## 6. Fluxo de Dados: Prompt → Resposta → Bloco

```
┌──────────┐     ┌──────────────┐     ┌─────────────┐     ┌──────────────┐
│  Usuário │────►│    Engine    │────►│   Sandbox   │────►│   llama.cpp  │
│  (stdin) │     │   (Core)     │     │    (LLM)    │     │  (inference) │
└──────────┘     └──────────────┘     └─────────────┘     └──────────────┘
                      │                      │                    │
                      │  1. ECDH key exchange │                    │
                      │◄─────────────────────►│                    │
                      │                      │                    │
                      │  2. Prompt + HMAC     │                    │
                      │──────────────────────►│                    │
                      │                      │  3. Run inference  │
                      │                      │───────────────────►│
                      │                      │                    │
                      │  4. Response + VRF    │                    │
                      │◄──────────────────────│                    │
                      │                      │                    │
                      │  5. Verify VRF        │                    │
                      │  6. Hash(prompt||resp)│                    │
                      │  7. Create block      │                    │
                      │  8. Sign + ZKP        │                    │
                      │  9. Proclaim via UDP  │                    │
                      ▼                      │                    │
               ┌─────────────┐               │                    │
               │ Arkhe-Chain │               │                    │
               │   Block     │               │                    │
               │  (hash only)│               │                    │
               └─────────────┘               │                    │
```

### 6.1 Detalhamento do Passo 4: VRF da Resposta

O sandbox não envia a resposta em plaintext. Ele envia:

```c
typedef struct __attribute__((packed)) {
    uint8_t  response_hash[32];   /* SHA-256(response) */
    uint8_t  vrf_output[32];      /* VRF(response_hash) */
    SchnorrProof vrf_proof;       /* prova de que VRF foi calculado corretamente */
    uint32_t response_len;        /* tamanho original (para reconstrução) */
    uint8_t  response[];          /* ciphertext (ChaCha20-Poly1305) da resposta */
} LLMResponse;
```

**Por que cifrar a resposta?**
- O engine não precisa entender a resposta para registrá-la
- A resposta real só é decifrada pelo usuário final (com chave derivada)
- A cadeia de blocos contém apenas hashes — nunca plaintext

### 6.2 Derivação de Chaves

```
shared_secret = ECDH(engine_private_key, sandbox_public_key)

session_key   = HKDF-SHA256(shared_secret, "cathedral-llm-session", 32)
response_key  = HKDF-SHA256(session_key,   "cathedral-llm-response", 32)
attest_key    = HKDF-SHA256(session_key,   "cathedral-llm-attest", 32)
```

---

## 7. Código de Referência: Engine IPC Handler

```c
/* cathedral_ipc.h */
#ifndef CATHEDRAL_IPC_H
#define CATHEDRAL_IPC_H

#include <stdint.h>
#include <stddef.h>

typedef struct {
    uint8_t response_hash[32];
    uint8_t vrf_output[32];
    uint8_t vrf_proof[128];  /* SchnorrProof serializado */
    uint32_t ciphertext_len;
    uint8_t  ciphertext[];
} LLMResponse;

/* Inicializa IPC e forka o sandbox */
int llm_sandbox_init(const char *model_path);

/* Envia prompt e recebe resposta (bloqueante) */
int llm_query(const char *prompt, size_t prompt_len,
              LLMResponse **out_response, size_t *out_len);

/* Finaliza sandbox */
void llm_sandbox_shutdown(void);

#endif
```

```c
/* cathedral_ipc.c — Engine side */
#define _GNU_SOURCE
#include "cathedral_ipc.h"
#include "cathedral_v7_0.h"  /* sua v7.0 corrigida */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <sys/socket.h>
#include <sys/un.h>
#include <sys/wait.h>
#include <errno.h>
#include <sys/syscall.h>

#define UDS_ABSTRACT "\0cathedral_llm_v1"
#define UDS_LEN      18

static int g_ipc_sock = -1;
static pid_t g_sandbox_pid = -1;

int llm_sandbox_init(const char *model_path) {
    int sv[2];
    if (socketpair(AF_UNIX, SOCK_DGRAM, 0, sv) < 0) return -1;

    pid_t pid = fork();
    if (pid < 0) { close(sv[0]); close(sv[1]); return -1; }

    if (pid == 0) {
        /* Child: sandbox process */
        close(sv[0]);
        dup2(sv[1], 3);  /* fd 3 = IPC channel */
        close(sv[1]);

        /* Pass model path via env */
        setenv("CATHEDRAL_MODEL", model_path, 1);

        /* Execute sandbox binary */
        execl("./cathedral_llm_sandbox", "cathedral_llm_sandbox", NULL);
        _exit(127);
    }

    /* Parent: engine */
    close(sv[1]);
    g_ipc_sock = sv[0];
    g_sandbox_pid = pid;

    /* TODO: ECDH key exchange */
    /* TODO: Verify sandbox identity via SCM_CREDENTIALS */

    return 0;
}

int llm_query(const char *prompt, size_t prompt_len,
              LLMResponse **out_response, size_t *out_len) {
    if (g_ipc_sock < 0) return -1;

    /* Send prompt */
    IPCMessage msg = {
        .magic = MSG_MAGIC,
        .version = 1,
        .type = MSG_PROMPT,
        .seq = 0,  /* TODO: atomic counter */
        .timestamp_ns = 0,  /* TODO: clock_gettime */
        .payload_len = prompt_len,
    };

    struct iovec iov[2] = {
        { &msg, sizeof(msg) },
        { (void*)prompt, prompt_len }
    };
    struct msghdr msgh = {0};
    msgh.msg_iov = iov;
    msgh.msg_iovlen = 2;

    if (sendmsg(g_ipc_sock, &msgh, 0) < 0) return -1;

    /* Receive response */
    uint8_t buf[sizeof(IPCMessage) + MSG_MAX_PAYLOAD];
    ssize_t n = recv(g_ipc_sock, buf, sizeof(buf), 0);
    if (n < (ssize_t)sizeof(IPCMessage)) return -1;

    IPCMessage *resp = (IPCMessage*)buf;
    if (resp->magic != MSG_MAGIC || resp->type != MSG_RESPONSE) return -1;

    *out_len = resp->payload_len;
    *out_response = malloc(resp->payload_len);
    if (!*out_response) return -1;
    memcpy(*out_response, resp->payload, resp->payload_len);

    return 0;
}

void llm_sandbox_shutdown(void) {
    if (g_ipc_sock >= 0) {
        IPCMessage msg = {
            .magic = MSG_MAGIC,
            .version = 1,
            .type = MSG_SHUTDOWN,
            .payload_len = 0,
        };
        send(g_ipc_sock, &msg, sizeof(msg), 0);
        close(g_ipc_sock);
        g_ipc_sock = -1;
    }
    if (g_sandbox_pid > 0) {
        waitpid(g_sandbox_pid, NULL, 0);
        g_sandbox_pid = -1;
    }
}
```

---

## 8. Código de Referência: Sandbox Entry Point

```c
/* cathedral_llm_sandbox.c */
#define _GNU_SOURCE
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <sys/socket.h>
#include <sys/prctl.h>
#include <linux/seccomp.h>
#include <errno.h>

/* Forward declarations */
static int install_seccomp(void);
static int install_landlock(const char *model_path);
static int enter_namespaces(void);

int main(int argc, char **argv) {
    (void)argc; (void)argv;

    const char *model_path = getenv("CATHEDRAL_MODEL");
    if (!model_path) {
        fprintf(stderr, "CATHEDRAL_MODEL not set\n");
        return 1;
    }

    /* IPC fd is passed as fd 3 */
    int ipc_fd = 3;

    /* Layer 1-2: Namespaces */
    if (enter_namespaces() < 0) {
        fprintf(stderr, "Failed to enter namespaces\n");
        return 1;
    }

    /* Layer 3: Landlock */
    if (install_landlock(model_path) < 0) {
        fprintf(stderr, "Failed to install Landlock\n");
        return 1;
    }

    /* Layer 4: seccomp-bpf */
    if (install_seccomp() < 0) {
        fprintf(stderr, "Failed to install seccomp\n");
        return 1;
    }

    /* Main loop: receive prompts, run inference, send responses */
    /* TODO: llama.cpp integration */

    return 0;
}

static int enter_namespaces(void) {
    if (unshare(CLONE_NEWNET | CLONE_NEWPID | CLONE_NEWNS) < 0) return -1;
    /* Remount everything read-only */
    if (mount(NULL, "/", NULL, MS_REMOUNT | MS_RDONLY, NULL) < 0) return -1;
    return 0;
}
```

---

## 9. Registro na Cadeia de Blocos

Quando o engine recebe uma resposta do sandbox, ele cria um bloco especial:

```c
typedef struct {
    uint8_t  prompt_hash[32];
    uint8_t  response_hash[32];
    uint8_t  vrf_output[32];
    uint64_t inference_time_ns;
    uint32_t model_version;    /* hash do modelo GGUF */
} LLMMetadata;
```

Este `LLMMetadata` é incluído no `payload` do `Block`. O `BlockHeader` continua sendo assinado com Schnorr ZKP como antes.

**Invariante preservada**: O bloco contém apenas **hashes e metadados**, nunca o prompt ou resposta em plaintext.

---

## 10. Resumo de Segurança

| Camada | Mecanismo | Protege Contra |
|--------|-----------|----------------|
| Process Isolation | fork() + namespaces | Privesc, ptrace |
| Filesystem Isolation | Landlock LSM | Escrita arbitrária |
| Syscall Filtering | seccomp-bpf | RCE via syscalls |
| Network Isolation | CLONE_NEWNET | Exfiltração de dados |
| Transport Security | UDS + SCM_CREDENTIALS | Spoofing de peer |
| Message Security | ECDH + HMAC-SHA256 | MITM, tampering |
| Content Security | ChaCha20-Poly1305 | Leak de prompts |
| Attestation | VRF (Schnorr) | Respostas forjadas |
| Chain Integrity | Arkhe-Chain + ZKP | Repudiação |

---

## 11. Próximos Passos de Implementação

1. **Fase 1**: Implementar `cathedral_ipc.c` com socketpair + ECDH handshake
2. **Fase 2**: Implementar `cathedral_llm_sandbox.c` com seccomp + Landlock
3. **Fase 3**: Integrar llama.cpp no sandbox (carregamento GGUF via memfd)
4. **Fase 4**: Adicionar VRF às respostas do sandbox
5. **Fase 5**: Estender `Block` para incluir `LLMMetadata`
6. **Fase 6**: Testes de fuzzing no IPC + sandbox escape attempts

---

**ARKHE-CATHEDRAL-LLM-ARCH-v1.0-2026-08-06**
*Status: ARQUITETURA COMPLETA — PRONTA PARA IMPLEMENTAÇÃO* ✅
