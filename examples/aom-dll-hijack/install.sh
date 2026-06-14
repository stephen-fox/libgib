#!/bin/sh

set -eu

dll_name="version"
path_prefix="/c/Program Files (x86)/Steam"

if [ -d "/d/Steam" ]
then
  path_prefix="/d/Steam"
fi

cp -v "target/i686-pc-windows-msvc/debug/${dll_name}.dll" "${path_prefix}/steamapps/common/Age of Mythology/"

orig_copy="${path_prefix}/steamapps/common/Age of Mythology/${dll_name}_orig.dll"

if [ ! -f "${orig_copy}" ]
then
  cp -v "/c/Windows/SysWOW64/${dll_name}.dll" "${orig_copy}"
fi
