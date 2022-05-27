fn main() {
    cc::Build::new()
        .file("src/hit_test.c")
        // .define("FOO", Some("bar"))
        .compile("hit_test");
}