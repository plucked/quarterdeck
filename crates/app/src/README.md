# quick-check source

The desktop app: starts gpui, wires the GitHub client and the cache together and renders the views. This is the only crate that knows about gpui. It does not talk HTTP or SQL itself; `qc-github` and `qc-cache` do, and the views call them on gpui's background executor.
