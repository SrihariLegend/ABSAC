#include <stddef.h>
long find_first(const unsigned char *a, long n, unsigned char c) {
    long i;
    for (i = 0; i < n; i++)
        if (a[i] == c) return i;
    return n;
}
