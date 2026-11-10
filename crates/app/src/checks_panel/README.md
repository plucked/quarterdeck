# checks_panel

The Checks tab of the side panel. It shows how the CI checks of the shown pull request stand on its head commit: in a stack, only the shown layer's, never the whole stack's.

A repository can run well over a hundred checks per pull request, so the tab does not show one long list. It leads with a verdict and an outcome bar. Below that, flat sections list what needs attention: failed and cancelled checks first, then running and queued ones, each with the workflow it belongs to. Every check then follows as a tree (workflow, then called workflow, then job) folded by default, with checks that were skipped hidden until asked for.

It reads the checks from GitHub through `qd-github` when the tab shows, and again while some are pending and the tab stays open. Nothing is cached, since checks change by the minute. It does not merge, re-run or cancel anything: those stay on GitHub, which a click on a check opens.
