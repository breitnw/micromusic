# micromusic
A tiny, lightweight miniplayer for Apple Music written in Rust with SDL2. Micromusic is minimalistic by design, with no text antialiasing or title bar, and just a few buttons. There's one line of text for the title, artist, and album, and it scrolls continuously. You can like/unlike, skip forward, skip backward, play/pause—everything you need, nothing you don't.

![micromusic 0.2.0 in use](https://user-images.githubusercontent.com/29758429/210676224-7c595c26-bee1-4359-b41a-eeaf79173818.gif)

Pressing the button in the top-left will lead you to the brand-new library screen! The first time you're opening this page, it might take a little while to load, since micromusic needs to cache the artwork of all of the albums in your library. To queue albums, simply drag them to the box in the bottom-right and let go—they're added to a temporary playlist in your Apple Music library, so your extended listening sessions can go uninterrupted.

To make sure you're always listening to something fresh, micromusic shuffles all of the albums in your library, displaying nine at a time. Don't like the ones it picks? simply drag albums outside of the app to remove them, or press the "shuffle" button (second from the left) for a fresh set of nine.

## Usage

This application has only been tested with Apple Music on MacOS Monterey, MacOS Ventura and MacOS Sonoma. Since it uses AppleScript to get player data, it will not work with other operating systems or music software. It will likely work on other versions of MacOS, but there aren't any guarantees. 

If you want to compile from source, you'll first need to install the SDL2 headers. One way to do this is with `brew install sdl2`, and then change the variable `include_path` in `build.rs` to the include path used by Homebrew (on M1 devices, this should be `/opt/homebrew/include`). You'll also need to add Homebrew's library installation directory (`/opt/homebrew/lib` on M1 devices) to your library search path, which can generally be done via the `LIBRARY_PATH` environment variable. 

To build the application, you first need to run the `build_xcode.sh` script. This will move the binary into the Xcode folder, bundle its `.dylib` dependencies, and correct the binary to point to these bundled dependencies. From there, you should be able to use Xcode to build the application with all required libraries.
