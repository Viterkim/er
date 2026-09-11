use er::*;

#[derive(Er)]
pub enum RequestEr {
    HttpError,
    HTTPError,
}

#[derive(Er)]
pub enum OtherEr {
    File,
    #[allow(non_camel_case_types)]
    file,
}

#[derive(Er)]
pub enum RawEr {
    Type,
    #[allow(non_camel_case_types)]
    r#type,
}

#[derive(Er)]
#[er(wrap)]
pub enum WrapEr {
    ErWrap,
}

#[derive(Er)]
#[er(wrap(name = r#SameEr))]
pub struct SameEr;

#[derive(Er)]
#[er(wrap(name = r#Boundary))]
pub struct JobEr<Boundary> {
    pub details: Boundary,
}

#[derive(Er)]
#[er(wrap)]
pub struct TaskEr<TaskErWrap> {
    pub details: TaskErWrap,
}

pub fn main() {}
