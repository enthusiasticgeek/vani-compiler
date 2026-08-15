#include <stdio.h>

extern long long vani_square(long long n);
extern long long vani_cube(long long n);

int main(void) {
    printf("square(4) = %lld\n", vani_square(4));
    printf("cube(3) = %lld\n", vani_cube(3));
    return 0;
}
