# Backlog

**What this directory is.** Task files that were specified and are not going to
be built as written. **Nothing here is deleted** — a superseded task file keeps
its reasoning, because that reasoning is usually why the next attempt is cheaper
than the first one was — and the file that replaced it says where it went.

**Why a task gets here rather than being rewritten.** A task file is a
specification, and a specification that is rewritten in place destroys the
record of what was decided and what it cost. `AGENTS.md` § *Working context*
carries the same rule for working documents — *"Superseded, not deleted, and
every entry dated and attributed"* — and this directory is where a task file
obeys it. The rule a file obeys belongs in the file; a directory that made its
own rules would be a second source, which is the thing `AGENTS.md` warns about.

**What moving a file here does and does not mean.** It means the task is not in
this sequence's build order and `doc/ui/IMPLEMENTATION_STATE_DEMO.md` records
no progress against it. It does **not** mean the idea is wrong, and it does not
close anything in `doc/ui/DEMO_APPLICATION.md` § *Library gaps* — those rows keep
their numbering because four files outside that document cite it by number.

**No file in here is a task in flight.** `task-sequence.md` § *Scope* names the
sequences this workflow applies to, and `doc/ui/backlog/` is in none of them:
nothing here has a state-file row, a review, or an operator-commit gate, because
nothing here is being built.

## Contents

| file | superseded by | why |
|---|---|---|
| [`TASK_UI_DEMO_01.md`](TASK_UI_DEMO_01.md) | [`doc/ui/done/TASK_UI_DEMO_01.md`](../done/TASK_UI_DEMO_01.md), 2026-10-09 | A procedural vector map — a seeded world, a camera, three road classes, a route, POIs and a car marker, all from `Rect`, `Path`, `Circle` and `Polygon` — was judged too ambitious on 2026-10-09. The replacement keeps the same seventh page and its name and drops the map's geometry: one map image, full-bleed, as the base layer every later panel goes over. The reasoning was condensed 2026-10-10. |