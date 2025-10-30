# comtains

`comtains` expands static byte sequences into zero-allocation matchers at compile time. The `byte_set!` macro emits a branch-ordered decision tree, keeping membership checks to a handful of predictable instructions—ideal for tight opcode dispatchers or protocol parsers.

```rust
use comtains::{byte_set, ByteSet};

const HTTP_METHODS: ByteSet = byte_set![b"GET", b"POST", b"PUT", b"PATCH"];

assert!(HTTP_METHODS.contains(b"GET"));
assert!(!HTTP_METHODS.contains(b"DELETE"));
```

## How it works

1. All inputs are parsed at macro expansion time into a trie that shares common prefixes.  
2. Each edge records how many sequences traverse it; siblings are sorted by descending weight to probe common paths first.  
3. The macro generates a nested `match` ladder that compares `candidate[depth]`, short-circuiting on the first mismatch.  
4. Debug metadata is emitted alongside the matcher so tests and benchmarks can assert branch ordering or inspect the trie layout.


## Motivation

This crate originated from a research project involving **QEMU introspection** for dynamic malware analysis.  
Part of the system hooks Translation Blocks (TCGs) during translation and needs to efficiently decide whether a given translated basic block ends with specific opcodes—such as `syscall` instructions.  
To keep this decision fast and early in the translation process, a **zero-allocation, compile-time matcher** for static byte patterns was required.  
`comtains` was built to fulfill this exact need: performing rapid membership checks on fixed opcode sets with minimal branching and no runtime overhead.
