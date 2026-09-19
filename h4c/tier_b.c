// H4c Tier B — fresh blind LLVM-source corpus (authored from the documented
// C3 frontier + D4 remediation record; distinct kernels from h3/h4b).
// Compiled with the C3 freeze flags:
//   clang -O1 -emit-llvm -S h4c/tier_b.c -o h4c/tier_b.ll

extern unsigned char h4c_g_a[96];
extern unsigned char h4c_g_b[160];
extern unsigned char h4c_g_c[24];
extern unsigned char h4c_g_d[96];
extern signed char h4c_g_s[96];

// h4c01 — any-or fixed-extent scan, extent 96 (positive)
unsigned long h4c01_any_or_global96(void) {
    unsigned char a = 0;
    for (unsigned long i = 0; i < 96; i++) { a |= h4c_g_a[i]; }
    return (a != 0) ? 1 : 0;
}

// h4c02 — predicate lt param, extent 160 (positive)
unsigned long h4c02_pred_lt_global160(unsigned char key) {
    unsigned char a = 0;
    for (unsigned long i = 0; i < 160; i++) { a |= (h4c_g_b[i] < key); }
    return (a != 0) ? 1 : 0;
}

// h4c03 — predicate gt literal, extent 24 (positive)
unsigned long h4c03_pred_gt_const_global24(void) {
    unsigned char a = 0;
    for (unsigned long i = 0; i < 24; i++) { a |= (h4c_g_c[i] > 0x40); }
    return (a != 0) ? 1 : 0;
}

// h4c04 — near-miss: pointer-parameter scan (runtime bound)
unsigned long h4c04_ptr_scan(const unsigned char *buf, unsigned long n) {
    unsigned char a = 0;
    for (unsigned long i = 0; i < n; i++) { a |= buf[i]; }
    return (a != 0) ? 1 : 0;
}

// h4c05 — near-miss: reverse traversal, must abstain cleanly (no panic)
unsigned long h4c05_reverse_scan96(void) {
    unsigned char a = 0;
    for (long i = 95; i >= 0; i--) { a |= h4c_g_a[i]; }
    return (a != 0) ? 1 : 0;
}

// h4c06 — near-miss: partial bound over a 96-byte global (bound 32 != extent 96)
unsigned long h4c06_partial_bound_global96(void) {
    unsigned char a = 0;
    for (unsigned long i = 0; i < 32; i++) { a |= h4c_g_a[i]; }
    return (a != 0) ? 1 : 0;
}

// h4c07 — near-miss: two sequential loops over globals of different extents
unsigned long h4c07_two_loops_diff_extents(void) {
    unsigned char a = 0, b = 0;
    for (unsigned long i = 0; i < 96; i++) { a |= h4c_g_a[i]; }
    for (unsigned long j = 0; j < 24; j++) { b |= h4c_g_c[j]; }
    return ((a != 0) && (b != 0)) ? 1 : 0;
}

// h4c08 — near-miss: signed-comparison scan
unsigned long h4c08_signed_lt_zero(void) {
    signed char a = 0;
    for (long i = 0; i < 96; i++) { a |= (h4c_g_s[i] < 0); }
    return (a != 0) ? 1 : 0;
}

// h4c09 — SAFETY near-miss (S2 class): predicate scalar derived from the
// induction counter; must never commit a rewrite
unsigned long h4c09_index_dependent_predicate(void) {
    unsigned char a = 0;
    for (unsigned long i = 0; i < 96; i++) { a |= (h4c_g_a[i] > (unsigned char)i); }
    return (a != 0) ? 1 : 0;
}

// h4c10 — near-miss: store to a global inside the loop
unsigned long h4c10_store_in_loop(unsigned char v) {
    unsigned char a = 0;
    for (unsigned long i = 0; i < 96; i++) { a |= h4c_g_a[i]; h4c_g_d[i] = v; }
    return (a != 0) ? 1 : 0;
}
