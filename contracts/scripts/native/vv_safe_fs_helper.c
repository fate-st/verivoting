/* Local, fail-closed dirFD writer. This is not a same-UID security sandbox.
 * Build: cc -std=c11 -Wall -Wextra -Werror -O2 vv_safe_fs_helper.c -o vv_safe_fs_helper
 * stdin is the exact file payload; stdout contains readiness/completion records.
 */
#define _DARWIN_C_SOURCE
#include <sys/stat.h>
#include <sys/types.h>
#include <errno.h>
#include <fcntl.h>
#include <inttypes.h>
#include <limits.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <unistd.h>

#define INTEGRATION "/private/tmp/vv-pk-reuse-integration.PohVG2/worktree"
#define ORIGINAL "/Users/lijian/Documents/02学习/论文/latex模版/experiment/verivoting-vscode-starter"
#define RAW INTEGRATION "/experiments/stage1_multibatch_raw/gate-a-20261117-08"
#define RESULTS INTEGRATION "/results/stage1_multibatch/gate-a-20261117-08"
#define TEST_PREFIX "/private/tmp/vv-a08-path-guard-test-"

static void fail(const char *operation) {
    int e = errno;
    fprintf(stderr, "VV_SAFE_FS_REJECT:%s:errno=%d:%s\n", operation, e, strerror(e));
    exit(1);
}
static void require(int ok, const char *message) {
    if (!ok) { errno = EINVAL; fail(message); }
}
static int within(const char *p, const char *r) {
    size_t n = strlen(r);
    return strncmp(p, r, n) == 0 && (p[n] == '\0' || p[n] == '/');
}
static int safe_component(const char *s, int ascii) {
    if (!*s || strcmp(s, ".") == 0 || strstr(s, "..")) return 0;
    for (const unsigned char *p = (const unsigned char *)s; *p; ++p) {
        if (*p == '/' || *p < 32 || *p == 127) return 0;
        if (ascii && !((*p >= 'a' && *p <= 'z') || (*p >= 'A' && *p <= 'Z') ||
                       (*p >= '0' && *p <= '9') || *p == '.' || *p == '_' || *p == '-')) return 0;
    }
    return 1;
}
static void check_path(const char *p, int absolute) {
    require(p && *p && strlen(p) < PATH_MAX, "PATH_LENGTH");
    require(absolute ? p[0] == '/' : p[0] != '/', "PATH_KIND");
    require(!strstr(p, "//") && p[strlen(p)-1] != '/', "EMPTY_COMPONENT");
    char *copy = strdup(p + (absolute ? 1 : 0));
    require(copy != NULL, "ALLOCATION");
    char *save = NULL;
    for (char *c = strtok_r(copy, "/", &save); c; c = strtok_r(NULL, "/", &save))
        require(safe_component(c, !absolute), "UNSAFE_COMPONENT");
    free(copy);
}
static uintmax_t number(const char *s) {
    require(s && *s && *s >= '0' && *s <= '9', "BAD_INTEGER");
    char *end = NULL; errno = 0;
    uintmax_t n = strtoumax(s, &end, 10);
    require(!errno && end && !*end, "BAD_INTEGER");
    return n;
}
static int directory_at(int fd, const char *name, int create) {
    int next = openat(fd, name, O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC);
    if (next < 0 && errno == ENOENT && create) {
        if (mkdirat(fd, name, 0700) < 0) fail("mkdirat");
        if (fsync(fd) < 0) fail("fsync_created_directory_parent");
        next = openat(fd, name, O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC);
    }
    if (next < 0) fail("openat_directory_NOFOLLOW");
    struct stat st;
    if (fstat(next, &st) < 0) fail("fstat_directory");
    require(S_ISDIR(st.st_mode), "NOT_DIRECTORY");
    return next;
}
/* Never open an unchecked absolute directory in one call. */
static int anchor_open(const char *path) {
    int fd = open("/", O_RDONLY | O_DIRECTORY | O_CLOEXEC);
    if (fd < 0) fail("open_root");
    char *copy = strdup(path + 1), *save = NULL;
    require(copy != NULL, "ALLOCATION");
    for (char *c = strtok_r(copy, "/", &save); c; c = strtok_r(NULL, "/", &save)) {
        int next = directory_at(fd, c, 0);
        close(fd); fd = next;
    }
    free(copy); return fd;
}
static void production_policy(const char *anchor, const char *registered, const char *target) {
    require(!strstr(anchor, "vv-a08-path-guard-test-") &&
            !strstr(registered, "vv-a08-path-guard-test-"), "PROD_REFUSES_TEST_ROOT");
    int admitted = strcmp(registered, RAW) == 0 || strcmp(registered, RESULTS) == 0;
    const char *keys[] = {"VV_INTEGRATION_TARGET_DIR", "CARGO_TARGET_DIR", "CARGO_HOME", "TMPDIR"};
    for (size_t i = 0; i < sizeof(keys)/sizeof(keys[0]); ++i) {
        const char *v = getenv(keys[i]);
        if (v && strcmp(v, registered) == 0) admitted = 1;
    }
    require(admitted, "UNREGISTERED_ROOT");
    require(within(registered, "/private/tmp") && strcmp(registered, "/private/tmp") != 0,
            "INDEPENDENT_PRIVATE_TMP_ROOT_REQUIRED");
    if (within(registered, INTEGRATION))
        require(strcmp(registered, RAW) == 0 || strcmp(registered, RESULTS) == 0, "FROZEN_OR_SOURCE_TREE");
    require(!within(registered, ORIGINAL), "ORIGINAL_READ_ONLY");
    const char *rustup = getenv("RUSTUP_HOME");
    require(rustup && *rustup && !within(registered, rustup) && !within(rustup, registered), "RUSTUP_READ_ONLY");
    require(within(registered, anchor), "ANCHOR_NOT_ANCESTOR_OF_REGISTERED_ROOT");
    char full[PATH_MAX];
    require(snprintf(full, sizeof(full), "%s/%s", anchor, target) < (int)sizeof(full), "TARGET_LENGTH");
    require(within(full, registered), "TARGET_OUTSIDE_REGISTERED_ROOT");
    /* Anchor must be the nearest existing registered-root prefix, not an arbitrary broad root. */
    if (strcmp(anchor, registered) != 0) {
        const char *suffix = registered + strlen(anchor) + 1;
        char first[PATH_MAX]; snprintf(first, sizeof(first), "%s", suffix);
        char *slash = strchr(first, '/'); if (slash) *slash = '\0';
        int a = anchor_open(anchor); struct stat st;
        int rc = fstatat(a, first, &st, AT_SYMLINK_NOFOLLOW); int e = errno; close(a);
        require(rc < 0 && e == ENOENT, "ANCHOR_MUST_BE_NEAREST_EXISTING_PREFIX");
    }
}
int main(int argc, char **argv) {
    const char *mode = "PROD", *anchor = NULL, *registered = NULL, *target = NULL;
    const char *dev = NULL, *ino = NULL, *pause_arg = NULL;
    int executable = 0;
    for (int i = 1; i < argc; ++i) {
        require(i + 1 < argc, "MISSING_ARGUMENT_VALUE");
        const char *key = argv[i], *value = argv[++i];
        if (!strcmp(key, "--mode")) mode = value;
        else if (!strcmp(key, "--allowed-root")) anchor = value;
        else if (!strcmp(key, "--registered-root")) registered = value;
        else if (!strcmp(key, "--target-relpath")) target = value;
        else if (!strcmp(key, "--expected-dev")) dev = value;
        else if (!strcmp(key, "--expected-ino")) ino = value;
        else if (!strcmp(key, "--test-pause-before-file-open-ms")) pause_arg = value;
        else if (!strcmp(key, "--executable")) { require(!strcmp(value, "true"), "EXECUTABLE_VALUE"); executable = 1; }
        else require(0, "UNKNOWN_ARGUMENT");
    }
    int test = !strcmp(mode, "TEST_ONLY");
    require(test || !strcmp(mode, "PROD"), "INVALID_MODE");
    require(test || !pause_arg, "PROD_REFUSES_TEST_PARAMETERS");
    check_path(anchor, 1); check_path(registered, 1); check_path(target, 0);
    uintmax_t expected_dev = number(dev), expected_ino = number(ino);
    uintmax_t pause_ms = pause_arg ? number(pause_arg) : 0;
    require(pause_ms <= 10000, "TEST_PAUSE_LIMIT");
    if (test) {
        require(strncmp(anchor, TEST_PREFIX, strlen(TEST_PREFIX)) == 0 &&
                strncmp(registered, TEST_PREFIX, strlen(TEST_PREFIX)) == 0, "TEST_ONLY_ROOT_REQUIRED");
        require(within(registered, anchor), "TEST_ANCHOR_SCOPE");
        char full[PATH_MAX]; require(snprintf(full, sizeof(full), "%s/%s", anchor, target) < (int)sizeof(full), "TARGET_LENGTH");
        require(within(full, registered), "TEST_TARGET_SCOPE");
    } else production_policy(anchor, registered, target);
    if (executable) {
        const char *ending = "/build-target/release/stage1_multibatch_materials";
        size_t t = strlen(target), n = strlen(ending);
        require(!strcmp(target, ending + 1) || (t > n && !strcmp(target + t - n, ending)), "EXECUTABLE_OUTPUT_SCOPE");
    }
    int fd = anchor_open(anchor); struct stat identity;
    if (fstat(fd, &identity) < 0) fail("fstat_anchor");
    require((uintmax_t)identity.st_dev == expected_dev && (uintmax_t)identity.st_ino == expected_ino,
            "ANCHOR_IDENTITY_MISMATCH");
    char *copy = strdup(target), *save = NULL;
    require(copy != NULL, "ALLOCATION");
    char *name = strtok_r(copy, "/", &save);
    for (;;) {
        char *next = strtok_r(NULL, "/", &save);
        if (!next) break;
        int child = directory_at(fd, name, 1); close(fd); fd = child; name = next;
    }
    struct stat parent;
    if (fstat(fd, &parent) < 0) fail("fstat_parent");
    printf("{\"event\":\"PARENT_OPEN\",\"anchor_dev\":\"%ju\",\"anchor_ino\":\"%ju\",\"parent_dev\":\"%ju\",\"parent_ino\":\"%ju\"}\n",
           (uintmax_t)identity.st_dev, (uintmax_t)identity.st_ino, (uintmax_t)parent.st_dev, (uintmax_t)parent.st_ino);
    fflush(stdout);
    if (pause_ms) {
        struct timespec delay = {(time_t)(pause_ms/1000), (long)(pause_ms%1000)*1000000};
        while (nanosleep(&delay, &delay) < 0) if (errno != EINTR) fail("nanosleep");
    }
    int file = openat(fd, name, O_CREAT | O_EXCL | O_NOFOLLOW | O_WRONLY | O_CLOEXEC, 0600);
    if (file < 0) fail("openat_file_EXCL_NOFOLLOW");
    puts("{\"event\":\"FILE_OPEN\"}"); fflush(stdout);
    unsigned char buffer[65536]; ssize_t n; uintmax_t bytes = 0;
    while ((n = read(STDIN_FILENO, buffer, sizeof(buffer))) != 0) {
        if (n < 0) { if (errno == EINTR) continue; fail("read_stdin"); }
        for (ssize_t offset = 0; offset < n;) {
            ssize_t count = write(file, buffer + offset, (size_t)(n-offset));
            if (count < 0) { if (errno == EINTR) continue; fail("write_file"); }
            require(count > 0, "ZERO_WRITE"); offset += count; bytes += (uintmax_t)count;
        }
    }
    /* An archived executable is created 0600, then made owner-executable through this same FD. */
    if (executable && fchmod(file, 0700) < 0) fail("fchmod_archived_executable");
    if (fsync(file) < 0) fail("fsync_file");
    if (fsync(fd) < 0) fail("fsync_parent");
    close(file); close(fd); free(copy);
    printf("{\"event\":\"WRITTEN\",\"bytes\":%ju,\"anchor_dev\":\"%ju\",\"anchor_ino\":\"%ju\",\"parent_ino\":\"%ju\"}\n",
           bytes, (uintmax_t)identity.st_dev, (uintmax_t)identity.st_ino, (uintmax_t)parent.st_ino);
    return 0;
}
