---
question: In what units are the speed limit classes of the GTSRB traffic sign benchmark expressed?
answer:  Metric only. Classes 0 through 8 are 20, 30, 50, 60, 70, 80, end-of-80, 100 and 120 kph. No miles-per-hour class exists in the 43-class label set. This contradicts doc/IDEA.md:62-64, which states the published weights are imperial and that metric recognition therefore needs retraining.
tag:      [A]
support:  2
evidence: tanganke/gtsrb dataset card README.md L16-24 — "'0': red and white circle 20 kph speed limit" through "'8': red and white circle 120 kph speed limit"; corroborated by signnames.csv mappings in two independent projects (canozcivelek/traffic-sign-recognition, felixlephuoc/traffic-sign-recognition) where ClassId 0=20km/h, 1=30, 2=50, 3=60, 4=70
read:     2026-09-27
decay:    stable
unblocks: Whether traffic sign recognition for a European car needs retraining at all, per doc/IDEA.md:62
---

# In what units are the GTSRB speed limit classes expressed?

Metric throughout. The speed limit classes are:

| ClassId | Label |
|---|---|
| 0 | 20 kph |
| 1 | 30 kph |
| 2 | 50 kph |
| 3 | 60 kph |
| 4 | 70 kph |
| 5 | 80 kph |
| 6 | end of / de-restriction of 80 kph |
| 7 | 100 kph |
| 8 | 120 kph |

Read directly from the dataset card. There is no miles-per-hour class
anywhere in the 43-class label set — unsurprising for the German Traffic Sign
Recognition Benchmark, but worth stating because the opposite assumption is
recorded in this repository.

**What this does and does not settle.** GTSRB is 39,209 training and 12,630
test single images with one class label each — a static image-classification
benchmark, not a driving video task. So a model trained on it is a turnkey
*classifier of cropped sign photographs*, not a live roadside detector. Those
are different problems, and a project needs both a detector and a classifier
to use it.

**Implication:** the gap in `doc/IDEA.md:62-64` runs the opposite way to the one
recorded. The text says the published weights are imperial and that metric
recognition needs retraining. The primary source shows metric labels, so a
model trained on GTSRB already reads the units the target car uses. What would
need retraining is **imperial** recognition — relevant only if the car is driven
in the UK or US. The statement is not to be repaired here; see the note below.

**Reverses if:** a source is found for the "imperial weights" claim. That claim
implies a specific artefact — some project's weights, not GTSRB — and naming
that artefact would settle which finding is right. Nothing matching it was
found in the session that produced this file.

## Searched

- fetched `huggingface.co/datasets/tanganke/gtsrb/raw/main/README.md` (237
  lines) and read the class listing at L16-24
- web search: "GTSRB dataset licence terms of use Institut Neuroinformatik
  speed limit classes km/h 43 classes" — returned the class listing and the
  official page mirrored by datasets-mila, which states the data is "free to
  use" with a request to cite Stallkamp et al., IJCNN 2011. **The licence terms
  on the benchmark host itself were not read**, which matters if a trained
  model is redistributed.

## Revisit log

- 2026-09-27 — initial finding, [A] from the dataset card, support 2
