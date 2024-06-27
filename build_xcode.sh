cargo build --release
mv target/release/micromusic xcode/bin/
dylibbundler -od -b -x xcode/bin/micromusic -d xcode/libs