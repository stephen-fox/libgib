# libgib

libgib is a collection of Rust libraries that provide an OS-agnostic
API for manipulating process memory on Unix-like and Windows operating
systems using minimal external dependencies. The main goal of the project
is to help facilitate process injection to aid in reverse engineering.
For example: overwriting a external function pointer in a process' global
offset table to point at a proxy function written in Rust.

## Overview

- `afnative` - afnative provides an abstraction for operating-system-specific
  sockets like Unix sockets and Windows named pipes
- `dlrkit` - dlrkit provides an API for interacting with the dynamic linker
  and loading libraries at runtime such as .so files and .dll files. Useful
  for executing code from external libraries using the C ABI
- `mmor` - mmor provides an OS-agnostic API for enumerating memory-mapped
  objects in the current process and resolving a symbol's address to its
  name and parent library. Useful for finding the addresses of libraries
  or the process' executable, as well as mapping the process global offset
  table without hardcoding each entry's offset in the table in your program
- `mrevise` - mrevise abstracts operating on the current process' memory,
  including: modifying memory protections (permissions), allocating memory,
  and finding byte patterns. This library is a fork of Jacob Read's 'mem'
  crate from their Pocket Relay project

## Examples

Refer to the [examples/](examples/) directory for examples.

## Special thanks

This codebase (namely the `mrevise` library) started as a fork of
Jacob Read's `mem` crate from their [Mass Effect 3 PocketRelay project][pr].
Even if you are not interested in gaming, you should check out Jacob's
work - their Rust code is an excellent reference resource.

The `dlrkit` library started as yet-another take on dynamic loading
of libraries at run time (but with minimal external dependencies).
I adopted the "phantom data" lifetime enforcement idea from OpenByteDev's
[dlopen2][dlopen2] library.

The `afnative` library is a fork of Daniel Griffen's excellent
[windows-named-pipe][wnp] library.

[pr]: https://github.com/PocketRelay/PocketRelayClientPlugin/blob/2faa7a2f718fec3cc90345bc9b3a84aa282a1e57/src/hooks/mem.rs
[dlopen2]: https://github.com/OpenByteDev/dlopen2
[wnp]: https://gitlab.com/dgriffen/windows-named-pipe/
