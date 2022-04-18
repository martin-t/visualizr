# Helpers for testing.
# LATER Hide/remove before release?

#' @export
rel <- function(obj) {
    # Rextendr only watches for changes of the lib, not its deps,
    # so it doesn't recompile if only a dep changed.
    # Touch the lib to always recompile.
    fs::file_touch("src/rust/src/lib.rs")

    rextendr::document()
    devtools::load_all()
}

#' @export
ins <- function(obj) {
    .Internal(inspect(obj))
}
