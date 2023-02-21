use std::path::Path;

fn main() {
    let include_path = Path::new("/opt/homebrew/include");
    cc::Build::new()
        .file("src/hit_test.c")
        .include(include_path)
        .compile("hit_test");
}
