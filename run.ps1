@((cargo run -- $args) -join "`n") | Invoke-Expression

