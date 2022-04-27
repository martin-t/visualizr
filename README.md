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

Misc note: `rextendr::document()` (sometimes?) doesn't notice when a dependency changes - you have to make a change in inspectr directly for commonr/bindingsr to be recompiled.

## Lessons Learned

- There are 2 libs for interfacing with R: rextendr and cargo-framework. Rextendr *appears* to be higher level while cargo-framework appears to expose raw SEXPs. In reality:
  - Cargo-framework has verious issues:
    - Doesn't handle nightly (fails to parse version string, who knows what other issues there are).
    - Some functions e.g. `run` fail even with stable.
    - You need to manually delete the .so file to trigger a recompile (I only tested on linux).
    - For some inexplicable reason it copies the entire source code of its sublibs into your source code.
    - Maybe I was holding it wrong but it seemed to (also?) create a new project in a different directory than I specified, twice.
  - Rextendr is much more mature and you can access raw SEXPs from it as well. I recommend using rextendr since I can't think of anything where cargo-framework would be better.
  - Rextendr's bindings don't include internal headers but it's trivial to create your own bindings with `#define USE_RINTERNALS` and use those.
    - You can cast between the `SEXP` type from the 2 libs.
- All Rust GUI libs are awful. I wanted something that offers draggable boxes for SEXPs and an easy way to draw lines between them.
  - I couldn't figure out how to get the position of a GUI element in egui. It is possible, [https://github.com/setzer22/egui_node_graph](egui_node_graph) does it, it's just a mess to figure out and i gave up.
  - Macroquad's GUI (megaui) is much easier to figure out but has various small but annoying issues: selecting or copying text randomly doesdn't work; if there's too much text, it all disappears until the user scrolls; it reports the wrong mouse position until it moves, ...
  - I don't have a good solution here, macroquad is probably the lesser evil.

## License

[AGPL-v3](LICENSE) or newer
