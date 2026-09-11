use er_macros::ErFormat;
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
