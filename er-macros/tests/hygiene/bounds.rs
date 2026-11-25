use super::NotPrintable;
use er_macros::Er;
use std::marker::PhantomData;

#[derive(Er)]
#[expect(clippy::use_self, reason = "the recursion is spelled out on purpose")]
pub struct GenericTreeErr<T> {
    pub value: T,
    pub below: Vec<GenericTreeErr<T>>,
}
#[test]
pub fn recursive() {
    let leaf = GenericTreeErr {
        value: 2u8,
        below: Vec::new(),
    };
    let error = GenericTreeErr {
        value: 1u8,
        below: vec![leaf],
    };

    assert_eq!(
        error.to_string(),
        "GenericTreeErr { value: 1, below: [GenericTreeErr { value: 2, below: [] }] }"
    );

    #[derive(Er)]
    #[er(display = "{below:?}")]
    struct Nested<T> {
        value: T,
        below: Vec<Self>,
    }

    let error = Nested::new(1_u8, vec![Nested::new(2, vec![])]);
    assert_eq!(error.to_string(), "[Nested { value: 2, below: [] }]");
}

#[derive(Er)]
#[expect(clippy::use_self, reason = "the recursion is spelled out on purpose")]
pub struct MixedTreeErr<T> {
    pub below: Vec<(T, MixedTreeErr<T>)>,
}
#[test]
pub fn recursive_payload() {
    let leaf = MixedTreeErr { below: Vec::new() };
    let error = MixedTreeErr::<u8> {
        below: vec![(7, leaf)],
    };

    assert_eq!(
        error.to_string(),
        "MixedTreeErr { below: [(7, MixedTreeErr { below: [] })] }"
    );
}

#[derive(Er)]
pub struct BoundedErr<T: Clone>
where
    T: Send,
{
    pub value: T,
}
#[test]
pub fn where_clause() {
    let error = BoundedErr::new(String::from("v"));

    assert_eq!(error.to_string(), "BoundedErr { value: \"v\" }");
}

#[derive(Er)]
pub enum EmptyEnum {}
#[test]
pub fn empty_enum() {
    pub fn assert_error<T: core::error::Error>() {}

    assert_error::<EmptyEnum>();
}

#[derive(Er)]
pub struct GenericErr<T> {
    pub marker: PhantomData<T>,
}

#[derive(Er)]
pub struct Tagged<T> {
    #[er(skip)]
    pub marker: PhantomData<T>,
    pub below: Vec<Tagged<T>>,
}

#[derive(Er)]
pub struct SelfTagged<T> {
    #[er(censor)]
    pub marker: PhantomData<T>,
    pub below: Vec<Self>,
}

pub trait HasItem {
    type Item;
}
impl HasItem for NotPrintable {
    type Item = u8;
}

#[derive(Er)]
pub struct Associated<T: HasItem> {
    pub below: Vec<(T::Item, Associated<T>)>,
}

#[derive(Er)]
pub struct Qualified<T: HasItem> {
    pub below: Vec<(<T as HasItem>::Item, Self)>,
}
#[test]
pub fn projections() {
    pub fn assert_error<T: core::error::Error>() {}

    assert_error::<GenericErr<NotPrintable>>();
    assert_error::<Tagged<NotPrintable>>();
    assert_error::<SelfTagged<NotPrintable>>();
    assert_error::<Associated<NotPrintable>>();
    assert_error::<Qualified<NotPrintable>>();
}
