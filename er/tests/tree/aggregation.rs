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
    let tree = er_all!(
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

    for results in [vec![], successes] {
        er_all!(parent, results).er_report()?;
    }

    er_all!(parent, []).er_report()?;

    let success: Result<(), Item> = Ok(());
    er_all!(parent, [success]).er_report()?;

    assert_eq!(parents.get(), 0);

    Ok(())
}

#[test]
pub fn batch() {
    let mut results: Vec<Er<(), Item>> = Vec::new();

    for index in 0..200 {
        results.push(Err(Item(index).er()));
    }
    #[cfg(feature = "src_locations")]
    let src = results[0].as_ref().unwrap_err().src_location;

    let mut calls = 0;
    let tree = er_all!(Batch::new, {
        calls += 1;
        results
    })
    .unwrap_err();

    assert_eq!(calls, 1);

    let items: Vec<_> = tree.er_find_all::<Item>().map(|e| e.0).collect();
    assert_eq!(items, (0..200).collect::<Vec<_>>());
    #[cfg(feature = "src_locations")]
    assert!(tree.nodes.iter().all(|node| node.src_location == src));
}

#[test]
pub fn empty_group() {
    let errors: Vec<ErTree<Item>> = Vec::new();
    let tree = ErTree::new(Batch::new(), errors);

    assert!(tree.nodes.is_empty());
    assert!(tree.er_contains::<Batch>());
}

#[test]
pub fn mixed() -> core::fmt::Result {
    let success: Result<u16, Item> = Ok(10);
    let plain: Result<bool, Item> = Err(Item(1));
    let boxed: Result<(), BoxError> = Err(Box::new(io::Error::other("disk gone")));
    let subtree = ErTree::new(Item(2), [Item(3).er()]);
    #[cfg(feature = "src_locations")]
    let src = (subtree.src_location, subtree.nodes[0].src_location);
    let existing: Er<u8, Item> = Err(subtree);
    let wrapped = Err::<(), _>(ItemWrap::from(ErTree::new(Item(4), [Item(5)])));
    let report = Err::<(), _>(ErTree::new(Item(6), [Item(7)]).into_er_report());
    let std_error = Err::<(), _>(StandardWrap::from(ErTree::new(Standard, [Item(8)])));
    let recovered = Err::<(), _>(StandardWrap::from(ErTree::new(Standard, [Item(9)]))).er_tree();

    let tree = er_all!(
        Batch::new,
        [
            success, plain, boxed, existing, wrapped, report, std_error, recovered
        ]
    )
    .err()
    .ok_or(core::fmt::Error)?;

    // The raw std_error hides Item(8). Extracting the other std_error's tree keeps Item(9).
    let items: Vec<_> = tree.er_find_all::<Item>().map(|item| item.0).collect();
    assert_eq!(items, [1, 2, 3, 4, 5, 6, 7, 9]);
    assert!(tree.er_contains::<StandardWrap>());
    assert!(tree.er_contains::<Standard>());

    assert!(tree.er_contains::<io::Error>());
    let children: Vec<_> = tree.nodes.iter().map(|node| node.nodes.len()).collect();
    assert_eq!(children, [0, 0, 1, 1, 1, 0, 1]);
    #[cfg(feature = "src_locations")]
    assert_eq!(
        (
            tree.nodes[2].src_location,
            tree.nodes[2].nodes[0].src_location
        ),
        src
    );
    Ok(())
}
