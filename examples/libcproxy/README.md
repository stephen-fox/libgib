# libc proxy example

Seung Kang and I were trying to solve the [library CTF][library-ctf] from
LA CTF 2025 which uses the `sendfile(2)` system call to read /proc/self/maps,
which fails. We used libgib to create a shared library (.so) which, when
loaded, overwrites the CTF program's global offset table entry for the
sendfile libc function pointer with a Rust function that tells us the actual
failure. The library is loaded into the CTF process using the `LD_PRELOAD`
environment variable (see `man ld.so` for details).

The library also includes an unused function that we used with the plaid
CTF. Our goal was to overwrite the global offset table entry for malloc(3)
with a Rust function pointer that logged the arguments to malloc. Doing
this required using a custom memory allocator to avoid Rust infinitely
calling our custom malloc function.

[library-ctf]: https://github.com/uclaacm/lactf-archive/blob/3379d4a7b36680764a34e7dc817cc3c94c244764/2025/pwn/library/library.c
