use er::*;
use std::{
    cell::{Cell, RefCell},
    error::Error,
    io,
};

#[derive(Er)]
pub struct Batch;

#[derive(Er)]
#[er(wrap(output = report))]
pub struct Item(pub u8);

#[derive(Er)]
#[er(wrap(output = top, std_error))]
pub struct Standard;

fn items<T: Error + 'static>(tree: &ErTree<T>) -> Vec<u8> {
    tree.er_find_all::<Item>().map(|item| item.0).collect()
}

#[test]
pub fn append() {
    let _: ErTree<Batch> = er_new!();
    let sub = ErTree::new(Item(1), [Item(2)]);
    let tree: ErTree<Batch> = er_new!((), [sub]);
    let tree = tree.er_add([io::Error::other("rollback failed")]);
    let tree = tree.er_add([Err(Item(3)), Ok(()), Err(Item(4))]);

    assert_eq!(items(&tree), [1, 2, 3, 4]);
    assert!(tree.er_contains::<io::Error>());
}

#[test]
pub fn direct_tree() {
    let receipt = ErTree::new(Item(1), [Item(2)]);
    let tree = ErTree::from(Batch).er_add(receipt);

    assert_eq!(items(&tree), [1, 2]);
}

#[test]
pub fn direct_result() {
    fn backup(succeeded: bool) -> ErResult<String, Item> {
        if succeeded {
            Ok("archive.sqlite".to_owned())
        } else {
            Err(Item(7).into())
        }
    }

    let success: ErResult<(), Batch> = er_all!(Batch, backup(true));
    assert!(success.is_ok());
    let failure: ErResult<(), Batch> = er_all!(Batch, backup(false));
    assert_eq!(items(&failure.unwrap_err()), [7]);

    assert!(ErTree::from(Batch).er_add(backup(true)).nodes.is_empty());
    assert_eq!(items(&ErTree::from(Batch).er_add(backup(false))), [7]);
}

#[test]
pub fn borrowed_failures() {
    let mut shared = [ErTree::new(Item(1), [Item(2)]).er_share()];
    let mut results = [Err::<(), _>(shared[0].clone())];

    let tree: ErTree<Batch> = er_all!((), &mut results[0]).unwrap_err();
    let tree = tree.er_add(&results[..]);
    let tree = tree.er_add(results.iter_mut());
    let tree = tree.er_add(shared.iter_mut());

    assert_eq!(items(&tree), [1, 2, 1, 2, 1, 2, 1, 2]);
}

#[test]
pub fn append_mixed() {
    #[derive(Er)]
    struct Named(String);

    #[derive(Er)]
    struct WithSource(#[er(source)] io::Error);

    #[derive(Er)]
    struct NamedItem(String, u8);

    let _: ErTree<Named> = er_new!("import");
    let _: ErTree<NamedItem> = er_new!(("import", 85u8).er_save("server", "busy"));
    let _: ErTree<Named> = er_new!(String::from("import"));
    let _: ErTree<WithSource> = er_new!(|_| io::Error::other("source"));
    let tree: ErTree<Item> = er_new!(0u8, [Item(1), Ok::<_, Item>(7)]);
    let sub = ErTree::new(Item(2), [Item(3)]);
    let tree = er_add!(
        tree,
        [
            sub,
            io::Error::other("cleanup failed"),
            Err::<(), _>(Item(4)),
            Ok::<_, Item>(7),
        ]
    );

    let tree = er_add!(tree, []);
    let tree = er_add!(tree, [Ok::<_, Item>(7)]);
    let results = [Ok(7), Err(Item(5)), Err(Item(6))].into_iter();
    let tree = er_add!(tree, results);

    let errors = vec![ErTree::from(Item(7)), ErTree::from(Item(8))];
    let tree = er_add!(ItemWrap::from(tree), errors);
    assert_eq!(items(&tree), [0, 1, 2, 3, 4, 5, 6, 7, 8]);
}

#[test]
pub fn every_result() {
    let mut visited = 0;
    let results = [Ok(7), Err(Item(1)), Ok(8), Err(Item(2))]
        .into_iter()
        .inspect(|_| {
            visited += 1;
        });

    let tree: ErTree<Batch> = er_all!(
        || {
            assert_eq!(visited, 4);
            Batch
        },
        results,
    )
    .unwrap_err();

    assert_eq!(items(&tree), [1, 2]);
}

#[test]
pub fn no_failures() -> ErTest {
    let parents = Cell::new(0);
    let parent = || {
        parents.set(parents.get() + 1);
        Batch
    };

    let successes: Vec<Result<u8, Item>> = vec![Ok(7), Ok(8)];
    er_all!(parent, successes)?;
    er_all!(parent, [])?;
    aggregate::collect::<Batch, _, _, _, _, _>((), [Ok::<_, Item>(7)])?;

    let value = RefCell::new(0);
    er_all!(
        parent,
        [Ok::<_, Item>(value.borrow_mut()), {
            *value.borrow_mut() = 7;
            Ok::<_, Item>(())
        },]
    )?;
    assert_eq!(parents.get(), 0);

    Ok(())
}

#[test]
pub fn batch() {
    let results: Vec<ErResult<(), Item>> = (0..3).map(|index| Err(Item(index).into())).collect();
    let tree: ErTree<Batch> = er_all!((), results).unwrap_err();
    assert_eq!(items(&tree), [0, 1, 2]);

    let errors = vec![ErTree::from(Item(3)), ErTree::from(Item(4))];
    let tree: ErTree<Batch> = er_all!((), errors).unwrap_err();
    assert_eq!(items(&tree), [3, 4]);
}

#[test]
pub fn inferred_batch() {
    fn close() -> ErResult<(), Batch> {
        let mut result = Ok(());

        for index in 0..2 {
            result = er_all!((), [result, Item(index)]);
        }

        result
    }

    assert_eq!(items(&close().unwrap_err()), [0, 1]);
}

#[test]
pub fn mixed() {
    let success: Result<u16, Item> = Ok(10);
    let plain = Item(1);
    let boxed: Result<(), BoxError> = Err(Box::new(io::Error::other("disk gone")));
    let existing = ErTree::new(Item(2), [ErTree::from(Item(3))]);
    let wrapped = ItemWrap::from(ErTree::new(Item(4), [Item(5)]));
    let report = ErTree::new(Item(6), [Item(7)]);
    let std_error = Err::<(), _>(StandardWrap::from(ErTree::new(Standard, [Item(8)])));

    let tree: ErTree<Batch> = er_all!(
        (),
        [success, plain, boxed, existing, wrapped, report, std_error]
    )
    .unwrap_err();
    assert_eq!(items(&tree), [1, 2, 3, 4, 5, 6, 7, 8]);

    fn recover(error: ErTree<Item>, reopen: Result<(), io::Error>) -> ErResult<u8, Batch> {
        er_all!((), [error, reopen])?;
        Ok(85)
    }

    for reopen in [Ok(()), Err(io::Error::other("reopen failed"))] {
        let count = if reopen.is_err() { 2 } else { 1 };
        let tree = recover(ErTree::new(Item(10), [Item(11)]), reopen).unwrap_err();
        assert_eq!(tree.nodes.len(), count);
    }
}

#[test]
pub fn collect() {
    fn decode(input: &[&str]) -> ErResult<Vec<u8>, Batch> {
        input.iter().map(|value| value.parse()).er_collect(())
    }

    assert_eq!(decode(&["7", "8"]).er_report().unwrap(), [7, 8]);
    assert!(decode(&[]).er_report().unwrap().is_empty());

    let grouped: ErResult<(), Item> = aggregate::collect(|_| 9, [Err::<(), _>(Item(1))]);
    assert_eq!(items(&grouped.unwrap_err()), [9, 1]);

    let parents = Cell::new(0);
    let context = |_| {
        parents.set(parents.get() + 1);
        9
    };
    let values: ErResult<Vec<_>, Item> = [Ok::<_, Item>(7)].into_iter().er_collect(context);
    values.er_report().unwrap();
    assert_eq!(parents.get(), 0);

    let error = ErTree::new(Item(1), [Item(2)]);
    let mut input = [Ok(7), Err(error), Ok(8)].into_iter();
    let result: ErResult<Vec<_>, Item> = input.by_ref().er_collect(context);
    assert_eq!(input.next().unwrap().er_report().unwrap(), 8);
    assert_eq!(items(&result.unwrap_err()), [9, 1, 2]);
}

#[test]
pub fn short_collection() {
    struct First;
    impl FromIterator<u8> for First {
        fn from_iter<T: IntoIterator<Item = u8>>(values: T) -> Self {
            values.into_iter().next();
            Self
        }
    }

    let result: ErResult<First, Batch> = [Ok(7), Err(Item(1))].into_iter().er_collect(());
    assert!(result.is_err());
}

#[test]
pub fn collection_stops_on_error() {
    struct TryAgain;
    impl FromIterator<u8> for TryAgain {
        fn from_iter<T: IntoIterator<Item = u8>>(values: T) -> Self {
            let mut values = values.into_iter();
            values.next();
            values.next();

            Self
        }
    }

    let mut pulled = 0;
    let source = [Err(Item(1)), Ok(7)].into_iter().inspect(|_| pulled += 1);
    let _: ErResult<TryAgain, Batch> = source.er_collect(());

    assert_eq!(pulled, 1);
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
    values.er_report().unwrap();
    assert_eq!(parents.get(), 0);

    let state = RefCell::new(7);
    let results = [Ok(state.borrow()), Err(Item(1)), Ok(state.borrow())];
    let visited = Cell::new(0);
    let results = results
        .into_iter()
        .inspect(|_| visited.set(visited.get() + 1));
    let result: ErResult<Vec<_>, Batch> = results.er_collect_all(|| {
        assert_eq!(visited.get(), 3);
        *state.borrow_mut() = 85;
        Batch
    });
    let _ = result.unwrap_err();
}

#[test]
pub fn try_values() -> ErTest {
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

    assert_eq!(
        (owned.as_str(), borrowed.as_str(), number),
        ("owned", "borrowed", 85)
    );
    assert_eq!(parents.get(), 0);

    let (_,): (u8,) = er_try!(Batch, [Ok::<_, Item>(7)])?;
    er_try!(|| Batch, [])?;
    Ok(())
}

pub mod constants {
    use super::Batch;
    use er::*;

    #[expect(non_upper_case_globals, reason = "macro binding collisions")]
    pub mod names {
        pub const __er_value_0: u8 = 85;
        pub const __er_value_1: u8 = 86;
        pub const __er_errors: u8 = 87;
        pub const __er_results: u8 = 88;
        pub const __er_tree: u8 = 89;
        pub const __er_item: u8 = 90;
        pub const __er_make: u8 = 91;
    }
    use names::*;

    #[test]
    pub fn bindings() -> ErTest {
        macro_rules! hidden {
            () => {
                __er_results + __er_make
            };
        }

        let values: ErResult<_, Batch> = er_try!(
            (),
            [Ok::<_, Batch>(__er_value_0), Ok::<_, Batch>(__er_value_1)]
        );
        assert_eq!(values.er_unwrap_report(), (85, 86));

        let mut number = 7;
        let _: ErResult<_, Batch> = er_try!(
            || {
                assert_eq!(number, 7);
                Batch
            },
            [Ok::<_, Batch>(&mut number), Err::<(), _>(Batch)]
        );
        let _: ErResult<(), Batch> = er_all!(
            || {
                assert_eq!(number, 7);
                Batch
            },
            [Ok::<_, Batch>(&mut number), Err::<&mut u8, _>(Batch)]
        );

        let result: ErResult<(), Batch> = er_all!((), [Ok::<_, Batch>(__er_results)]);
        result?;
        let failed: ErResult<(), super::Item> = er_all!(|_| hidden!(), [Err::<(), _>(Batch)]);
        assert_eq!(failed.unwrap_err().top.0, 179);
        let result: ErResult<(), Batch> = er_all!((), [Ok::<_, Batch>(__er_item)].into_iter());
        result?;

        let tree = er_add!(er_new!(Batch), [Ok::<_, Batch>(__er_tree)]);
        let _ = er_add!(tree, [Ok::<_, Batch>(__er_item)].into_iter());

        Ok(())
    }
}

#[test]
pub fn try_tree() {
    let result: ErResult<((), String, (), bool), Item> = er_try!(
        |_| 9u8,
        [
            ErTree::new(Item(1), [Item(2)]),
            Ok::<_, Item>(String::from("discarded")),
            io::Error::other("disk"),
            Err::<bool, _>(Item(3)),
        ],
    );
    assert_eq!(items(&result.unwrap_err()), [9, 1, 2, 3]);

    let state = RefCell::new(7);
    let result: ErResult<_, Batch> = er_try!(
        || {
            *state.borrow_mut() = 85;
            Batch
        },
        [Ok::<_, Item>(state.borrow()), Err::<(), _>(Item(1)), {
            assert!(state.try_borrow_mut().is_err());
            Ok::<_, Item>(state.borrow())
        }]
    );
    let _ = result.unwrap_err();
}
