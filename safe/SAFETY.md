# Safety Audit

This port still uses `unsafe`, but the remaining usage is intentionally confined to ABI-preserving edges that cannot be expressed in safe Rust alone.

## Remaining `unsafe` categories

1. FFI entry points in `src/lib.rs`, `src/api.rs`, `src/document.rs`, `src/dumper.rs`, `src/emitter.rs`, `src/event.rs`, `src/loader.rs`, `src/parser.rs`, `src/reader.rs`, `src/scanner.rs`, and `src/writer.rs`.
   Every exported `extern "C"` symbol accepts raw C pointers and is wrapped in the shared `src/ffi.rs` unwind guards so Rust panics do not cross the C ABI.

2. Raw pointer projection over C-visible libyaml structs in `src/types.rs` consumers.
   Parser, emitter, event, token, document, node, stack, queue, and buffer state must stay layout-compatible with upstream `yaml.h`, so these modules read and mutate fields through raw pointers and pointer arithmetic.

3. Libc-style memory and string operations in `src/alloc.rs` and `src/lib.rs`.
   Allocation, reallocation, free, `memcpy`-style byte movement, and C-string traversal are required to preserve libyaml's public ABI and callback contracts.

4. Callback interaction in `src/api.rs`, `src/reader.rs`, and `src/writer.rs`.
   User-supplied read and write handlers are invoked through raw function pointers with caller-owned buffers, so the boundary must remain `unsafe`.

## Audit notes

- The final sweep keeps parser, emitter, and document runtime state in the ABI-visible structs rather than adding shadow Rust-only state.
- Regression coverage in `tests/` exercises the remaining high-risk paths: malformed UTF handling, oversized reader input, CVE-2014-9130, the checked-in clusterfuzz input, staged C probe execution, and staged object-link compatibility.
- Build configuration is expected to keep unwinding enabled for the library; the final-sweep tests also reject `panic = "abort"` or `-Cpanic=abort` in committed build configuration.
