/* Embench-compatible BEEBS random generator support object.
 *
 * Embench's support/beebsc.c defines these two symbols; linking a small
 * independent implementation lets the differential harness test
 * functions that call them without pulling in the whole support library.
 */
static unsigned long sir_beebs_seed = 0;

int rand_beebs(void) {
    sir_beebs_seed =
        (sir_beebs_seed * 1103515245UL + 12345UL) & ((1UL << 31) - 1UL);
    return (int)(sir_beebs_seed >> 16);
}

void srand_beebs(unsigned int new_seed) {
    sir_beebs_seed = (unsigned long)new_seed;
}

/* Called by the harness before each side so stateful support code runs
 * from the same initial state on the reference and emitted sides. */
void __sir_ffi_reset(void) {
    srand_beebs(0);
}
