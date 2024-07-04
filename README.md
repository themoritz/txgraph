# txgraph

 [txgraph.info](https://txgraph.info)

Interactive [Sankey style](https://en.wikipedia.org/wiki/Sankey_diagram) visualization of the Bitcoin transaction graph.

<img width="597" alt="txgraph.info" src="https://github.com/themoritz/themoritz/assets/3522732/3a0935b6-84ed-4380-9bdf-8ad97ce12ab8">

#### Features
* Expand/collapse transaction inputs and outputs
* Annotate and colorize transactions as well as inputs/outputs
* Export transaction details to [Beancount](https://beancount.github.io/).
* Infinite zoom/pan
* Adjust layout parameters

## Development

This repo covers just the fontend. The backend is a [fork of electrs](https://github.com/themoritz/electrs/tree/txgraph).

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
