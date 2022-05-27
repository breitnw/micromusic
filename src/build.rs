fn main() {
    cc::Build::new()
        .file("src/hittest.c")
        // .define("FOO", Some("bar"))
        .compile("hittest");
}