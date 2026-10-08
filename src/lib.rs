//! Retain a clamped range without replacing the vector allocation.

use std::{
    ops::{Bound, RangeBounds},
    ptr,
};

pub trait VecRemain<R: RangeBounds<usize>> {
    /// Retain the clamped range in place, returning its length. Reversed ranges
    /// are empty. The allocation is preserved. If prefix drop panics, the
    /// retained values are still owned by this vector.
    fn remain(&mut self, range: R) -> usize;
    /// Append the clamped range to `other`, emptying this vector on success.
    /// Destination capacity is reserved before changing either owner. A tail
    /// drop panic leaves the truncated source owned; a prefix drop panic leaves
    /// the retained values owned by `other` and the source empty.
    fn remain_to(&mut self, range: R, other: &mut Self) -> usize;
}

fn bounds(range: &impl RangeBounds<usize>, len: usize) -> (usize, usize) {
    let end = match range.end_bound() {
        Bound::Included(&n) => n.saturating_add(1),
        Bound::Excluded(&n) => n,
        Bound::Unbounded => len,
    }
    .min(len);
    let start = match range.start_bound() {
        Bound::Included(&n) => n,
        Bound::Excluded(&n) => n.saturating_add(1),
        Bound::Unbounded => 0,
    }
    .min(end);
    (start, end)
}

impl<T, R: RangeBounds<usize>> VecRemain<R> for Vec<T> {
    fn remain(&mut self, range: R) -> usize {
        let (start, end) = bounds(&range, self.len());
        self.truncate(end);
        if start == 0 {
            return end;
        }
        // The guard owns the retained suffix while slice drop owns the prefix.
        // Slice drop finishes the prefix even if one destructor unwinds.
        struct Compact<'a, T> {
            vec: &'a mut Vec<T>,
            start: usize,
            count: usize,
        }
        impl<T> Drop for Compact<'_, T> {
            fn drop(&mut self) {
                unsafe {
                    let p = self.vec.as_mut_ptr();
                    ptr::copy(p.add(self.start), p, self.count);
                    self.vec.set_len(self.count);
                }
            }
        }
        let count = end - start;
        let p = self.as_mut_ptr();
        // No element remains owned by Vec until Compact restores the suffix.
        unsafe {
            self.set_len(0);
        }
        let _guard = Compact {
            vec: self,
            start,
            count,
        };
        unsafe {
            ptr::drop_in_place(ptr::slice_from_raw_parts_mut(p, start));
        }
        count
    }

    fn remain_to(&mut self, range: R, other: &mut Self) -> usize {
        let (start, end) = bounds(&range, self.len());
        let count = end - start;
        // Reserve before any destructor or ownership transfer can run.
        other.reserve(count);
        self.truncate(end);
        unsafe {
            let p = self.as_mut_ptr();
            // Distinct mutable Vec borrows own disjoint allocations (also valid for ZST).
            ptr::copy_nonoverlapping(p.add(start), other.as_mut_ptr().add(other.len()), count);
            other.set_len(other.len() + count);
            self.set_len(0);
            // The destination owns the suffix; slice drop owns all remaining values.
            ptr::drop_in_place(ptr::slice_from_raw_parts_mut(p, start));
        }
        count
    }
}
