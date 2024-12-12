//! 保留vec中指定范围的数据，其余部分释放

#![feature(test)]
extern crate test;

use std::{
    ops::{Bound, RangeBounds},
    ptr,
};

pub trait VecRemain<R: RangeBounds<usize>> {
    fn remain(&mut self, range: R);
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
}
