#![cfg(feature = "stack_traces")]

use er::*;
use std::fmt;

#[test]
pub fn capture() {
    let tree = ErTree::from(fmt::Error);
    let first = line!() + 1;
    let tree = tree.er_trace();
    let second = line!() + 1;
    let tree = Err::<(), _>(tree).er_trace().unwrap_err();
    let traces: Vec<_> = tree
        .stack_traces
        .iter()
        .map(|trace| (trace.error_index, trace.trace_location.line()))
        .collect();
    assert_eq!(
        traces,
        [(ErErrorIndex(0), first), (ErErrorIndex(0), second)]
    );

    let tree = tree.er(|| fmt::Error).er_trace();
    let indices: Vec<_> = tree
        .stack_traces
        .iter()
        .map(|trace| trace.error_index)
        .collect();
    assert_eq!(indices, [ErErrorIndex(1), ErErrorIndex(1), ErErrorIndex(0)]);

    let report = Box::new(ErTree::from(fmt::Error).into_er_report());
    let line = line!() + 1;
    let report = Err::<(), _>(report).er_trace().unwrap_err();
    assert_eq!(report.tree.stack_traces[0].trace_location.line(), line);
}

#[cfg(feature = "macros")]
pub mod composed {
    use er::*;
    use std::{error::Error, fmt, path::PathBuf};

    #[derive(Er)]
    #[er(wrap(output = report, std_error))]
    pub struct ReadErr {
        pub path: PathBuf,
    }

    #[derive(Er)]
    pub enum JobErr {
        Failed { input: String },
    }

    #[derive(Er)]
    #[er(wrap)]
    pub struct Layer;

    #[derive(Er)]
    pub struct Batch;

    pub fn read(input: &str) -> ErResult<u16, ReadErr> {
        input.parse().er(|_| input).er_trace()
    }

    pub fn job(input: &str) -> ErResult<u16, JobErr> {
        read(input).er(|| JobErr::failed(input)).er_trace()
    }

    fn traced<E: Error + 'static>(tree: &ErTree<E>) -> Vec<String> {
        tree.stack_traces
            .iter()
            .map(|trace| {
                let error = tree.er_at_index(trace.error_index).unwrap();
                match error.downcast_ref::<ReadErr>() {
                    Some(read) => read.path.display().to_string(),
                    None if error.is::<JobErr>() => "JobErr".into(),
                    None if error.is::<Batch>() => "Batch".into(),
                    None => "unexpected error".into(),
                }
            })
            .collect()
    }

    #[test]
    pub fn composition() {
        let left = job("left").unwrap_err();
        let right = ErTree::from(ReadErr::new("right")).er_trace();
        let right: Result<(), ReadErrWrap> = Err(right.into());
        let batch: ErTree<Batch> = er_all!(
            (),
            [
                left.into_er_report(),
                right.er_wrap(Layer::new),
                "bad".parse::<bool>(),
            ]
        )
        .unwrap_err();
        let wrapped: LayerWrap = batch.er(()).into();
        let tree = Err::<(), _>(wrapped).er_tree().er(Batch::new).unwrap_err();
        assert_eq!(traced(&tree), ["left", "JobErr", "right"]);

        let tree = ErTree::new(Batch, [read("first").unwrap_err()]).er_trace();
        let tree = er_add!(tree, [job("second"), read("third")]);
        assert_eq!(
            traced(&tree),
            ["first", "Batch", "second", "JobErr", "third"]
        );

        let children = (0..12).map(|index| {
            let tree = ErTree::new(
                ReadErr::new(index.to_string()),
                (0..index % 4).map(|_| fmt::Error),
            );

            if index % 3 == 2 {
                tree.er_trace()
            } else {
                tree
            }
        });
        let tree = aggregate::collect(|| Batch, children.map(Err::<(), _>)).unwrap_err();
        assert_eq!(traced(&tree), ["2", "5", "8", "11"]);
    }
}
