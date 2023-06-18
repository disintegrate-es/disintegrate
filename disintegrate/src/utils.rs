#[macro_export]
#[doc(hidden)]
macro_rules! const_slice_unique {
    ($a:expr) => {
        &{
            const A: &[&str] = $crate::const_slice_sort!($a);
            const LEN: usize = A.len() - disintegrate::utils::count_dup(A);

            let mut out: [_; LEN] = if LEN == 0 {
                unsafe { std::mem::transmute([0u8; std::mem::size_of::<&str>() * LEN]) }
            } else {
                [A[0]; LEN]
            };

            let mut r: usize = 1;
            let mut w: usize = 1;
            while r < A.len() {
                if !disintegrate::utils::eq(A[r], out[w - 1]) {
                    out[w] = A[r];
                    w += 1;
                }
                r += 1;
            }
            out
        }
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! const_slices_concat {
    ($a:expr, $b:expr) => {
        &{
            const A: &[&str] = $a;
            const B: &[&str] = $b;
            let mut out: [_; { A.len() + B.len() }] = if A.len() == 0 && B.len() == 0 {
                unsafe {
                    std::mem::transmute([0u8; std::mem::size_of::<&str>() * (A.len() + B.len())])
                }
            } else if A.len() == 0 {
                [B[0]; { A.len() + B.len() }]
            } else {
                [A[0]; { A.len() + B.len() }]
            };
            let mut i = 0;
            while i < A.len() {
                out[i] = A[i];
                i += 1;
            }
            i = 0;
            while i < B.len() {
                out[i + A.len()] = B[i];
                i += 1;
            }
            out
        }
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! const_slice_sort {
    ($a:expr) => {
        &{
            const A: &[&str] = $a;
            let mut out: [_; A.len()] = if A.len() == 0 {
                unsafe { std::mem::transmute([0u8; std::mem::size_of::<&str>() * A.len()]) }
            } else {
                [A[0]; A.len()]
            };

            let mut i = 1;
            while i < A.len() {
                out[i] = A[i];
                let mut j = i;
                while j > 0 && disintegrate::utils::compare(out[j], out[j - 1]) == -1 {
                    //swap
                    let tmp = out[j];
                    out[j] = out[j - 1];
                    out[j - 1] = tmp;

                    j -= 1;
                }
                i += 1;
            }
            out
        }
    };
}

pub const fn include(a: &[&str], b: &[&str]) -> bool {
    let mut i = 0;
    let mut j = 0;

    while i < a.len() && j < b.len() {
        if eq(a[i], b[j]) {
            j += 1;
            i = 0;
        } else {
            i += 1;
        }
    }

    j == b.len()
}

pub const fn count_dup(slice: &[&str]) -> usize {
    let mut count = 0;
    let mut i = 0;
    while i + 1 < slice.len() {
        if eq(slice[i], slice[i + 1]) {
            count += 1;
        }
        i += 1;
    }

    count
}

pub const fn compare(lhs: &str, rhs: &str) -> i8 {
    let lhs = lhs.as_bytes();
    let rhs = rhs.as_bytes();
    let lhs_len = lhs.len();
    let rhs_len = rhs.len();
    let min_len = if lhs_len < rhs_len { lhs_len } else { rhs_len };

    let mut i = 0;
    while i < min_len {
        if lhs[i] < rhs[i] {
            return -1;
        }
        if lhs[i] > rhs[i] {
            return 1;
        }
        i += 1;
    }

    if lhs_len < rhs_len {
        -1
    } else if lhs_len > rhs_len {
        1
    } else {
        0
    }
}

pub const fn eq(lhs: &str, rhs: &str) -> bool {
    let lhs = lhs.as_bytes();
    let rhs = rhs.as_bytes();
    let lhs_len = lhs.len();
    let rhs_len = rhs.len();

    if lhs_len != rhs_len {
        return false;
    }

    let mut i = 0;
    while i < lhs_len {
        if lhs[i] != rhs[i] {
            return false;
        }
        i += 1;
    }

    true
}
