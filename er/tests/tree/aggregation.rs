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

    assert_eq!(parents.get(), 0);

    Ok(())
}

#[test]
pub fn batch() {
    let mut results: Vec<ErResult<(), Item>> = Vec::new();

    for index in 0..3 {
        results.push(Err(Item(index).er()));
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
    let plain: Result<bool, Item> = Err(Item(1));
    let boxed: Result<(), BoxError> = Err(Box::new(io::Error::other("disk gone")));
    let subtree = ErTree::new(Item(2), [Item(3).er()]);
    let existing: ErResult<u8, Item> = Err(subtree);
    let wrapped = Err::<(), _>(ItemWrap::from(ErTree::new(Item(4), [Item(5)])));
    let report = Err::<(), _>(ErTree::new(Item(6), [Item(7)]).into_er_report());
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
}
