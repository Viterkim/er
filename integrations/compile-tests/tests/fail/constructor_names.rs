use er::*;

#[derive(Er)]
pub enum RequestErr {
    HttpError,
    HTTPError,
}

#[derive(Er)]
pub enum OtherErr {
    File,
    #[allow(non_camel_case_types)]
    file,
}

#[derive(Er)]
pub enum RawErr {
    Type,
    #[allow(non_camel_case_types)]
    r#type,
}

#[derive(Er)]
#[er(wrap)]
pub enum WrapErr {
    ErWrap,
}

#[derive(Er)]
#[er(wrap(name = r#SameErr))]
pub struct SameErr;

#[derive(Er)]
#[er(wrap(name = r#Boundary))]
pub struct JobErr<Boundary> {
    pub details: Boundary,
}

#[derive(Er)]
#[er(wrap)]
pub struct TaskErr<TaskErrWrap> {
    pub details: TaskErrWrap,
}

pub fn main() {}
