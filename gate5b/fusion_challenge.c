// Gate 5B — Multi-reduction fusion challenge (simplified benchmark)
#define _GNU_SOURCE
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <sched.h>
#include <immintrin.h>

typedef struct { uint64_t count; uint64_t sum; uint64_t all; } ReductionResult;

static ReductionResult three_loops(const uint8_t *buf, uint64_t n, uint8_t mask, uint8_t target, uint8_t val) {
    ReductionResult r;
    r.count = 0; for (uint64_t i = 0; i < n; i++) r.count += ((buf[i] & mask) == target);
    r.sum = 0; for (uint64_t i = 0; i < n; i++) r.sum += buf[i];
    r.all = 1; for (uint64_t i = 0; i < n; i++) r.all &= (buf[i] == val);
    return r;
}

static ReductionResult three_vector_loops(const uint8_t *buf, uint64_t n, uint8_t mask, uint8_t target, uint8_t val) {
    ReductionResult r;
    __m256i zero = _mm256_setzero_si256();
    r.count = 0;
    { __m256i mv = _mm256_set1_epi8((char)mask), tv = _mm256_set1_epi8((char)target);
      uint64_t i = 0;
      while (i + 32 <= n) { __m256i c = _mm256_loadu_si256((__m256i*)(buf+i));
        r.count += __builtin_popcount(_mm256_movemask_epi8(_mm256_cmpeq_epi8(_mm256_and_si256(c,mv),tv))); i += 32; }
      while (i + 16 <= n) { __m128i c = _mm_loadu_si128((__m128i*)(buf+i));
        r.count += __builtin_popcount(_mm_movemask_epi8(_mm_cmpeq_epi8(_mm_and_si128(c,_mm_set1_epi8((char)mask)),_mm_set1_epi8((char)target)))); i += 16; }
      while (i < n) { r.count += ((buf[i]&mask)==target); i++; } }
    r.sum = 0;
    { __m256i acc = zero; uint64_t i = 0;
      while (i + 32 <= n) { acc = _mm256_add_epi64(acc, _mm256_sad_epu8(_mm256_loadu_si256((__m256i*)(buf+i)), zero)); i += 32; }
      __m128i lo=_mm256_castsi256_si128(acc), hi=_mm256_extracti128_si256(acc,1);
      r.sum = (uint64_t)_mm_cvtsi128_si64(lo)+(uint64_t)_mm_extract_epi64(lo,1)+(uint64_t)_mm_cvtsi128_si64(hi)+(uint64_t)_mm_extract_epi64(hi,1);
      while (i + 16 <= n) { __m128i s=_mm_sad_epu8(_mm_loadu_si128((__m128i*)(buf+i)),_mm_setzero_si128());
        r.sum += (uint64_t)_mm_cvtsi128_si64(s)+(uint64_t)_mm_extract_epi64(s,1); i += 16; }
      while (i < n) { r.sum += buf[i]; i++; } }
    r.all = 1;
    { __m256i vv = _mm256_set1_epi8((char)val); uint64_t i = 0;
      while (i + 32 <= n) { if (_mm256_movemask_epi8(_mm256_cmpeq_epi8(_mm256_loadu_si256((__m256i*)(buf+i)),vv)) != -1) { r.all=0; break; } i += 32; }
      if (r.all) while (i + 16 <= n) { if (_mm_movemask_epi8(_mm_cmpeq_epi8(_mm_loadu_si128((__m128i*)(buf+i)),_mm_set1_epi8((char)val))) != 0xFFFF) { r.all=0; break; } i += 16; }
      if (r.all) while (i < n) { if (buf[i]!=val) { r.all=0; break; } i++; } }
    return r;
}

static ReductionResult fused_vector(const uint8_t *buf, uint64_t n, uint8_t mask, uint8_t target, uint8_t val) {
    ReductionResult r; r.count = 0; r.sum = 0; r.all = 1;
    __m256i zero = _mm256_setzero_si256();
    __m256i mv = _mm256_set1_epi8((char)mask), tv = _mm256_set1_epi8((char)target), vv = _mm256_set1_epi8((char)val);
    __m256i sum_acc = zero;
    uint64_t i = 0;
    while (i + 32 <= n) {
        __m256i chunk = _mm256_loadu_si256((__m256i*)(buf+i)); // ONE load
        r.count += __builtin_popcount(_mm256_movemask_epi8(_mm256_cmpeq_epi8(_mm256_and_si256(chunk,mv),tv)));
        sum_acc = _mm256_add_epi64(sum_acc, _mm256_sad_epu8(chunk, zero));
        if (_mm256_movemask_epi8(_mm256_cmpeq_epi8(chunk,vv)) != -1) r.all = 0;
        i += 32;
    }
    __m128i lo=_mm256_castsi256_si128(sum_acc), hi=_mm256_extracti128_si256(sum_acc,1);
    r.sum = (uint64_t)_mm_cvtsi128_si64(lo)+(uint64_t)_mm_extract_epi64(lo,1)+(uint64_t)_mm_cvtsi128_si64(hi)+(uint64_t)_mm_extract_epi64(hi,1);
    while (i + 16 <= n) {
        __m128i chunk = _mm_loadu_si128((__m128i*)(buf+i));
        r.count += __builtin_popcount(_mm_movemask_epi8(_mm_cmpeq_epi8(_mm_and_si128(chunk,_mm_set1_epi8((char)mask)),_mm_set1_epi8((char)target))));
        __m128i s = _mm_sad_epu8(chunk, _mm_setzero_si128());
        r.sum += (uint64_t)_mm_cvtsi128_si64(s)+(uint64_t)_mm_extract_epi64(s,1);
        if (_mm_movemask_epi8(_mm_cmpeq_epi8(chunk,_mm_set1_epi8((char)val))) != 0xFFFF) r.all = 0;
        i += 16;
    }
    while (i < n) { r.count += ((buf[i]&mask)==target); r.sum += buf[i]; if (buf[i]!=val) r.all=0; i++; }
    return r;
}

static volatile uint64_t g_sink = 0;
static inline double now_sec(void) { struct timespec ts; clock_gettime(CLOCK_MONOTONIC,&ts); return (double)ts.tv_sec+(double)ts.tv_nsec*1e-9; }

#define NTRIALS 25
#define ITERS 20000

#define BENCH(variant) \
    { volatile uint64_t offset = 0; \
      double times[NTRIALS]; \
      for (int t = 0; t < NTRIALS; t++) { \
        double t0 = now_sec(); \
        for (uint64_t i = 0; i < ITERS; i++) { \
          offset = (offset+1)&0xFFF; \
          ReductionResult r = variant(buf+offset, n, mask, target, val); \
          g_sink ^= r.count ^ r.sum ^ r.all; \
        } \
        times[t] = (now_sec()-t0)/(double)ITERS*1e9; \
      } \
      for (int a=0;a<NTRIALS-1;a++) for(int b=a+1;b<NTRIALS;b++) if(times[b]<times[a]){double tmp=times[a];times[a]=times[b];times[b]=tmp;} \
      t_##variant = times[NTRIALS/2]; \
    }

int main(void) {
    cpu_set_t cpuset; CPU_ZERO(&cpuset); CPU_SET(0,&cpuset);
    sched_setaffinity(0, sizeof(cpuset), &cpuset);
    uint8_t *buf = aligned_alloc(64, 1048576);
    uint8_t *eqbuf = aligned_alloc(64, 1048576);
    uint8_t mask=0x0F, target=0x05, val=0x42;

    // Correctness
    printf("=== CORRECTNESS ===\n");
    int errors = 0;
    srand(42); for(int i=0;i<1048576;i++) buf[i]=(uint8_t)(rand()&0xFF);
    for (uint64_t n = 0; n <= 200; n++) {
        ReductionResult ro = three_loops(buf,n,mask,target,val);
        ReductionResult rv = three_vector_loops(buf,n,mask,target,val);
        ReductionResult rf = fused_vector(buf,n,mask,target,val);
        if (rv.count!=ro.count||rv.sum!=ro.sum||rv.all!=ro.all) { printf("FAIL 3vec n=%lu\n",n); errors++; }
        if (rf.count!=ro.count||rf.sum!=ro.sum||rf.all!=ro.all) { printf("FAIL fused n=%lu\n",n); errors++; }
    }
    memset(eqbuf,val,1048576);
    for (uint64_t n = 0; n <= 200; n++) {
        ReductionResult ro = three_loops(eqbuf,n,mask,target,val);
        ReductionResult rf = fused_vector(eqbuf,n,mask,target,val);
        if (rf.count!=ro.count||rf.sum!=ro.sum||rf.all!=ro.all) { printf("FAIL fused(eq) n=%lu\n",n); errors++; }
    }
    if (errors==0) printf("ALL CORRECTNESS TESTS PASSED\n");
    else { printf("%d ERRORS\n",errors); return 1; }

    // Benchmark
    uint64_t sizes[] = {64, 256, 4096, 65536};
    const char *sn[] = {"64","256","4096","65536"};
    printf("\n=== BENCHMARK (HOT CACHE-RESIDENT) ===\n");
    printf("%-12s %-15s %-15s %-15s %-15s %-15s\n","n","orig(ns)","3vec(ns)","fused(ns)","fused/orig","fused/3vec");
    printf("%-12s %-15s %-15s %-15s %-15s %-15s\n","---","-------","-------","---------","----------","----------");

    for (int si = 0; si < 4; si++) {
        uint64_t n = sizes[si];
        double t_three_loops, t_three_vector_loops, t_fused_vector;
        BENCH(three_loops);
        BENCH(three_vector_loops);
        BENCH(fused_vector);
        printf("%-12s %-15.1f %-15.1f %-15.1f %-15.3f %-15.3f\n",
               sn[si], t_three_loops, t_three_vector_loops, t_fused_vector,
               t_fused_vector/t_three_loops, t_fused_vector/t_three_vector_loops);
    }

    // All-equal distribution
    printf("\n--- all_equal distribution (n=4096) ---\n");
    memset(eqbuf, val, 1048576);
    uint64_t n = 4096;
    // Swap buf to eqbuf for this test
    uint8_t *tmp = buf; buf = eqbuf;
    double t_three_loops, t_three_vector_loops, t_fused_vector;
    BENCH(three_loops);
    BENCH(three_vector_loops);
    BENCH(fused_vector);
    printf("%-12s %-15.1f %-15.1f %-15.1f %-15.3f %-15.3f\n",
           "4096(eq)", t_three_loops, t_three_vector_loops, t_fused_vector,
           t_fused_vector/t_three_loops, t_fused_vector/t_three_vector_loops);
    buf = tmp;

    printf("\n=== Gate 5B Analysis ===\n");
    printf("Fused composes: recognize 3 reductions + fuse traversals + select targets.\n");
    printf("No single registered recipe contains this fusion.\n");
    printf("The deterministic pipeline vectorizes each loop independently (3vec).\n");

    free(buf); free(eqbuf);
    return 0;
}
