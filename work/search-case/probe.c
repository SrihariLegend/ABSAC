#include <stddef.h>
long find_first(const unsigned char *a, long n, unsigned char c) {
    for (long i = 0; i < n; i++)
        if (a[i] == c) return i;
    return -1;
}
