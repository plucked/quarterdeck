# qc-diff tests

Checks `FileDiff::build` on small patches written the way GitHub sends them: line numbering, side-by-side pairing, whitespace folding and inline change marks, through the public API only. Unit tests of the parser and the inline comparison sit next to their code in `src/`.
