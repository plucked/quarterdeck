# qc-file-tree source

Turns a pull request's changed file paths into the folder tree the review pane shows: folders before files, sorted by name, chains of single folders joined into one node, and the depth-first order the diff then follows so both agree. It also decides which files a filter hides (viewed, tests, generated, extensions), judging tests and generated files from path conventions because GitHub does not say. It renders nothing and knows nothing about viewed marks beyond the flag it is given.
