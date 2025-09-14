# libgib

libgib is a collection of Rust libraries for manipulating process memory
on Unix-like and Windows operating systems. It is designed to facilitate
process injection and aid in reverse engineering contexts (e.g., overwriting
a function with a trampoline that executes a proxy function written in Rust).
It also aims to have minimal external dependencies.

## Special thanks

This codebase (namely the `mrevise` library) started as a fork of
Jacob Read's `mem` crate from their [Mass Effect 3 PocketRelay project][pr].
Even if you are not interested in gaming, you should check out Jacob's
work - their Rust code is an excellent reference resource.

The `dlrkit` library started as yet-another take on dyamic loading
of libraries at run time (but with minimal external dependencies).
I adopted the "phantom data" lifetime enforcement idea from OpenByteDev's
[dlopen2][dlopen2] library.

The `afnative` library is a fork of Daniel Griffen's excellent
[windows-named-pipe][wnp] library.

[pr]: https://github.com/PocketRelay/PocketRelayClientPlugin/blob/2faa7a2f718fec3cc90345bc9b3a84aa282a1e57/src/hooks/mem.rs
[dlopen2]: https://github.com/OpenByteDev/dlopen2
[wnp]: https://gitlab.com/dgriffen/windows-named-pipe/
