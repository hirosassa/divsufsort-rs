/* Minimal config.h for vendor build (non-cmake path) */
#ifndef _CONFIG_H
#define _CONFIG_H 1

#define HAVE_STRING_H  1
#define HAVE_STDLIB_H  1
#define HAVE_STDDEF_H  1
#define HAVE_STDINT_H  1
#define HAVE_INTTYPES_H 1

/* Inline keyword used by sssort.c / trsort.c */
#define INLINE __inline__

/* printf format specifier for saidx_t (32-bit int) */
#ifndef PRIdSAIDX_T
#define PRIdSAIDX_T "d"
#endif
#ifndef PRIdSAINT_T
#define PRIdSAINT_T "d"
#endif

#endif /* _CONFIG_H */
