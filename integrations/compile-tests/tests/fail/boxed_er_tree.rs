use core::error::Error;
use er::*;

#[derive(Er)]
pub struct AppErr;

pub fn main() {
    let error: ErTree<AppErr> = ErTree::from(AppErr);
    let _boxed: Box<dyn Error> = Box::new(error);
}
