//! 保留vec中指定范围的数据，其余部分释放

// #![feature(test)]
// extern crate test;

use std::{
    ops::{Bound, RangeBounds},
    ptr,
};

pub trait VecRemain<R: RangeBounds<usize>> {
    fn remain(&mut self, range: R) -> usize;
    fn remain_to(&mut self, range: R, other: &mut Self) -> usize;
}

impl<T, R: RangeBounds<usize>> VecRemain<R> for Vec<T> {
    fn remain(&mut self, range: R) -> usize {
        let end = end(self, range.end_bound());
        let start = match range.start_bound() {
            Bound::Included(&start) => {
                if start == 0 {
                    return end;
                }
                if start >= end {
                    self.clear();
                    return 0;
                }
                start
            }
            Bound::Excluded(&start) => {
                if start.saturating_add(1) >= end {
                    self.clear();
                    return 0;
                }
                start.saturating_add(1)
            }
            Bound::Unbounded => {
                return end;
            }
        };
        let ptr = self.as_mut_ptr();
        unsafe {
            // 释放start之前的内存
            let s = ptr::slice_from_raw_parts_mut(ptr, start);
            ptr::drop_in_place(s);
            self.set_len(self.len() - start);
            ptr.add(start).copy_to(ptr, self.len());
        }
        end - start
    }
    fn remain_to(&mut self, range: R, other: &mut Self) -> usize {
        let end = end(self, range.end_bound());
        let start = match range.start_bound() {
            Bound::Included(&start) => {
                if start >= end {
                    self.clear();
                    return 0;
                }
                start
            }
            Bound::Excluded(&start) => {
                if start.saturating_add(1) >= end {
                    self.clear();
                    return 0;
                }
                start.saturating_add(1)
            }
            Bound::Unbounded => 0,
        };
        let count = end - start;
        let ptr = self.as_mut_ptr();
        unsafe {
            // 释放start之前的内存
            let s = ptr::slice_from_raw_parts_mut(ptr, start);
            ptr::drop_in_place(s);

            other.reserve(count);
            ptr.add(start)
                .copy_to_nonoverlapping(other.as_mut_ptr().add(other.len()), count);
            other.set_len(other.len() + count);
            self.set_len(0);
        }
        count
    }
}
fn end<T>(vec: &mut Vec<T>, bound: Bound<&usize>) -> usize {
    match bound {
        Bound::Included(end) => {
            let end = end.saturating_add(1);
            if end < vec.len() {
                vec.truncate(end);
                end
            } else {
                vec.len()
            }
        }
        Bound::Excluded(end) => {
            if *end < vec.len() {
                vec.truncate(*end);
                *end
            } else {
                vec.len()
            }
        }
        Bound::Unbounded => vec.len(),
    }
}
#[test]
fn test_vec_remain() {
    let mut vec = vec![1, 2, 3, 4, 5];
    vec.remain(1..);
    assert_eq!(vec, &[2, 3, 4, 5]);
    let mut vec = vec![1, 2, 3, 4, 5];
    vec.remain(1..3);
    assert_eq!(vec, &[2, 3]);
    let mut vec = vec![1, 2, 3, 4, 5];
    vec.remain(1..=3);
    assert_eq!(vec, &[2, 3, 4]);
    let mut vec = vec![1, 2, 3, 4, 5];
    vec.remain(..=3);
    assert_eq!(vec, &[1, 2, 3, 4]);
    let mut vec = vec![1, 2, 3, 4, 5];
    vec.remain_to(..=3, &mut vec![6, 7, 8, 9]);
    assert_eq!(vec, &[]);
    let mut vec = vec![1, 2, 3, 4, 5];
    let mut other = vec![6, 7, 8, 9];
    vec.remain_to(1..=3, &mut other);
    assert_eq!(other, &[6, 7, 8, 9, 2, 3, 4]);
    assert_eq!(vec, &[]);
}
