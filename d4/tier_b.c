// H3 Tier B — LLVM-source corpus (compiled with clang -O1, the freeze flags).
// Fixed-extent (extern global u8[256]) scans: the only lowered shapes that reach
// Any candidate generation. Pointer-param, effectful, and control-shape variants
// are near-miss negatives.
extern unsigned char h3_g_a[256];
extern unsigned char h3_g_b[256];

// h3b01 — any-or over an extern global (positive-shaped; eq-next rotation expected)
unsigned long h3b01_any_or_global(void) {
    unsigned char a = 0;
    for (unsigned long i = 0; i < 256; i++) { a |= h3_g_a[i]; }
    return (a != 0) ? 1 : 0;
}

// h3b02 — predicate any: element > key (param scalar)
unsigned long h3b02_any_gt_global(unsigned char key) {
    unsigned char a = 0;
    for (unsigned long i = 0; i < 256; i++) { a |= (h3_g_a[i] > key); }
    return (a != 0) ? 1 : 0;
}

// h3b03 — predicate any: element == key
unsigned long h3b03_any_eq_global(unsigned char key) {
    unsigned char a = 0;
    for (unsigned long i = 0; i < 256; i++) { a |= (h3_g_a[i] == key); }
    return (a != 0) ? 1 : 0;
}

// h3b04 — predicate any: element < key
unsigned long h3b04_any_lt_global(unsigned char key) {
    unsigned char a = 0;
    for (unsigned long i = 0; i < 256; i++) { a |= (h3_g_a[i] < key); }
    return (a != 0) ? 1 : 0;
}

// h3b05 — near-miss negative: pointer-parameter scan (no fixed extent)
unsigned long h3b05_ptr_scan(const unsigned char *buf, unsigned long n) {
    unsigned char a = 0;
    for (unsigned long i = 0; i < n; i++) { a |= buf[i]; }
    return (a != 0) ? 1 : 0;
}

// h3b06 — near-miss negative: volatile global read inside the loop
unsigned long h3b06_volatile_load(void) {
    volatile unsigned char *g = (volatile unsigned char *)h3_g_a;
    unsigned char a = 0;
    for (unsigned long i = 0; i < 256; i++) { a |= g[i]; }
    return (a != 0) ? 1 : 0;
}

// h3b07 — near-miss negative: store to a global inside the loop
unsigned long h3b07_store_in_loop(unsigned char v) {
    unsigned char a = 0;
    for (unsigned long i = 0; i < 256; i++) { a |= h3_g_a[i]; h3_g_b[i] = v; }
    return (a != 0) ? 1 : 0;
}

// h3b08 — near-miss negative: two sequential Any loops over two globals
unsigned long h3b08_two_loops(void) {
    unsigned char a = 0, b = 0;
    for (unsigned long i = 0; i < 256; i++) { a |= h3_g_a[i]; }
    for (unsigned long j = 0; j < 256; j++) { b |= h3_g_b[j]; }
    return ((a != 0) && (b != 0)) ? 1 : 0;
}

// h3b09 — near-miss negative: reverse traversal of a global
unsigned long h3b09_reverse_scan(void) {
    unsigned char a = 0;
    for (long i = 255; i >= 0; i--) { a |= h3_g_a[i]; }
    return (a != 0) ? 1 : 0;
}

// h3b10 — near-miss negative: early-exit loop (break on first true)
unsigned long h3b10_early_exit(void) {
    for (unsigned long i = 0; i < 256; i++) {
        if (h3_g_a[i] != 0) { return 1; }
    }
    return 0;
}

// h3b11 — near-miss negative: index observed through an out-parameter
unsigned long h3b11_out_param(unsigned char *out_pos) {
    unsigned char a = 0;
    for (unsigned long i = 0; i < 256; i++) {
        if (h3_g_a[i] != 0) { *out_pos = (unsigned char)i; return 1; }
    }
    *out_pos = 255;
    return 0;
}

// h3b12 — near-miss negative: signed-comparison global scan (icmp slt)
extern signed char h3_g_s[256];
long h3b12_signed_lt_zero(void) {
    signed char a = 0;
    for (long i = 0; i < 256; i++) { a |= (h3_g_s[i] < 0); }
    return (a != 0) ? 1 : 0;
}
