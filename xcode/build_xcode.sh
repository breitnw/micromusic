cargo build --release
mv target/release/micromusic micromusic-xcode/bin/
dylibbundler -od -b -x micromusic-xcode/bin/micromusic -d micromusic-xcode/libs