use er_macros::{Er, ErFormat};
use std::{cell::Cell, fmt, rc::Rc};

pub struct FormatProbe(pub Rc<Cell<usize>>);
impl fmt::Debug for FormatProbe {
    fn fmt(&self, _formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.set(self.0.get() + 1);
        Ok(())
    }
}
impl fmt::Display for FormatProbe {
    fn fmt(&self, _formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.set(self.0.get() + 1);
        Ok(())
    }
}

#[derive(Er)]
pub struct SecretErr(
    #[er(censor)] pub FormatProbe,
    #[r#er(censor)] pub FormatProbe,
    #[r#er(skip)] pub FormatProbe,
);
#[derive(ErFormat)]
pub struct Secret(
    #[er(censor)] pub FormatProbe,
    #[r#er(censor)] pub FormatProbe,
    #[r#er(skip)] pub FormatProbe,
);
#[test]
pub fn censor() {
    let formats = Rc::new(Cell::new(0));
    let probe = || FormatProbe(Rc::clone(&formats));
    let error = SecretErr(probe(), probe(), probe());
    let value = Secret(probe(), probe(), probe());

    let text = format!("{error} {error:?} {value} {value:?}");
    assert_eq!(text.matches("*CENSORED*").count(), 8);
    assert_eq!(formats.get(), 0, "{text}");
}
