#define _GNU_SOURCE
#include "cathedral_ipc.h"
#include "include/cathedral_v7_0.h"
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
