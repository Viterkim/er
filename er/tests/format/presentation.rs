use er::*;
use std::{rc::Rc, sync::Arc};

#[derive(Er)]
pub struct InnerErr;

#[derive(Er)]
pub struct AppErr;
pub fn failing() -> ErResult<(), AppErr> {
    Err(ErTree::new(AppErr, [ErTree::from(InnerErr)]))
}

#[test]
pub fn results() {
    fn check<'a, P: IntoErTreeRef<'a, Error = AppErr>>(error: P, expected: &str) {
        assert_eq!(Err::<(), _>(error).er_report().unwrap_err(), expected);
    }

    let tree = failing().unwrap_err();
    let expected = tree.er_report();

    assert_eq!(failing().er_report().unwrap_err(), expected);

    check(&tree, &expected);
    check(&Arc::new(&tree), &expected);
    check(&Rc::new(&tree), &expected);
}

#[test]
pub fn unwrap_messages() {
    fn panic_message(panic: Box<dyn std::any::Any + Send>) -> String {
        *panic.downcast::<String>().unwrap()
    }

    let expected = failing().unwrap_err().er_report();
    let panic = std::panic::catch_unwind(|| failing().er_expect_report("app failure")).unwrap_err();
    let message = panic_message(panic);

    assert!(message.contains("app failure"));
    assert!(message.contains(&expected));
}

#[test]
pub fn lines() {
    let tree = failing().unwrap_err();
    let mut lines = Vec::new();

    tree.for_each_line(|line| lines.push(line.to_owned()))
        .unwrap();
    assert_eq!(lines.join("\n"), tree.er_report());
}
