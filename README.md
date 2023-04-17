# coin tracker

Follow the coins [Sankey](https://en.wikipedia.org/wiki/Sankey_diagram) style.

![](./docs/screenshot.png)

### Testing locally

#### Native

Make sure you are using the latest version of stable rust by running `rustup update`.

`cargo run --release`

On Linux you need to first run:

`sudo apt-get install libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libxkbcommon-dev libssl-dev`

#### Web

We use [Trunk](https://trunkrs.dev/) to build for web target.

1. Install Trunk with `cargo install --locked trunk`.
2. Run `trunk serve` to build and serve on `http://127.0.0.1:8080`. Trunk will rebuild automatically if you edit the project.
3. Open `http://127.0.0.1:8080/index.html
