$env:RUST_BACKTRACE=1
@((cargo run --bin wts -- $args) -join "`n")

