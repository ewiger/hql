# Bootstrap decisions

- Scaffolded the empty standalone `hql` directory with `grem init . -t rust
  --name hql`, equivalent to `grem init ./hql -t rust` from its parent.
- Used installed grem 0.3.0 backed by the local checkout
  `/Users/yy/code/ai/grem`, revision `4a8942619cf2f32a2082e214c896dec1116b87aa`.
  Inspected the Rust template before initialization and generated instructions
  afterward. Keep `.grem/**` dormant and unchanged; customize project-owned files.
- Inspected HyperMarkDown at revision
  `74487940adb82654207cfec4cfd934d03eeee8f9`; see
  [integration evidence](../wiki/hmd-integration.hmd). Reuse its parser/resolver
  through a future adapter. Do not couple the language core to raw HMD text.
- Use a small expression slice with Int, Bool, and addition to demonstrate type
  checking before evaluation. The scanner lives in the parser; no empty modules.
  Signed 64-bit integers and 256-literal maximum are provisional implementation
  choices. Keep broader syntax, Card schemas, and host transport unresolved.
- Follow the stamped stack: thiserror for structured library failures, tempfile
  for isolated filesystem tests, clap derive for the CLI. The CLI formats errors
  directly; it does not need an additional general-purpose error dependency.
- Toolchain remains the generated `stable` channel (not an exact compiler pin).
  Cargo.lock records dependency resolution; no promise of exact compiler
  reproducibility is made.
- User instruction: never add the assistant as a co-author or committer. Do not
  add assistant attribution trailers or configure an assistant Git identity.
