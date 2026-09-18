#![allow(non_upper_case_globals)]

use er::*;
use std::marker::PhantomData;

pub trait HasValue {
    type Value;
}

#[derive(Er)]
pub struct Projected<T> {
    #[er(skip)]
    pub marker: PhantomData<T>,
    pub value: <Self as HasValue>::Value,
}
impl<T> HasValue for Projected<T> {
    type Value = T;
}

#[derive(Er)]
pub struct ProjectionTree<T> {
    #[er(skip)]
    pub marker: PhantomData<T>,
    pub children: Vec<(Self, <Self as HasValue>::Value)>,
}
impl<T> HasValue for ProjectionTree<T> {
    type Value = u8;
}

#[test]
pub fn self_projection() {
    let error = Projected::<u8> {
        marker: PhantomData,
        value: 7,
    };

    assert_eq!(error.to_string(), "Projected { value: 7 }");

    pub struct NotDebug;
    let leaf = ProjectionTree::<NotDebug> {
        marker: PhantomData,
        children: Vec::new(),
    };
    let tree = ProjectionTree {
        marker: PhantomData,
        children: vec![(leaf, 7)],
    };

    assert_eq!(
        tree.to_string(),
        "ProjectionTree { children: [(ProjectionTree { children: [] }, 7)] }"
    );
}

impl<'a> HasValue for &'a str {
    type Value = &'a str;
}

#[derive(Er)]
#[er(wrap(output = top))]
pub struct BorrowedProjection<'a>
where
    &'a str: HasValue,
{
    pub value: <&'a str as HasValue>::Value,
}
#[test]
pub fn lifetime_projection() {
    let text = String::from("borrowed");
    let error = BorrowedProjection::new(text.as_str());

    assert_eq!(
        error.to_string(),
        "BorrowedProjection { value: \"borrowed\" }"
    );

    let tree = ErTree {
        top: error,
        nodes: Vec::new(),
        #[cfg(feature = "src_locations")]
        src_location: std::panic::Location::caller(),
    };
    let wrapped = BorrowedProjectionWrap::from(tree);

    assert_eq!(wrapped.to_string(), wrapped.er_top().to_string());
    assert_eq!(wrapped.tree.top.value, text);
}

pub mod other {
    use std::{fmt, marker::PhantomData};
    #[derive(Debug)]
    pub struct Failure<T>(pub T);

    pub struct Tag<T>(pub PhantomData<T>);
    impl<T> fmt::Debug for Tag<T> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("tag")
        }
    }
}

#[derive(Er)]
pub struct Tag<T> {
    pub tag: other::Tag<T>,
}

#[derive(Er)]
pub struct Failure<T> {
    pub details: other::Failure<T>,
}

#[derive(Er)]
pub struct Recursive<T> {
    pub value: T,
    pub nodes: Vec<self::Recursive<T>>,
}
#[test]
pub fn qualified_types() {
    pub struct NotDebug;
    let tag = Tag::new(other::Tag::<NotDebug>(PhantomData));

    assert_eq!(tag.to_string(), "Tag { tag: tag }");
    assert!(Failure::new(other::Failure(7)).to_string().contains('7'));
    assert!(Recursive::new(7, Vec::new()).to_string().contains('7'));
}

#[derive(Er)]
pub struct Borrowed<'a> {
    pub text: &'a str,
}

#[derive(Er)]
pub struct ValueErr<T> {
    pub value: T,
}

pub type Port = u16;
#[derive(Er)]
pub struct ConnectErr {
    #[er(exact, censor)]
    pub port: Port,
}

#[derive(Er)]
pub struct CallbackErr {
    pub callback: fn(u8) -> u8,
}
#[test]
pub fn inference() {
    let error = ValueErr::new(7u8);
    assert_eq!(error.value.count_ones(), 3);
    assert_eq!(ConnectErr::new(85).port, 85);
    assert_eq!(
        ConnectErr::new(85).to_string(),
        "ConnectErr { port: *CENSORED* }"
    );

    let text = String::from("borrowed");
    assert_eq!(Borrowed::new(&text).text, "borrowed");

    pub fn identity(value: u8) -> u8 {
        value
    }

    assert_eq!((CallbackErr::new(identity).callback)(7), 7);
}

#[derive(Er)]
#[er(wrap(output = report))]
pub struct BytesErr<
    const __er_f: usize,
    const __er_d: usize,
    const tree: usize,
    const error: usize,
    const report: usize,
    const top: usize,
    const wrapped: usize,
> {
    pub bytes: [u8; __er_f],
}
#[test]
pub fn const_names() {
    let wrapped: BytesErrWrap<2, 3, 4, 5, 6, 7, 8> = BytesErr::new([1, 2]).er_wrap();
    let wrapped = BytesErrWrap::from(wrapped.into_er_top());
    let report: ErReport<_> = wrapped.into();
    let wrapped = BytesErrWrap::from(report);

    assert!(wrapped.to_string().contains("[1, 2]"));
}

#[derive(Er)]
pub enum Kind<const __er_0: usize, const __er_d: usize> {
    Named { value: [u8; __er_0] },
    Tuple([u8; __er_d]),
}

#[test]
pub fn enum_bindings() {
    assert_eq!(
        Kind::<2, 1>::named([1, 2]).to_string(),
        "Kind::Named { value: [1, 2] }"
    );
    assert_eq!(Kind::<2, 1>::tuple([3]).to_string(), "Kind::Tuple([3])");
}

pub struct Payload;

#[derive(Er)]
pub struct BorrowingCallbackErr<T> {
    pub callback: for<'a> fn(&'a Self, &'a T),
}
pub fn callback(_: &BorrowingCallbackErr<Payload>, _: &Payload) {}

#[derive(Er)]
pub struct PointerErr<T> {
    pub pointer: *const (Self, T),
}
#[test]
pub fn pointer_fields() {
    let error: BorrowingCallbackErr<Payload> = BorrowingCallbackErr::new(callback);
    assert!(
        error
            .to_string()
            .starts_with("BorrowingCallbackErr { callback:")
    );

    let error = PointerErr::<Payload>::new(std::ptr::null());
    assert!(error.to_string().starts_with("PointerErr { pointer:"));
}

pub trait PointerValue {
    type Value;
}
impl<T> PointerValue for *const T {
    type Value = T;
}
impl<T> PointerValue for fn(T) {
    type Value = T;
}

#[derive(Er)]
pub struct PointerProjection<T> {
    pub value: <*const T as PointerValue>::Value,
    pub callback_value: <fn(T) as PointerValue>::Value,
}
#[test]
pub fn pointer_projections() {
    let error = PointerProjection::<u8>::new(7, 8);
    assert_eq!(
        error.to_string(),
        "PointerProjection { value: 7, callback_value: 8 }"
    );
}

pub trait Callback<Argument, Owner>: std::fmt::Debug {}

#[derive(Debug)]
pub struct Printer;
impl<Argument, Owner> Callback<Argument, Owner> for Printer {}

#[derive(Er)]
pub struct ObjectCallbackErr<T: 'static> {
    pub callback: Box<dyn for<'a> Callback<&'a T, Self>>,
}
#[test]
pub fn trait_object_binder() {
    let error = ObjectCallbackErr::<Payload> {
        callback: Box::new(Printer),
    };

    assert_eq!(error.to_string(), "ObjectCallbackErr { callback: Printer }");
}

#[derive(Er)]
pub struct RawValueErr<T> {
    pub value: r#T,
}

#[derive(Er)]
pub struct PlainValueErr<r#T> {
    pub value: T,
}

#[derive(Er)]
pub struct r#RawTree<T> {
    pub value: T,
    pub children: Vec<self::RawTree<T>>,
}
#[test]
pub fn raw_type_names() {
    let raw = RawValueErr::new(7u8);
    let plain = PlainValueErr::new(8u8);
    let _: u8 = raw.value;
    let _: u8 = plain.value;

    assert_eq!(raw.to_string(), "RawValueErr { value: 7 }");
    assert_eq!(plain.to_string(), "PlainValueErr { value: 8 }");

    let tree = RawTree::new(9u8, Vec::new());
    assert_eq!(tree.to_string(), "RawTree { value: 9, children: [] }");
}

#[derive(Er)]
pub enum StageErr {
    Gen,
}
#[test]
pub fn edition_keyword() {
    assert_eq!(StageErr::r#gen().to_string(), "StageErr::Gen");
}
