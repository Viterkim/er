#![cfg(feature = "stack_traces")]

use er::*;
use std::fmt;

#[test]
pub fn capture() {
    let ok: ErResult<u8, fmt::Error> = Ok(8);
    assert_eq!(ok.er_trace().ok(), Some(8));

    let _source = line!() + 1;
    let tree = ErTree::from(fmt::Error);
    let report = tree.er_report().to_string();
    let first = line!() + 1;
    let tree = tree.er_trace();
    let second = line!() + 1;
    let tree = Err::<(), _>(tree).er_trace().unwrap_err();
    assert_eq!(tree.stack_traces.len(), 2);
    assert_eq!(tree.stack_traces[0].trace_location.line(), first);
    assert_eq!(tree.stack_traces[1].trace_location.line(), second);
    assert!(tree.stack_traces.iter().all(|trace| {
        trace.error_index == ErErrorIndex(0) && trace.trace_location.file() == file!()
    }));
    assert_eq!(tree.er_report().to_string(), report);

    let trace = &tree.stack_traces[0];
    let printed = trace.to_string();
    let (header, stack) = printed.split_once('\n').unwrap();
    assert_eq!(
        header,
        format!("{} @ {}", trace.error_name, trace.trace_location)
    );
    assert_eq!(stack, trace.capture.to_string());
    assert_eq!(format!("{trace}"), printed);
    assert_eq!(format!("{trace:?}"), printed);

    #[cfg(all(target_os = "linux", target_env = "gnu", target_arch = "x86_64"))]
    assert!(
        tree.stack_traces
            .iter()
            .all(|trace| trace.capture.status() == std::backtrace::BacktraceStatus::Captured)
    );
    #[cfg(feature = "src_locations")]
    assert_eq!(tree.src_location.line(), _source);

    let tree = tree.er_with::<fmt::Error>(|_| fmt::Error).er_trace();
    assert_eq!(tree.stack_traces.len(), 3);
    assert_eq!(tree.stack_traces[0].error_index, ErErrorIndex(1));
    assert_eq!(tree.stack_traces[1].error_index, ErErrorIndex(1));
    assert_eq!(tree.stack_traces[2].error_index, ErErrorIndex(0));
    assert!(tree.into_er_node().er_find::<fmt::Error>().is_some());
}

#[cfg(feature = "macros")]
pub mod composed {
    use er::*;
    use std::{backtrace::Backtrace, path::PathBuf};

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

    pub fn trace_section<E>(tree: &ErTree<E>) -> String {
        let mut section = String::new();
        for trace in &tree.stack_traces {
            use std::fmt::Write;
            writeln!(section, "{trace}").unwrap();
        }
        section
    }

    #[test]
    pub fn composition() {
        let left = job("bad left").unwrap_err();
        let left_captures: Vec<_> = left
            .stack_traces
            .iter()
            .map(|t| &*t.capture as *const Backtrace)
            .collect();
        let right = ErTree::from(ReadErr::new("manual")).er_trace();
        let right_capture = &*right.stack_traces[0].capture as *const Backtrace;
        let right: Result<(), ReadErrWrap> = Err(right.into());

        let batch: ErTree<Batch> = er_all!(
            (),
            [
                left.into_er_report(),
                right.er_wrap::<Layer, _>(()),
                "bad".parse::<bool>(),
            ]
        )
        .unwrap_err();
        let records = batch.stack_traces.as_ptr();
        let outer: ErTree<Layer> = batch.er(());
        assert_eq!(outer.stack_traces.as_ptr(), records);
        let wrapped: LayerWrap = outer.into();
        let result: Result<(), LayerWrap> = Err(wrapped);
        let tree = result.er_tree().er_with::<Batch>(|_| Batch).unwrap_err();
        assert_eq!(tree.stack_traces.as_ptr(), records);

        let traces = &tree.stack_traces;
        assert_eq!(traces.len(), 3);
        assert_eq!(traces[0].error_index, ErErrorIndex(4));
        assert_eq!(traces[1].error_index, ErErrorIndex(3));
        assert_eq!(traces[2].error_index, ErErrorIndex(7));
        assert_eq!(&*traces[0].capture as *const Backtrace, left_captures[0]);
        assert_eq!(&*traces[1].capture as *const Backtrace, left_captures[1]);
        assert_eq!(&*traces[2].capture as *const Backtrace, right_capture);
        assert_eq!(tree.er_find_all::<ReadErr>().count(), 2);
        assert_eq!(
            tree.er_at_index(traces[0].error_index)
                .unwrap()
                .downcast_ref::<ReadErr>()
                .unwrap()
                .path,
            PathBuf::from("bad left")
        );
        assert_eq!(
            tree.er_at_index(traces[2].error_index)
                .unwrap()
                .downcast_ref::<ReadErr>()
                .unwrap()
                .path,
            PathBuf::from("manual")
        );
        assert!(tree.er_contains::<JobErr>());
        let printed = trace_section(&tree);
        assert!(printed.contains("ReadErr"));
        assert!(printed.contains("JobErr"));

        let records = tree.stack_traces.as_ptr();
        let tree = ErTree::new(Batch, [tree.into_er_top()]);
        assert_eq!(tree.stack_traces.as_ptr(), records);
        assert_eq!(tree.stack_traces[0].error_index, ErErrorIndex(5));

        let mut tree = ErTree::new(Batch, [read("first").unwrap_err()]).er_trace();
        tree.push_part(job("second").unwrap_err().into_er_part());
        assert_eq!(
            tree.stack_traces
                .iter()
                .map(|trace| trace.error_index)
                .collect::<Vec<_>>(),
            [
                ErErrorIndex(1),
                ErErrorIndex(0),
                ErErrorIndex(4),
                ErErrorIndex(3)
            ]
        );
        for (trace, path) in [(0, "first"), (2, "second")] {
            let error = tree
                .er_at_index(tree.stack_traces[trace].error_index)
                .unwrap();
            assert_eq!(
                error.downcast_ref::<ReadErr>().unwrap().path,
                PathBuf::from(path)
            );
        }
        assert!(
            tree.er_at_index(tree.stack_traces[1].error_index)
                .unwrap()
                .is::<Batch>()
        );
        assert!(
            tree.er_at_index(tree.stack_traces[3].error_index)
                .unwrap()
                .is::<JobErr>()
        );
    }

    #[test]
    pub fn construction() {
        let result: ErResult<(), ReadErr> = Err::<(), _>(std::fmt::Error).er(|_| "fields");
        assert!(result.as_ref().unwrap_err().stack_traces.is_empty());
        assert_eq!(result.er_trace().unwrap_err().stack_traces.len(), 1);

        let tree = read("bad")
            .er_with::<JobErr>(|old| JobErr::failed(old.top.path.to_str().unwrap()))
            .er_trace()
            .unwrap_err();
        assert_eq!(tree.stack_traces.len(), 2);
        assert_eq!(tree.stack_traces[0].error_index, ErErrorIndex(1));
        assert_eq!(tree.stack_traces[1].error_index, ErErrorIndex(0));

        let tree = Err::<(), _>("raw value")
            .er_val(ReadErr::new)
            .er_trace()
            .unwrap_err();
        assert_eq!(tree.stack_traces.len(), 1);
        assert_eq!(tree.top.path, PathBuf::from("raw value"));

        let _source = line!() + 1;
        let batch: ErResult<(), Batch> = er_all!((), [read("batch")]);
        let requested = line!() + 1;
        let batch = batch.er_trace().unwrap_err();
        assert_eq!(batch.stack_traces.len(), 2);
        assert_eq!(batch.stack_traces[0].error_index, ErErrorIndex(1));
        assert_eq!(batch.stack_traces[1].error_index, ErErrorIndex(0));
        assert_eq!(batch.stack_traces[1].trace_location.line(), requested);
        #[cfg(feature = "src_locations")]
        assert_eq!(batch.src_location.line(), _source);

        for collect in [false, true] {
            let children = (0..12).map(|index| {
                let tree = ErTree::new(
                    ReadErr::new(index.to_string()),
                    (0..index % 4).map(|_| std::fmt::Error),
                );
                if index % 3 == 2 {
                    tree.er_trace()
                } else {
                    tree
                }
            });
            let tree = if collect {
                aggregate::collect(|| Batch, children.map(Err::<(), _>)).unwrap_err()
            } else {
                ErTree::new(Batch, children)
            };
            assert_eq!(tree.stack_traces.len(), 4);
            for (trace, index) in tree.stack_traces.iter().zip([2, 5, 8, 11]) {
                let error = tree.er_at_index(trace.error_index).unwrap();
                assert_eq!(
                    error.downcast_ref::<ReadErr>().unwrap().path,
                    PathBuf::from(index.to_string())
                );
            }
        }
    }
}
