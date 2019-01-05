extern crate cc;

fn main() {
    cc::Build::new()
        .flag("-std=c11")
        .file("atomic_helper.c")
        .compile("atomic_helper");
}
