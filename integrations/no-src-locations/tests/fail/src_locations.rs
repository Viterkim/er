pub fn main() {
    let tree = no_src_locations::diagnostic();
    tree.src_location;
    tree.er_report().tree.src_location;
    tree.nodes[0].src_location;
    tree.er_entries().next().unwrap().src_location;
    tree.er_snapshot().entries[0].src_location;
}
