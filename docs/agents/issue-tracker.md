# Issue tracker: GitHub

Issues and specs live in GitHub Issues for `archbung/precision`.
Use the `gh` CLI from this repo; it infers the repository from the remote.

## Operations

- Create: `gh issue create --title "..." --body-file <path>`.
- Read: `gh issue view <number> --comments`; include labels when needed.
- List: `gh issue list --state open --json number,title,body,labels,comments`.
  Apply label and state filters as appropriate.
- Comment: `gh issue comment <number> --body-file <path>`.
- Label: `gh issue edit <number> --add-label "..."` or `--remove-label "..."`.
- Close: `gh issue close <number> --comment "..."`.

For multiline bodies, write the text to a temporary file and use
`--body-file`.

When a skill says "publish to the issue tracker", create a GitHub issue.
When it says "fetch the relevant ticket", read the issue and its comments.

## Pull requests as a triage surface

**PRs as a request surface: no.**

## Wayfinding operations

The map is an issue labelled `wayfinder:map`, containing Notes,
Decisions-so-far, and Fog. Child tickets use `wayfinder:<type>` labels:
`research`, `prototype`, `grilling`, or `task`.

Link children as GitHub sub-issues. If unavailable, list them as tasks
in the map and add `Part of #<map>` to each child.

Represent blockers with native GitHub issue dependencies, using the
blocker's numeric database ID. If unavailable, add
`Blocked by: #<number>, #<number>` to the child.

The frontier is the map's open, unassigned children with no open blockers.
Choose the first eligible child in map order.

Claim with `gh issue edit <number> --add-assignee @me`.
Resolve by commenting with the answer, closing the ticket, and adding
a gist plus link to the map's Decisions-so-far.
