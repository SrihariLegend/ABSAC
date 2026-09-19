
uint64_t k18_all_equal(const uint8_t * p0, uint64_t p1, uint8_t p2) {
    __m256i target_v = _mm256_set1_epi8((char)p2);
    uint64_t i = 0;
    while (i + 32 <= p1) {
        __m256i chunk = _mm256_loadu_si256((__m256i *)(p0 + i));
        __m256i cmp = _mm256_cmpeq_epi8(chunk, target_v);
        int mask = _mm256_movemask_epi8(cmp);
        if (mask != -1) return 0;
        i += 32;
    }
    while (i + 16 <= p1) {
        __m128i chunk = _mm_loadu_si128((__m128i *)(p0 + i));
        __m128i cmp = _mm_cmpeq_epi8(chunk, _mm_set1_epi8((char)p2));
        int mask = _mm_movemask_epi8(cmp);
        if (mask != 0xFFFF) return 0;
        i += 16;
    }
    while (i < p1) {
        if (p0[i] != p2) return 0;
        i++;
    }
    return 1;
}

