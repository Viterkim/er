use er::*;

#[derive(Er)]
pub struct AppErr;

#[derive(Er)]
pub struct DbErr;

pub fn find_from_report<'a>(report: ErReportRef<'a, AppErr>) -> Option<&'a DbErr> {
    report.er_find::<DbErr>()
}

pub fn find_from_top<'a>(top: ErTopRef<'a, AppErr>) -> Option<&'a DbErr> {
    top.er_find::<DbErr>()
}

pub fn main() {
    let tree = ErTree::new(AppErr, [DbErr.er()]);

    let error = &tree.er_report().tree.top;
    let found = tree.er_report().er_find::<DbErr>();
    let direct = tree.er_report().tree.nodes.as_slice();
    let nodes = tree.er_report().er_descendants();
    let sources = tree.er_report().er_sources();

    let _ = (error, found, direct, nodes, sources);
    let _ = find_from_report(tree.er_report());
    let _ = find_from_top(tree.er_top());
}
