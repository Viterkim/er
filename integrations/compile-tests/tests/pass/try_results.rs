extern crate er as renamed;

#[derive(renamed::Er)]
#[er(crate = renamed)]
pub struct ConfigErr {
    pub input: String,
}

pub fn read(input: &str) -> renamed::ErResult<(u16, bool), ConfigErr> {
    renamed::er_try!(|_| input, [input.parse::<u16>(), "true".parse::<bool>()])
}

pub fn main() {
    use renamed::ErPresentationExt;

    assert_eq!(read("85").unwrap_report(), (85, true));

    let __er_value_0 = "bad";
    let __er_errors = "also bad";
    let error: renamed::ErResult<_, ConfigErr> = renamed::er_try!(
        |_| __er_value_0,
        [__er_value_0.parse::<u16>(), __er_errors.parse::<bool>()],
    );
    let error = error.unwrap_err();
    assert_eq!(error.top.input, "bad");
    assert_eq!(error.nodes.len(), 2);
}
