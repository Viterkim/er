use er::*;
use std::{cell::Cell, marker::PhantomData};

#[derive(Er)]
#[er(wrap)]
pub struct Parent;
#[test]
pub fn success_drop() {
    pub struct Guard<'a>(pub &'a Cell<usize>);
    impl Drop for Guard<'_> {
        fn drop(&mut self) {
            self.0.set(self.0.get() + 1);
        }
    }

    let drops = Cell::new(0);
    let success: Result<Guard<'_>, Parent> = Ok(Guard(&drops));
    let failure: Result<Guard<'_>, Parent> = Err(Parent);
    let next = || {
        assert_eq!(drops.get(), 1);
        failure
    };
    let result: Er<(), Parent> = er_all!(
        || {
            assert_eq!(drops.get(), 1);
            Parent
        },
        [success, next()]
    );

    assert_eq!(drops.get(), 1);
    assert_eq!(result.unwrap_err().nodes.len(), 1);
}

#[derive(Er)]
pub struct Collision;
impl Collision {
    pub fn into_er_part(self) {
        panic!("caller method selected")
    }
}

#[test]
pub fn method_collision() {
    let result: Result<(), Collision> = Err(Collision);
    let tree: ErTree<Parent> = er_all!(|| Parent, [result]).unwrap_err();

    assert!(tree.er_contains::<Collision>());

    struct __ErAllIntoErPart;
    impl __ErAllIntoErPart {
        fn result() -> Result<(), Collision> {
            Err(Collision)
        }
    }
    macro_rules! make_result {
        () => {
            __ErAllIntoErPart::result()
        };
    }

    let tree: ErTree<Parent> = er_all!(|| Parent, [make_result!()]).unwrap_err();
    assert!(tree.er_contains::<Collision>());
}

pub trait Relates<T> {}
pub use Relates as __ErNodeRoot;
pub struct Token;

#[derive(Er)]
#[er(wrap(output = report))]
pub struct Bound<T: __ErNodeRoot<Self>> {
    #[er(skip)]
    pub marker: PhantomData<T>,
}
impl Relates<Bound<Token>> for Token {}

#[derive(Er)]
#[er(wrap(output = top))]
pub struct WhereBound<T>
where
    T: __ErNodeRoot<Self>,
{
    #[er(censor)]
    pub marker: PhantomData<T>,
}
impl Relates<WhereBound<Token>> for Token {}

#[test]
pub fn self_bounds() {
    let wrapped = Bound::<Token> {
        marker: PhantomData,
    }
    .er_wrap();

    assert_eq!(wrapped.tree.top.to_string(), "Bound");
    assert!(wrapped.to_string().starts_with("Bound"));

    let wrapped = WhereBoundWrap::from(WhereBound::<Token> {
        marker: PhantomData,
    });

    assert!(wrapped.to_string().contains("*CENSORED*"));
}
