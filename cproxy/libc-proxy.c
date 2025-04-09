#include <dlfcn.h>
#include <unistd.h>

// Compile with:
//   gcc -fPIC -shared proxy.c -o proxy.so
// Load with:
//   LD_PRELOAD=./proxy.so ./some-program

static void _on_load() __attribute__((constructor));

char *(*c_fgets)(char *s, int size, void *stream);

// openbsd/src/blob/master/lib/libc/string/strlen.c
size_t
_strlen(const char *str)
{
  const char *s;

  for (s = str; *s; ++s)
    ;
  return (s - str);
}

void
_die(const char *err, const char *details)
{
  write(2, "fatal: ",  7);

  write(2, err, _strlen(err));

  if (details)
  {
    write(2, " - ", 3);
    write(2, details, _strlen(details));
  }

  write(2, "\n", 1);

  _exit(1);
}

void
_must_dlsym(void *dl_handle, const char *symbol, void *target)
{
  *(void **)(target) = dlsym(dl_handle, symbol);
  if (!*(void **)(target))
  {
    _die("dlsym failed", dlerror());
  }
}

void
_on_load()
{
  void *handle = dlopen("libc.so.6", RTLD_NOW);
  if (!handle) {
    _die("dlopen failed", dlerror());
  }

  _must_dlsym(handle, "fgets", &c_fgets);
}

char
*fgets(char *s, int size, void *stream)
{
  write(2, "hello\n", 6);

  char *x = c_fgets(s, size, stream);

  return x;
}
