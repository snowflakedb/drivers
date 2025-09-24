include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../build_common.rs"));

fn main() {
    emit_loader_rpaths();
}
