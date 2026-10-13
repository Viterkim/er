use er_macros::Er;

#[derive(Er)]
pub enum CollidingEnum {
    Named {
        formatter: String,
        debug: String,
    },
    #[expect(
        non_snake_case,
        reason = "deliberately collides with generated identifiers"
    )]
    Internals {
        __er_f: String,
        __er_d: String,
        __ErCensored: String,
    },
}
#[test]
pub fn enum_collisions() {
    let named = CollidingEnum::named("a", "b");
    assert_eq!(
        named.to_string(),
        "CollidingEnum::Named { formatter: \"a\", debug: \"b\" }"
    );

    let internals = CollidingEnum::internals("c", "d", "e");
    assert_eq!(
        internals.to_string(),
        "CollidingEnum::Internals { __er_f: \"c\", __er_d: \"d\", __ErCensored: \"e\" }"
    );
}

#[derive(Er)]
pub struct CollidingStruct {
    pub formatter: String,
    pub debug: String,
    pub __er_f: String,
}
#[test]
pub fn struct_collisions() {
    let error = CollidingStruct::new("a", "b", "c");

    assert_eq!(
        error.to_string(),
        "CollidingStruct { formatter: \"a\", debug: \"b\", __er_f: \"c\" }"
    );
}

type __ErInput0 = String;

#[derive(Er)]
pub struct AliasErr {
    pub value: __ErInput0,
}
#[test]
pub fn field_type_name_collision() {
    let _: AliasErr = ("hello",).into();
}

mod bound_name {
    use er_macros::Er;

    pub trait __ErInput0 {}
    impl __ErInput0 for () {}

    #[derive(Er)]
    pub struct BoundErr<T: __ErInput0> {
        pub message: String,
        #[er(skip)]
        pub marker: core::marker::PhantomData<T>,
    }
    #[test]
    pub fn collision() {
        let _ = BoundErr::<()>::new("hello", core::marker::PhantomData);
    }
}

#[derive(Er)]
pub enum RawEnum {
    r#Struct {
        r#type: String,
        r#fn: ::r#core::r#primitive::u8,
    },
    r#Match(String),
}

#[derive(Er)]
pub struct RawStruct {
    pub r#type: String,
    pub r#match: r#std::r#primitive::u8,
}
#[test]
pub fn raw_identifiers() {
    let error = RawEnum::r#struct("x", 3);
    assert_eq!(error.to_string(), "RawEnum::Struct { type: \"x\", fn: 3 }");

    assert_eq!(RawEnum::r#match("y").to_string(), "RawEnum::Match(\"y\")");

    let plain = RawStruct::new("z", 1);
    assert_eq!(plain.to_string(), "RawStruct { type: \"z\", match: 1 }");
}

#[derive(Er)]
#[expect(non_snake_case, reason = "constructor name collisions")]
pub struct ConstNames<const N: usize> {
    pub N: [u8; N],
    pub __er_arg_0: usize,
}
#[test]
pub fn const_generics() {
    let error = ConstNames::<2>::new([1, 2], 3);
    assert_eq!(error.N, [1, 2]);
    assert_eq!(error.__er_arg_0, 3);
}

#[no_implicit_prelude]
pub mod source_paths {
    extern crate core;
    extern crate er_macros;

    pub trait Sized {}

    #[derive(er_macros::Er)]
    #[er(format = "failed")]
    pub struct Failure {
        #[er(source)]
        pub cause: core::fmt::Error,
    }

    #[test]
    fn source() {
        let err = Failure::new(core::fmt::Error);
        core::assert!(
            core::error::Error::source(&err)
                .unwrap()
                .is::<core::fmt::Error>()
        );
    }
}
