# Log Viewer

Simple log viewer.
Works with log that have json on each line.
Works in both Web (WASM) and on native.

You can use the deployed version without install at <https://c-git.github.io/log-viewer/>

# Description of expected log file format

It is expected that the file will contain multiple json objects separated by new lines.
See [samples](tests/sample_logs/).
A toy example would be:

```
{"v":0,"name":"my_server","msg":"Server address is: 127.0.0.1:8000","level":30,"hostname":"my_computer","pid":42127,"time":"2024-02-10T03:10:25.952130465Z","target":"my_server::startup","line":34,"file":"crates/wic-server/src/startup.rs"}
{"v":0,"name":"my_server","msg":"starting 8 workers","level":30,"hostname":"my_computer","pid":42127,"time":"2024-02-10T03:10:25.952653399Z","target":"actix_server::builder","line":240,"file":"/home/user/.cargo/registry/src/index.crates.io-6f17d22bba15001f/actix-server-2.3.0/src/builder.rs"}
{"v":0,"name":"my_server","msg":"Tokio runtime found; starting in existing Tokio runtime","level":30,"hostname":"my_computer","pid":42127,"time":"2024-02-10T03:10:25.952767514Z","target":"actix_server::server","line":197,"file":"/home/user/.cargo/registry/src/index.crates.io-6f17d22bba15001f/actix-server-2.3.0/src/server.rs"}
```

# How to run

Make sure you are using the latest version of stable rust by running `rustup update`.

## Native

`cargo run --release`

On Linux you need to first run:

`sudo apt-get install libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libxkbcommon-dev libssl-dev`

On Fedora Rawhide you need to run:

`dnf install clang clang-devel clang-tools-extra libxkbcommon-devel pkg-config openssl-devel libxcb-devel gtk3-devel atk fontconfig-devel`

## Web Locally

You can compile your app to [WASM](https://en.wikipedia.org/wiki/WebAssembly) and publish it as a web page.

We use [Trunk](https://trunkrs.dev/) to build for web target.

1. Install the required target with `rustup target add wasm32-unknown-unknown`.
2. Install Trunk with `cargo install --locked trunk`.
3. Run `trunk serve` to build and serve on `http://127.0.0.1:8080`. Trunk will rebuild automatically if you edit the project.
4. Open `http://127.0.0.1:8080/index.html#dev` in a browser. See the warning below.

> `assets/sw.js` script will try to cache our app, and loads the cached version when it cannot connect to server allowing your app to work offline (like PWA).
> appending `#dev` to `index.html` will skip this caching, allowing us to load the latest builds during development.

## Web Deploy

1. Just run `trunk build --release`.
2. It will generate a `dist` directory as a "static html" website
3. Upload the `dist` directory to any of the numerous free hosting websites including [GitHub Pages](https://docs.github.com/en/free-pro-team@latest/github/working-with-github-pages/configuring-a-publishing-source-for-your-github-pages-site).
4. we already provide a workflow that auto-deploys our app to GitHub pages if you enable it.

> To enable Github Pages, you need to go to Repository -> Settings -> Pages -> Source -> set to `gh-pages` branch and `/` (root).
>
> If `gh-pages` is not available in `Source`, just create and push a branch called `gh-pages` and it should be available.

You can see the deployed version here <https://c-git.github.io/log-viewer/>

# Credits

Built on [egui](https://www.egui.rs/) and started from [egui template](https://github.com/emilk/eframe_template/)

## License

All code in this repository is dual-licensed under either:

- Apache License, Version 2.0
- MIT license

at your option.
This means you can select the license you prefer!
This dual-licensing approach is the de-facto standard in the Rust ecosystem and there are very good reasons to include both as noted in
this [issue](https://github.com/bevyengine/bevy/issues/2373) on [Bevy](https://bevyengine.org)'s repo.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.
