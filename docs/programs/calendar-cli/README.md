# Convert calendars from the command line

[All programs](../README.md) · [Manifest](mod.mwy) · [Entry](main.mwy) · [Command description](command.mwy)

The command module exports an immutable, typed CLI description. The entry owns
argv storage, handles help/version/usage results explicitly, validates a Gregorian
date, and converts its calendar view without changing the day.

```sh
cd docs/programs/calendar-cli
meowy check
meowy run -- --calendar chinese 2026-02-17
meowy run -- --help
```

The conversion prints:

```text
chinese:2026-M01-01
```

Use `-c` for `--calendar` and `-v` to print the selected calendar rule version.
Other choices are `gregorian`, `julian`, `hebrew`, `islamic-civil`, and `buddhist`.
The [time and calendar guide](../../guide/time-and-date.md) explains the Chinese
example's source, leap-month identity, and the difference between durations and
calendar arithmetic.

| Outcome                                           | Status |
| ------------------------------------------------- | ------ |
| Help, version, successful conversion              | 0      |
| Invalid option/date or unsupported calendar range | 2      |
| Argv acquisition or output failure                | 1      |

Command metadata and parse results use inline storage. `process.arguments` takes
an explicit allocator; its owner remains alive until every parsed string view has
expired. Date conversion allocates nothing. Rendering receives explicit writer
callables, and every write result is checked. No parser branch exits the process
or invokes an application handler implicitly.

The [CLI library](../../reference/stdlib/cli.md) also supports custom parsers,
bounded repeated options, global options, and nested subcommands. The
[duration CLI](../duration-cli/README.md) demonstrates that command hierarchy.
