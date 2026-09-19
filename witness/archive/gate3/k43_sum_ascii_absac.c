
uint64_t k43_sum_ascii(const uint8_t * p0, uint64_t p1) {
    __m256i zero = _mm256_setzero_si256();
    __m256i acc = zero;
    uint64_t i = 0;
    while (i + 32 <= p1) {
        __m256i chunk = _mm256_loadu_si256((__m256i *)(p0 + i));
        __m256i sums = _mm256_sad_epu8(chunk, zero);
        acc = _mm256_add_epi64(acc, sums);
        i += 32;
    }
    __m128i lo = _mm256_castsi256_si128(acc);
    __m128i hi = _mm256_extracti128_si256(acc, 1);
    uint64_t sum = (uint64_t)_mm_cvtsi128_si64(lo)
                 + (uint64_t)_mm_extract_epi64(lo, 1)
                 + (uint64_t)_mm_cvtsi128_si64(hi)
                 + (uint64_t)_mm_extract_epi64(hi, 1);
    while (i + 16 <= p1) {
        __m128i chunk = _mm_loadu_si128((__m128i *)(p0 + i));
        __m128i sums = _mm_sad_epu8(chunk, _mm_setzero_si128());
        sum += (uint64_t)_mm_cvtsi128_si64(sums) + (uint64_t)_mm_extract_epi64(sums, 1);
        i += 16;
    }
    while (i < p1) {
        sum += p0[i];
        i++;
    }
    return sum;
}

