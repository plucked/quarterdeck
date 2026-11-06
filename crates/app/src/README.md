# Quarterdeck source

The desktop app: starts gpui, wires the GitHub client and the cache together and renders the views. This is the only crate that knows about gpui. It does not talk HTTP or SQL itself; `qd-github` and `qd-cache` do, and `qd-update` downloads and installs new versions; the views call them on gpui's background executor.
