//backpack:forward-to version_orig

// for fn in $(rz-bin -E -q ~/version.dll  | cut -d ' ' -f 4 | cut -d '@' -f 1); do echo -e "#[unsafe(no_mangle)]\nfn ${fn##*_}() {}\n"; done

#[unsafe(no_mangle)]
fn GetFileVersionInfoA() {}

#[unsafe(no_mangle)]
fn GetFileVersionInfoByHandle() {}

#[unsafe(no_mangle)]
fn GetFileVersionInfoExA() {}

#[unsafe(no_mangle)]
fn GetFileVersionInfoExW() {}

#[unsafe(no_mangle)]
fn GetFileVersionInfoSizeA() {}

#[unsafe(no_mangle)]
fn GetFileVersionInfoSizeExA() {}

#[unsafe(no_mangle)]
fn GetFileVersionInfoSizeExW() {}

#[unsafe(no_mangle)]
fn GetFileVersionInfoSizeW() {}

#[unsafe(no_mangle)]
fn GetFileVersionInfoW() {}

#[unsafe(no_mangle)]
fn VerFindFileA() {}

#[unsafe(no_mangle)]
fn VerFindFileW() {}

#[unsafe(no_mangle)]
fn VerInstallFileA() {}

#[unsafe(no_mangle)]
fn VerInstallFileW() {}

#[unsafe(no_mangle)]
fn VerLanguageNameA() {}

#[unsafe(no_mangle)]
fn VerLanguageNameW() {}

#[unsafe(no_mangle)]
fn VerQueryValueA() {}

#[unsafe(no_mangle)]
fn VerQueryValueW() {}
