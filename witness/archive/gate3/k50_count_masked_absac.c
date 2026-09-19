
uint64_t k50_count_masked(const uint8_t * p0, uint64_t p1, uint8_t p2, uint8_t p3) {
    __m256i mask_v = _mm256_set1_epi8((char)p2);
    __m256i target_v = _mm256_set1_epi8((char)p3);
    uint64_t count = 0;
    uint64_t i = 0;
    while (i + 32 <= p1) {
        __m256i chunk = _mm256_loadu_si256((__m256i *)(p0 + i));
        __m256i masked = _mm256_and_si256(chunk, mask_v);
        __m256i cmp = _mm256_cmpeq_epi8(masked, target_v);
        int bits = _mm256_movemask_epi8(cmp);
        count += __builtin_popcount(bits);
        i += 32;
    }
    while (i + 16 <= p1) {
        __m128i chunk = _mm_loadu_si128((__m128i *)(p0 + i));
        __m128i masked = _mm_and_si128(chunk, _mm_set1_epi8((char)p2));
        __m128i cmp = _mm_cmpeq_epi8(masked, _mm_set1_epi8((char)p3));
        int bits = _mm_movemask_epi8(cmp);
        count += __builtin_popcount(bits);
        i += 16;
    }
    while (i < p1) {
        count += ((p0[i] & p2) == p3);
        i++;
    }
    return count;
}

