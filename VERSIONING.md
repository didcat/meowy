# Versioning, allegedly

We looked at semantic versioning and decided two of the numbers were doing too much.

meowy releases are **v0.0.N**. The major stays at zero. The minor stays at zero.
The patch number carries the entire operation. Please respect its privacy during
this difficult time.

## 0.0 is a lifestyle

| Other projects, probably | meowy, minding its business |
| ------------------------ | --------------------------- |
| `v1.0.0`                 | `v0.0.1`                    |
| `v2.0.0`                 | `v0.0.348`                  |
| `v3.0.0`                 | `v0.0.1293`                 |

Bug fix? Patch number. New feature? Patch number. Entire compiler replaced by
three cats in a trench coat? Believe it or not, patch number.

These are examples, not a formula. The numbers can skip or arrive out of order.
There is no hidden roadmap in `348`. Numerology will not help you here.

And no, `0.0` does not mean we're waiting to become a real project. `v0.0.1` is
the current first-full-release target. The zeros are not going through a phase.

## Release schedule: vibes

A release happens when we feel like making one.

Not every commit gets a release. Not every release needs a dramatic reveal.
Sometimes something big lands. Sometimes a small fix deserves to see the world.
There is no calendar invitation. The calendar was not consulted.

## Pre-releases: we ain't so sure yet

That's the whole idea. A release, but with a little hesitation in its voice.

| Example            | Translation                      |
| ------------------ | -------------------------------- |
| `v0.0.348-maybe.1` | we ain't so sure yet             |
| `v0.0.348-maybe.2` | still looking at it suspiciously |
| `v0.0.348`         | alright, out you go              |

`maybe` is an example suffix, not a sacred naming ritual. There is no required
alpha → beta → rc obstacle course, and no promise that a candidate eventually
loses its suffix. Some releases need to sit with their feelings.

## The tiny amount of fine print

Only the patch field changes, but that does **not** mean “backward-compatible.”
Breaking changes can live there too. Read release notes for changes and migration
instructions; use release dates for chronology. The number is not your project manager.

A published tag keeps its contents. If we mess it up, we make another one.
Rewriting history is Git's problem, not a release strategy.

Language-contract revisions, schemas, protocols, and dependency versions still
follow their own rules. We are being silly about release names, not wire formats.
The [compiler plan](COMPILER.md) and [status](STATUS.md) still say what actually works.

Those numbers have jobs. These numbers have vibes.
