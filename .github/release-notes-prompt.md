# Release notes prompt template

This file is the prompt sent to GitHub Models by the `Release` workflow.
Committing it to the repo makes tone, structure, and constraints code-
reviewable. Anyone with workflow-edit rights would otherwise be able
to silently change them.

Substitutions performed by `release.yml` before the prompt is sent:

| Placeholder        | Replaced with                                    |
|--------------------|--------------------------------------------------|
| `${VERSION}`       | The release version (e.g. `0.2.0`)              |
| `${PREV_VERSION}`  | The previous git tag, or `(initial release)`    |
| `${INPUTS}`        | `git log` + `git diff --stat` since `PREV_VERSION` |

---

You are writing release notes for **game-of-life ${VERSION}**.

Below is the git log and diffstat since **${PREV_VERSION}**.

Produce GitHub-flavoured Markdown with **exactly** these two sections, in
this order, with these exact headings:

```
## What's new

## Downloads
```

Rules for "What's new":

* Group bullets under bold sub-headers in this order, omitting any group that has no entries: **Core library**, **Desktop app**, **CLI**, **CI & release**, **Docs**.
* Assign each commit to a group using the **file paths in the diffstat**, not the commit subject. Commits touching `src/` go to Core library; `desktop/src/` or `desktop/ui/` go to Desktop app; `src/main.rs` and CLI test files go to CLI; `.github/workflows/`, `Cargo.toml` go to CI & release; `docs/` or `README.md` go to Docs.
* One sentence per bullet, present tense, terse.
* Do not invent features that are not in the diff. If the diff is empty, write "_No changes._" under the appropriate group.
* Do not include a "Known issues" section. The model has no access to the issue tracker and would have to guess.

Rules for "Downloads":

* Three lines, exactly:
  * Windows: portable installer (`.exe`), MSI installer (`.msi`), and the CLI binary (`.exe`).
  * Linux: AppImage, Debian package (`.deb`), and the CLI binary.
  * SmartScreen will warn on first launch of the unsigned Windows installers — choose **More info** → **Run anyway**.

Tone:

* Terse, matter-of-fact, present tense.
* No emoji.
* No marketing language ("exciting", "powerful", "now even better").
* No outer Markdown code fence around the document.

---

## Inputs

${INPUTS}
