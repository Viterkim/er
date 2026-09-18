use super::NotPrintable;
use er_macros::Er;
use std::marker::PhantomData;

#[derive(Er)]
#[expect(clippy::use_self, reason = "the recursion is spelled out on purpose")]
pub struct GenericTreeErr<T> {
    pub value: T,
    pub children: Vec<GenericTreeErr<T>>,
}
#[test]
pub fn recursive() {
    let leaf = GenericTreeErr {
        value: 2u8,
        children: Vec::new(),
    };
    let error = GenericTreeErr {
        value: 1u8,
        children: vec![leaf],
    };

    assert_eq!(
        error.to_string(),
        "GenericTreeErr { value: 1, children: [GenericTreeErr { value: 2, children: [] }] }"
    );
}

#[derive(Er)]
#[expect(clippy::use_self, reason = "the recursion is spelled out on purpose")]
pub struct MixedTreeErr<T> {
    pub children: Vec<(T, MixedTreeErr<T>)>,
}
#[test]
pub fn recursive_payload() {
    let leaf = MixedTreeErr {
        children: Vec::new(),
    };
    let error = MixedTreeErr::<u8> {
        children: vec![(7, leaf)],
    };

    assert_eq!(
        error.to_string(),
        "MixedTreeErr { children: [(7, MixedTreeErr { children: [] })] }"
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
    pub children: Vec<Tagged<T>>,
}

#[derive(Er)]
pub struct SelfTagged<T> {
    #[er(censor)]
    pub marker: PhantomData<T>,
    pub children: Vec<Self>,
}

pub trait HasItem {
    type Item;
}
impl HasItem for NotPrintable {
    type Item = u8;
}

#[derive(Er)]
pub struct Associated<T: HasItem> {
    pub children: Vec<(T::Item, Associated<T>)>,
}

#[derive(Er)]
pub struct Qualified<T: HasItem> {
    pub children: Vec<(<T as HasItem>::Item, Self)>,
}
#[test]
pub fn projections() {
    pub fn assert_error<T: core::error::Error>() {}

    assert_error::<GenericErr<NotPrintable>>();
    assert_error::<Tagged<NotPrintable>>();
    assert_error::<SelfTagged<NotPrintable>>();
    assert_error::<Associated<NotPrintable>>();
    assert_error::<Qualified<NotPrintable>>();

    let leaf = Associated {
        children: Vec::new(),
    };
    let error = Associated::<NotPrintable> {
        children: vec![(7, leaf)],
    };
    assert!(error.to_string().contains("7"));
}
