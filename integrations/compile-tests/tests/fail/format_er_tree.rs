use er::*;

#[derive(Er)]
pub struct AppErr;

pub fn main() {
    let error: ErTree<AppErr> = ErTree::from(AppErr);

    println!("{error}");
    println!("{error:?}");
}
