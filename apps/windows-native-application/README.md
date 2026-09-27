# Windows application layer

Portable .NET presentation policy and JSON DTOs used by the WinUI host. This
project contains no ASR, model, session, persistence, capture, WinUI, Win32 or
FFI implementation; Rust remains the shared source of truth. It exists to let
small native UI policies be tested without loading Windows App SDK or the Rust
DLL.
