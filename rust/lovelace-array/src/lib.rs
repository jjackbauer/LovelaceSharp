//! Generic N-dimensional arrays with row-major storage, shape algebra, elementwise
//! operations and reductions.
//!
//! An [NdArray] is a shape (dimension sizes, outermost first) plus a flat element vector
//! holding the elements in row-major (C) order. A rank-0 array has an empty shape and
//! exactly one element. Dimensions of length zero are allowed, so an array can be empty
//! while keeping its shape.
//!
//! The crate has no dependencies and does no floating-point arithmetic. Numeric
//! operations are expressed through the [Field] trait, which is implemented for i64
//! so generic algorithms can be tested with ordinary integer arithmetic.

#![forbid(unsafe_code)]

use std::fmt;

/// An exact arithmetic field used by the generic constructors and reductions.
///
/// Implementors are expected to satisfy the usual field axioms. Division by zero is
/// whatever the implementation defines; the array library itself only uses the
/// arithmetic methods while folding elements.
pub trait Field: Clone + PartialEq + fmt::Debug {
    /// The additive identity.
    fn zero() -> Self;
    /// The multiplicative identity.
    fn one() -> Self;
    /// Field addition.
    fn add(&self, rhs: &Self) -> Self;
    /// Field subtraction.
    fn sub(&self, rhs: &Self) -> Self;
    /// Field multiplication.
    fn mul(&self, rhs: &Self) -> Self;
    /// Field division.
    fn div(&self, rhs: &Self) -> Self;
    /// Additive inverse.
    fn neg(&self) -> Self;
    /// Returns true when the value is the additive identity.
    fn is_zero(&self) -> bool;
    /// Lifts a signed integer into the field.
    fn from_i64(v: i64) -> Self;
}

impl Field for i64 {
    fn zero() -> Self {
        0
    }

    fn one() -> Self {
        1
    }

    fn add(&self, rhs: &Self) -> Self {
        *self + *rhs
    }

    fn sub(&self, rhs: &Self) -> Self {
        *self - *rhs
    }

    fn mul(&self, rhs: &Self) -> Self {
        *self * *rhs
    }

    fn div(&self, rhs: &Self) -> Self {
        *self / *rhs
    }

    fn neg(&self) -> Self {
        -*self
    }

    fn is_zero(&self) -> bool {
        *self == 0
    }

    fn from_i64(v: i64) -> Self {
        v
    }
}

/// Errors returned by fallible array operations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArrayError {
    /// Shapes, ranks or element counts are incompatible.
    ShapeMismatch,
    /// The rows handed to [NdArray::from_rows] have different lengths.
    Ragged,
    /// An index or partial index is outside its dimension.
    IndexOutOfBounds,
    /// A slice list is malformed: wrong length, or a step of zero.
    BadSlice,
    /// A reduction was asked for the minimum or maximum of an empty array.
    EmptyReduction,
    /// An axis or axis permutation is invalid for the array's rank.
    BadAxis,
}

impl fmt::Display for ArrayError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::ShapeMismatch => "shape mismatch",
            Self::Ragged => "ragged rows: every row must have the same length",
            Self::IndexOutOfBounds => "index out of bounds",
            Self::BadSlice => "invalid slice: wrong number of slices or a step of zero",
            Self::EmptyReduction => "cannot reduce an empty array",
            Self::BadAxis => "axis or axis permutation out of range",
        };
        f.write_str(message)
    }
}

impl std::error::Error for ArrayError {}

/// One axis slice, with Python-like half-open start/stop semantics.
///
/// Negative bounds count from the end. The step defaults to 1 and must not be zero.
/// Bounds may be omitted; their defaults depend on the sign of the step:
///
/// - positive step: start = 0, stop = len
/// - negative step: start = len - 1, stop = "before index 0"
///
/// Out-of-range bounds are clamped, as in Python. The fields are public so callers can
/// build a slice directly; [Slice::new], [Slice::full] and [Slice::range] are
/// convenience constructors.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Slice {
    /// Inclusive start index; None means the step-dependent default.
    pub start: Option<i64>,
    /// Exclusive stop index; None means the step-dependent default.
    pub stop: Option<i64>,
    /// Signed step; None means 1, and Some(0) is rejected by [NdArray::slice].
    pub step: Option<i64>,
}

impl Slice {
    /// Builds a slice from explicit optional bounds.
    pub const fn new(start: Option<i64>, stop: Option<i64>, step: Option<i64>) -> Self {
        Self { start, stop, step }
    }

    /// Builds the full slice (all indices in order).
    pub const fn full() -> Self {
        Self { start: None, stop: None, step: None }
    }

    /// Builds the half-open range start..stop with the default step.
    pub const fn range(start: i64, stop: i64) -> Self {
        Self { start: Some(start), stop: Some(stop), step: None }
    }

    /// Returns this slice with the step replaced.
    pub const fn with_step(mut self, step: i64) -> Self {
        self.step = Some(step);
        self
    }
}

impl From<std::ops::Range<i64>> for Slice {
    fn from(range: std::ops::Range<i64>) -> Self {
        Self::range(range.start, range.end)
    }
}

impl From<std::ops::RangeFull> for Slice {
    fn from(_: std::ops::RangeFull) -> Self {
        Self::full()
    }
}

/// A shape plus row-major element storage.
///
/// The shape's product always equals the number of stored elements; constructors
/// enforce that invariant. The fields are private and all operations leave self
/// untouched, returning new arrays.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NdArray<T> {
    shape: Vec<usize>,
    data: Vec<T>,
}

/// Returns the product of shape, or None on usize overflow.
///
/// The empty shape has product 1 (a rank-0 array holds one element). Any zero
/// dimension makes the product zero; that is reported before overflow so shapes such
/// as (usize::MAX, 0) still describe an empty array.
fn element_count(shape: &[usize]) -> Option<usize> {
    let mut product = 1usize;
    for &dim in shape {
        if dim == 0 {
            return Some(0);
        }
        product = product.checked_mul(dim)?;
    }
    Some(product)
}

/// Row-major strides of length rank + 1; the last entry is the element stride 1.
fn compute_strides(shape: &[usize]) -> Vec<usize> {
    let mut strides = vec![1usize; shape.len() + 1];
    for axis in (0..shape.len()).rev() {
        strides[axis] = strides[axis + 1] * shape[axis];
    }
    strides
}

/// Normalizes one Python-style slice to (start, step, count).
fn normalize_slice(slice: &Slice, len: usize) -> Result<(i128, i128, usize), ArrayError> {
    let step = i128::from(slice.step.unwrap_or(1));
    if step == 0 {
        return Err(ArrayError::BadSlice);
    }
    let n = len as i128;
    let (start, stop) = if step > 0 {
        (
            clamp_forward(slice.start, n, 0),
            clamp_forward(slice.stop, n, n),
        )
    } else {
        (
            clamp_backward(slice.start, n, n - 1),
            clamp_backward(slice.stop, n, -1),
        )
    };

    let count = if step > 0 {
        if start < stop {
            (stop - start + step - 1) / step
        } else {
            0
        }
    } else {
        let stride = -step;
        if start > stop {
            (start - stop + stride - 1) / stride
        } else {
            0
        }
    };
    let count = usize::try_from(count).map_err(|_| ArrayError::BadSlice)?;
    Ok((start, step, count))
}

/// Python forward-slice bound: negative values count from the end, then clamp to 0..=n.
fn clamp_forward(value: Option<i64>, n: i128, default: i128) -> i128 {
    match value {
        None => default,
        Some(v) => {
            let mut x = i128::from(v);
            if x < 0 {
                x += n;
            }
            x.clamp(0, n)
        }
    }
}

/// Python backward-slice bound: negative values count from the end, then clamp to
/// -1..=n-1, where -1 is the sentinel "before the first element".
fn clamp_backward(value: Option<i64>, n: i128, default: i128) -> i128 {
    match value {
        None => default,
        Some(v) => {
            let mut x = i128::from(v);
            if x < 0 {
                x += n;
            }
            if x < 0 {
                -1
            } else if x >= n {
                n - 1
            } else {
                x
            }
        }
    }
}

impl<T> NdArray<T> {
    /// Builds an array from shape and row-major data.
    ///
    /// # Errors
    ///
    /// Returns [ArrayError::ShapeMismatch] when the product of shape differs from
    /// data.len() or when the product overflows usize. Zero-length dimensions are
    /// allowed; the empty shape denotes a rank-0 array with exactly one element.
    pub fn from_vec(shape: Vec<usize>, data: Vec<T>) -> Result<Self, ArrayError> {
        match element_count(&shape) {
            Some(count) if count == data.len() => Ok(Self { shape, data }),
            _ => Err(ArrayError::ShapeMismatch),
        }
    }

    /// Builds a rank-0 array holding value.
    pub fn scalar(value: T) -> Self {
        Self { shape: Vec::new(), data: vec![value] }
    }

    /// Builds a rank-2 array from rows, rejecting ragged input.
    ///
    /// No rows produce a 0 by 0 shape; rows of length zero produce an n by 0 shape.
    ///
    /// # Errors
    ///
    /// Returns [ArrayError::Ragged] when the rows have different lengths.
    pub fn from_rows(rows: Vec<Vec<T>>) -> Result<Self, ArrayError> {
        let row_count = rows.len();
        let col_count = rows.first().map_or(0, Vec::len);
        if rows.iter().any(|row| row.len() != col_count) {
            return Err(ArrayError::Ragged);
        }
        let data: Vec<T> = rows.into_iter().flatten().collect();
        Ok(Self { shape: vec![row_count, col_count], data })
    }

    /// The shape, outermost dimension first. The empty slice denotes rank 0.
    pub fn shape(&self) -> &[usize] {
        &self.shape
    }

    /// The number of dimensions.
    pub fn rank(&self) -> usize {
        self.shape.len()
    }

    /// The total number of stored elements.
    pub fn numel(&self) -> usize {
        self.data.len()
    }

    /// Returns true when the array stores no elements.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Returns true when the array stores exactly one element, whether rank 0 or not.
    pub fn is_scalar(&self) -> bool {
        self.data.len() == 1
    }

    /// Row-major strides of length rank + 1 (the final entry is 1).
    pub fn strides(&self) -> Vec<usize> {
        compute_strides(&self.shape)
    }

    /// Returns the element at a full index, or None if the index has the wrong rank
    /// or is out of bounds in any dimension.
    pub fn get(&self, index: &[usize]) -> Option<&T> {
        self.offset(index).map(|offset| &self.data[offset])
    }

    /// Returns a mutable element at a full index, with the same checks as [Self::get].
    pub fn get_mut(&mut self, index: &[usize]) -> Option<&mut T> {
        let offset = self.offset(index)?;
        self.data.get_mut(offset)
    }

    /// Computes a row-major offset after validating rank and bounds.
    fn offset(&self, index: &[usize]) -> Option<usize> {
        if index.len() != self.shape.len() {
            return None;
        }
        let mut offset = 0usize;
        let mut stride = 1usize;
        for axis in (0..self.rank()).rev() {
            let i = index[axis];
            if i >= self.shape[axis] {
                return None;
            }
            offset += i * stride;
            stride *= self.shape[axis];
        }
        Some(offset)
    }

    /// The flat row-major storage.
    pub fn as_slice(&self) -> &[T] {
        &self.data
    }

    /// Iterates over the elements in row-major order.
    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        self.data.iter()
    }

    /// Applies f to every element, preserving the shape.
    pub fn map<U, F>(&self, f: F) -> NdArray<U>
    where
        F: Fn(&T) -> U,
    {
        NdArray {
            shape: self.shape.clone(),
            data: self.data.iter().map(f).collect(),
        }
    }

    /// Applies f elementwise to two arrays with equal shapes.
    ///
    /// # Errors
    ///
    /// Returns [ArrayError::ShapeMismatch] when the shapes differ.
    pub fn zip_map<U, F>(&self, other: &Self, f: F) -> Result<NdArray<U>, ArrayError>
    where
        F: Fn(&T, &T) -> U,
    {
        if self.shape != other.shape {
            return Err(ArrayError::ShapeMismatch);
        }
        Ok(NdArray {
            shape: self.shape.clone(),
            data: self
                .data
                .iter()
                .zip(other.data.iter())
                .map(|(left, right)| f(left, right))
                .collect(),
        })
    }

    /// Applies f elementwise, broadcasting a scalar operand.
    ///
    /// If the shapes are equal this behaves like [Self::zip_map]. Otherwise one side
    /// must be a scalar in the sense of [Self::is_scalar] (rank 0 or any shape with a
    /// single element); its value is reused for every element of the other side. If
    /// both sides are scalar with different shapes, the higher-rank shape wins (ties
    /// keep the left shape).
    ///
    /// # Errors
    ///
    /// Returns [ArrayError::ShapeMismatch] when neither equal shapes nor a scalar
    /// broadcast applies.
    pub fn zip_map_broadcast<U, F>(
        &self,
        other: &Self,
        f: F,
    ) -> Result<NdArray<U>, ArrayError>
    where
        F: Fn(&T, &T) -> U,
    {
        if self.shape == other.shape {
            return self.zip_map(other, f);
        }
        if self.is_scalar() && other.is_scalar() {
            let shape = if self.rank() >= other.rank() {
                self.shape.clone()
            } else {
                other.shape.clone()
            };
            return Ok(NdArray {
                shape,
                data: vec![f(&self.data[0], &other.data[0])],
            });
        }
        if self.is_scalar() {
            return Ok(NdArray {
                shape: other.shape.clone(),
                data: other.data.iter().map(|right| f(&self.data[0], right)).collect(),
            });
        }
        if other.is_scalar() {
            return Ok(NdArray {
                shape: self.shape.clone(),
                data: self.data.iter().map(|left| f(left, &other.data[0])).collect(),
            });
        }
        Err(ArrayError::ShapeMismatch)
    }
}

impl<T: Clone> NdArray<T> {
    /// Returns the lower-rank sub-array selected by a prefix of full indices.
    ///
    /// The prefix may have any length from 0 through [Self::rank]. A full index on
    /// any array, including a rank-1 array, yields a rank-0 array. A length-0 prefix
    /// returns a copy of the whole array.
    ///
    /// # Errors
    ///
    /// Returns [ArrayError::IndexOutOfBounds] when the prefix is longer than the rank
    /// or any prefix coordinate is outside its dimension.
    pub fn index_partial(&self, indices: &[usize]) -> Result<Self, ArrayError> {
        if indices.len() > self.rank() {
            return Err(ArrayError::IndexOutOfBounds);
        }
        let strides = compute_strides(&self.shape);
        let mut offset = 0usize;
        for (axis, &index) in indices.iter().enumerate() {
            if index >= self.shape[axis] {
                return Err(ArrayError::IndexOutOfBounds);
            }
            offset += index * strides[axis + 1];
        }
        let shape = self.shape[indices.len()..].to_vec();
        let count = strides[indices.len()];
        Ok(Self { shape, data: self.data[offset..offset + count].to_vec() })
    }

    /// Slices every axis; there must be exactly one [Slice] per dimension.
    ///
    /// The semantics are Python's: half-open bounds, negative indices counted from the
    /// end, clamped out-of-range bounds, and a signed non-zero step. Slicing a rank-0
    /// array requires an empty slice list.
    ///
    /// # Errors
    ///
    /// Returns [ArrayError::BadSlice] when the number of slices differs from the rank
    /// or any step is zero.
    pub fn slice(&self, slices: &[Slice]) -> Result<Self, ArrayError> {
        if slices.len() != self.rank() {
            return Err(ArrayError::BadSlice);
        }
        let strides = compute_strides(&self.shape);
        let mut out_shape = Vec::with_capacity(self.rank());
        let mut axis_indices: Vec<Vec<usize>> = Vec::with_capacity(self.rank());
        for (axis, slice) in slices.iter().enumerate() {
            let (start, step, count) = normalize_slice(slice, self.shape[axis])?;
            let mut selected = Vec::with_capacity(count);
            let mut value = start;
            for _ in 0..count {
                selected.push(value as usize);
                value += step;
            }
            out_shape.push(count);
            axis_indices.push(selected);
        }
        let out_numel = element_count(&out_shape).ok_or(ArrayError::BadSlice)?;
        let mut data = Vec::with_capacity(out_numel);
        for linear in 0..out_numel {
            let mut remainder = linear;
            let mut offset = 0usize;
            for axis in (0..self.rank()).rev() {
                let dim = out_shape[axis];
                let coordinate = remainder % dim;
                remainder /= dim;
                offset += axis_indices[axis][coordinate] * strides[axis + 1];
            }
            data.push(self.data[offset].clone());
        }
        Ok(Self { shape: out_shape, data })
    }

    /// Returns the same row-major data under a new shape.
    ///
    /// # Errors
    ///
    /// Returns [ArrayError::ShapeMismatch] when the new shape's product differs from
    /// [Self::numel].
    pub fn reshape(&self, shape: Vec<usize>) -> Result<Self, ArrayError> {
        if element_count(&shape) != Some(self.numel()) {
            return Err(ArrayError::ShapeMismatch);
        }
        Ok(Self { shape, data: self.data.clone() })
    }

    /// Reverses the axes.
    pub fn transpose(&self) -> Self {
        let permutation: Vec<usize> = (0..self.rank()).rev().collect();
        self.transpose_axes(&permutation)
            .expect("a reversed axis list is always a valid permutation")
    }

    /// Reorders the axes according to axes, where output axis i comes from input
    /// the i-th entry of axes.
    ///
    /// # Errors
    ///
    /// Returns [ArrayError::BadAxis] unless axes is a permutation of 0..rank.
    pub fn transpose_axes(&self, axes: &[usize]) -> Result<Self, ArrayError> {
        let rank = self.rank();
        if axes.len() != rank {
            return Err(ArrayError::BadAxis);
        }
        let mut seen = vec![false; rank];
        for &axis in axes {
            if axis >= rank || seen[axis] {
                return Err(ArrayError::BadAxis);
            }
            seen[axis] = true;
        }
        let out_shape: Vec<usize> = axes.iter().map(|&axis| self.shape[axis]).collect();
        let out_numel = element_count(&out_shape).ok_or(ArrayError::BadAxis)?;
        let strides = compute_strides(&self.shape);
        let mut data = Vec::with_capacity(out_numel);
        for linear in 0..out_numel {
            let mut remainder = linear;
            let mut offset = 0usize;
            for out_axis in (0..rank).rev() {
                let dim = out_shape[out_axis];
                let coordinate = remainder % dim;
                remainder /= dim;
                offset += coordinate * strides[axes[out_axis] + 1];
            }
            data.push(self.data[offset].clone());
        }
        Ok(Self { shape: out_shape, data })
    }

    /// Removes every dimension of length 1.
    ///
    /// A shape made only of singletons therefore collapses to rank 0, which still
    /// holds the same single element. A rank-0 array is unchanged.
    pub fn squeeze(&self) -> Self {
        let shape = self.shape.iter().copied().filter(|&dim| dim != 1).collect();
        Self { shape, data: self.data.clone() }
    }

    /// Returns a rank-1 copy of the row-major data.
    pub fn flatten(&self) -> Self {
        Self { shape: vec![self.numel()], data: self.data.clone() }
    }

    /// Concatenates arrays along axis.
    ///
    /// All operands must have the same rank and the same sizes on every axis other
    /// than axis. At least one operand is required.
    ///
    /// # Errors
    ///
    /// Returns [ArrayError::ShapeMismatch] for rank or shape disagreements and
    /// [ArrayError::BadAxis] for an out-of-range axis (including an empty input list,
    /// for which no axis can be validated).
    pub fn concat(arrays: &[Self], axis: usize) -> Result<Self, ArrayError> {
        let parts: Vec<&Self> = arrays.iter().collect();
        Self::concat_refs(&parts, axis)
    }

    /// Concatenates this array and other along axis.
    ///
    /// # Errors
    ///
    /// See [Self::concat].
    pub fn append(&self, other: &Self, axis: usize) -> Result<Self, ArrayError> {
        Self::concat_refs(&[self, other], axis)
    }

    /// Shared implementation of [Self::concat] and [Self::append].
    fn concat_refs(parts: &[&Self], axis: usize) -> Result<Self, ArrayError> {
        let first = parts.first().ok_or(ArrayError::BadAxis)?;
        let rank = first.rank();
        if axis >= rank {
            return Err(ArrayError::BadAxis);
        }
        let mut out_shape = first.shape.clone();
        let mut axis_total = 0usize;
        for part in parts {
            if part.rank() != rank {
                return Err(ArrayError::ShapeMismatch);
            }
            for dim in 0..rank {
                if dim != axis && part.shape[dim] != first.shape[dim] {
                    return Err(ArrayError::ShapeMismatch);
                }
            }
            axis_total = axis_total
                .checked_add(part.shape[axis])
                .ok_or(ArrayError::ShapeMismatch)?;
        }
        out_shape[axis] = axis_total;
        let out_numel = element_count(&out_shape).ok_or(ArrayError::ShapeMismatch)?;
        let strides: Vec<Vec<usize>> =
            parts.iter().map(|part| compute_strides(&part.shape)).collect();
        let mut data = Vec::with_capacity(out_numel);
        for linear in 0..out_numel {
            let mut remainder = linear;
            let mut coordinates = vec![0usize; rank];
            for dim in (0..rank).rev() {
                coordinates[dim] = remainder % out_shape[dim];
                remainder /= out_shape[dim];
            }
            let mut part_index = 0usize;
            let mut axis_coordinate = coordinates[axis];
            while axis_coordinate >= parts[part_index].shape[axis] {
                axis_coordinate -= parts[part_index].shape[axis];
                part_index += 1;
            }
            let part = parts[part_index];
            let mut offset = 0usize;
            for dim in 0..rank {
                let coordinate = if dim == axis {
                    axis_coordinate
                } else {
                    coordinates[dim]
                };
                offset += coordinate * strides[part_index][dim + 1];
            }
            data.push(part.data[offset].clone());
        }
        Ok(Self { shape: out_shape, data })
    }
}

impl<T: Field> NdArray<T> {
    /// Builds an array of shape filled with [Field::zero].
    pub fn zeros(shape: Vec<usize>) -> Self {
        let count = element_count(&shape).expect("array shape overflows usize");
        Self { shape, data: vec![T::zero(); count] }
    }

    /// Builds an array of shape filled with [Field::one].
    pub fn ones(shape: Vec<usize>) -> Self {
        let count = element_count(&shape).expect("array shape overflows usize");
        Self { shape, data: vec![T::one(); count] }
    }

    /// Builds the n by n identity matrix.
    pub fn eye(n: usize) -> Self {
        let count = element_count(&[n, n]).expect("identity shape overflows usize");
        let mut data = vec![T::zero(); count];
        for i in 0..n {
            data[i * n + i] = T::one();
        }
        Self { shape: vec![n, n], data }
    }

    /// Sums all elements. The empty sum is [Field::zero].
    pub fn sum(&self) -> T {
        let mut accumulator = T::zero();
        for element in &self.data {
            accumulator = T::add(&accumulator, element);
        }
        accumulator
    }

    /// Multiplies all elements. The empty product is [Field::one].
    pub fn prod(&self) -> T {
        let mut accumulator = T::one();
        for element in &self.data {
            accumulator = T::mul(&accumulator, element);
        }
        accumulator
    }

    /// Reduces axis to a single value per output cell by folding with op.
    ///
    /// The output has the input shape with axis removed; reducing a rank-1 array
    /// therefore yields a rank-0 array. Reducing a zero-length axis yields the seed for
    /// every output cell.
    fn reduce_axis<F>(&self, axis: usize, seed: T, op: F) -> Result<Self, ArrayError>
    where
        F: Fn(&T, &T) -> T,
    {
        if axis >= self.rank() {
            return Err(ArrayError::BadAxis);
        }
        let out_shape: Vec<usize> = self
            .shape
            .iter()
            .enumerate()
            .filter_map(|(dim, &size)| (dim != axis).then_some(size))
            .collect();
        let out_numel = element_count(&out_shape).ok_or(ArrayError::BadAxis)?;
        let strides = compute_strides(&self.shape);
        let mut data = Vec::with_capacity(out_numel);
        for linear in 0..out_numel {
            let mut remainder = linear;
            let mut base = 0usize;
            for out_axis in (0..out_shape.len()).rev() {
                let dim = out_shape[out_axis];
                let coordinate = remainder % dim;
                remainder /= dim;
                let source_axis = if out_axis < axis { out_axis } else { out_axis + 1 };
                base += coordinate * strides[source_axis + 1];
            }
            let mut accumulator = seed.clone();
            for step in 0..self.shape[axis] {
                accumulator = op(&accumulator, &self.data[base + step * strides[axis + 1]]);
            }
            data.push(accumulator);
        }
        Ok(Self { shape: out_shape, data })
    }

    /// Sums along one axis, removing it from the output shape.
    ///
    /// Reducing a zero-length axis produces zeros, consistent with the empty sum.
    ///
    /// # Errors
    ///
    /// Returns [ArrayError::BadAxis] when axis >= rank.
    pub fn sum_axis(&self, axis: usize) -> Result<Self, ArrayError> {
        self.reduce_axis(axis, T::zero(), T::add)
    }

    /// Multiplies along one axis, removing it from the output shape.
    ///
    /// Reducing a zero-length axis produces ones, consistent with the empty product.
    ///
    /// # Errors
    ///
    /// Returns [ArrayError::BadAxis] when axis >= rank.
    pub fn prod_axis(&self, axis: usize) -> Result<Self, ArrayError> {
        self.reduce_axis(axis, T::one(), T::mul)
    }
}

impl<T: Ord + Clone> NdArray<T> {
    /// Returns the smallest element.
    ///
    /// # Errors
    ///
    /// Returns [ArrayError::EmptyReduction] when the array stores no elements.
    pub fn min(&self) -> Result<T, ArrayError> {
        self.data
            .iter()
            .min()
            .cloned()
            .ok_or(ArrayError::EmptyReduction)
    }

    /// Returns the largest element.
    ///
    /// # Errors
    ///
    /// Returns [ArrayError::EmptyReduction] when the array stores no elements.
    pub fn max(&self) -> Result<T, ArrayError> {
        self.data
            .iter()
            .max()
            .cloned()
            .ok_or(ArrayError::EmptyReduction)
    }
}

impl<'a, T> IntoIterator for &'a NdArray<T> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.data.iter()
    }
}

#[cfg(test)]
#[allow(non_snake_case)]
mod tests {
    use super::*;

    fn arr(shape: Vec<usize>, data: Vec<i64>) -> NdArray<i64> {
        NdArray::from_vec(shape, data).expect("test fixture must be valid")
    }

    #[test]
    fn FromVec_GivenShapeAndData_ExposesShapeRankNumelAndStrides() {
        let a = arr(vec![2, 3, 4], (1..=24).collect());
        assert_eq!(a.shape(), &[2, 3, 4]);
        assert_eq!(a.rank(), 3);
        assert_eq!(a.numel(), 24);
        assert!(!a.is_empty());
        assert_eq!(a.strides(), vec![24, 12, 4, 1]);
        assert_eq!(a.get(&[0, 0, 0]), Some(&1));
        assert_eq!(a.get(&[1, 2, 3]), Some(&24));
    }

    #[test]
    fn FromVec_GivenRankZeroShape_IsScalar() {
        let s = NdArray::from_vec(vec![], vec![42]).unwrap();
        assert!(s.shape().is_empty());
        assert_eq!(s.rank(), 0);
        assert_eq!(s.numel(), 1);
        assert!(!s.is_empty());
        assert_eq!(s.as_slice(), &[42]);
        assert_eq!(s.get(&[]), Some(&42));
        assert_eq!(s.strides(), vec![1]);
    }

    #[test]
    fn FromVec_GivenMismatchedData_ReturnsShapeMismatch() {
        let error = NdArray::from_vec(vec![2, 3], vec![1, 2, 3, 4, 5]).unwrap_err();
        assert_eq!(error, ArrayError::ShapeMismatch);
        assert!(!error.to_string().is_empty());
    }

    #[test]
    fn FromVec_GivenZeroLengthDimension_AllowsEmptyArray() {
        let a = arr(vec![2, 0, 3], vec![]);
        assert_eq!(a.shape(), &[2, 0, 3]);
        assert_eq!(a.numel(), 0);
        assert!(a.is_empty());
        assert_eq!(a.strides(), vec![0, 0, 3, 1]);
        assert_eq!(a.get(&[0, 0, 0]), None);
        let zero = arr(vec![0], vec![]);
        assert_eq!(zero.shape(), &[0]);
        assert_eq!(zero.numel(), 0);
    }

    #[test]
    fn Scalar_GivenValue_HasRankZeroAndSingleElement() {
        let s = NdArray::scalar(7);
        assert_eq!(s.shape(), &[] as &[usize]);
        assert_eq!(s.rank(), 0);
        assert_eq!(s.numel(), 1);
        assert!(s.is_scalar());
        assert_eq!(s.as_slice(), &[7]);
        assert_eq!(s.iter().copied().collect::<Vec<_>>(), vec![7]);
    }

    #[test]
    fn ZerosAndOnes_GivenShape_FillConstantsInRowMajorOrder() {
        let z: NdArray<i64> = NdArray::zeros(vec![2, 3]);
        assert_eq!(z.shape(), &[2, 3]);
        assert_eq!(z.as_slice(), &[0, 0, 0, 0, 0, 0]);
        let o: NdArray<i64> = NdArray::ones(vec![2, 2]);
        assert_eq!(o.as_slice(), &[1, 1, 1, 1]);
        let scalar_zero: NdArray<i64> = NdArray::zeros(vec![]);
        assert_eq!(scalar_zero.rank(), 0);
        assert_eq!(scalar_zero.as_slice(), &[0]);
        let empty: NdArray<i64> = NdArray::ones(vec![2, 0]);
        assert!(empty.is_empty());
        assert_eq!(empty.shape(), &[2, 0]);
    }

    #[test]
    fn Eye_GivenSize_PlacesOnesOnDiagonal() {
        let e: NdArray<i64> = NdArray::eye(3);
        assert_eq!(e.shape(), &[3, 3]);
        assert_eq!(e.as_slice(), &[1, 0, 0, 0, 1, 0, 0, 0, 1]);
        let z: NdArray<i64> = NdArray::eye(0);
        assert_eq!(z.shape(), &[0, 0]);
        assert!(z.is_empty());
    }

    #[test]
    fn FromRows_GivenRectangularRows_ProducesMatrix() {
        let m = NdArray::from_rows(vec![vec![1, 2, 3], vec![4, 5, 6]]).unwrap();
        assert_eq!(m.shape(), &[2, 3]);
        assert_eq!(m.as_slice(), &[1, 2, 3, 4, 5, 6]);
        assert_eq!(m.get(&[1, 2]), Some(&6));
    }

    #[test]
    fn FromRows_GivenRaggedRows_ReturnsRagged() {
        let error = NdArray::from_rows(vec![vec![1, 2], vec![3]]).unwrap_err();
        assert_eq!(error, ArrayError::Ragged);
    }

    #[test]
    fn FromRows_GivenNoRows_ProducesEmptyMatrix() {
        let m = NdArray::<i64>::from_rows(vec![]).unwrap();
        assert_eq!(m.shape(), &[0, 0]);
        assert!(m.is_empty());
        let zero_columns = NdArray::<i64>::from_rows(vec![vec![], vec![]]).unwrap();
        assert_eq!(zero_columns.shape(), &[2, 0]);
        assert!(zero_columns.is_empty());
    }

    #[test]
    fn GetAndGetMut_GivenIndices_ReadWriteAndBoundsCheck() {
        let mut a = arr(vec![2, 3], vec![1, 2, 3, 4, 5, 6]);
        assert_eq!(a.get(&[0, 0]), Some(&1));
        assert_eq!(a.get(&[1, 2]), Some(&6));
        assert_eq!(a.get(&[2, 0]), None);
        assert_eq!(a.get(&[0]), None);
        assert_eq!(a.get(&[0, 1, 2]), None);
        *a.get_mut(&[0, 1]).unwrap() = 20;
        assert_eq!(a.get(&[0, 1]), Some(&20));
        assert_eq!(a.get_mut(&[9, 9]), None);
    }

    #[test]
    fn IterAndIntoIterator_GivenMatrix_YieldRowMajorOrder() {
        let m = arr(vec![2, 2], vec![1, 2, 3, 4]);
        assert_eq!(m.iter().copied().collect::<Vec<_>>(), vec![1, 2, 3, 4]);
        let mut from_into = Vec::new();
        for element in &m {
            from_into.push(*element);
        }
        assert_eq!(from_into, vec![1, 2, 3, 4]);
    }

    #[test]
    fn IndexPartial_GivenMatrixPrefix_ReturnsLowerRankSubArray() {
        let m = arr(vec![2, 3], vec![1, 2, 3, 4, 5, 6]);
        let row = m.index_partial(&[1]).unwrap();
        assert_eq!(row.shape(), &[3]);
        assert_eq!(row.as_slice(), &[4, 5, 6]);
        let identity = m.index_partial(&[]).unwrap();
        assert_eq!(identity, m);
    }

    #[test]
    fn IndexPartial_GivenFullIndex_ReturnsRankZero() {
        let v = arr(vec![3], vec![10, 20, 30]);
        let element = v.index_partial(&[1]).unwrap();
        assert_eq!(element.rank(), 0);
        assert!(element.shape().is_empty());
        assert_eq!(element.as_slice(), &[20]);
        let m = arr(vec![2, 3], vec![1, 2, 3, 4, 5, 6]);
        let corner = m.index_partial(&[1, 2]).unwrap();
        assert_eq!(corner.rank(), 0);
        assert_eq!(corner.as_slice(), &[6]);
        let cube = arr(vec![2, 3, 2], (0..12).collect());
        let plane = cube.index_partial(&[1]).unwrap();
        assert_eq!(plane.shape(), &[3, 2]);
        assert_eq!(plane.as_slice(), &[6, 7, 8, 9, 10, 11]);
        assert_eq!(cube.index_partial(&[1, 2]).unwrap().as_slice(), &[10, 11]);
    }

    #[test]
    fn IndexPartial_GivenOutOfBounds_ReturnsIndexOutOfBounds() {
        let m = arr(vec![2, 3], vec![1, 2, 3, 4, 5, 6]);
        assert_eq!(m.index_partial(&[2]).unwrap_err(), ArrayError::IndexOutOfBounds);
        assert_eq!(m.index_partial(&[0, 3]).unwrap_err(), ArrayError::IndexOutOfBounds);
        assert_eq!(m.index_partial(&[0, 0, 0]).unwrap_err(), ArrayError::IndexOutOfBounds);
    }

    #[test]
    fn Slice_GivenNegativeBounds_UsesPythonSemantics() {
        let v = arr(vec![10], (0..10).collect());
        let tail = v.slice(&[Slice::new(Some(-3), None, None)]).unwrap();
        assert_eq!(tail.shape(), &[3]);
        assert_eq!(tail.as_slice(), &[7, 8, 9]);
        let head = v.slice(&[Slice::new(None, Some(-2), None)]).unwrap();
        assert_eq!(head.as_slice(), &[0, 1, 2, 3, 4, 5, 6, 7]);
        let clamped = v.slice(&[Slice::new(Some(-100), Some(100), Some(3))]).unwrap();
        assert_eq!(clamped.as_slice(), &[0, 3, 6, 9]);
    }

    #[test]
    fn Slice_GivenPositiveStep_SelectsSubsequence() {
        let v = arr(vec![10], (0..10).collect());
        let selected = v.slice(&[Slice::new(Some(1), Some(8), Some(2))]).unwrap();
        assert_eq!(selected.shape(), &[4]);
        assert_eq!(selected.as_slice(), &[1, 3, 5, 7]);
        let empty = v.slice(&[Slice::range(2, 2)]).unwrap();
        assert_eq!(empty.shape(), &[0]);
        assert!(empty.is_empty());
    }

    #[test]
    fn Slice_GivenNegativeStep_TraversesBackwards() {
        let v = arr(vec![10], (0..10).collect());
        let reversed = v.slice(&[Slice::full().with_step(-1)]).unwrap();
        assert_eq!(reversed.shape(), &[10]);
        assert_eq!(reversed.as_slice(), &[9, 8, 7, 6, 5, 4, 3, 2, 1, 0]);
        let evens = v.slice(&[Slice::new(Some(8), Some(2), Some(-2))]).unwrap();
        assert_eq!(evens.as_slice(), &[8, 6, 4]);
    }

    #[test]
    fn Slice_GivenMatrixSlices_ReturnsSubMatrix() {
        let m = arr(vec![3, 4], (1..=12).collect());
        let sub = m
            .slice(&[Slice::range(1, 3), Slice::new(Some(1), Some(4), Some(2))])
            .unwrap();
        assert_eq!(sub.shape(), &[2, 2]);
        assert_eq!(sub.as_slice(), &[6, 8, 10, 12]);
        let negative = m
            .slice(&[Slice::range(1, -1), Slice::new(Some(-2), None, None)])
            .unwrap();
        assert_eq!(negative.shape(), &[1, 2]);
        assert_eq!(negative.as_slice(), &[7, 8]);
        let empty = m
            .slice(&[Slice::range(0, 0), Slice::range(0, 4)])
            .unwrap();
        assert_eq!(empty.shape(), &[0, 4]);
        assert!(empty.is_empty());
        let scalar = NdArray::<i64>::scalar(5);
        assert_eq!(scalar.slice(&[]).unwrap(), scalar);
    }

    #[test]
    fn Slice_GivenWrongCountOrZeroStep_ReturnsBadSlice() {
        let m = arr(vec![3, 4], (1..=12).collect());
        assert_eq!(m.slice(&[Slice::full()]).unwrap_err(), ArrayError::BadSlice);
        let v = arr(vec![3], vec![1, 2, 3]);
        assert_eq!(
            v.slice(&[Slice::new(None, None, Some(0))]).unwrap_err(),
            ArrayError::BadSlice
        );
    }

    #[test]
    fn Reshape_GivenMatchingNumel_PreservesRowMajorOrder() {
        let m = arr(vec![2, 3], (1..=6).collect());
        let reshaped = m.reshape(vec![3, 2]).unwrap();
        assert_eq!(reshaped.shape(), &[3, 2]);
        assert_eq!(reshaped.as_slice(), &[1, 2, 3, 4, 5, 6]);
        assert_eq!(reshaped.get(&[2, 1]), Some(&6));
        let single = NdArray::<i64>::scalar(7).reshape(vec![1, 1]).unwrap();
        assert_eq!(single.shape(), &[1, 1]);
        assert_eq!(single.as_slice(), &[7]);
        let empty = arr(vec![0], vec![]);
        assert_eq!(empty.reshape(vec![2, 0]).unwrap().shape(), &[2, 0]);
    }

    #[test]
    fn Reshape_GivenMismatchedNumel_ReturnsShapeMismatch() {
        let m = arr(vec![2, 3], (1..=6).collect());
        assert_eq!(m.reshape(vec![4, 2]).unwrap_err(), ArrayError::ShapeMismatch);
        assert_eq!(
            NdArray::<i64>::scalar(1).reshape(vec![0]).unwrap_err(),
            ArrayError::ShapeMismatch
        );
    }

    #[test]
    fn Transpose_GivenMatrix_ReversesAxes() {
        let m = arr(vec![2, 3], vec![1, 2, 3, 4, 5, 6]);
        let t = m.transpose();
        assert_eq!(t.shape(), &[3, 2]);
        assert_eq!(t.as_slice(), &[1, 4, 2, 5, 3, 6]);
        assert_eq!(t.get(&[2, 1]), Some(&6));
    }

    #[test]
    fn Transpose_GivenMatrixTwice_ReturnsOriginal() {
        let m = arr(vec![2, 3], vec![1, 2, 3, 4, 5, 6]);
        assert_eq!(m.transpose().transpose(), m);
        let v = arr(vec![3], vec![1, 2, 3]);
        assert_eq!(v.transpose(), v);
        let s = NdArray::<i64>::scalar(9);
        assert_eq!(s.transpose(), s);
    }

    #[test]
    fn TransposeAxes_GivenPermutation_ReordersData() {
        let a = arr(vec![2, 3, 4], (0..24).collect());
        let p = a.transpose_axes(&[2, 0, 1]).unwrap();
        assert_eq!(p.shape(), &[4, 2, 3]);
        assert_eq!(p.get(&[0, 0, 0]), Some(&0));
        assert_eq!(p.get(&[0, 0, 1]), Some(&4));
        assert_eq!(p.get(&[0, 1, 0]), Some(&12));
        assert_eq!(p.get(&[3, 1, 2]), a.get(&[1, 2, 3]));
        assert_eq!(p.get(&[3, 1, 2]), Some(&23));
    }

    #[test]
    fn TransposeAxes_GivenInvalidPermutation_ReturnsBadAxis() {
        let a = arr(vec![2, 3, 4], (0..24).collect());
        assert_eq!(a.transpose_axes(&[0, 0, 2]).unwrap_err(), ArrayError::BadAxis);
        assert_eq!(a.transpose_axes(&[0, 1]).unwrap_err(), ArrayError::BadAxis);
        assert_eq!(a.transpose_axes(&[0, 1, 3]).unwrap_err(), ArrayError::BadAxis);
    }

    #[test]
    fn Squeeze_GivenSingletonDimensions_RemovesThem() {
        let a = arr(vec![1, 3, 1, 2], (1..=6).collect());
        let squeezed = a.squeeze();
        assert_eq!(squeezed.shape(), &[3, 2]);
        assert_eq!(squeezed.as_slice(), &[1, 2, 3, 4, 5, 6]);
        let with_zero = arr(vec![1, 0, 1], vec![]);
        assert_eq!(with_zero.squeeze().shape(), &[0]);
    }

    #[test]
    fn Squeeze_GivenAllSingletons_ReturnsRankZero() {
        let a = arr(vec![1, 1], vec![5]);
        let squeezed = a.squeeze();
        assert_eq!(squeezed.rank(), 0);
        assert_eq!(squeezed.as_slice(), &[5]);
        let scalar = NdArray::<i64>::scalar(5).squeeze();
        assert_eq!(scalar.rank(), 0);
        assert_eq!(scalar.as_slice(), &[5]);
    }

    #[test]
    fn Flatten_GivenMatrix_ReturnsRowMajorVector() {
        let m = arr(vec![2, 3], vec![1, 2, 3, 4, 5, 6]);
        let flat = m.flatten();
        assert_eq!(flat.shape(), &[6]);
        assert_eq!(flat.as_slice(), &[1, 2, 3, 4, 5, 6]);
        let t = m.transpose();
        assert_eq!(t.flatten().as_slice(), &[1, 4, 2, 5, 3, 6]);
        let s = NdArray::<i64>::scalar(5).flatten();
        assert_eq!(s.shape(), &[1]);
        assert_eq!(s.as_slice(), &[5]);
    }

    #[test]
    fn Concat_GivenMatricesOnAxisZero_StacksRows() {
        let a = arr(vec![2, 2], vec![1, 2, 3, 4]);
        let b = arr(vec![2, 2], vec![5, 6, 7, 8]);
        let joined = NdArray::concat(&[a, b], 0).unwrap();
        assert_eq!(joined.shape(), &[4, 2]);
        assert_eq!(joined.as_slice(), &[1, 2, 3, 4, 5, 6, 7, 8]);
    }

    #[test]
    fn Concat_GivenMatricesOnAxisOne_AppendsColumns() {
        let a = arr(vec![2, 2], vec![1, 2, 3, 4]);
        let b = arr(vec![2, 2], vec![5, 6, 7, 8]);
        let joined = NdArray::concat(&[a, b], 1).unwrap();
        assert_eq!(joined.shape(), &[2, 4]);
        assert_eq!(joined.as_slice(), &[1, 2, 5, 6, 3, 4, 7, 8]);
    }

    #[test]
    fn Concat_GivenThreeVectorsAndEmptyPart_KeepsOrder() {
        let x = arr(vec![2], vec![1, 2]);
        let y = arr(vec![1], vec![3]);
        let z = arr(vec![3], vec![4, 5, 6]);
        let joined = NdArray::concat(&[x, y, z], 0).unwrap();
        assert_eq!(joined.shape(), &[6]);
        assert_eq!(joined.as_slice(), &[1, 2, 3, 4, 5, 6]);
        let empty = arr(vec![0, 2], vec![]);
        let a = arr(vec![2, 2], vec![1, 2, 3, 4]);
        let with_empty = NdArray::concat(&[empty, a], 0).unwrap();
        assert_eq!(with_empty.shape(), &[2, 2]);
        assert_eq!(with_empty.as_slice(), &[1, 2, 3, 4]);
    }

    #[test]
    fn Concat_GivenMismatchOrBadAxis_ReturnsErrors() {
        let a = arr(vec![2, 2], vec![1, 2, 3, 4]);
        let vector = arr(vec![3], vec![1, 2, 3]);
        assert_eq!(
            NdArray::concat(&[a.clone(), vector], 0).unwrap_err(),
            ArrayError::ShapeMismatch
        );
        let different = arr(vec![2, 3], (1..=6).collect());
        assert_eq!(
            NdArray::concat(&[a.clone(), different], 0).unwrap_err(),
            ArrayError::ShapeMismatch
        );
        assert_eq!(
            NdArray::concat(&[a.clone(), a.clone()], 2).unwrap_err(),
            ArrayError::BadAxis
        );
        assert_eq!(NdArray::<i64>::concat(&[], 0).unwrap_err(), ArrayError::BadAxis);
    }

    #[test]
    fn Append_GivenMatrixAlongAxisZero_Concatenates() {
        let a = arr(vec![2, 2], vec![1, 2, 3, 4]);
        let b = arr(vec![1, 2], vec![5, 6]);
        let joined = a.append(&b, 0).unwrap();
        assert_eq!(joined.shape(), &[3, 2]);
        assert_eq!(joined.as_slice(), &[1, 2, 3, 4, 5, 6]);
        assert_eq!(a.append(&b, 2).unwrap_err(), ArrayError::BadAxis);
    }

    #[test]
    fn Map_GivenFunction_PreservesShape() {
        let m = arr(vec![2, 2], vec![1, 2, 3, 4]);
        let mapped = m.map(|value| *value * 10);
        assert_eq!(mapped.shape(), &[2, 2]);
        assert_eq!(mapped.as_slice(), &[10, 20, 30, 40]);
        let scalar = NdArray::<i64>::scalar(4).map(|value| value + 1);
        assert_eq!(scalar.rank(), 0);
        assert_eq!(scalar.as_slice(), &[5]);
    }

    #[test]
    fn ZipMap_GivenEqualShapes_AppliesElementwise() {
        let a = arr(vec![2, 2], vec![1, 2, 3, 4]);
        let b = arr(vec![2, 2], vec![10, 20, 30, 40]);
        let sum = a.zip_map(&b, |left, right| *left + *right).unwrap();
        assert_eq!(sum.shape(), &[2, 2]);
        assert_eq!(sum.as_slice(), &[11, 22, 33, 44]);
    }

    #[test]
    fn ZipMap_GivenDifferentShapes_ReturnsShapeMismatch() {
        let a = arr(vec![2, 2], vec![1, 2, 3, 4]);
        let b = arr(vec![4], vec![1, 2, 3, 4]);
        assert_eq!(
            a.zip_map(&b, |left, right| *left + *right).unwrap_err(),
            ArrayError::ShapeMismatch
        );
    }

    #[test]
    fn ZipMapBroadcast_GivenScalar_ReusesValue() {
        let m = arr(vec![2, 3], vec![1, 2, 3, 4, 5, 6]);
        let scalar = NdArray::<i64>::scalar(10);
        let added = m.zip_map_broadcast(&scalar, |left, right| *left + *right).unwrap();
        assert_eq!(added.shape(), &[2, 3]);
        assert_eq!(added.as_slice(), &[11, 12, 13, 14, 15, 16]);
        let multiplied = scalar
            .zip_map_broadcast(&m, |left, right| *left * *right)
            .unwrap();
        assert_eq!(multiplied.as_slice(), &[10, 20, 30, 40, 50, 60]);
    }

    #[test]
    fn ZipMapBroadcast_GivenSingleElementArray_ReusesValue() {
        let row = arr(vec![1], vec![3]);
        let vector = arr(vec![3], vec![1, 2, 3]);
        let result = vector
            .zip_map_broadcast(&row, |left, right| *left * *right)
            .unwrap();
        assert_eq!(result.shape(), &[3]);
        assert_eq!(result.as_slice(), &[3, 6, 9]);
        let left = NdArray::<i64>::scalar(2);
        let right = arr(vec![1, 1], vec![3]);
        let both = left
            .zip_map_broadcast(&right, |a, b| *a + *b)
            .unwrap();
        assert_eq!(both.shape(), &[1, 1]);
        assert_eq!(both.as_slice(), &[5]);
        let empty = arr(vec![0, 3], vec![]);
        let broadcast = empty
            .zip_map_broadcast(&NdArray::<i64>::scalar(2), |a, b| *a + *b)
            .unwrap();
        assert_eq!(broadcast.shape(), &[0, 3]);
        assert!(broadcast.is_empty());
    }

    #[test]
    fn ZipMapBroadcast_GivenIncompatibleShapes_ReturnsShapeMismatch() {
        let m = arr(vec![2, 3], vec![1, 2, 3, 4, 5, 6]);
        let other = arr(vec![2], vec![1, 2]);
        assert_eq!(
            m.zip_map_broadcast(&other, |a, b| *a + *b).unwrap_err(),
            ArrayError::ShapeMismatch
        );
    }

    #[test]
    fn SumAndProd_GivenMatrix_ReduceAllElements() {
        let m = arr(vec![2, 3], vec![1, 2, 3, 4, 5, 6]);
        assert_eq!(m.sum(), 21);
        assert_eq!(m.prod(), 720);
    }

    #[test]
    fn MinAndMax_GivenVector_ReturnExtremes() {
        let v = arr(vec![6], vec![3, -2, 7, 0, 5, 1]);
        assert_eq!(v.min().unwrap(), -2);
        assert_eq!(v.max().unwrap(), 7);
    }

    #[test]
    fn Reductions_GivenEmptyArray_UseIdentitiesAndEmptyReduction() {
        let empty = arr(vec![0, 3], vec![]);
        assert_eq!(empty.sum(), 0);
        assert_eq!(empty.prod(), 1);
        assert_eq!(empty.min(), Err(ArrayError::EmptyReduction));
        assert_eq!(empty.max(), Err(ArrayError::EmptyReduction));
    }

    #[test]
    fn SumAxis_GivenMatrix_ReducesRequestedAxis() {
        let m = arr(vec![2, 3], vec![1, 2, 3, 4, 5, 6]);
        let columns = m.sum_axis(0).unwrap();
        assert_eq!(columns.shape(), &[3]);
        assert_eq!(columns.as_slice(), &[5, 7, 9]);
        let rows = m.sum_axis(1).unwrap();
        assert_eq!(rows.shape(), &[2]);
        assert_eq!(rows.as_slice(), &[6, 15]);
    }

    #[test]
    fn SumAxis_GivenVector_ReturnsRankZero() {
        let v = arr(vec![3], vec![1, 2, 3]);
        let total = v.sum_axis(0).unwrap();
        assert_eq!(total.rank(), 0);
        assert_eq!(total.as_slice(), &[6]);
    }

    #[test]
    fn SumAxis_GivenBadAxis_ReturnsBadAxis() {
        let m = arr(vec![2, 3], vec![1, 2, 3, 4, 5, 6]);
        assert_eq!(m.sum_axis(2).unwrap_err(), ArrayError::BadAxis);
        let scalar = NdArray::<i64>::scalar(5);
        assert_eq!(scalar.sum_axis(0).unwrap_err(), ArrayError::BadAxis);
    }

    #[test]
    fn SumAxis_GivenZeroLengthAxis_ReturnsIdentity() {
        let m = arr(vec![2, 0], vec![]);
        let sums = m.sum_axis(1).unwrap();
        assert_eq!(sums.shape(), &[2]);
        assert_eq!(sums.as_slice(), &[0, 0]);
        let products = m.prod_axis(1).unwrap();
        assert_eq!(products.as_slice(), &[1, 1]);
    }

    #[test]
    fn ProdAxis_GivenMatrix_ReducesRequestedAxis() {
        let m = arr(vec![2, 3], vec![1, 2, 3, 4, 5, 6]);
        let columns = m.prod_axis(0).unwrap();
        assert_eq!(columns.shape(), &[3]);
        assert_eq!(columns.as_slice(), &[4, 10, 18]);
        let rows = m.prod_axis(1).unwrap();
        assert_eq!(rows.as_slice(), &[6, 120]);
        assert_eq!(m.prod_axis(3).unwrap_err(), ArrayError::BadAxis);
    }

    #[test]
    fn FieldForI64_GivenOperations_MatchesIntegerArithmetic() {
        assert_eq!(<i64 as Field>::zero(), 0);
        assert_eq!(<i64 as Field>::one(), 1);
        assert_eq!(<i64 as Field>::add(&2, &3), 5);
        assert_eq!(<i64 as Field>::sub(&2, &3), -1);
        assert_eq!(<i64 as Field>::mul(&2, &3), 6);
        assert_eq!(<i64 as Field>::div(&7, &2), 3);
        assert_eq!(<i64 as Field>::neg(&2), -2);
        assert!(<i64 as Field>::is_zero(&0));
        assert!(!<i64 as Field>::is_zero(&1));
        assert_eq!(<i64 as Field>::from_i64(-7), -7);
    }

    #[test]
    fn ArrayError_GivenEachVariant_DisplaysAndImplementsError() {
        let errors = [
            ArrayError::ShapeMismatch,
            ArrayError::Ragged,
            ArrayError::IndexOutOfBounds,
            ArrayError::BadSlice,
            ArrayError::EmptyReduction,
            ArrayError::BadAxis,
        ];
        for error in errors {
            assert!(!error.to_string().is_empty());
            let as_trait_object: &dyn std::error::Error = &error;
            assert!(!as_trait_object.to_string().is_empty());
        }
    }
}
