# Decode a binary packet header

[All programs](../README.md) · [Manifest](mod.mwy) · [Entry](main.mwy) · [Decoder](codec/header.mwy)

The manifest maps `wire` to `./codec`. The entry imports `@"wire/header.mwy"`, and
the manifest exports that decoder as its package facade without exporting the
entry. This project combines explicit path aliases, a reusable module, borrowed
bytes, and a closed structural result union.

From the repository root:

```sh
cd docs/programs/packet
meowy check
meowy run
```

Expected output:

```text
Version: 1
Flags: 0
Payload size: 300
Truncated header: need 4 bytes; got 2
```

| Wire byte offset | meowy position | Meaning                   |
| ---------------- | -------------- | ------------------------- |
| 0                | 1              | Version                   |
| 1                | 2              | Flags                     |
| 2                | 3              | High byte of payload size |
| 3                | 4              | Low byte of payload size  |

The first input is `[1, 0, 1, 44]`; the decoder widens both length bytes to uint16
before shifting and combining them. The second input demonstrates `Truncated`,
which carries required and actual lengths. Both are deliberately exercised, so
handling the short header is a successful demo outcome rather than a panic.

Extra bytes are permitted. The decoder reads only the header; it does not assert
that the announced payload exists. Its borrowed slice and inline result allocate
nothing, and wire layout does not depend on native padding, alignment, or host
endianness. See [collections](../../reference/collections.md) and
[native boundaries](../../reference/modules-and-ffi.md#native-interfaces).
