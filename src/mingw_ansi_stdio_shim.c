/* BUG-5 / L25: `__mingw_vsnprintf`/`__mingw_vprintf` (MinGW's ANSI/C99-
 * compliant printf-family reimplementation, e.g. 2-digit scientific-
 * notation exponents) live in the static archive `libmingwex.a`.
 * `vanic build`'s AOT link resolves them fine via ordinary static
 * linking, but `lli -load=<path>` only sees symbols exported from an
 * actual shared library. This file's only job is to force the linker
 * to pull those two object files out of the archive so the matching
 * `.def` file can re-export them by their real names -- the LLVM
 * backend's IR keeps declaring/calling `@__mingw_vsnprintf`/
 * `@__mingw_vprintf` unchanged on both the AOT and JIT paths. */
#include <stdio.h>
#include <stdarg.h>

int __mingw_vsnprintf(char *buf, size_t sz, const char *fmt, va_list ap);
int __mingw_vprintf(const char *fmt, va_list ap);

void *__vani_force_link_mingw_ansi_stdio[] = {
    (void *)__mingw_vsnprintf,
    (void *)__mingw_vprintf,
};
