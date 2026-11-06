# facet_chips

The chips that filter a list: a row per facet (a file's kind, a pull request's label), one chip per value with how many items picking it would show. A click shows only the picked values of that row; an option-click hides a value. Both the file tree's filter and the Others tab's repository filters draw their chips with these pieces, so they look and behave the same.

It draws and decodes clicks only; what a chip means and what passes lives with each filter (`qd-file-tree`'s `FileFilter`, the queue's Others filtering).
