extern crate cc;

fn main() {
    cc::Build::new()
        .file("atomic_helper.c")
        .compile("atomic_helper");
}
