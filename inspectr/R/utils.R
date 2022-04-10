# Helpers for testing.
# LATER Hide/remove before release?

#' @export
rel <- function(obj) {
    rextendr::document()
    devtools::load_all()
}

#' @export
ins <- function(obj) {
    .Internal(inspect(obj))
}
