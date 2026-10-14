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

#[test]
pub fn nested_data() {
    let connection = Connection::new("ComputerKatten", "secret");
    let tree = ErTree::from(ConnectErr::new(connection));

    let expected =
        "ConnectErr { connection: Connection { host: \"ComputerKatten\", password: *CENSORED* } }";

    assert_eq!(tree.er_top().to_string(), expected);
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
    assert_eq!(
        error.to_string(),
        "couldn't read \"/tmp/config\": 3 attempts"
    );
}

#[test]
pub fn field_access() {
    struct Revision(u64);

    #[derive(Er)]
    #[er(format = "history from {oldest.0} to {latest.0:04x}")]
    struct History {
        oldest: Revision,
        latest: Revision,
    }

    let err = History::new(Revision(7), Revision(85));
    assert_eq!(err.to_string(), "history from 7 to 0055");

    struct Details {
        text: String,
    }

    #[derive(Er)]
    enum Nested<T> {
        #[er(format = "{details.text}: {values.0.0}")]
        Named {
            details: Details,
            values: ((T,), NotPrintable),
        },
        #[er(format = "{0.text}: {1.0}")]
        Tuple(Details, (T, NotPrintable)),
    }

    let err = Nested::named(
        Details {
            text: "oldest".into(),
        },
        ((OnlyDisplay(7),), NotPrintable),
    );
    assert_eq!(err.to_string(), "oldest: 7");
    let _: &dyn std::error::Error = &err;

    let err = Nested::tuple(
        Details {
            text: "latest".into(),
        },
        (OnlyDisplay(85), NotPrintable),
    );
    assert_eq!(format!("{err:?}"), "latest: 85");

    #[derive(ErFormat)]
    #[er(format = "hidden", display_alt = "{secret.0.text}")]
    struct Secret {
        #[er(censor)]
        secret: (Details,),
    }

    let secret = Secret::new((Details {
        text: "sentinel".into(),
    },));
    assert_eq!(format!("{secret:#}"), "*CENSORED*");
    let _ = secret.secret;
}

#[test]
pub fn overrides() {
    #[derive(Er)]
    #[er(display = "failed {label}")]
    struct DisplayErr<T> {
        label: String,
        #[er(skip)]
        data: T,
    }

    let err = DisplayErr::new("read", NotPrintable);
    assert_eq!(err.to_string(), "failed read");
    let NotPrintable = err.data;

    #[derive(ErFormat)]
    #[er(debug = "debug {label}")]
    struct DebugData<T> {
        label: String,
        #[er(skip)]
        data: T,
    }

    let err = DebugData::new("read", NotPrintable);
    assert_eq!(format!("{err:?}"), "debug read");
    let NotPrintable = err.data;

    #[derive(Er)]
    #[er(
        display = "display {label}",
        format = "shared {data:?}",
        debug = "debug {label}"
    )]
    pub struct OverrideErr<T> {
        pub label: String,
        pub data: T,
    }

    let err = OverrideErr::new("read", NotPrintable);
    assert_eq!(err.to_string(), "display read");
    assert_eq!(format!("{err:?}"), "debug read");
    let _: &dyn std::error::Error = &err;
    let NotPrintable = err.data;

    #[derive(Er)]
    #[er(display = "status failed")]
    enum StatusErr {
        #[er(display = "missing", debug = "debug missing")]
        Missing,
        #[er(format = "shared {0}", debug = "debug {0}")]
        Failed(String),
        Other(u8),
    }

    assert_eq!(StatusErr::missing().to_string(), "missing");
    assert_eq!(StatusErr::failed("read").to_string(), "shared read");
    assert_eq!(format!("{:?}", StatusErr::failed("read")), "debug read");
    assert_eq!(StatusErr::other(85).to_string(), "status failed");
    assert_eq!(
        format!("{:?}", StatusErr::other(85)),
        "StatusErr::Other(85)"
    );
}

#[test]
pub fn alternate_output() {
    #[derive(Er)]
    #[er(format = "lifecycle failed", display_alt = "{diagnostics}")]
    struct Saved<T> {
        diagnostics: String,
        data: T,
    }

    let err = Saved::new("first failure\nsecond failure", NotPrintable);
    assert_eq!(format!("{err:#}"), err.diagnostics);
    assert_eq!(format!("{err:#?}"), "lifecycle failed");

    let tree = ErTree::from(err);
    assert!(!tree.er_report_string().contains("first failure"));
    let NotPrintable = tree.top.data;

    struct DisplayOnly;
    impl fmt::Display for DisplayOnly {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("details")
        }
    }

    #[derive(Er)]
    #[er(format = "failed", display_alt = "{data}")]
    struct Generic<T> {
        data: T,
    }

    assert_eq!(format!("{:#}", Generic::new(DisplayOnly)), "details");
    assert_eq!(format!("{:?}", Generic::new(DisplayOnly)), "failed");
    let err = Generic::new(DisplayOnly);
    let _: &dyn std::error::Error = &err;

    #[derive(ErFormat)]
    #[er(display_alt = Self::details)]
    enum Phase {
        #[er(format = "start")]
        Start,
        #[er(display_alt = "stop details")]
        Stop,
    }
    impl Phase {
        fn details(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            assert!(f.alternate());
            f.write_str("phase details")
        }
    }

    assert_eq!(format!("{:#}", Phase::start()), "phase details");
    assert_eq!(format!("{:#}", Phase::stop()), "stop details");

    #[derive(Er)]
    enum Mixed {
        #[er(display = "failed", display_alt = "details {0}", debug = "hidden")]
        Failed(String),
        Ordinary(u8),
    }

    assert_eq!(
        format!("{:#}", Mixed::failed("sentinel")),
        "details sentinel"
    );
    assert_eq!(
        format!("{:#}", Mixed::ordinary(85)),
        "Mixed::Ordinary(\n    85,\n)"
    );
}

#[test]
pub fn formatter_functions() {
    #[derive(Er)]
    #[er(display = Self::display, debug = Self::debug)]
    pub struct LifecycleErr<T> {
        pub diagnostics: String,
        pub data: T,
    }
    impl<T> LifecycleErr<T> {
        fn display(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("lifecycle failed")?;

            if f.alternate() && !self.diagnostics.is_empty() {
                write!(f, ": {}", self.diagnostics)?;
            }

            Ok(())
        }

        fn debug(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "{self:#}")
        }
    }

    let err = LifecycleErr::new("sentinel", NotPrintable);
    assert_eq!(format!("{err:#}"), "lifecycle failed: sentinel");
    assert_eq!(format!("{err:?}"), "lifecycle failed: sentinel");
    let NotPrintable = err.data;

    fn shared(err: &NumberErr, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "number {}", err.0)
    }

    #[derive(ErFormat)]
    #[er(format = shared)]
    struct NumberErr(u8);

    assert_eq!(NumberErr::new(85).to_string(), "number 85");

    #[derive(Er)]
    #[er(display = Self::display)]
    enum PhaseErr {
        Start,
        Stop,
    }
    impl PhaseErr {
        fn display(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            let phase = match self {
                Self::Start => "start",
                Self::Stop => "stop",
            };
            write!(f, "couldn't {phase}")
        }
    }

    let _ = PhaseErr::start();
    assert_eq!(PhaseErr::stop().to_string(), "couldn't stop");

    #[derive(ErFormat)]
    #[er(display = <Self as fmt::Debug>::fmt)]
    struct Qualified(u8);

    assert_eq!(Qualified::new(85).to_string(), "Qualified(85)");
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
#[er(format = "{label}: {secret} / {secret:?} / {secret:04x} / {secret:.2} / {secret:🦀>14}")]
pub struct SecretErr<T> {
    pub label: String,
    #[er(censor)]
    pub secret: T,
    #[er(skip)]
    pub hidden: T,
}
#[test]
pub fn censor_and_skip() {
    let tree = ErTree::from(SecretErr::new("login", Secret, Secret));
    assert_eq!(
        tree.er_top().to_string(),
        "login: *CENSORED* / *CENSORED* / *CENSORED* / *C / 🦀🦀🦀🦀*CENSORED*"
    );

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
        #[cfg(feature = "stack_traces")]
        stack_traces: Vec::new(),
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
