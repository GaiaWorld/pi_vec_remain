//! 保留vec中指定范围的数据，其余部分释放

#![feature(test)]
extern crate test;

use std::{
    ops::{Bound, RangeBounds},
    ptr,
};

pub trait VecRemain<R: RangeBounds<usize>> {
    fn remain(&mut self, range: R);
    fn remain_to(&mut self, range: R, other: &mut Self);
}

impl<T, R: RangeBounds<usize>> VecRemain<R> for Vec<T> {
    fn remain(&mut self, range: R) {
        match range.end_bound() {
            Bound::Included(&end) => {
                if end + 1 < self.len() {
                    self.truncate(end + 1);
                }
            }
            Bound::Excluded(&end) => {
                if end < self.len() {
                    self.truncate(end);
                }
            }
            Bound::Unbounded => {}
        }
        let start = match range.start_bound() {
            Bound::Included(&start) => {
                if start == 0 {
                    return;
                }
                if start >= self.len() {
                    self.clear();
                    return;
                }
                start
            }
            Bound::Excluded(&start) => {
                if start >= self.len() + 1 {
                    self.clear();
                    return;
                }
                start - 1
            }
            Bound::Unbounded => {
                return;
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
    }
    fn remain_to(&mut self, range: R, other: &mut Self) {
        let end = match range.end_bound() {
            Bound::Included(&end) => {
                let end = end + 1;
                if end < self.len() {
                    self.truncate(end);
                    end
                } else {
                    self.len()
                }
            }
            Bound::Excluded(&end) => {
                if end < self.len() {
                    self.truncate(end);
                    end
                } else {
                    self.len()
                }
            }
            Bound::Unbounded => self.len(),
        };
        let start = match range.start_bound() {
            Bound::Included(&start) => {
                if start >= end {
                    self.clear();
                    return;
                }
                start
            }
            Bound::Excluded(&start) => {
                if start >= end + 1 {
                    self.clear();
                    return;
                }
                start - 1
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
