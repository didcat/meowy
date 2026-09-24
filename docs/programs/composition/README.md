# Preserve a primary value through composition

[All programs](../README.md) · [Manifest](mod.mwy) · [Entry](main.mwy) · [Reading types](readings.mwy)

A pipeline labels an integer reading, checks the sensor range, and preserves the
primary together with its named fields. The module exports exact structural types
for both stages; the entry composes the exported functions through dispatch.

```sh
cd docs/programs/composition
meowy check
meowy run
```

Expected output:

```text
Reading: 21 celsius from rack-a
Within the sensor range
Composition retained the fields and primary
Copied primary: 21
```

`check` emits the entire Reading as its primary and adds a `checked` field. The
result's shape includes the original sensor, unit, and integer primary; no dynamic
lookup table is created. Its scalar range comparisons inspect the primary. A type
predicate instead checks the complete Checked shape.

The matcher `reading<readings.Checked>` tests the complete result. Its body can
then bind `checked : reading~<readings.Checked>` using that proof. The predicate
and ascription have distinct punctuation in every expression position. Copying
the integer primary into `value` leaves the copyable aggregate intact.

Change the reading to 120 to see `checked` become false without changing its type
or discarding fields. The other three output lines still run: matchers are
independent. All values and static string views are inline or borrowed, and the
program needs neither a heap allocator nor an executor. See
[primary composition](../../reference/values-and-blocks.md#primary-composition).
