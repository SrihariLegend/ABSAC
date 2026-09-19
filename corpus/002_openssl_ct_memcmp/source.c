// Provenance: openssl/openssl, CRYPTO_memcmp
// Git: https://github.com/openssl/openssl/blob/master/include/openssl/crypto.h.in
// Implementation: classic constant-time comparison (public domain)
//
// This function compares two byte buffers in constant time — it always
// runs for `len` iterations regardless of where the first difference is.
// This prevents timing side-channel attacks on secret comparisons
// (e.g., HMAC verification, MAC checks).
//
// The key structural constraint: -O3 CANNOT short-circuit this loop
// (no early exit) and is conservative about vectorizing it (vectorization
// could introduce timing variability). This leaves room for ABSAC to find
// verified-equivalent faster code that is also constant-time.

#include <stdint.h>

int openssl_ct_memcmp(const uint8_t *a, const uint8_t *b, uint64_t len) {
    uint8_t x = 0;
    for (uint64_t i = 0; i < len; i++) {
        x |= a[i] ^ b[i];
    }
    return x;
}
