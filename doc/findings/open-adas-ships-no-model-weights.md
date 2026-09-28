---
question: Does the open-adas repository ship trained model weights, or only the code to run them?
answer:  Code only. A recursive GitHub tree listing of vietanhdev/open-adas returns 376 files outside third_party and zero model artifacts of any recognised weight format. It is TensorRT inference plumbing for a detector the user must train and convert themselves.
tag:      [A]
support:  1
evidence: GitHub API repos/vietanhdev/open-adas/git/trees/master?recursive=1 — 376 non-third_party entries, 0 matches for .onnx/.pb/.tflite/.pt/.pth/.h5/.weights/.param/.caffemodel/.prototxt/.bin; README states the repo contains "source code for Jetson Nano, not including the source code for model training and conversion"
read:     2026-09-27
decay:    1 month
unblocks: Whether an MIT-licensed traffic sign recognition project is available to build on, or whether a model has to be trained from scratch
---

# Does the open-adas repository ship trained model weights, or only the code to run them?

Code only. This is an L6 absence — "the gap between what the docs promise and
what the repository contains" — established by enumerating the repository
rather than by reading a claim about it.

A recursive tree listing of `vietanhdev/open-adas` at branch `master`, taken
2026-09-27, returns **376 files outside `third_party` and zero model
artifacts** across every recognised weight format (`.onnx`, `.pb`, `.tflite`,
`.pt`, `.pth`, `.h5`, `.weights`, `.param`, `.caffemodel`, `.prototxt`,
`.bin`). The repository also vendors the entire `onnx-tensorrt` third-party
tree in-tree, which is what a naive weight search hits first and why the
earlier reading of this project as "ships weights" was wrong.

The README agrees, and says so plainly: the repository "contains source code
for Jetson Nano, not including the source code for model training and
conversion."

**Licence and currency, read rather than assumed:** MIT, per the repository
`LICENSE` — a licence compatible with this project's GPL-3.0-or-later. Last
commit 2023-10-24 ("Create FUNDING.yml"), which is a liveness signal rather
than a maintenance one; contributor count and issue close rate were not
examined. Target hardware is Jetson Nano, not a Raspberry Pi.

**Implication:** open-adas is worth reading for TensorRT inference structure —
preprocessing, tensor shapes, the plumbing around a detector — and is not a
traffic sign recognition implementation you can run. A model has to be
trained and converted separately, which makes the training data and its
licence the load-bearing choice rather than this repository. Combined with
`gtsrb-speed-limit-class-units.md`, the practical shape is: GTSRB supplies
metric class labels for static images, open-adas supplies inference plumbing
for hardware this project is not targeting, and the detector in between is
unbuilt.

**Reverses if:** weights are added to the repository. Re-run the tree listing;
roughly a minute. They would most likely arrive as release assets rather than
in-tree, which is a separate check not performed here.

## Revisit log

- 2026-09-27 — initial finding, [A] L6 absence, tree listing at `master`
