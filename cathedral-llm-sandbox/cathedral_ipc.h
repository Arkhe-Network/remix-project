#ifndef CATHEDRAL_IPC_H
#define CATHEDRAL_IPC_H

#include <stdint.h>
#include <stddef.h>

#define MSG_MAX_PAYLOAD 65536  /* 64KB max */
#define MSG_MAGIC       0xCA78E0A1

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
