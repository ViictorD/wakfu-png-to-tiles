# Wakfu

A tool that convert the result of [wakfu-png](https://github.com/ViictorD/wakfu-png) into 256x256 tiles of differents zoom. That can be used with leafletjs.

## Usage

This project requires [cargo](https://crates.io) to build.

First copy the output of [wakfu-png](https://github.com/ViictorD/wakfu-png) into the `input` folder.

Then simply build and run the binary:

```bash
cargo build --release
wakfu-png-to-tiles
```

After the program is done, you can find the result in the `output` folder.