# comtains

`comtains` builds zero-allocation byte-set matchers at compile time. Use the `byte_set!` macro to expand your literals into an inlined decision tree that short-circuits on the first mismatching byte, giving predictable instruction counts for hot opcode or protocol classifiers.

```rust
use comtains::{byte_set, ByteSet};

const OPCODES: ByteSet = byte_set![
    b"\xA0\xB1",
    b"\xA1\xB2",
    b"\xA1\xB2\xC3",
    b"\xA1\xB2\xC4",
];

assert!(OPCODES.contains(b"\xA1\xB2"));
assert!(!OPCODES.contains(b"\xA1\xB3"));
```

## How it works

1. At macro expansion time every byte string, string literal, or byte array is converted into a shared trie.  
2. Each trie node counts how often its edges are used; siblings are sorted by descending weight (ties broken by byte value).  
3. The macro emits a `match` ladder that compares `candidate.get(depth)` against those ordered edges, recursing into the subtree or failing fast.  
4. Optional debug metadata (enabled in tests or with the `debug-metadata` feature) exposes the trie layout for inspection and benchmarking.
