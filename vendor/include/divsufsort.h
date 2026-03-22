/*
 * divsufsort.h for libdivsufsort
 * Copyright (c) 2003-2008 Yuta Mori All Rights Reserved.
 * MIT License
 *
 * Generated from divsufsort.h.cmake for 32-bit saidx_t (int).
 */

#ifndef _DIVSUFSORT_H
#define _DIVSUFSORT_H 1

#ifdef __cplusplus
extern "C" {
#endif /* __cplusplus */

#ifndef DIVSUFSORT_API
# define DIVSUFSORT_API
#endif

/*- Datatypes -*/
#ifndef SAUCHAR_T
#define SAUCHAR_T
typedef unsigned char sauchar_t;
#endif
#ifndef SAINT_T
#define SAINT_T
typedef int saint_t;
#endif
#ifndef SAIDX_T
#define SAIDX_T
typedef int saidx_t;
#endif

/*- Prototypes -*/

DIVSUFSORT_API
saint_t divsufsort(const sauchar_t *T, saidx_t *SA, saidx_t n);

DIVSUFSORT_API
saidx_t divbwt(const sauchar_t *T, sauchar_t *U, saidx_t *A, saidx_t n);

DIVSUFSORT_API
const char *divsufsort_version(void);

DIVSUFSORT_API
saint_t bw_transform(const sauchar_t *T, sauchar_t *U,
                     saidx_t *SA, saidx_t n, saidx_t *idx);

DIVSUFSORT_API
saint_t inverse_bw_transform(const sauchar_t *T, sauchar_t *U,
                              saidx_t *A, saidx_t n, saidx_t idx);

DIVSUFSORT_API
saint_t sufcheck(const sauchar_t *T, const saidx_t *SA, saidx_t n, saint_t verbose);

DIVSUFSORT_API
saidx_t sa_search(const sauchar_t *T, saidx_t Tsize,
                  const sauchar_t *P, saidx_t Psize,
                  const saidx_t *SA, saidx_t SAsize,
                  saidx_t *left);

DIVSUFSORT_API
saidx_t sa_simplesearch(const sauchar_t *T, saidx_t Tsize,
                        const saidx_t *SA, saidx_t SAsize,
                        saint_t c, saidx_t *left);

#ifdef __cplusplus
} /* extern "C" */
#endif /* __cplusplus */

#endif /* _DIVSUFSORT_H */
