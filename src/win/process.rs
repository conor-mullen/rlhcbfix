use std::ffi::OsString;
use std::mem;
use std::os::windows::prelude::*;
use std::path::PathBuf;

use winapi::shared::basetsd::{ULONG64, DWORD_PTR};
use winapi::shared::minwindef::{DWORD, MAX_PATH};
use winapi::um::handleapi::INVALID_HANDLE_VALUE;
use winapi::um::processthreadsapi::{GetExitCodeProcess, GetProcessId, GetThreadId,
                                    GetThreadIdealProcessorEx, OpenProcess, OpenThread,
                                    SetThreadIdealProcessor};
use winapi::um::realtimeapiset::QueryThreadCycleTime;
use winapi::um::tlhelp32::{CreateToolhelp32Snapshot, PROCESSENTRY32, Process32Next,
                           TH32CS_SNAPALL, TH32CS_SNAPTHREAD, THREADENTRY32, Thread32Next};
use winapi::um::winbase::{GetProcessAffinityMask, QueryFullProcessImageNameW,
                          SetThreadAffinityMask};
use winapi::um::winnt::{PROCESSOR_NUMBER, PROCESS_ALL_ACCESS, THREAD_ALL_ACCESS, WCHAR};

use win::{self, Handle, WinResult};

#[derive(Debug)]
pub struct Process {
    handle: Handle,
}

impl Process {
    /// Creates a process handle a PID.
    pub fn from_id(id: u32) -> WinResult<Process> {
        unsafe {
            let handle = OpenProcess(PROCESS_ALL_ACCESS, 0, id);
            if handle.is_null() {
                Err(win::Error::last())
            } else {
                Ok(Process {
                    handle: Handle::new(handle),
                })
            }
        }
    }

    /// Enumerates all running processes.
    pub fn all() -> WinResult<impl Iterator<Item = Process>> {
        unsafe {
            let snap = CreateToolhelp32Snapshot(TH32CS_SNAPALL, 0);
            if snap == INVALID_HANDLE_VALUE {
                Err(win::Error::last())
            } else {
                Ok(ProcessIter {
                    snapshot: Handle::new(snap),
                }.filter_map(Result::ok))
            }
        }
    }

    /// Returns the process's id.
    pub fn id(&self) -> u32 {
        unsafe { GetProcessId(self.handle.as_raw_handle()) }
    }

    /// Returns true if the process is running.
    pub fn running(&self) -> bool {
        unsafe {
            let mut status = 0;
            GetExitCodeProcess(self.handle.as_raw_handle(), &mut status);
            status == 259
        }
    }

    /// Returns the path of the executable of the process.
    pub fn path(&self) -> WinResult<PathBuf> {
        unsafe {
            let mut size = MAX_PATH as u32;
            let mut buffer: [WCHAR; MAX_PATH] = mem::zeroed();
            let ret = QueryFullProcessImageNameW(
                self.handle.as_raw_handle(),
                0,
                buffer.as_mut_ptr(),
                &mut size,
            );
            if ret == 0 {
                Err(win::Error::last())
            } else {
                Ok(OsString::from_wide(&buffer[0..size as usize]).into())
            }
        }
    }

    /// Returns the unqualified name of the executable of the process.
    pub fn name(&self) -> WinResult<String> {
        Ok(self.path()?
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned())
    }

    /// Returns the affinity mask of the process.
    pub fn affinity_mask(&self) -> WinResult<usize> {
        unsafe {
            let mut process_mask: DWORD_PTR = 0;
            let mut system_mask: DWORD_PTR = 0;
            let ret = GetProcessAffinityMask(
                self.handle.as_raw_handle(),
                &mut process_mask,
                &mut system_mask,
            );
            if ret == 0 {
                Err(win::Error::last())
            } else {
                Ok(process_mask as usize)
            }
        }
    }

    pub fn threads<'a>(&'a self) -> WinResult<impl Iterator<Item = Thread> + 'a> {
        unsafe {
            let snap = CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0);
            if snap == INVALID_HANDLE_VALUE {
                Err(win::Error::last())
            } else {
                Ok(ThreadIter {
                    process: &self,
                    snapshot: Handle::new(snap),
                }.filter_map(Result::ok))
            }
        }
    }

    pub fn thread_ids<'a>(&'a self) -> WinResult<impl Iterator<Item = u32> + 'a> {
        unsafe {
            let snap = CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0);
            if snap == INVALID_HANDLE_VALUE {
                Err(win::Error::last())
            } else {
                Ok(ThreadIdIter {
                    process: &self,
                    snapshot: Handle::new(snap),
                })
            }
        }
    }
}

struct ProcessIter {
    snapshot: Handle,
}

impl Iterator for ProcessIter {
    type Item = WinResult<Process>;

    fn next(&mut self) -> Option<WinResult<Process>> {
        unsafe {
            let mut entry: PROCESSENTRY32 = mem::zeroed();
            entry.dwSize = mem::size_of::<PROCESSENTRY32>() as DWORD;
            let ret = Process32Next(self.snapshot.as_raw_handle(), &mut entry);
            //            if ret == 0 || win::Error::last().code() == 18 {
            if ret == 0 {
                None
            } else {
                Some(Process::from_id(entry.th32ProcessID))
            }
        }
    }
}

#[derive(Debug)]
pub struct Thread {
    handle: Handle,
}

impl Thread {
    /// Creates a thread handle from a thread ID.
    pub fn from_id(id: u32) -> WinResult<Thread> {
        unsafe {
            let handle = OpenThread(THREAD_ALL_ACCESS, 0, id);
            if handle.is_null() {
                Err(win::Error::last())
            } else {
                Ok(Thread {
                    handle: Handle::new(handle),
                })
            }
        }
    }

    pub fn id(&self) -> u32 {
        unsafe { GetThreadId(self.handle.as_raw_handle()) }
    }

    /// Returns the thread's cycle time.
    pub fn cycle_time(&self) -> WinResult<u64> {
        unsafe {
            let mut cycles: ULONG64 = 0;
            let ret = QueryThreadCycleTime(self.handle.as_raw_handle(), &mut cycles);
            if ret == 0 {
                Err(win::Error::last())
            } else {
                Ok(cycles as u64)
            }
        }
    }

    /// Gets the preferred processor for the thread.
    pub fn ideal_processor(&mut self) -> WinResult<u32> {
        unsafe {
            let mut ideal: PROCESSOR_NUMBER = mem::zeroed();
            let ret = GetThreadIdealProcessorEx(self.handle.as_raw_handle(), &mut ideal);
            if ret == 0 {
                Err(win::Error::last())
            } else {
                Ok(ideal.Number as u32)
            }
        }
    }

    /// Sets the preferred processor for the thread.
    /// On success, returns the previous idea processor.
    pub fn set_ideal_processor(&mut self, processor: u32) -> WinResult<u32> {
        unsafe {
            let ret = SetThreadIdealProcessor(self.handle.as_raw_handle(), processor as DWORD);
            if ret == DWORD::max_value() {
                Err(win::Error::last())
            } else {
                Ok(ret)
            }
        }
    }

    /// Sets the affinity of the thread. On success, returns the previous affinity mask.
    ///
    /// A thread affinity mask is a bit vector in which each bit represents a logical processor
    /// that a thread is allowed to run on. A thread affinity mask must be a subset of the process
    /// affinity mask for the containing process of a thread. A thread can only run on the
    /// processors its process can run on. Therefore, the thread affinity mask cannot specify a
    /// 1 bit for a processor when the process affinity mask specifies a 0 bit for that processor.
    ///
    /// Setting an affinity mask for a process or thread can result in threads receiving less
    /// processor time, as the system is restricted from running the threads on certain processors.
    /// In most cases, it is better to let the system select an available processor.
    ///
    /// If the new thread affinity mask does not specify the processor that is currently running
    /// the thread, the thread is rescheduled on one of the allowable processors.
    pub fn set_affinity_mask(&mut self, mask: usize) -> WinResult<usize> {
        unsafe {
            let ret = SetThreadAffinityMask(self.handle.as_raw_handle(), mask as DWORD_PTR);
            if ret == 0 {
                Err(win::Error::last())
            } else {
                Ok(ret)
            }
        }
    }
}

#[derive(Debug)]
struct ThreadIter<'a> {
    process: &'a Process,
    snapshot: Handle,
}

impl<'a> Iterator for ThreadIter<'a> {
    type Item = WinResult<Thread>;

    fn next(&mut self) -> Option<WinResult<Thread>> {
        unsafe {
            loop {
                let mut entry: THREADENTRY32 = mem::zeroed();
                entry.dwSize = mem::size_of::<THREADENTRY32>() as DWORD;
                let ret = Thread32Next(self.snapshot.as_raw_handle(), &mut entry);
                if ret == 0 {
                    return None;
                } else {
                    if entry.th32OwnerProcessID == self.process.id() {
                        return Some(Thread::from_id(entry.th32ThreadID));
                    }
                }
            }
        }
    }
}

#[derive(Debug)]
struct ThreadIdIter<'a> {
    process: &'a Process,
    snapshot: Handle,
}

impl<'a> Iterator for ThreadIdIter<'a> {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        unsafe {
            loop {
                let mut entry: THREADENTRY32 = mem::zeroed();
                entry.dwSize = mem::size_of::<THREADENTRY32>() as DWORD;
                let ret = Thread32Next(self.snapshot.as_raw_handle(), &mut entry);
                if ret == 0 {
                    return None;
                } else {
                    if entry.th32OwnerProcessID == self.process.id() {
                        return Some(entry.th32ThreadID);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enumerates_processes() {
        let procs: Vec<_> = Process::all().unwrap().collect();
        assert_eq!(procs.is_empty(), false);
        println!("{:?}", procs);
    }

    #[test]
    fn accesses_process_names() {
        let names: Vec<_> = Process::all()
            .unwrap()
            .filter_map(|p| p.name().ok())
            .collect();
        assert_eq!(names.is_empty(), false);
        println!("{:?}", names);
    }

    #[test]
    fn enumerates_threads() {
        let process = Process::all().unwrap().nth(0).unwrap();
        let threads: Vec<_> = process.threads().unwrap().collect();
        assert_eq!(threads.is_empty(), false);
        println!("{:?}", threads);
    }
}
