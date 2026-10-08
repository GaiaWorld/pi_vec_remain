use pi_vec_remain::VecRemain;
use std::cell::Cell;
use std::ops::Bound::{Excluded, Included, Unbounded};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Rc;

#[test]
fn empty_sources_and_full_transfer_preserve_allocations() {
    let mut source = Vec::<String>::with_capacity(8);
    let p = source.as_ptr();
    let mut destination = vec!["existing".to_owned()];
    assert_eq!(source.remain(..), 0);
    assert_eq!(source.remain_to(.., &mut destination), 0);
    assert_eq!(source.as_ptr(), p);
    source.extend(["first".to_owned(), "second".to_owned()]);
    assert_eq!(source.remain_to(.., &mut destination), 2);
    assert_eq!(destination, ["existing", "first", "second"]);
    assert!(source.is_empty());
    assert_eq!(source.as_ptr(), p);
}

#[test]
fn clamped_bounds_and_allocation_reuse() {
    let bounds = [
        Unbounded,
        Included(0),
        Included(2),
        Included(8),
        Included(usize::MAX),
        Excluded(0),
        Excluded(2),
        Excluded(usize::MAX),
    ];
    for start in bounds {
        for end in bounds {
            let left = match start {
                Included(n) => n,
                Excluded(n) => n.saturating_add(1),
                Unbounded => 0,
            };
            let right = match end {
                Included(n) => n.saturating_add(1),
                Excluded(n) => n,
                Unbounded => 5,
            }
            .min(5);
            let expected: Vec<_> = (left.min(right)..right).map(|i| i.to_string()).collect();
            let mut source: Vec<_> = (0..5).map(|i| i.to_string()).collect();
            let p = source.as_ptr();
            let capacity = source.capacity();
            assert_eq!(source.remain((start, end)), expected.len());
            assert_eq!(source, expected);
            assert_eq!(source.as_ptr(), p);
            assert_eq!(source.capacity(), capacity);
            let mut source: Vec<_> = (0..5).map(|i| i.to_string()).collect();
            let p = source.as_ptr();
            let mut destination = Vec::with_capacity(10);
            destination.push("existing".to_owned());
            let q = destination.as_ptr();
            assert_eq!(
                source.remain_to((start, end), &mut destination),
                expected.len()
            );
            assert!(source.is_empty());
            assert_eq!(source.as_ptr(), p);
            assert_eq!(destination.as_ptr(), q);
            assert_eq!(destination[0], "existing");
            assert_eq!(destination[1..], expected);
        }
    }
}

struct Tracked {
    id: usize,
    drops: Rc<Vec<Cell<usize>>>,
    panic: usize,
}
impl Drop for Tracked {
    fn drop(&mut self) {
        let slot = &self.drops[self.id];
        slot.set(slot.get() + 1);
        assert_ne!(self.id, self.panic, "injected destructor panic");
    }
}

#[test]
fn destructor_panics_preserve_unique_ownership() {
    for transfer in [false, true] {
        for panic in [0, 1, 4, 5, usize::MAX] {
            let drops = Rc::new((0..6).map(|_| Cell::new(0)).collect::<Vec<_>>());
            let mut source: Vec<_> = (0..6)
                .map(|id| Tracked {
                    id,
                    drops: drops.clone(),
                    panic,
                })
                .collect();
            let mut destination = Vec::new();
            let outcome = catch_unwind(AssertUnwindSafe(|| {
                if transfer {
                    source.remain_to(2..4, &mut destination);
                } else {
                    source.remain(2..4);
                }
            }));
            assert_eq!(outcome.is_err(), panic != usize::MAX);
            // Tail panic happens before transfer; prefix panic happens after it.
            let expected: Vec<_> = if panic == 4 || panic == 5 {
                (0..4).collect()
            } else if transfer {
                vec![]
            } else {
                vec![2, 3]
            };
            assert_eq!(source.iter().map(|v| v.id).collect::<Vec<_>>(), expected);
            drop(source);
            drop(destination);
            assert!(drops.iter().all(|slot| slot.get() == 1));
        }
    }
}

thread_local! { static ZST_DROPS: Cell<usize> = const { Cell::new(0) }; }
struct Zst;
impl Drop for Zst {
    fn drop(&mut self) {
        ZST_DROPS.set(ZST_DROPS.get() + 1);
    }
}
#[test]
fn zero_sized_values_have_one_owner_per_slot() {
    ZST_DROPS.set(0);
    let mut source: Vec<_> = (0..8).map(|_| Zst).collect();
    assert_eq!(source.remain(2..6), 4);
    let mut destination = vec![Zst];
    assert_eq!(source.remain_to(1..3, &mut destination), 2);
    assert_eq!(destination.len(), 3);
    drop(source);
    drop(destination);
    assert_eq!(ZST_DROPS.get(), 9);
}
