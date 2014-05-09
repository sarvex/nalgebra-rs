//! Matrix with dimensions unknown at compile-time.

#![allow(missing_doc)] // we hide doc to not have to document the $trhs double dispatch trait.

use rand::Rand;
use rand;
use std::num::{One, Zero, Float};
use traits::operations::ApproxEq;
use std::mem;
use structs::dvec::{DVec, DVecMulRhs};
use traits::operations::{Inv, Transpose, Mean, Cov};
use traits::structure::Cast;
use traits::geometry::Norm;
use std::cmp::min;
use std::fmt::{Show, Formatter, Result};

#[doc(hidden)]
mod metal;

/// Matrix with dimensions unknown at compile-time.
#[deriving(TotalEq, Eq, Clone)]
pub struct DMat<N> {
    nrows: uint,
    ncols: uint,
    mij:   Vec<N>
}

double_dispatch_binop_decl_trait!(DMat, DMatMulRhs)
double_dispatch_binop_decl_trait!(DMat, DMatDivRhs)
double_dispatch_binop_decl_trait!(DMat, DMatAddRhs)
double_dispatch_binop_decl_trait!(DMat, DMatSubRhs)

mul_redispatch_impl!(DMat, DMatMulRhs)
div_redispatch_impl!(DMat, DMatDivRhs)
add_redispatch_impl!(DMat, DMatAddRhs)
sub_redispatch_impl!(DMat, DMatSubRhs)

impl<N> DMat<N> {
    /// Creates an uninitialized matrix.
    #[inline]
    pub unsafe fn new_uninitialized(nrows: uint, ncols: uint) -> DMat<N> {
        let mut vec = Vec::with_capacity(nrows * ncols);
        vec.set_len(nrows * ncols);

        DMat {
            nrows: nrows,
            ncols: ncols,
            mij:   vec
        }
    }
}

impl<N: Zero + Clone> DMat<N> {
    /// Builds a matrix filled with zeros.
    /// 
    /// # Arguments
    ///   * `dim` - The dimension of the matrix. A `dim`-dimensional matrix contains `dim * dim`
    ///   components.
    #[inline]
    pub fn new_zeros(nrows: uint, ncols: uint) -> DMat<N> {
        DMat::from_elem(nrows, ncols, Zero::zero())
    }

    /// Tests if all components of the matrix are zeroes.
    #[inline]
    pub fn is_zero(&self) -> bool {
        self.mij.iter().all(|e| e.is_zero())
    }

    #[inline]
    pub fn reset(&mut self) {
        for mij in self.mij.mut_iter() {
            *mij = Zero::zero();
        }
    }
}

impl<N: Rand> DMat<N> {
    /// Builds a matrix filled with random values.
    #[inline]
    pub fn new_random(nrows: uint, ncols: uint) -> DMat<N> {
        DMat::from_fn(nrows, ncols, |_, _| rand::random())
    }
}

impl<N: One + Clone> DMat<N> {
    /// Builds a matrix filled with a given constant.
    #[inline]
    pub fn new_ones(nrows: uint, ncols: uint) -> DMat<N> {
        DMat::from_elem(nrows, ncols, One::one())
    }
}

impl<N: Clone> DMat<N> {
    /// Builds a matrix filled with a given constant.
    #[inline]
    pub fn from_elem(nrows: uint, ncols: uint, val: N) -> DMat<N> {
        DMat {
            nrows: nrows,
            ncols: ncols,
            mij:   Vec::from_elem(nrows * ncols, val)
        }
    }

    /// Builds a matrix filled with the components provided by a vector.
    /// The vector contains the matrix data in row-major order.
    /// Note that `from_col_vec` is a lot faster than `from_row_vec` since a `DMat` stores its data
    /// in column-major order.
    ///
    /// The vector must have at least `nrows * ncols` elements.
    #[inline]
    pub fn from_row_vec(nrows: uint, ncols: uint, vec: &[N]) -> DMat<N> {
        let mut res = DMat::from_col_vec(ncols, nrows, vec);

        // we transpose because the buffer is row_major
        res.transpose();

        res
    }

    /// Builds a matrix filled with the components provided by a vector.
    /// The vector contains the matrix data in column-major order.
    /// Note that `from_col_vec` is a lot faster than `from_row_vec` since a `DMat` stores its data
    /// in column-major order.
    ///
    /// The vector must have at least `nrows * ncols` elements.
    #[inline]
    pub fn from_col_vec(nrows: uint, ncols: uint, vec: &[N]) -> DMat<N> {
        assert!(nrows * ncols == vec.len());

        DMat {
            nrows: nrows,
            ncols: ncols,
            mij:   Vec::from_slice(vec)
        }
    }
}

impl<N> DMat<N> {
    /// Builds a matrix filled with a given constant.
    #[inline(always)]
    pub fn from_fn(nrows: uint, ncols: uint, f: |uint, uint| -> N) -> DMat<N> {
        DMat {
            nrows: nrows,
            ncols: ncols,
            mij:   Vec::from_fn(nrows * ncols, |i| { let m = i % ncols; f(m, m - i * ncols) })
        }
    }

    /// The number of row on the matrix.
    #[inline]
    pub fn nrows(&self) -> uint {
        self.nrows
    }

    /// The number of columns on the matrix.
    #[inline]
    pub fn ncols(&self) -> uint {
        self.ncols
    }

    /// Transforms this matrix into an array. This consumes the matrix and is O(1).
    /// The returned vector contains the matrix data in column-major order.
    #[inline]
    pub fn to_vec(self) -> Vec<N> {
        self.mij
    }

    /// Gets a reference to this matrix data.
    /// The returned vector contains the matrix data in column-major order.
    #[inline]
    pub fn as_vec<'r>(&'r self) -> &'r [N] {
        self.mij.as_slice()
    }

    /// Gets a mutable reference to this matrix data.
    /// The returned vector contains the matrix data in column-major order.
    #[inline]
    pub fn as_mut_vec<'r>(&'r mut self) -> &'r mut [N] {
         self.mij.as_mut_slice()
    }
}

// FIXME: add a function to modify the dimension (to avoid useless allocations)?

impl<N: One + Zero + Clone> DMat<N> {
    /// Builds an identity matrix.
    /// 
    /// # Arguments
    /// * `dim` - The dimension of the matrix. A `dim`-dimensional matrix contains `dim * dim`
    /// components.
    #[inline]
    pub fn new_identity(dim: uint) -> DMat<N> {
        let mut res = DMat::new_zeros(dim, dim);

        for i in range(0u, dim) {
            let _1: N  = One::one();
            res.set(i, i, _1);
        }

        res
    }
}

impl<N: Clone> DMat<N> {
    #[inline(always)]
    fn offset(&self, i: uint, j: uint) -> uint {
        i + j * self.nrows
    }

    /// Changes the value of a component of the matrix.
    ///
    /// # Arguments
    ///   * `row` - 0-based index of the line to be changed
    ///   * `col` - 0-based index of the column to be changed
    #[inline]
    pub fn set(&mut self, row: uint, col: uint, val: N) {
        assert!(row < self.nrows);
        assert!(col < self.ncols);

        let offset = self.offset(row, col);
        *self.mij.get_mut(offset) = val
    }

    /// Just like `set` without bounds checking.
    #[inline]
    pub unsafe fn set_fast(&mut self, row: uint, col: uint, val: N) {
        let offset = self.offset(row, col);
        *self.mij.as_mut_slice().unsafe_mut_ref(offset) = val
    }

    /// Reads the value of a component of the matrix.
    ///
    /// # Arguments
    /// * `row` - 0-based index of the line to be read
    /// * `col` - 0-based index of the column to be read
    #[inline]
    pub fn at(&self, row: uint, col: uint) -> N {
        assert!(row < self.nrows);
        assert!(col < self.ncols);
        unsafe { self.at_fast(row, col) }
    }

    /// Just like `at` without bounds checking.
    #[inline]
    pub unsafe fn at_fast(&self, row: uint, col: uint) -> N {
        (*self.mij.as_slice().unsafe_ref(self.offset(row, col))).clone()
    }
}

impl<N: Clone + Mul<N, N> + Add<N, N> + Zero> DMatMulRhs<N, DMat<N>> for DMat<N> {
    fn binop(left: &DMat<N>, right: &DMat<N>) -> DMat<N> {
        assert!(left.ncols == right.nrows);

        let mut res = unsafe { DMat::new_uninitialized(left.nrows, right.ncols) };

        for i in range(0u, left.nrows) {
            for j in range(0u, right.ncols) {
                let mut acc: N = Zero::zero();

                unsafe {
                    for k in range(0u, left.ncols) {
                        acc = acc + left.at_fast(i, k) * right.at_fast(k, j);
                    }

                    res.set_fast(i, j, acc);
                }
            }
        }

        res
    }
}

impl<N: Clone + Add<N, N> + Mul<N, N> + Zero>
DMatMulRhs<N, DVec<N>> for DVec<N> {
    fn binop(left: &DMat<N>, right: &DVec<N>) -> DVec<N> {
        assert!(left.ncols == right.at.len());

        let mut res : DVec<N> = unsafe { DVec::new_uninitialized(left.nrows) };

        for i in range(0u, left.nrows) {
            let mut acc: N = Zero::zero();

            for j in range(0u, left.ncols) {
                unsafe {
                    acc = acc + left.at_fast(i, j) * right.at_fast(j);
                }
            }

            *res.at.get_mut(i) = acc;
        }

        res
    }
}


impl<N: Clone + Add<N, N> + Mul<N, N> + Zero>
DVecMulRhs<N, DVec<N>> for DMat<N> {
    fn binop(left: &DVec<N>, right: &DMat<N>) -> DVec<N> {
        assert!(right.nrows == left.at.len());

        let mut res : DVec<N> = unsafe { DVec::new_uninitialized(right.ncols) };

        for i in range(0u, right.ncols) {
            let mut acc: N = Zero::zero();

            for j in range(0u, right.nrows) {
                unsafe {
                    acc = acc + left.at_fast(j) * right.at_fast(j, i);
                }
            }

            *res.at.get_mut(i) = acc;
        }

        res
    }
}

impl<N: Clone + Num>
Inv for DMat<N> {
    #[inline]
    fn inv_cpy(m: &DMat<N>) -> Option<DMat<N>> {
        let mut res : DMat<N> = m.clone();

        if res.inv() {
            Some(res)
        }
        else {
            None
        }
    }

    fn inv(&mut self) -> bool {
        assert!(self.nrows == self.ncols);

        let dim              = self.nrows;
        let mut res: DMat<N> = DMat::new_identity(dim);
        let     _0T: N       = Zero::zero();

        // inversion using Gauss-Jordan elimination
        for k in range(0u, dim) {
            // search a non-zero value on the k-th column
            // FIXME: would it be worth it to spend some more time searching for the
            // max instead?

            let mut n0 = k; // index of a non-zero entry

            while n0 != dim {
                if unsafe { self.at_fast(n0, k) } != _0T {
                    break;
                }

                n0 = n0 + 1;
            }

            if n0 == dim { 
                return false
            }

            // swap pivot line
            if n0 != k {
                for j in range(0u, dim) {
                    let off_n0_j = self.offset(n0, j);
                    let off_k_j  = self.offset(k, j);

                    self.mij.as_mut_slice().swap(off_n0_j, off_k_j);
                    res.mij.as_mut_slice().swap(off_n0_j, off_k_j);
                }
            }

            unsafe {
                let pivot = self.at_fast(k, k);

                for j in range(k, dim) {
                    let selfval = self.at_fast(k, j) / pivot;
                    self.set_fast(k, j, selfval);
                }

                for j in range(0u, dim) {
                    let resval = res.at_fast(k, j) / pivot;
                    res.set_fast(k, j, resval);
                }

                for l in range(0u, dim) {
                    if l != k {
                        let normalizer = self.at_fast(l, k);

                        for j in range(k, dim) {
                            let selfval = self.at_fast(l, j) - self.at_fast(k, j) * normalizer;
                            self.set_fast(l, j, selfval);
                        }

                        for j in range(0u, dim) {
                            let resval = res.at_fast(l, j) - res.at_fast(k, j) * normalizer;
                            res.set_fast(l, j, resval);
                        }
                    }
                }
            }
        }

        *self = res;

        true
    }
}

impl<N: Clone> Transpose for DMat<N> {
    #[inline]
    fn transpose_cpy(m: &DMat<N>) -> DMat<N> {
        if m.nrows == m.ncols {
            let mut res = m.clone();

            res.transpose();

            res
        }
        else {
            let mut res = unsafe { DMat::new_uninitialized(m.ncols, m.nrows) };

            for i in range(0u, m.nrows) {
                for j in range(0u, m.ncols) {
                    unsafe {
                        res.set_fast(j, i, m.at_fast(i, j))
                    }
                }
            }

            res
        }
    }

    #[inline]
    fn transpose(&mut self) {
        if self.nrows == self.ncols {
            for i in range(1u, self.nrows) {
                for j in range(0u, self.ncols - 1) {
                    let off_i_j = self.offset(i, j);
                    let off_j_i = self.offset(j, i);

                    self.mij.as_mut_slice().swap(off_i_j, off_j_i);
                }
            }

            mem::swap(&mut self.nrows, &mut self.ncols);
        }
        else {
            // FIXME: implement a better algorithm which does that in-place.
            *self = Transpose::transpose_cpy(self);
        }
    }
}

impl<N: Num + Cast<f32> + Clone> Mean<DVec<N>> for DMat<N> {
    fn mean(m: &DMat<N>) -> DVec<N> {
        let mut res: DVec<N> = DVec::new_zeros(m.ncols);
        let normalizer: N    = Cast::from(1.0f32 / Cast::from(m.nrows));

        for i in range(0u, m.nrows) {
            for j in range(0u, m.ncols) {
                unsafe {
                    let acc = res.at_fast(j) + m.at_fast(i, j) * normalizer;
                    res.set_fast(j, acc);
                }
            }
        }

        res
    }
}

impl<N: Clone + Num + Cast<f32> + DMatDivRhs<N, DMat<N>> + ToStr > Cov<DMat<N>> for DMat<N> {
    // FIXME: this could be heavily optimized, removing all temporaries by merging loops.
    fn cov(m: &DMat<N>) -> DMat<N> {
        assert!(m.nrows > 1);

        let mut centered = unsafe { DMat::new_uninitialized(m.nrows, m.ncols) };
        let mean = Mean::mean(m);

        // FIXME: use the rows iterator when available
        for i in range(0u, m.nrows) {
            for j in range(0u, m.ncols) {
                unsafe {
                    centered.set_fast(i, j, m.at_fast(i, j) - mean.at_fast(j));
                }
            }
        }

        // FIXME: return a triangular matrix?
        let fnormalizer: f32 = Cast::from(m.nrows() - 1);
        let normalizer: N    = Cast::from(fnormalizer);
        // FIXME: this will do 2 allocations for temporaries!
        (Transpose::transpose_cpy(&centered) * centered) / normalizer
    }
}


/// QR decomposition using Householder reflections
/// # Arguments
/// * `m` matrix to decompose
pub fn decomp_qr<N: Clone + Num + Float + Show>(m: &DMat<N>) -> (DMat<N>, DMat<N>) {
  let rows = m.nrows();
  let cols = m.ncols();
  assert!(rows >= cols);
  let mut q : DMat<N> = DMat::new_identity(rows);
  let mut r = m.clone();

  let subtract_reflection = |vec: DVec<N>| -> DMat<N> {
      // FIXME: we don't handle the complex case here
      let mut qk : DMat<N> = DMat::new_identity(rows);
      let start = rows - vec.at.len();
      for j in range(start, rows) {
          for i in range(start, rows) {
              unsafe {
                  let vv = vec.at_fast(i-start)*vec.at_fast(j-start);
                  let qkij = qk.at_fast(i,j);
                  qk.set_fast(i, j, qkij - vv - vv);
              }
          }
      }
      qk
  };

  let iterations = min(rows-1, cols);

  for ite in range(0u, iterations) {
      // we get the ite-th column truncated from its first ite elements,
      // this is fast thanks to the matrix being column major
      let start= m.offset(ite, ite);
      let stop = m.offset(rows, ite);
      let mut v = DVec::from_vec(rows - ite, r.mij.slice(start, stop));
      let alpha =
          if unsafe { v.at_fast(ite) } >= Zero::zero() {
              -Norm::norm(&v)
          }
          else {
              Norm::norm(&v)
          };
      unsafe {
          let x = v.at_fast(0);
          v.set_fast(0, x - alpha);
      }
      let _ = v.normalize();
      let qk = subtract_reflection(v);
      r = qk * r;
      q = q * Transpose::transpose_cpy(&qk);
  }

  (q, r)
}


impl<N: ApproxEq<N>> ApproxEq<N> for DMat<N> {
    #[inline]
    fn approx_epsilon(_: Option<DMat<N>>) -> N {
        ApproxEq::approx_epsilon(None::<N>)
    }

    #[inline]
    fn approx_eq(a: &DMat<N>, b: &DMat<N>) -> bool {
        let mut zip = a.mij.iter().zip(b.mij.iter());

        zip.all(|(a, b)| ApproxEq::approx_eq(a, b))
    }

    #[inline]
    fn approx_eq_eps(a: &DMat<N>, b: &DMat<N>, epsilon: &N) -> bool {
        let mut zip = a.mij.iter().zip(b.mij.iter());

        zip.all(|(a, b)| ApproxEq::approx_eq_eps(a, b, epsilon))
    }
}

impl<N: Show + Clone> Show for DMat<N> {
    fn fmt(&self, form:&mut Formatter) -> Result {
        for i in range(0u, self.nrows()) {
            for j in range(0u, self.ncols()) {
                write!(form.buf, "{} ", self.at(i, j));
            }
            write!(form.buf, "\n");
        }
        write!(form.buf, "\n")
    }
}

macro_rules! scalar_mul_impl (
    ($n: ident) => (
        impl DMatMulRhs<$n, DMat<$n>> for $n {
            #[inline]
            fn binop(left: &DMat<$n>, right: &$n) -> DMat<$n> {
                DMat {
                    nrows: left.nrows,
                    ncols: left.ncols,
                    mij:   left.mij.iter().map(|a| a * *right).collect()
                }
            }
        }
    )
)

macro_rules! scalar_div_impl (
    ($n: ident) => (
        impl DMatDivRhs<$n, DMat<$n>> for $n {
            #[inline]
            fn binop(left: &DMat<$n>, right: &$n) -> DMat<$n> {
                DMat {
                    nrows: left.nrows,
                    ncols: left.ncols,
                    mij:   left.mij.iter().map(|a| a / *right).collect()
                }
            }
        }
    )
)

macro_rules! scalar_add_impl (
    ($n: ident) => (
        impl DMatAddRhs<$n, DMat<$n>> for $n {
            #[inline]
            fn binop(left: &DMat<$n>, right: &$n) -> DMat<$n> {
                DMat {
                    nrows: left.nrows,
                    ncols: left.ncols,
                    mij:   left.mij.iter().map(|a| a + *right).collect()
                }
            }
        }
    )
)

macro_rules! scalar_sub_impl (
    ($n: ident) => (
        impl DMatSubRhs<$n, DMat<$n>> for $n {
            #[inline]
            fn binop(left: &DMat<$n>, right: &$n) -> DMat<$n> {
                DMat {
                    nrows: left.nrows,
                    ncols: left.ncols,
                    mij:   left.mij.iter().map(|a| a - *right).collect()
                }
            }
        }
    )
)

scalar_mul_impl!(f64)
scalar_mul_impl!(f32)
scalar_mul_impl!(u64)
scalar_mul_impl!(u32)
scalar_mul_impl!(u16)
scalar_mul_impl!(u8)
scalar_mul_impl!(i64)
scalar_mul_impl!(i32)
scalar_mul_impl!(i16)
scalar_mul_impl!(i8)
scalar_mul_impl!(uint)
scalar_mul_impl!(int)

scalar_div_impl!(f64)
scalar_div_impl!(f32)
scalar_div_impl!(u64)
scalar_div_impl!(u32)
scalar_div_impl!(u16)
scalar_div_impl!(u8)
scalar_div_impl!(i64)
scalar_div_impl!(i32)
scalar_div_impl!(i16)
scalar_div_impl!(i8)
scalar_div_impl!(uint)
scalar_div_impl!(int)

scalar_add_impl!(f64)
scalar_add_impl!(f32)
scalar_add_impl!(u64)
scalar_add_impl!(u32)
scalar_add_impl!(u16)
scalar_add_impl!(u8)
scalar_add_impl!(i64)
scalar_add_impl!(i32)
scalar_add_impl!(i16)
scalar_add_impl!(i8)
scalar_add_impl!(uint)
scalar_add_impl!(int)

scalar_sub_impl!(f64)
scalar_sub_impl!(f32)
scalar_sub_impl!(u64)
scalar_sub_impl!(u32)
scalar_sub_impl!(u16)
scalar_sub_impl!(u8)
scalar_sub_impl!(i64)
scalar_sub_impl!(i32)
scalar_sub_impl!(i16)
scalar_sub_impl!(i8)
scalar_sub_impl!(uint)
scalar_sub_impl!(int)
