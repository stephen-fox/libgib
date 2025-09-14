#![allow(non_snake_case)]

use core::ffi::c_void;

use std::ffi::OsString;
use std::io::{self, Read, Write};
use std::os::windows::prelude::*;
use std::path::Path;

const GENERIC_READ: u32 = 0x80000000;
const GENERIC_WRITE: u32 = 0x40000000;
const OPEN_EXISTING: u32 = 0x03;

const FILE_ATTRIBUTE_NORMAL: u32 = 0x80;
const FILE_FLAG_FIRST_PIPE_INSTANCE: u32 = 0x00080000;

const PIPE_ACCESS_DUPLEX: u32 = 0x00000003;
const PIPE_TYPE_BYTE: u32 = 0x00000000;
const PIPE_READMODE_BYTE: u32 = 0x00000000;
const PIPE_WAIT: u32 = 0x00000000;
const PIPE_UNLIMITED_INSTANCES: u32 = 255;

const INVALID_HANDLE_VALUE: *mut c_void = -1i32 as *mut c_void;
const ERROR_PIPE_NOT_CONNECTED: u32 = 233;
const ERROR_PIPE_CONNECTED: u32 = 535;

#[link(name = "kernel32")]
unsafe extern "system" {
    /// * LPCWSTR lpFileName:
    ///   The name of the file or device to be created or opened.
    /// * DWORD dwDesiredAccess:
    ///   The requested access to the file or device, which can be summarized
    ///   as read, write, both or neither zero).
    /// * DWORD dwShareMode:
    ///   The requested sharing mode of the file or device, which can
    ///   be read, write, both, delete, all of these, or none.
    /// * LPSECURITY_ATTRIBUTES lpSecurityAttributes:
    ///   A pointer to a SECURITY_ATTRIBUTES structure that contains
    ///   two separate but related data members: an optional security
    ///   descriptor, and a Boolean value that determines whether the
    ///   returned handle can be inherited by child processes.
    /// * DWORD  dwCreationDisposition:
    ///   An action to take on a file or device that exists or does not exist.
    /// * DWORD  dwFlagsAndAttributes:
    ///   The file or device attributes and flags, FILE_ATTRIBUTE_NORMAL
    ///   being the most common default value for files.
    /// * HANDLE hTemplateFile:
    ///   A valid handle to a template file with the GENERIC_READ access right.
    fn CreateFileW(
        lpfilename: *const u16,
        dwdesiredaccess: u32,
        dwsharemode: u32,
        lpsecurityattributes: *const SECURITY_ATTRIBUTES,
        dwcreationdisposition: u32,
        dwflagsandattributes: u32,
        htemplatefile: *mut c_void,
    ) -> *mut c_void;

    /// * HANDLE hFile:
    ///   A handle to the device (for example, a file, file stream,
    ///   physical disk, volume, console buffer, tape drive, socket,
    ///   communications resource, mailslot, or pipe).
    /// * LPVOID lpBuffer:
    ///   A pointer to the buffer that receives the data read from a file
    ///   or device.
    /// * DWORD nNumberOfBytesToRead:
    ///   The maximum number of bytes to be read.
    /// * LPDWORD lpNumberOfBytesRead:
    ///   A pointer to the variable that receives the number of bytes read
    ///   when using a synchronous hFile parameter.
    /// * LPOVERLAPPED lpOverlapped:
    ///   A pointer to an OVERLAPPED structure is required if the hFile
    ///   parameter was opened with FILE_FLAG_OVERLAPPED, otherwise it
    ///   can be NULL.
    fn ReadFile(
        hfile: *mut c_void,
        lpbuffer: *mut u8,
        nnumberofbytestoread: u32,
        lpnumberofbytesread: *mut u32,
        lpoverlapped: *mut OVERLAPPED,
    ) -> bool;

    /// * HANDLE hFile:
    ///   A handle to the file or I/O device (for example, a file,
    ///   file stream, physical disk, volume, console buffer, tape
    ///   drive, socket, communications resource, mailslot, or pipe).
    /// * LPCVOID lpBuffer:
    ///   A pointer to the buffer containing the data to be written
    ///   to the file or device.
    /// * DWORD nNumberOfBytesToWrite:
    ///   The number of bytes to be written to the file or device.
    /// * LPDWORD lpNumberOfBytesWritten:
    ///   A pointer to the variable that receives the number of bytes
    ///   written when using a synchronous hFile parameter.
    /// * LPOVERLAPPED lpOverlapped:
    ///   A pointer to an OVERLAPPED structure is required if the hFile
    ///   parameter was opened with FILE_FLAG_OVERLAPPED, otherwise this
    ///   parameter can be NULL.
    fn WriteFile(
        hfile: *mut c_void,
        lpbuffer: *const u8,
        nnumberofbytestowrite: u32,
        lpnumberofbyteswritten: *mut u32,
        lpoverlapped: *mut OVERLAPPED,
    ) -> bool;

    /// * LPCWSTR lpName:
    ///   The unique pipe name.
    /// * DWORD dwOpenMode:
    ///   The open mode.
    /// * DWORD dwPipeMode:
    ///   The pipe mode.
    /// * DWORD nMaxInstances:
    ///   The maximum number of instances that can be created
    ///   for this pipe.
    /// * DWORD nOutBufferSize:
    ///   The number of bytes to reserve for the output buffer.
    /// * DWORD nInBufferSize:
    ///   The number of bytes to reserve for the input buffer.
    /// * DWORD nDefaultTimeOut:
    ///   The default time-out value, in milliseconds, if the
    ///   WaitNamedPipe function specifies NMPWAIT_USE_DEFAULT_WAIT.
    /// * *LPSECURITY_ATTRIBUTES lpSecurityAttributes:
    ///   A pointer to a SECURITY_ATTRIBUTES structure that specifies
    ///   a security descriptor for the new named pipe and determines
    ///   whether child processes can inherit the returned handle.
    fn CreateNamedPipeW(
        lpName: *const u16,
        dwOpenMode: u32,
        dwPipeMode: u32,
        nMaxInstances: u32,
        nOutBufferSize: u32,
        nInBufferSize: u32,
        nDefaultTimeOut: u32,
        lpSecurityAttributes: *const SECURITY_ATTRIBUTES,
    ) -> *mut c_void;

    /// * HANDLE hNamedPipe:
    ///   The name of the named pipe.
    /// * LPOVERLAPPED lpOverlapped:
    ///   A pointer to an OVERLAPPED structure.
    ///   If hNamedPipe was opened with FILE_FLAG_OVERLAPPED,
    ///   the lpOverlapped parameter must not be NULL. It must
    ///   point to a valid OVERLAPPED structure. If hNamedPipe
    ///   was opened with FILE_FLAG_OVERLAPPED and lpOverlapped
    ///   is NULL, the function can incorrectly report that the
    ///   connect operation is complete.
    fn ConnectNamedPipe(hNamedPipe: *mut c_void, lpOverlapped: *mut OVERLAPPED) -> bool;

    /// * HANDLE hNamedPipe:
    ///   A handle to an instance of a named pipe.
    fn DisconnectNamedPipe(hNamedPipe: *mut c_void) -> bool;

    /// * LPCWSTR lpNamedPipeName:
    ///   The name of the named pipe.
    /// * DWORD nTimeOut:
    ///   The number of milliseconds that the function will wait
    ///   for an instance of the named pipe to be available.
    fn WaitNamedPipeW(lpNamedPipeName: *const u16, nTimeOut: u32) -> bool;

    /// * HANDLE hObject:
    ///  A handle to the open file.
    fn FlushFileBuffers(hFile: *mut c_void) -> bool;

    /// * HANDLE hObject:
    ///  A valid handle to an open object.
    fn CloseHandle(hObject: *mut c_void) -> bool;
}

#[repr(C)]
#[derive(Clone, Copy)]
struct OVERLAPPED {
    pub Internal: usize,
    pub InternalHigh: usize,
    pub Anonymous: OVERLAPPED_0,
    pub hEvent: *mut c_void,
}

#[repr(C)]
#[derive(Clone, Copy)]
union OVERLAPPED_0 {
    pub Anonymous: OVERLAPPED_0_0,
    pub Pointer: *mut c_void,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct OVERLAPPED_0_0 {
    pub Offset: u32,
    pub OffsetHigh: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct SECURITY_ATTRIBUTES {
    pub nLength: u32,
    pub lpSecurityDescriptor: *mut c_void,
    pub bInheritHandle: bool,
}

#[derive(Debug)]
pub struct PipeStream {
    server_half: bool,
    handle: Handle,
}

impl PipeStream {
    fn create_pipe(path: &Path) -> io::Result<*mut c_void> {
        let mut os_str: OsString = path.as_os_str().into();
        os_str.push("\x00");

        let u16_slice = os_str.encode_wide().collect::<Vec<u16>>();

        let _ = unsafe { WaitNamedPipeW(u16_slice.as_ptr(), 0) };

        let handle = unsafe {
            CreateFileW(
                u16_slice.as_ptr(),
                GENERIC_READ | GENERIC_WRITE,
                0,
                core::ptr::null_mut(),
                OPEN_EXISTING,
                FILE_ATTRIBUTE_NORMAL,
                core::ptr::null_mut(),
            )
        };

        if handle == INVALID_HANDLE_VALUE {
            Err(io::Error::last_os_error())
        } else {
            Ok(handle)
        }
    }

    pub fn connect<P: AsRef<Path>>(path: P) -> io::Result<PipeStream> {
        let handle = PipeStream::create_pipe(path.as_ref())?;

        Ok(PipeStream {
            handle: Handle { inner: handle },
            server_half: false,
        })
    }
}

impl crate::Conn for PipeStream {}

impl Drop for PipeStream {
    fn drop(&mut self) {
        let _ = unsafe { FlushFileBuffers(self.handle.inner) };

        if self.server_half {
            let _ = unsafe { DisconnectNamedPipe(self.handle.inner) };
        }
    }
}

impl Read for PipeStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let mut bytes_read = 0;

        let ok = unsafe {
            ReadFile(
                self.handle.inner,
                buf.as_mut_ptr(),
                buf.len() as u32,
                &mut bytes_read,
                core::ptr::null_mut(),
            )
        };

        if ok {
            Ok(bytes_read as usize)
        } else {
            let last_err = io::Error::last_os_error();

            if last_err
                .raw_os_error()
                .is_some_and(|code| code as u32 == ERROR_PIPE_NOT_CONNECTED)
            {
                return Ok(0);
            }

            Err(last_err)
        }
    }
}

impl Write for PipeStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let mut bytes_written = 0;

        let ok = unsafe {
            WriteFile(
                self.handle.inner,
                buf.as_ptr(),
                buf.len() as u32,
                &mut bytes_written,
                core::ptr::null_mut(),
            )
        };

        if ok {
            Ok(bytes_written as usize)
        } else {
            Err(io::Error::last_os_error())
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        let ok = unsafe { FlushFileBuffers(self.handle.inner) };

        if ok {
            Ok(())
        } else {
            Err(io::Error::last_os_error())
        }
    }
}

impl AsRawHandle for PipeStream {
    fn as_raw_handle(&self) -> RawHandle {
        self.handle.inner
    }
}

impl IntoRawHandle for PipeStream {
    fn into_raw_handle(self) -> RawHandle {
        self.handle.inner
    }
}

impl FromRawHandle for PipeStream {
    unsafe fn from_raw_handle(handle: RawHandle) -> Self {
        PipeStream {
            handle: Handle { inner: handle },
            server_half: false,
        }
    }
}

unsafe impl Send for PipeStream {}

#[derive(Debug)]
pub struct PipeListener {
    path_u16: Vec<u16>,
    next_pipe: Handle,
}

impl PipeListener {
    pub fn bind(path: &Path) -> io::Result<Self> {
        let mut os_str: OsString = path.as_os_str().into();
        os_str.push("\x00");

        let path_u16 = os_str.encode_wide().collect::<Vec<u16>>();

        let handle = PipeListener::create_pipe(&path_u16, true)?;

        Ok(PipeListener {
            path_u16: path_u16,
            next_pipe: handle,
        })
    }

    fn create_pipe(path_u16: &Vec<u16>, first: bool) -> io::Result<Handle> {
        let mut access_flags = PIPE_ACCESS_DUPLEX;

        if first {
            access_flags |= FILE_FLAG_FIRST_PIPE_INSTANCE;
        }

        let handle = unsafe {
            CreateNamedPipeW(
                path_u16.as_ptr(),
                access_flags,
                PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT,
                PIPE_UNLIMITED_INSTANCES,
                65536,
                65536,
                50,
                core::ptr::null_mut(),
            )
        };

        if handle != INVALID_HANDLE_VALUE {
            Ok(Handle { inner: handle })
        } else {
            Err(io::Error::last_os_error())
        }
    }

    fn connect_pipe(handle: &Handle) -> io::Result<()> {
        let ok = unsafe { ConnectNamedPipe(handle.inner, core::ptr::null_mut()) };

        if ok {
            Ok(())
        } else {
            let last_err = io::Error::last_os_error();

            if last_err
                .raw_os_error()
                .is_some_and(|code| code as u32 == ERROR_PIPE_CONNECTED)
            {
                return Ok(());
            }

            Err(last_err)
        }
    }

    pub fn accept(&mut self) -> io::Result<PipeStream> {
        let handle = core::mem::replace(
            &mut self.next_pipe,
            PipeListener::create_pipe(&self.path_u16, false)?,
        );

        PipeListener::connect_pipe(&handle)?;

        Ok(PipeStream {
            handle: handle,
            server_half: true,
        })
    }
}

impl crate::Listener for PipeListener {
    fn accept(&mut self) -> std::io::Result<Box<dyn crate::Conn>> {
        let stream = self.accept()?;

        Ok(Box::new(stream))
    }
}

#[derive(Debug)]
struct Handle {
    inner: *mut c_void,
}

impl Drop for Handle {
    fn drop(&mut self) {
        let _ = unsafe { CloseHandle(self.inner) };
    }
}

unsafe impl Sync for Handle {}
unsafe impl Send for Handle {}
