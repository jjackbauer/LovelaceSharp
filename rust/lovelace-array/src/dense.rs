//! Homogeneous dense arrays with first-class dtype and precision metadata.
//!
//! This is the Rust port of the reference's Lovelace.Abstractions.DenseArray<T> packed
//! constructor: one buffer, a row-major shape/strides pair, and the DType/Precision
//! metadata an array-valued kernel dispatches on. The language's own value type stays
//! NdArray; DenseArray is the typed-array surface a host (or a DSP kernel) uses when the
//! element type and the language dtype are both known.
//!
//! The packed form is always contiguous: the buffer holds exactly the shape's elements in
//! row-major order and the offset is zero. A strided view constructor is deliberately absent
//! until a caller needs one; as_span can therefore answer the whole buffer without a
//! materialisation step, which is the property the reference's typed-array tests pin.

use std::any::TypeId;
use std::fmt;

use crate::ArrayError;

/// The homogeneous element type of a dense array (the reference's DType).
///
/// The first three variants are ordered from narrowest to widest so the language's scalar
/// widening lattice Natural -> Integer -> Real is expressible as an ordering. DType::Complex
/// is a distinct domain type outside that lattice: it is never a widening target and is only
/// produced by an explicit complex construction.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DType {
    /// Arbitrary-precision non-negative integers.
    Natural,
    /// Arbitrary-precision signed integers.
    Integer,
    /// Arbitrary-precision exact/truncated reals.
    Real,
    /// Arbitrary-precision complex numbers (a pair of reals).
    Complex,
}

impl DType {
    /// The reference's member name, e.g. Complex in a refusal such as DType.Complex.
    pub const fn name(self) -> &'static str {
        match self {
            DType::Natural => "Natural",
            DType::Integer => "Integer",
            DType::Real => "Real",
            DType::Complex => "Complex",
        }
    }
}

impl fmt::Display for DType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// The precision an array's element arithmetic runs at, as a significant-digit count (the
/// reference's Precision record struct).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Precision {
    significant_digits: usize,
}

impl Precision {
    /// Builds a precision from its significant-digit count.
    pub const fn new(significant_digits: usize) -> Precision {
        Precision { significant_digits }
    }

    /// The significant-digit count.
    pub const fn significant_digits(self) -> usize {
        self.significant_digits
    }
}

impl Default for Precision {
    fn default() -> Precision {
        Precision::new(0)
    }
}

/// A packed, row-major homogeneous array with dtype and precision metadata.
///
/// The reference has two constructors (packed and strided-view). Only the packed one is ported
/// here because it is the one the typed-array surface's tests exercise: the buffer length must
/// equal the shape's element count, the strides are the canonical row-major strides, and the
/// offset is zero.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DenseArray<T> {
    shape: Vec<usize>,
    strides: Vec<usize>,
    data: Vec<T>,
    dtype: DType,
    precision: Precision,
}

impl<T> DenseArray<T> {
    /// Builds a packed array from its shape, elements and metadata.
    ///
    /// The shape must have at least one dimension, its element count must equal data.len(),
    /// and the product must not overflow usize; otherwise the reference's ArgumentException
    /// is reported as ArrayError::ShapeMismatch.
    pub fn new(
        shape: Vec<usize>,
        data: Vec<T>,
        dtype: DType,
        precision: Precision,
    ) -> Result<DenseArray<T>, ArrayError> {
        if shape.is_empty() {
            return Err(ArrayError::ShapeMismatch);
        }
        let numel = crate::element_count(&shape).ok_or(ArrayError::ShapeMismatch)?;
        if numel != data.len() {
            return Err(ArrayError::ShapeMismatch);
        }
        let strides = row_major_strides(&shape);
        Ok(DenseArray {
            shape,
            strides,
            data,
            dtype,
            precision,
        })
    }

    /// The language dtype of the elements.
    pub fn dtype(&self) -> DType {
        self.dtype
    }

    /// The precision the element arithmetic runs at.
    pub fn precision(&self) -> Precision {
        self.precision
    }

    /// The number of dimensions.
    pub fn rank(&self) -> usize {
        self.shape.len()
    }

    /// The dimension sizes, outermost first.
    pub fn shape(&self) -> &[usize] {
        &self.shape
    }

    /// The row-major strides, one per axis.
    pub fn strides(&self) -> &[usize] {
        &self.strides
    }

    /// The buffer offset (always zero for the packed constructor).
    pub const fn offset(&self) -> usize {
        0
    }

    /// The total number of elements.
    pub fn numel(&self) -> usize {
        self.data.len()
    }

    /// The Rust type of one element (the reference's ElementType).
    pub fn element_type(&self) -> TypeId
    where
        T: 'static,
    {
        TypeId::of::<T>()
    }

    /// True when the buffer is packed row-major; always true for this constructor.
    pub const fn is_contiguous(&self) -> bool {
        true
    }

    /// The element at a logical flat row-major index, or None when out of range.
    pub fn get_element(&self, flat_index: usize) -> Option<&T> {
        self.data.get(flat_index)
    }

    /// The packed buffer as one contiguous span.
    pub fn as_span(&self) -> &[T] {
        &self.data
    }

    /// Iterates the elements in row-major order.
    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        self.data.iter()
    }
}

impl<T: Clone> DenseArray<T> {
    /// Materialises an independent packed copy (the reference's AsContiguous).
    pub fn as_contiguous(&self) -> DenseArray<T> {
        self.clone()
    }
}

impl<T> std::ops::Index<usize> for DenseArray<T> {
    type Output = T;

    fn index(&self, index: usize) -> &T {
        &self.data[index]
    }
}

/// Canonical row-major strides: strides[i] = product(shape[i + 1..]).
fn row_major_strides(shape: &[usize]) -> Vec<usize> {
    let mut strides = vec![1usize; shape.len()];
    let mut acc = 1usize;
    for i in (0..shape.len()).rev() {
        strides[i] = acc;
        acc = acc.saturating_mul(shape[i]);
    }
    strides
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packed_construction_exposes_shape_rank_numel_and_strides() {
        let array = DenseArray::new(
            vec![2, 2],
            vec![1, 2, 3, 4],
            DType::Integer,
            Precision::new(16),
        )
        .expect("a packed 2x2 array");
        assert_eq!(array.rank(), 2);
        assert_eq!(array.shape(), &[2, 2]);
        assert_eq!(array.strides(), &[2, 1]);
        assert_eq!(array.numel(), 4);
        assert_eq!(array.offset(), 0);
        assert_eq!(array.dtype(), DType::Integer);
        assert_eq!(array.precision().significant_digits(), 16);
        assert!(array.is_contiguous());
        assert_eq!(array.element_type(), TypeId::of::<i32>());
        assert_eq!(array.get_element(2), Some(&3));
        assert_eq!(array.as_span(), &[1, 2, 3, 4]);
        assert_eq!(array.iter().copied().sum::<i32>(), 10);
    }

    #[test]
    fn a_buffer_whose_length_differs_from_the_shape_is_refused() {
        assert_eq!(
            DenseArray::new(
                vec![2, 2],
                vec![1, 2, 3],
                DType::Integer,
                Precision::new(16)
            )
            .unwrap_err(),
            ArrayError::ShapeMismatch
        );
    }

    #[test]
    fn an_empty_shape_is_refused() {
        assert_eq!(
            DenseArray::new(vec![], vec![1], DType::Natural, Precision::new(16)).unwrap_err(),
            ArrayError::ShapeMismatch
        );
    }
}
