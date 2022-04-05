# Bindingsr

- Temporary (?) alternative to libR-sys with internal headers included

## How to regenerate the bindings

- `bindgen wrapper.h  --blocklist-item FP_NAN --blocklist-item FP_INFINITE --blocklist-item FP_ZERO --blocklist-item FP_SUBNORMAL --blocklist-item FP_NORMAL -- -I/usr/share/R/include > src/bindings.rs`

Note that currently this is based on cargo-framework's way of generating the bindings because it was easier to figure out. Extendr does something very similar so there should be no issues.
