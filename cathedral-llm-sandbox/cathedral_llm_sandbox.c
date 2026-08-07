#define _GNU_SOURCE
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <sys/socket.h>
#include <sys/prctl.h>
#include <linux/seccomp.h>
#include <linux/filter.h>
#include <linux/audit.h>
#include <linux/landlock.h>
#include <sys/syscall.h>
#include <fcntl.h>
#include <sched.h>
#include <sys/mount.h>
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
        fprintf(stderr, "Failed to enter namespaces: %s\n", strerror(errno));
        return 1;
    }

    /* Layer 3: Landlock */
    if (install_landlock(model_path) < 0) {
        fprintf(stderr, "Failed to install Landlock: %s\n", strerror(errno));
        return 1;
    }

    /* Layer 4: seccomp-bpf */
    if (install_seccomp() < 0) {
        fprintf(stderr, "Failed to install seccomp: %s\n", strerror(errno));
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

#ifndef landlock_create_ruleset
static inline int landlock_create_ruleset(
    const struct landlock_ruleset_attr *const attr,
    const size_t size, const __u32 flags) {
    return syscall(__NR_landlock_create_ruleset, attr, size, flags);
}
#endif

#ifndef landlock_add_rule
static inline int landlock_add_rule(const int ruleset_fd,
    const enum landlock_rule_type rule_type,
    const void *const rule_attr, const __u32 flags) {
    return syscall(__NR_landlock_add_rule, ruleset_fd, rule_type, rule_attr, flags);
}
#endif

#ifndef landlock_restrict_self
static inline int landlock_restrict_self(const int ruleset_fd,
    const __u32 flags) {
    return syscall(__NR_landlock_restrict_self, ruleset_fd, flags);
}
#endif

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
