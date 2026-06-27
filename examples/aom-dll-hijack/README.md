# Age of Mythology DLL hijack

SeungKang and I used libgib to mess around with examining memory allocations
in Age of Mythology Extended Edition. Our goal was to traceback calls to
malloc(3) and better understand how the game parses save game files. We never
got quite that far, but this code still serves as a real-world example of
using libgib to inject code into Age of Mythology using
[DLL forwarding][dll-forwarding].

[dll-forwarding]: https://github.com/stephen-fox/rust-windows-library-forwarding-example

## Structure

Note: Unfortunately, there seems to be a limitation with Rust's LSP server
in that it does not allow nested Rust workspaces. This code was originally
in a separate git repository and utilized its own Rust workspace. I tried
to carry that over here, but that seemed to break the Rust LSP. As a result,
I had to add the code base's individual crates as members of libgib's
workspace, which to me feels like an ugly hack.

- `aomlib` - Produces a DLL named `version.dll` that replaces the game's
  version.dll. When loaded by the game, it overwrites the memcpy function
  pointer in the import table with a pointer to a Rust function that stores
  information about the calls to the memcpy function. It then starts a thread
  that serves the information it saved about calls to memcpy using a Windows
  named pipe. Note: It may actually be overwriting memmove by accident, but
  you get the idea
- `client` - Connects to the Windows named pipe created by `aomlib` and
  writes the information to stdout
- `server` - Provides the code used by `aomlib`
