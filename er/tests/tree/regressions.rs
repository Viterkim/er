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
    let named: Result<_, Parent> = Ok(Guard(&drops));
    let temporary: Result<_, Parent> = Ok(Guard(&drops));
    let result = er_all!(|| Parent, [named, temporary]);

    assert_eq!(drops.get(), 2);
    assert!(result.is_ok());

    let named: Result<_, Parent> = Ok(Guard(&drops));
    let result = er_all!(
        || {
            assert_eq!(drops.get(), 3);
            Parent
        },
        [named, {
            assert_eq!(drops.get(), 3);

            let failure: Result<(), Parent> = Err(Parent);
            failure
        }]
    );

    assert_eq!(result.unwrap_err().nodes.len(), 1);
}

#[derive(Er)]
pub struct Collision;
impl Collision {
    pub fn into_er_node(self) {
        panic!("caller method selected")
    }
}

#[test]
pub fn method_collision() {
    let result: Result<(), Collision> = Err(Collision);
    let tree = er_all!(|| Parent, [result]).unwrap_err();

    assert!(tree.er_contains::<Collision>());
}

pub trait Relates<T> {}
pub struct Token;

#[derive(Er)]
#[er(wrap(output = report))]
pub struct Bound<T: Relates<Self>> {
    #[er(skip)]
    pub marker: PhantomData<T>,
}
impl Relates<Bound<Token>> for Token {}

#[derive(Er)]
#[er(wrap(output = top))]
pub struct WhereBound<T>
where
    T: Relates<Self>,
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
