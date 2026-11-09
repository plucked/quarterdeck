# brief_gallery

A window for working on how AI-written briefs look. It shows the status card in each of its states and a written brief that uses every widget, tone and severity, plus widgets the anchor check leaves out, so a layout change can be judged at once without running an agent on a pull request.

The sample is data compiled into the app (`samples/`): an agent answer as JSON, written by hand, and the two patches its anchors point into. It goes through the same check and preparation as a real run, and is drawn by `brief_view`, the code the Brief pane in the pull request detail uses, so what the gallery shows is what a brief looks like.

It does not run agents, open diffs or store anything.
