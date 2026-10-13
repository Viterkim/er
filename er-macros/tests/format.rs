use er_macros::{Er, ErFormat};
use std::{fmt, marker::PhantomData};

pub struct NotPrintable;

#[derive(ErFormat)]
#[er(no_constructors)]
pub struct Payload<T: ?Sized> {
    pub value: T,
}
#[test]
pub fn unsized_field() {
    let payload = Payload { value: [85, 8] };
    let payload: &Payload<[u8]> = &payload;

    assert_eq!(format!("{payload}"), "Payload { value: [85, 8] }");
}

#[derive(ErFormat)]
pub struct Data<T> {
    pub name: String,
    #[er(censor)]
    pub secret: T,
    #[er(skip)]
    pub handle: T,
    pub marker: PhantomData<T>,
}
#[test]
pub fn fields() {
    let data = Data::new("ComputerKatten", NotPrintable, NotPrintable, PhantomData);
    let text = data.to_string();

    assert!(
        text.starts_with(
            "Data { name: \"ComputerKatten\", secret: *CENSORED*, marker: PhantomData<"
        )
    );
    assert!(!text.contains("handle:"));
    assert_eq!(format!("{data:?}"), text);
    assert_eq!(format!("{data:#?}"), format!("{data:#}"));
}

pub struct OnlyDisplay(pub u8);
impl fmt::Display for OnlyDisplay {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

#[derive(ErFormat)]
pub enum Choice<'a, T, U> {
    Empty,
    Named {
        text: &'a str,
    },
    Tuple(u8, #[er(skip)] U, #[er(censor)] U),
    #[er(format = "value {value}; secret {secret:x}")]
    Custom {
        value: T,
        #[er(censor)]
        secret: U,
    },
}
#[test]
pub fn variants() {
    let text = String::from("hello");
    let named = Choice::named(&text);
    let custom = Choice::custom(OnlyDisplay(9), NotPrintable);

    for (value, expected) in [
        (Choice::empty(), "Choice::Empty"),
        (named, "Choice::Named { text: \"hello\" }"),
        (
            Choice::tuple(7, NotPrintable, NotPrintable),
            "Choice::Tuple(7, *CENSORED*)",
        ),
        (custom, "value 9; secret *CENSORED*"),
    ] {
        assert_eq!(value.to_string(), expected);
    }
}

#[derive(ErFormat)]
pub struct Unit;

#[derive(ErFormat)]
#[er(format = "{0:?}")]
pub struct Tuple(pub String);
#[test]
pub fn unit_and_tuple() {
    assert_eq!(Unit::new().to_string(), "Unit");
    assert_eq!(Tuple::new("hi").to_string(), "\"hi\"");
}

#[derive(Er)]
#[er(display = "failed {name}", display_alt = Self::details, debug = Self::details)]
pub struct Failure<T> {
    pub name: String,
    pub data: T,
}
impl<T> Failure<T> {
    fn details(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "diagnostics for {}", self.name)
    }
}
#[test]
pub fn independent_output() {
    let failure = Failure::new("read", NotPrintable);
    assert_eq!(failure.to_string(), "failed read");
    assert_eq!(format!("{failure:#}"), "diagnostics for read");
    assert_eq!(format!("{failure:?}"), "diagnostics for read");
    let _: &dyn std::error::Error = &failure;
}

#[test]
#[deny(clippy::too_many_arguments)]
pub fn wide_constructors() {
    #[derive(Er)]
    struct Fields {
        a: u8,
        b: u8,
        c: u8,
        d: u8,
        e: u8,
        f: u8,
        g: u8,
        h: u8,
    }

    #[derive(ErFormat)]
    enum Data {
        Fields(u8, u8, u8, u8, u8, u8, u8, u8),
    }

    let _ = Fields::new(1, 2, 3, 4, 5, 6, 7, 8);
    let _ = Data::fields(1, 2, 3, 4, 5, 6, 7, 8);
}
