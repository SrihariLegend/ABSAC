// H4 Tier B — fresh LLVM-source corpus, blind-authored from the documented
// C3 frontier and D4 remediation record. Compiled with the C3 freeze flags:
//   clang -O1 -emit-llvm -S h4/tier_b.c -o h4/tier_b.ll
// Positive-shaped kernels are fixed-extent extern-global scans (clang
// eq-next rotation); negatives cover the documented near-miss classes.

extern unsigned char h4_g_a[64];
extern unsigned char h4_g_b[64];
extern unsigned char h4_g_c[128];
extern unsigned char h4_g_d[32];
extern signed char h4_g_s[64];

// h4b01 — any-or over a fixed-extent extern global (positive)
unsigned long h4b01_any_or_global64(void) {
    unsigned char a = 0;
    for (unsigned long i = 0; i < 64; i++) { a |= h4_g_a[i]; }
    return (a != 0) ? 1 : 0;
}

// h4b02 — predicate any: element >= key (positive)
unsigned long h4b02_pred_ge_global128(unsigned char key) {
    unsigned char a = 0;
    for (unsigned long i = 0; i < 128; i++) { a |= (h4_g_c[i] >= key); }
    return (a != 0) ? 1 : 0;
}

// h4b03 — predicate any: element != literal (positive)
unsigned long h4b03_pred_ne_const64(void) {
    unsigned char a = 0;
    for (unsigned long i = 0; i < 64; i++) { a |= (h4_g_a[i] != 0x5A); }
    return (a != 0) ? 1 : 0;
}

// h4b04 — predicate any: element <= key (positive)
unsigned long h4b04_pred_le_global32(unsigned char key) {
    unsigned char a = 0;
    for (unsigned long i = 0; i < 32; i++) { a |= (h4_g_d[i] <= key); }
    return (a != 0) ? 1 : 0;
}

// h4b05 — near-miss negative: pointer-parameter scan (no fixed extent)
unsigned long h4b05_ptr_scan(const unsigned char *buf, unsigned long n) {
    unsigned char a = 0;
    for (unsigned long i = 0; i < n; i++) { a |= buf[i]; }
    return (a != 0) ? 1 : 0;
}

// h4b06 — near-miss negative: volatile global read inside the loop
unsigned long h4b06_volatile_load(void) {
    volatile unsigned char *g = (volatile unsigned char *)h4_g_a;
    unsigned char a = 0;
    for (unsigned long i = 0; i < 64; i++) { a |= g[i]; }
    return (a != 0) ? 1 : 0;
}

// h4b07 — near-miss negative: store to a global inside the loop
unsigned long h4b07_store_in_loop(unsigned char v) {
    unsigned char a = 0;
    for (unsigned long i = 0; i < 64; i++) { a |= h4_g_a[i]; h4_g_b[i] = v; }
    return (a != 0) ? 1 : 0;
}

// h4b08 — near-miss negative: two sequential Any loops over two globals
unsigned long h4b08_two_loops(void) {
    unsigned char a = 0, b = 0;
    for (unsigned long i = 0; i < 64; i++) { a |= h4_g_a[i]; }
    for (unsigned long j = 0; j < 64; j++) { b |= h4_g_b[j]; }
    return ((a != 0) && (b != 0)) ? 1 : 0;
}

// h4b09 — near-miss negative: reverse traversal of a global (no panic allowed)
unsigned long h4b09_reverse_scan(void) {
    unsigned char a = 0;
    for (long i = 63; i >= 0; i--) { a |= h4_g_a[i]; }
    return (a != 0) ? 1 : 0;
}

// h4b10 — near-miss negative: early-exit loop (break on first true)
unsigned long h4b10_early_exit(void) {
    for (unsigned long i = 0; i < 64; i++) {
        if (h4_g_a[i] != 0) { return 1; }
    }
    return 0;
}

// h4b11 — near-miss negative: index observed through an out-parameter
unsigned long h4b11_out_param(unsigned char *out_pos) {
    unsigned char a = 0;
    for (unsigned long i = 0; i < 64; i++) {
        if (h4_g_a[i] != 0) { *out_pos = (unsigned char)i; return 1; }
    }
    *out_pos = 63;
    return 0;
}

// h4b12 — near-miss negative: signed-comparison global scan
unsigned long h4b12_signed_lt_zero(void) {
    signed char a = 0;
    for (long i = 0; i < 64; i++) { a |= (h4_g_s[i] < 0); }
    return (a != 0) ? 1 : 0;
}

// h4b13 — SAFETY near-miss: predicate scalar derived from the induction
// counter (S2 class); must never commit a rewrite
unsigned long h4b13_index_dependent_predicate(void) {
    unsigned char a = 0;
    for (unsigned long i = 0; i < 64; i++) { a |= (h4_g_a[i] > (unsigned char)i); }
    return (a != 0) ? 1 : 0;
}
