// Array sum + running max.

#include <stdio.h>

static int running_max(const int *xs, int n) {
    int best = xs[0];
    for (int i = 1; i < n; i++) {
        if (xs[i] > best) {
            best = xs[i];
        }
    }
    return best;
}

static long sum(const int *xs, int n) {
    long total = 0;
    for (int i = 0; i < n; i++) {
        total += xs[i];
    }
    return total;
}

int main(void) {
    int data[] = {3, 1, 4, 1, 5, 9, 2, 6, 5, 3};
    int n = (int)(sizeof(data) / sizeof(data[0]));

    printf("n      = %d\n", n);
    printf("sum    = %ld\n", sum(data, n));
    printf("max    = %d\n", running_max(data, n));
    return 0;
}