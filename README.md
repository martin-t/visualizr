# Visualizr

A tool for visualizing the internal representation of objects in the R language

TODO screenshot

The code is split into 2 parts:
- Inspectr - an R package which reads the internals of R objects and sends them to visualizr
- Visualizr - a GUI which draws the internals and relationships between objects

## Dependencies

- [Rust](https://www.rust-lang.org/learn/get-started)
- Visualizr uses the macroquad game engine - if you're on Linux, you need to install [its dependencies](https://github.com/not-fl3/macroquad#linux).
- Inspectr uses rextendr so you need to [install it](https://github.com/extendr/rextendr#installation). The [devtools](https://github.com/r-lib/devtools) package is also recommended.

## Usage

Currently you need to compile this project from source to use it.

- Compile and run visualizr - `cargo run` in the project's root.
- Run `R` in `inspectr/`
    - Compile and load inspectr: `rextendr::document() ; devtools::load_all()`
    - Now the `visualize` function should be available - use it on arbitrary R objects and visualizr will draw them.

## Development

Misc note: `rextendr::document()` *sometimes* doesn't notice when a dependency changes - you have to make a change in inspectr directly for common/bindingsr to be recompiled.

## License

[AGPL-v3](LICENSE) or newer
