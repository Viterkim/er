use er::*;
use std::cell::Cell;
use std::io;

#[derive(Er)]
pub struct Batch;

#[derive(Er)]
#[er(wrap(output = report))]
pub struct Item(pub u8);

#[derive(Er)]
#[er(wrap(output = top, std_error))]
pub struct Standard;

#[test]
pub fn append() {
    let child = ErTree::new(Item(1), [Item(2)]);
    #[cfg(feature = "src_locations")]
    let child_location = child.src_location;
    let tree = ErTree::from(Batch).er_add([child]);
    let _error_line = line!() + 1;
    let tree = tree.er_add([io::Error::other("rollback failed")]);
    let _items_line = line!() + 1;
    let tree = tree.er_add([Item(3), Item(4)]);

    assert_eq!(tree.nodes.len(), 4);
    assert_eq!(
        tree.er_find_all::<Item>()
            .map(|item| item.0)
            .collect::<Vec<_>>(),
        [1, 2, 3, 4]
    );
    assert_eq!(
        tree.er_find::<io::Error>().unwrap().to_string(),
        "rollback failed"
    );
    #[cfg(feature = "src_locations")]
    {
        assert_eq!(tree.nodes[0].src_location, child_location);
        assert_eq!(tree.nodes[1].src_location.file(), file!());
        assert_eq!(tree.nodes[1].src_location.line(), _error_line);
        assert_eq!(tree.nodes[2].src_location.file(), file!());
        assert_eq!(tree.nodes[2].src_location.line(), _items_line);
        assert_eq!(tree.nodes[3].src_location.line(), _items_line);
    }
}

#[test]
pub fn append_mixed() {
    let tree = ErTree::new(Item(0), [Item(1)]);
    #[cfg(feature = "src_locations")]
    let location = tree.src_location;
    let child = ErTree::new(Item(2), [Item(3)]);
    let visited = Cell::new(0);

    let _error_line = line!() + 5;
    let tree = er_add!(
        tree,
        [
            child,
            io::Error::other("cleanup failed"),
            Err::<(), _>(Item(4)),
            {
                visited.set(visited.get() + 1);
                Ok::<_, Item>(7)
            },
        ]
    );

    assert_eq!(visited.get(), 1);
    assert_eq!(tree.top.0, 0);
    assert_eq!(tree.nodes.len(), 4);
    assert_eq!(
        tree.er_find_all::<Item>()
            .map(|item| item.0)
            .collect::<Vec<_>>(),
        [0, 1, 2, 3, 4]
    );
    assert_eq!(
        tree.er_find::<io::Error>().unwrap().to_string(),
        "cleanup failed"
    );
    #[cfg(feature = "src_locations")]
    {
        assert_eq!(tree.src_location, location);
        assert_eq!(tree.nodes[2].src_location.line(), _error_line);
    }

    let tree = er_add!(tree, []);
    let tree = er_add!(tree, [Ok::<_, Item>(7)]);
    assert_eq!(tree.nodes.len(), 4);
    let results = [Ok(7), Err(Item(5)), Err(Item(6))].into_iter();
    let tree = er_add!(tree, results);
    assert_eq!(tree.nodes.len(), 6);
    assert_eq!(tree.top.0, 0);
}

#[test]
pub fn every_result() {
    let parents = Cell::new(0);
    let visited = Cell::new(0);
    let results = [Ok(7), Err(Item(1)), Ok(8), Err(Item(2))]
        .into_iter()
        .inspect(|_| {
            visited.set(visited.get() + 1);
        });

    let expected = line!() + 1;
    let tree: ErTree<Batch> = er_all!(
        || {
            parents.set(parents.get() + 1);
            Batch
        },
        results,
    )
    .unwrap_err();

    assert_eq!(parents.get(), 1);
    assert_eq!(visited.get(), 4);

    let items: Vec<_> = tree
        .nodes
        .iter()
        .map(|n| n.er_find::<Item>().unwrap().0)
        .collect();

    assert_eq!(items, [1, 2]);

    #[cfg(feature = "src_locations")]
    {
        assert_eq!(tree.src_location.line(), expected);
        assert!(tree.nodes.iter().all(|n| n.src_location.line() == expected));
    }

    let _ = expected;
}

#[test]
pub fn no_failures() -> Result<(), ErReport<Batch>> {
    let parents = Cell::new(0);
    let parent = || {
        parents.set(parents.get() + 1);
        Batch
    };

    let successes: Vec<Result<u8, Item>> = vec![Ok(7), Ok(8)];
    er_all!(parent, successes).er_report()?;
    er_all!(parent, []).er_report()?;

    let value = std::cell::RefCell::new(0);
    er_all!(
        parent,
        [Ok::<_, Item>(value.borrow_mut()), {
            *value.borrow_mut() = 7;
            Ok::<_, Item>(())
        },]
    )
    .er_report()?;
    assert_eq!(*value.borrow(), 7);
    assert_eq!(parents.get(), 0);

    Ok(())
}

#[test]
pub fn batch() {
    let mut results: Vec<ErResult<(), Item>> = Vec::new();

    for index in 0..3 {
        results.push(Err(ErTree::from(Item(index))));
    }
    #[cfg(feature = "src_locations")]
    let src = results[0].as_ref().unwrap_err().src_location;

    let mut calls = 0;
    let tree: ErTree<Batch> = er_all!((), {
        calls += 1;
        results
    })
    .unwrap_err();

    assert_eq!(calls, 1);

    let items: Vec<_> = tree.er_find_all::<Item>().map(|e| e.0).collect();
    assert_eq!(items, [0, 1, 2]);
    #[cfg(feature = "src_locations")]
    assert!(tree.nodes.iter().all(|node| node.src_location == src));
}

#[test]
pub fn mixed() {
    let success: Result<u16, Item> = Ok(10);
    let plain = Item(1);
    let boxed: Result<(), BoxError> = Err(Box::new(io::Error::other("disk gone")));
    let subtree = ErTree::new(Item(2), [ErTree::from(Item(3))]);
    let existing = subtree;
    let wrapped = ItemWrap::from(ErTree::new(Item(4), [Item(5)]));
    let report = ErTree::new(Item(6), [Item(7)]).into_er_report();
    let std_error = Err::<(), _>(StandardWrap::from(ErTree::new(Standard, [Item(8)])));
    let recovered = Err::<(), _>(StandardWrap::from(ErTree::new(Standard, [Item(9)]))).er_tree();

    let tree: ErTree<Batch> = er_all!(
        (),
        [
            success, plain, boxed, existing, wrapped, report, std_error, recovered
        ]
    )
    .unwrap_err();

    let items: Vec<_> = tree.er_find_all::<Item>().map(|item| item.0).collect();
    assert_eq!(items, [1, 2, 3, 4, 5, 6, 7, 9]);
    assert!(tree.er_contains::<StandardWrap>());
    assert!(tree.er_contains::<Standard>());
    assert!(tree.er_contains::<io::Error>());

    fn recover(error: ErTree<Item>, reopen: Result<(), io::Error>) -> ErResult<u8, Batch> {
        er_all!((), [error, reopen])?;
        Ok(85)
    }

    for reopen in [Ok(()), Err(io::Error::other("reopen failed"))] {
        let count = if reopen.is_err() { 2 } else { 1 };
        let error = ErTree::new(Item(10), [Item(11)]);
        let tree = recover(error, reopen).unwrap_err();
        assert_eq!(tree.nodes.len(), count);
        let items: Vec<_> = tree.er_find_all::<Item>().map(|item| item.0).collect();
        assert_eq!(items, [10, 11]);
    }
}

#[test]
pub fn collect() {
    fn decode(input: &[&str]) -> ErResult<Vec<u8>, Batch> {
        input.iter().map(|value| value.parse()).er_collect(())
    }

    assert_eq!(decode(&["7", "8"]).er_report().unwrap(), [7, 8]);
    assert!(decode(&[]).er_report().unwrap().is_empty());
    let failed = decode(&["7", "bad", "8"]).unwrap_err();
    assert!(failed.er_contains::<std::num::ParseIntError>());

    let parents = Cell::new(0);
    let context = |_| {
        parents.set(parents.get() + 1);
        9
    };
    let values: ErResult<Vec<_>, Item> = [Ok::<_, Item>(7)].into_iter().er_collect(context);
    assert_eq!(values.er_report().unwrap(), [7]);
    assert_eq!(parents.get(), 0);

    let error = ErTree::new(Item(1), [Item(2)]);
    #[cfg(feature = "src_locations")]
    let source = error.src_location;
    #[cfg(feature = "stack_traces")]
    let error = error.er_trace();
    let visited = Cell::new(0);
    let mut input = [Ok(7), Err(error), Ok(8)].into_iter().inspect(|_| {
        visited.set(visited.get() + 1);
    });
    let _line = line!() + 1;
    let result: ErResult<Vec<_>, Item> = input.by_ref().er_collect(context);
    let tree = result.unwrap_err();
    assert_eq!(parents.get(), 1);
    assert_eq!(visited.get(), 2);
    assert_eq!(input.next().unwrap().er_report().unwrap(), 8);
    assert_eq!(tree.top.0, 9);
    assert_eq!(
        tree.er_find_all::<Item>()
            .map(|item| item.0)
            .collect::<Vec<_>>(),
        [9, 1, 2]
    );
    #[cfg(feature = "src_locations")]
    {
        assert_eq!(tree.src_location.line(), _line);
        assert_eq!(tree.nodes[0].src_location, source);
    }
    #[cfg(feature = "stack_traces")]
    assert_eq!(tree.stack_traces[0].error_index, ErErrorIndex(1));
}

#[test]
pub fn collect_all() {
    fn decode(input: &[&str]) -> ErResult<Vec<u8>, Batch> {
        input.iter().map(|value| value.parse()).er_collect_all(())
    }

    assert_eq!(decode(&["7", "8"]).er_report().unwrap(), [7, 8]);
    assert!(decode(&[]).er_report().unwrap().is_empty());
    let failed = decode(&["bad", "7", "also bad"]).unwrap_err();
    assert_eq!(failed.nodes.len(), 2);

    let parents = Cell::new(0);
    let context = || {
        parents.set(parents.get() + 1);
        Batch
    };
    let values: ErResult<Vec<_>, Batch> = [Ok::<_, Item>(7)].into_iter().er_collect_all(context);
    assert_eq!(values.er_report().unwrap(), [7]);
    assert_eq!(parents.get(), 0);

    let left = ErTree::new(Item(1), [Item(2)]);
    #[cfg(feature = "src_locations")]
    let source = left.src_location;
    #[cfg(feature = "stack_traces")]
    let left = left.er_trace();
    let right = ErTree::from(Item(3));
    #[cfg(feature = "stack_traces")]
    let right = right.er_trace();

    #[derive(Debug)]
    struct Held<'a>(&'a Cell<usize>);
    impl Drop for Held<'_> {
        fn drop(&mut self) {
            self.0.set(self.0.get() + 1);
        }
    }

    let drops = Cell::new(0);
    let visited = Cell::new(0);
    let input = [
        Ok(Held(&drops)),
        Err(left),
        Ok(Held(&drops)),
        Err(right),
        Ok(Held(&drops)),
    ]
    .into_iter()
    .inspect(|_| {
        assert_eq!(drops.get(), 0);
        visited.set(visited.get() + 1);
    });

    let _line = line!() + 1;
    let result: ErResult<Vec<_>, Batch> = input.er_collect_all(context);
    let tree = result.unwrap_err();

    assert_eq!(visited.get(), 5);
    assert_eq!(drops.get(), 3);
    assert_eq!(parents.get(), 1);
    assert_eq!(
        tree.er_find_all::<Item>()
            .map(|item| item.0)
            .collect::<Vec<_>>(),
        [1, 2, 3]
    );
    #[cfg(feature = "src_locations")]
    {
        assert_eq!(tree.src_location.line(), _line);
        assert_eq!(tree.nodes[0].src_location, source);
    }
    #[cfg(feature = "stack_traces")]
    {
        assert_eq!(tree.stack_traces[0].error_index, ErErrorIndex(1));
        assert_eq!(tree.stack_traces[1].error_index, ErErrorIndex(3));
    }
}

#[test]
pub fn try_values() -> Result<(), ErReport<Batch>> {
    let parents = Cell::new(0);
    let text = String::from("borrowed");
    let owned = String::from("owned");
    let (owned, borrowed, number) = er_try!(
        {
            parents.set(parents.get() + 1);
            || Batch
        },
        [
            Ok::<_, Item>(owned),
            Ok::<_, io::Error>(&text),
            "85".parse::<u16>()
        ],
    )?;

    assert_eq!(owned, "owned");
    assert!(std::ptr::eq(borrowed, &text));
    assert_eq!(number, 85);
    assert_eq!(parents.get(), 0);

    let (number,) = er_try!((), [Ok::<_, Item>(7)])?;
    assert_eq!(number, 7);
    er_try!((), [])?;
    Ok(())
}

#[test]
pub fn try_results() {
    struct Value<'a>(&'a Cell<usize>);
    impl Drop for Value<'_> {
        fn drop(&mut self) {
            self.0.set(self.0.get() + 1);
        }
    }

    for failures in 0u32..4 {
        let drops = Cell::new(0);
        let calls = Cell::new(0);
        let parents = Cell::new(0);

        let result = er_try!(
            || {
                parents.set(parents.get() + 1);
                Batch
            },
            [
                {
                    assert_eq!(calls.replace(1), 0);

                    if failures & 1 == 0 {
                        Ok(Value(&drops))
                    } else {
                        Err(Item(1))
                    }
                },
                {
                    assert_eq!(calls.replace(2), 1);
                    assert_eq!(drops.get(), 0);

                    if failures & 2 == 0 {
                        Ok((Value(&drops), true))
                    } else {
                        Err(io::Error::other("failed"))
                    }
                },
            ],
        );

        assert_eq!(calls.get(), 2);

        match result {
            Ok(values) => {
                assert_eq!(failures, 0);
                assert_eq!(parents.get(), 0);
                assert_eq!(drops.get(), 0);
                drop(values);
            }
            Err(tree) => {
                assert_ne!(failures, 0);
                assert_eq!(parents.get(), 1);
                assert_eq!(tree.nodes.len(), failures.count_ones() as usize);
                assert_eq!(tree.er_contains::<Item>(), failures & 1 != 0);
                assert_eq!(tree.er_contains::<io::Error>(), failures & 2 != 0);
            }
        }

        assert_eq!(drops.get(), 2 - failures.count_ones() as usize);
    }
}

#[test]
pub fn try_tree() {
    let child = ErTree::new(Item(1), [Item(2)]);
    #[cfg(feature = "stack_traces")]
    let child = child.er_trace();
    #[cfg(feature = "src_locations")]
    let location = child.src_location;
    let original = child.nodes[0].error.downcast_ref::<Item>().unwrap() as *const Item;
    let _line = line!() + 1;
    let result: ErResult<((), String, (), bool), Item> = er_try!(
        |_| 9u8,
        [
            child.into_er_report(),
            Ok::<_, Item>(String::from("discarded")),
            io::Error::other("disk"),
            Err::<bool, _>(Item(3)),
        ],
    );
    let tree = result.unwrap_err();
    assert_eq!(tree.top.0, 9);
    assert_eq!(tree.nodes.len(), 3);
    assert!(std::ptr::eq(
        original,
        tree.nodes[0].nodes[0].error.downcast_ref::<Item>().unwrap()
    ));
    #[cfg(feature = "src_locations")]
    {
        assert_eq!(tree.src_location.line(), _line);
        assert_eq!(tree.nodes[0].src_location, location);
        assert_eq!(tree.nodes[1].src_location.line(), _line + 5);
    }
    #[cfg(feature = "stack_traces")]
    {
        assert_eq!(tree.stack_traces.len(), 1);
        assert_eq!(tree.stack_traces[0].error_index, ErErrorIndex(1));
    }
}
