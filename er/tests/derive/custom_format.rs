use er::*;
use std::{fmt, marker::PhantomData, path::PathBuf};

#[derive(ErFormat)]
pub struct Connection {
    pub host: String,
    #[er(censor)]
    pub password: String,
}

#[derive(Er)]
pub struct ConnectErr {
    pub connection: Connection,
}

#[derive(Er)]
pub struct LoginErr;
#[test]
pub fn nested_data() {
    let connection = Connection::new("ComputerKatten", "secret");
    let tree = ConnectErr::new(connection).er();

    let expected =
        "ConnectErr { connection: Connection { host: \"ComputerKatten\", password: *CENSORED* } }";

    assert_eq!(tree.er_top().to_string(), expected);

    let tree = tree.er::<LoginErr>(LoginErr::new);
    for layout in [Layout::Multiline, Layout::SingleLine] {
        let report = tree.er_report().layout(layout).to_string();
        assert!(report.contains(expected), "{report}");
    }

    let connection = &tree.er_find::<ConnectErr>().unwrap().connection;
    assert_eq!(connection.password, "secret");
}

#[derive(Er)]
#[er(format = "couldn't read {path:?}: {attempts} attempts")]
pub struct ReadErr {
    pub path: PathBuf,
    pub attempts: u8,
}
#[test]
pub fn named_fields() {
    let error = ReadErr::new("/tmp/config", 3);
    let expected = "couldn't read \"/tmp/config\": 3 attempts";

    assert_eq!(error.to_string(), expected);
    assert_eq!(format!("{error:?}"), expected);
}

#[derive(Er)]
pub enum AccountErr<T> {
    #[er(format = "account {account} wasn't found")]
    Missing {
        account: u64,
        metadata: T,
    },
    #[er(format = "account service unavailable")]
    Unavailable,
    #[er(format = "{1:?} / {0}")]
    Status(u8, String),
    #[er(format = "{2}: {0}")]
    Later(u8, #[er(skip)] T, String),
    InvalidName {
        name: String,
    },
}
pub struct NotPrintable;
#[test]
pub fn variants() {
    let error = AccountErr::missing(7, NotPrintable);
    assert_eq!(error.to_string(), "account 7 wasn't found");
    assert_eq!(format!("{error:?}"), error.to_string());
    assert_eq!(
        AccountErr::later(85, NotPrintable, "port").to_string(),
        "port: 85"
    );
    assert_eq!(
        AccountErr::<NotPrintable>::unavailable().to_string(),
        "account service unavailable"
    );
    assert_eq!(
        AccountErr::<NotPrintable>::status(7, "retry").to_string(),
        "\"retry\" / 7"
    );
    assert_eq!(
        AccountErr::<NotPrintable>::invalid_name("").to_string(),
        "AccountErr::InvalidName { name: \"\" }"
    );
}

pub struct OnlyDisplay(pub u8);
impl fmt::Display for OnlyDisplay {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Er)]
#[er(format = "{value} / {marker:?}")]
pub struct GenericErr<T, U> {
    pub value: T,
    pub marker: PhantomData<U>,
    pub unused: U,
}
#[test]
pub fn field_bounds() {
    let error = GenericErr::new(OnlyDisplay(7), PhantomData, NotPrintable);
    assert!(error.to_string().starts_with("7 / PhantomData<"));

    #[derive(Er)]
    #[er(format = "{value} / {value:?} / {value:x} / {value:b}")]
    pub struct NumberErr<T> {
        pub value: T,
    }

    assert_eq!(NumberErr::new(10u8).to_string(), "10 / 10 / a / 1010");

    #[derive(Er)]
    #[er(format = "{value} [{children:?}]")]
    pub struct RecursiveErr<T> {
        pub value: T,
        pub children: Vec<Self>,
    }

    let error = RecursiveErr::new(
        OnlyDisplay(1),
        vec![RecursiveErr::new(OnlyDisplay(2), vec![])],
    );

    assert_eq!(error.to_string(), "1 [[2 [[]]]]");
}

pub struct Secret;
impl fmt::Debug for Secret {
    fn fmt(&self, _: &mut fmt::Formatter<'_>) -> fmt::Result {
        panic!("secret must not be formatted")
    }
}

#[derive(Er)]
pub enum Choice<T> {
    #[er(format = "{1}: {0:?}")]
    Tuple(#[er(censor)] T, u8, #[er(skip)] T),
    #[er(format = "hidden")]
    Omitted {
        #[er(censor)]
        secret: T,
    },
}

#[derive(Er)]
#[er(format = "{label}: {secret} / {secret:?} / {secret:04x} / {secret:.2}")]
pub struct SecretErr<T> {
    pub label: String,
    #[er(censor)]
    pub secret: T,
    #[er(skip)]
    pub hidden: T,
}
#[test]
pub fn censor_and_skip() {
    let tree = SecretErr::new("login", Secret, Secret).er();
    assert_eq!(
        tree.er_top().to_string(),
        "login: *CENSORED* / *CENSORED* / *CENSORED* / *CENSORED*"
    );

    let error = SecretErr::new("login", NotPrintable, NotPrintable);
    assert_eq!(format!("{error:?}"), error.to_string());

    assert_eq!(
        Choice::tuple(Secret, 7, Secret).to_string(),
        "7: *CENSORED*"
    );

    let error = Choice::omitted(Secret);
    assert_eq!(error.to_string(), "hidden");
}

#[derive(Er)]
#[er(format = "{{{value:🦀>width$.precision$}}} {value:+08.2e} {width:#x?}")]
pub struct PrecisionErr {
    pub value: f64,
    pub width: usize,
    pub precision: usize,
}
#[test]
pub fn rust_format_syntax() {
    let error = PrecisionErr::new(1.25, 7, 1);
    let value = error.value;
    let width = error.width;
    let precision = error.precision;

    assert_eq!(
        error.to_string(),
        format!("{{{value:🦀>width$.precision$}}} {value:+08.2e} {width:#x?}")
    );

    #[derive(Er)]
    #[er(format = "{02} {:.*} / {1:.00$} / {1:>2$} / {{done}}")]
    pub struct TupleErr(pub usize, pub f64, pub usize);

    assert_eq!(
        TupleErr::new(2, 1.25, 8).to_string(),
        format!("{2} {:.*} / {1:.0$} / {1:>2$} / {{done}}", 2, 1.25, 8)
    );

    #[derive(Er)]
    #[er(format = "{:}>4}|{0:{<4}")]
    pub struct BraceFill(pub u8);

    assert_eq!(BraceFill::new(7).to_string(), format!("{:}>4}|{0:{<4}", 7));

    #[derive(Er)]
    #[er(format = "a {{literal}}\nnext line")]
    pub struct UnitErr;

    assert_eq!(UnitErr.to_string(), "a {literal}\nnext line");
}

#[test]
#[allow(non_upper_case_globals)]
pub fn names_and_wrap() {
    #[derive(Er)]
    #[er(format = "{type} {__er_f} {__er_d:?} {café}", wrap)]
    pub struct NamesErr<const __er_f: usize, const __er_d: usize, r#T> {
        pub r#type: r#T,
        pub __er_f: String,
        pub __er_d: u8,
        pub café: u8,
    }

    let tree = NamesErr::<1, 2, _>::new(OnlyDisplay(7), "ok", 3, 4)
        .er_wrap()
        .tree;

    assert_eq!(tree.er_top().to_string(), "7 ok 3 4");
}

#[derive(Er)]
#[er(format = "{0:p}")]
pub struct PointerErr<T>(pub *const T);
#[test]
pub fn pointer_value() {
    let value = NotPrintable;
    let pointer = &value as *const _;

    assert_eq!(PointerErr::new(pointer).to_string(), format!("{pointer:p}"));
}

#[test]
pub fn projected_and_borrowed_fields() {
    pub trait HasValue {
        type Value;
    }

    #[derive(Er)]
    #[er(format = "{value}")]
    pub struct Projected<T> {
        pub marker: PhantomData<T>,
        pub value: <Self as HasValue>::Value,
    }
    impl<T> HasValue for Projected<T> {
        type Value = T;
    }

    assert_eq!(
        Projected::<OnlyDisplay>::new(PhantomData, OnlyDisplay(7)).to_string(),
        "7"
    );

    pub trait Callback<T, Owner>: fmt::Debug {}
    pub struct CallbackValue;
    impl fmt::Debug for CallbackValue {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("callback")
        }
    }
    impl<T, Owner> Callback<T, Owner> for CallbackValue {}
    #[derive(Er)]
    #[er(format = "{callback:?}")]
    pub struct CallbackErr<T: 'static> {
        pub callback: Box<dyn for<'a> Callback<&'a T, Self>>,
    }

    let error = CallbackErr::<NotPrintable> {
        callback: Box::new(CallbackValue),
    };

    assert_eq!(error.to_string(), "callback");

    #[derive(Er)]
    #[er(format = "{text}", wrap(output = top))]
    pub struct BorrowedErr<'a> {
        pub text: &'a str,
    }

    let text = String::from("borrowed");
    let tree = ErTree {
        top: BorrowedErr::new(&text),
        nodes: Vec::new(),
        #[cfg(feature = "src_locations")]
        src_location: std::panic::Location::caller(),
    };
    let wrapped: BorrowedErrWrap<'_> = tree.into();

    assert_eq!(wrapped.to_string(), text);
}

#[test]
pub fn numeric_traits() {
    pub struct OnlyHex(pub u8);
    impl fmt::LowerHex for OnlyHex {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            fmt::LowerHex::fmt(&self.0, f)
        }
    }

    #[derive(Er)]
    #[er(format = "{0:#04x}")]
    pub struct HexErr<T>(pub T);

    assert_eq!(HexErr::new(OnlyHex(7)).to_string(), "0x07");

    #[derive(Er)]
    #[er(format = "{0:b}/{0:o}/{0:x}/{0:X}/{0:?}/{0:x?}/{0:X?}/{0:e}/{0:E}")]
    pub struct NumberErr<T>(pub T);

    assert_eq!(
        NumberErr::new(42u16).to_string(),
        format!(
            "{0:b}/{0:o}/{0:x}/{0:X}/{0:?}/{0:x?}/{0:X?}/{0:e}/{0:E}",
            42u16
        )
    );
}
