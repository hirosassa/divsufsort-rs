use alloc::format;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use crate::DivSufSortError;
use crate::constants::ALPHABET_SIZE;

/// Error returned by [`sufcheck`] when the suffix array is found to be invalid.
#[derive(Debug, PartialEq, Eq)]
pub struct SufCheckError {
    /// Human-readable description of the first inconsistency found.
    pub message: String,
}

impl core::fmt::Display for SufCheckError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "sufcheck: {}", self.message)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for SufCheckError {}

/// Verifies that `sa` is the correct suffix array of `t`.
///
/// Checks that all SA entries are in range, that suffixes are lexicographically ordered,
/// and that the inverse SA mapping is consistent.
///
/// If `verbose` is `true`, diagnostic messages are printed to stderr.
///
/// # Errors
///
/// Returns [`SufCheckError`] describing the first inconsistency found.
pub fn sufcheck(t: &[u8], sa: &[i32], #[allow(unused)] verbose: bool) -> Result<(), SufCheckError> {
    let n = t.len();

    if n == 0 {
        #[cfg(feature = "std")]
        if verbose {
            std::eprintln!("sufcheck: Done.");
        }
        return Ok(());
    }

    if sa.len() != n {
        let msg = format!("SA length {} != T length {}", sa.len(), n);
        #[cfg(feature = "std")]
        if verbose {
            std::eprintln!("sufcheck: {msg}");
        }
        return Err(SufCheckError { message: msg });
    }

    for (i, &s) in sa.iter().enumerate().take(n) {
        if s < 0 || s as usize >= n {
            let msg = format!("Out of the range [0,{}]. SA[{}]={}", n - 1, i, s);
            #[cfg(feature = "std")]
            if verbose {
                std::eprintln!("sufcheck: {msg}");
            }
            return Err(SufCheckError { message: msg });
        }
    }

    for i in 1..n {
        if t[sa[i - 1] as usize] > t[sa[i] as usize] {
            let msg = format!(
                "Suffixes in wrong order. T[SA[{}]={}]={} > T[SA[{}]={}]={}",
                i - 1,
                sa[i - 1],
                t[sa[i - 1] as usize],
                i,
                sa[i],
                t[sa[i] as usize]
            );
            #[cfg(feature = "std")]
            if verbose {
                std::eprintln!("sufcheck: {msg}");
            }
            return Err(SufCheckError { message: msg });
        }
    }

    let mut c_table = [0i32; ALPHABET_SIZE];
    for &ch in t {
        c_table[ch as usize] += 1;
    }
    let mut p = 0i32;
    for entry in c_table.iter_mut() {
        let t_val = *entry;
        *entry = p;
        p += t_val;
    }

    let q = c_table[t[n - 1] as usize];
    c_table[t[n - 1] as usize] += 1;

    for i in 0..n {
        let sai = sa[i];
        let (c, t_val) = if sai > 0 {
            let c = t[(sai - 1) as usize] as usize;
            (c, c_table[c])
        } else {
            let c = t[n - 1] as usize;
            (c, q)
        };

        if t_val < 0 || sa[t_val as usize] != (if sai > 0 { sai - 1 } else { (n - 1) as i32 }) {
            let msg = format!(
                "Suffix in wrong position. SA[{}]={} or SA[{}]={}",
                t_val,
                if t_val >= 0 { sa[t_val as usize] } else { -1 },
                i,
                sa[i]
            );
            #[cfg(feature = "std")]
            if verbose {
                std::eprintln!("sufcheck: {msg}");
            }
            return Err(SufCheckError { message: msg });
        }

        if t_val != q {
            c_table[c] += 1;
            if c_table[c] as usize >= n || t[sa[c_table[c] as usize] as usize] as usize != c {
                c_table[c] = -1;
            }
        }
    }

    #[cfg(feature = "std")]
    if verbose {
        std::eprintln!("sufcheck: Done.");
    }
    Ok(())
}

fn binarysearch_lower(a: &[i32], mut size: i32, value: i32) -> i32 {
    let mut i = 0i32;
    let mut half = size >> 1;
    while 0 < size {
        if a[(i + half) as usize] < value {
            i += half + 1;
            half -= (size & 1) ^ 1;
        }
        size = half;
        half >>= 1;
    }
    i
}

/// Computes the Burrows-Wheeler Transform of `t`, writing the result into `u`.
///
/// If `sa` is `Some`, the provided suffix array is used directly. If `sa` is `None`,
/// the transform is computed via [`crate::divbwt`].
///
/// On success, `*idx` is set to the primary index (1-based).
///
/// # Errors
///
/// Returns [`DivSufSortError::InvalidArgument`] if arguments are inconsistent.
pub fn bw_transform(
    t: &[u8],
    u: &mut [u8],
    sa: Option<&mut [i32]>,
    idx: &mut i32,
) -> Result<(), DivSufSortError> {
    let n = t.len();

    if n == 0 {
        *idx = 0;
        return Ok(());
    }
    if n == 1 {
        u[0] = t[0];
        *idx = 1;
        return Ok(());
    }

    match sa {
        None => {
            // delegate to divbwt
            let primary_idx = crate::divbwt(t, u, None)?;
            *idx = primary_idx;
            Ok(())
        }
        Some(a) => {
            // T != U case (straightforward implementation)
            u[0] = t[n - 1];
            let mut i = 0usize;
            while a[i] != 0 {
                u[i + 1] = t[(a[i] - 1) as usize];
                i += 1;
            }
            *idx = (i + 1) as i32;
            i += 1;
            while i < n {
                u[i] = t[(a[i] - 1) as usize];
                i += 1;
            }
            Ok(())
        }
    }
}

/// Inverts the Burrows-Wheeler Transform.
///
/// Given the BWT `t` and its primary index `idx`, reconstructs the original string
/// into `u`. `a` is an optional scratch buffer of length ≥ `t.len()`; if `None`,
/// an internal allocation is used.
///
/// # Errors
///
/// Returns [`DivSufSortError::InvalidArgument`] if `idx` is out of range or `a` is
/// too short.
pub fn inverse_bw_transform(
    t: &[u8],
    u: &mut [u8],
    a: Option<&mut [i32]>,
    idx: i32,
) -> Result<(), DivSufSortError> {
    let n = t.len();

    if idx < 0 || idx as usize > n || (n > 0 && idx == 0) {
        return Err(DivSufSortError::InvalidArgument);
    }
    if n <= 1 {
        if n == 1 {
            u[0] = t[0];
        }
        return Ok(());
    }

    let mut b_buf: Vec<i32>;
    let b: &mut [i32] = match a {
        Some(ref_a) => {
            if ref_a.len() < n {
                return Err(DivSufSortError::InvalidArgument);
            }
            ref_a
        }
        None => {
            b_buf = vec![0i32; n];
            &mut b_buf
        }
    };

    let mut c_table = [0i32; ALPHABET_SIZE];
    for &ch in t {
        c_table[ch as usize] += 1;
    }

    // convert C table to cumulative sums and record seen characters in D
    let mut d_buf = [0u8; ALPHABET_SIZE];
    let mut d_len = 0usize;
    let mut acc = 0i32;
    for (c, cnt_ref) in c_table.iter_mut().enumerate() {
        let cnt = *cnt_ref;
        if cnt > 0 {
            *cnt_ref = acc;
            d_buf[d_len] = c as u8;
            d_len += 1;
            acc += cnt;
        }
    }

    let idx_usize = idx as usize;
    for i in 0..idx_usize {
        b[c_table[t[i] as usize] as usize] = i as i32;
        c_table[t[i] as usize] += 1;
    }
    for i in idx_usize..n {
        b[c_table[t[i] as usize] as usize] = (i + 1) as i32;
        c_table[t[i] as usize] += 1;
    }

    let d = &d_buf[..d_len];
    let mut c_d = vec![0i32; d_len];
    for (ci, &dc) in d.iter().enumerate() {
        c_d[ci] = c_table[dc as usize];
    }

    let mut p = idx;
    for u_elem in u.iter_mut().take(n) {
        let pos = binarysearch_lower(&c_d, d_len as i32, p);
        *u_elem = d[pos as usize];
        p = b[(p - 1) as usize];
    }

    Ok(())
}

fn compare(t: &[u8], p: &[u8], suf: i32, match_len: &mut i32) -> i32 {
    let tsize = t.len() as i32;
    let psize = p.len() as i32;
    let mut i = suf + *match_len;
    let mut j = *match_len;
    let mut r = 0i32;
    while i < tsize && j < psize {
        r = t[i as usize] as i32 - p[j as usize] as i32;
        if r != 0 {
            break;
        }
        i += 1;
        j += 1;
    }
    *match_len = j;
    if r == 0 {
        -(if j != psize { 1 } else { 0 })
    } else {
        r
    }
}

/// Searches for pattern `p` in text `t` using the suffix array `sa`.
///
/// Returns `(count, left)` where `count` is the number of occurrences and `left` is the
/// leftmost index in `sa` at which a match starts. If `count == 0`, `left` is undefined.
pub fn sa_search(t: &[u8], p: &[u8], sa: &[i32]) -> (i32, i32) {
    let tsize = t.len() as i32;
    let psize = p.len() as i32;
    let sasize = sa.len() as i32;

    if sasize == 0 || tsize == 0 {
        return (0, -1);
    }
    if psize == 0 {
        return (sasize, 0);
    }

    let mut i = 0i32;
    let mut j = 0i32;
    let mut k = 0i32;
    let mut lmatch = 0i32;
    let mut rmatch = 0i32;
    let mut size = sasize;
    let mut half = size >> 1;

    while 0 < size {
        let mut match_len = lmatch.min(rmatch);
        let r = compare(t, p, sa[(i + half) as usize], &mut match_len);
        if r < 0 {
            i += half + 1;
            half -= (size & 1) ^ 1;
            lmatch = match_len;
        } else if r > 0 {
            rmatch = match_len;
        } else {
            let lsize = half;
            j = i;
            let rsize = size - half - 1;
            k = i + half + 1;

            let mut llmatch = lmatch;
            let mut lrmatch = match_len;
            let mut lsize2 = lsize;
            let mut lhalf = lsize2 >> 1;
            while 0 < lsize2 {
                let mut lm = llmatch.min(lrmatch);
                let lr = compare(t, p, sa[(j + lhalf) as usize], &mut lm);
                if lr < 0 {
                    j += lhalf + 1;
                    lhalf -= (lsize2 & 1) ^ 1;
                    llmatch = lm;
                } else {
                    lrmatch = lm;
                }
                lsize2 = lhalf;
                lhalf >>= 1;
            }

            let mut rlmatch = match_len;
            let mut rrmatch = rmatch;
            let mut rsize2 = rsize;
            let mut rhalf = rsize2 >> 1;
            while 0 < rsize2 {
                let mut rm = rlmatch.min(rrmatch);
                let rr = compare(t, p, sa[(k + rhalf) as usize], &mut rm);
                if rr <= 0 {
                    k += rhalf + 1;
                    rhalf -= (rsize2 & 1) ^ 1;
                    rlmatch = rm;
                } else {
                    rrmatch = rm;
                }
                rsize2 = rhalf;
                rhalf >>= 1;
            }

            break;
        }
        size = half;
        half >>= 1;
    }

    let count = k - j;
    let left = if count > 0 { j } else { i };
    (count, left)
}

/// Searches for a single character `c` in text `t` using the suffix array `sa`.
///
/// Returns `(count, left)` where `count` is the number of occurrences and `left` is the
/// leftmost index in `sa` at which a match starts. If `count == 0`, `left` is undefined.
pub fn sa_simplesearch(t: &[u8], sa: &[i32], c: u8) -> (i32, i32) {
    let tsize = t.len() as i32;
    let sasize = sa.len() as i32;
    let c = c as i32;

    if sasize == 0 || tsize == 0 {
        return (0, -1);
    }

    let mut i = 0i32;
    let mut j = 0i32;
    let mut k = 0i32;
    let mut size = sasize;
    let mut half = size >> 1;

    while 0 < size {
        let p = sa[(i + half) as usize];
        let r = if p < tsize {
            t[p as usize] as i32 - c
        } else {
            -1
        };
        if r < 0 {
            i += half + 1;
            half -= (size & 1) ^ 1;
        } else if r == 0 {
            let lsize = half;
            j = i;
            let rsize = size - half - 1;
            k = i + half + 1;

            let mut lsize2 = lsize;
            let mut lhalf = lsize2 >> 1;
            while 0 < lsize2 {
                let lp = sa[(j + lhalf) as usize];
                let lr = if lp < tsize {
                    t[lp as usize] as i32 - c
                } else {
                    -1
                };
                if lr < 0 {
                    j += lhalf + 1;
                    lhalf -= (lsize2 & 1) ^ 1;
                }
                lsize2 = lhalf;
                lhalf >>= 1;
            }

            let mut rsize2 = rsize;
            let mut rhalf = rsize2 >> 1;
            while 0 < rsize2 {
                let rp = sa[(k + rhalf) as usize];
                let rr = if rp < tsize {
                    t[rp as usize] as i32 - c
                } else {
                    -1
                };
                if rr <= 0 {
                    k += rhalf + 1;
                    rhalf -= (rsize2 & 1) ^ 1;
                }
                rsize2 = rhalf;
                rhalf >>= 1;
            }

            break;
        }
        size = half;
        half >>= 1;
    }

    let count = k - j;
    let left = if count > 0 { j } else { i };
    (count, left)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sufcheck_empty() {
        assert_eq!(sufcheck(b"", &[], false), Ok(()));
    }

    #[test]
    fn test_sufcheck_single() {
        assert_eq!(sufcheck(b"a", &[0], false), Ok(()));
    }

    #[test]
    fn test_sufcheck_ba() {
        // "ba": suffixes are "a"(1) < "ba"(0)
        assert_eq!(sufcheck(b"ba", &[1, 0], false), Ok(()));
    }

    #[test]
    fn test_sufcheck_wrong_order() {
        // SA is in reverse order, so this should fail
        assert!(sufcheck(b"ba", &[0, 1], false).is_err());
    }

    #[test]
    fn test_sufcheck_out_of_range() {
        assert!(sufcheck(b"ba", &[0, 2], false).is_err());
    }

    #[test]
    fn test_sufcheck_banana() {
        // known suffix array for "banana"
        assert_eq!(sufcheck(b"banana", &[5, 3, 1, 0, 4, 2], false), Ok(()));
    }

    #[test]
    fn test_sa_search_empty_sa() {
        let (count, _) = sa_search(b"banana", b"an", &[]);
        assert_eq!(count, 0);
    }

    #[test]
    fn test_sa_search_empty_pattern() {
        let sa = [5i32, 3, 1, 0, 4, 2];
        let (count, left) = sa_search(b"banana", b"", &sa);
        assert_eq!(count, 6);
        assert_eq!(left, 0);
    }

    #[test]
    fn test_sa_search_found() {
        // SA of "banana" = [5,3,1,0,4,2]
        // "an" occurs at positions 1, 3 → ranks 1,2 in SA
        let sa = [5i32, 3, 1, 0, 4, 2];
        let (count, left) = sa_search(b"banana", b"an", &sa);
        assert_eq!(count, 2);
        // count matches starting at left
        for idx in left..left + count {
            let suf_start = sa[idx as usize] as usize;
            assert!(b"banana"[suf_start..].starts_with(b"an"));
        }
    }

    #[test]
    fn test_sa_search_not_found() {
        let sa = [5i32, 3, 1, 0, 4, 2];
        let (count, _) = sa_search(b"banana", b"xyz", &sa);
        assert_eq!(count, 0);
    }

    #[test]
    fn test_sa_simplesearch_found() {
        // 'a' appears 3 times in "banana"
        let sa = [5i32, 3, 1, 0, 4, 2];
        let (count, left) = sa_simplesearch(b"banana", &sa, b'a');
        assert_eq!(count, 3);
        for idx in left..left + count {
            let suf_start = sa[idx as usize] as usize;
            assert_eq!(b"banana"[suf_start], b'a');
        }
    }

    #[test]
    fn test_sa_simplesearch_not_found() {
        let sa = [5i32, 3, 1, 0, 4, 2];
        let (count, _) = sa_simplesearch(b"banana", &sa, b'z');
        assert_eq!(count, 0);
    }

    #[test]
    fn test_bw_transform_banana() {
        // "banana" SA=[5,3,1,0,4,2] → BWT="annbaa", idx=4
        let t = b"banana";
        let mut sa = vec![5i32, 3, 1, 0, 4, 2];
        let mut u = vec![0u8; t.len()];
        let mut idx = 0i32;
        bw_transform(t, &mut u, Some(&mut sa), &mut idx).unwrap();
        assert_eq!(&u, b"annbaa");
        assert_eq!(idx, 4);
    }

    #[test]
    fn test_inverse_bw_transform_banana() {
        // BWT="annbaa", idx=4 → "banana"
        let bwt = b"annbaa";
        let mut u = vec![0u8; bwt.len()];
        inverse_bw_transform(bwt, &mut u, None, 4).unwrap();
        assert_eq!(&u, b"banana");
    }

    #[test]
    fn test_bw_roundtrip() {
        let t = b"mississippi";
        // known suffix array for "mississippi"
        let mut sa = vec![10i32, 7, 4, 1, 0, 9, 8, 6, 3, 5, 2];
        let mut bwt = vec![0u8; t.len()];
        let mut idx = 0i32;
        bw_transform(t, &mut bwt, Some(&mut sa), &mut idx).unwrap();

        let mut restored = vec![0u8; t.len()];
        inverse_bw_transform(&bwt, &mut restored, None, idx).unwrap();
        assert_eq!(restored, t);
    }

    #[test]
    fn test_binarysearch_lower() {
        let a = [3i32, 4, 6];
        assert_eq!(binarysearch_lower(&a, 3, 3), 0);
        assert_eq!(binarysearch_lower(&a, 3, 4), 1);
        assert_eq!(binarysearch_lower(&a, 3, 5), 2);
        assert_eq!(binarysearch_lower(&a, 3, 6), 2);
        assert_eq!(binarysearch_lower(&a, 3, 7), 3);
    }
}
