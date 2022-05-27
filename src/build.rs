fn main() {
    cc::Build::new()
        .file("src/hit_test.c")
        .compile("hit_test");
}