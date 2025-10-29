use libc::{c_int, c_void};
use std::fs::File;
use std::io::{self, Read, Write};
use std::mem::{self, MaybeUninit};
use std::os::unix::fs::FileExt;
use std::os::unix::io::{FromRawFd, RawFd};
use std::os::unix::process::CommandExt;
use std::process::{Child, Command};

const REPRL_CRFD: RawFd = 100; // child reads control
const REPRL_CWFD: RawFd = 101; // child writes status
const REPRL_DRFD: RawFd = 102; // child reads program bytes
const REPRL_DWFD: RawFd = 103; // child writes fuzzer prints / logs

#[derive(Debug)]
pub struct FuzzTarget {
    pub child: Child,
    data_tx: File,
    data_rx: File,
    ctrl_tx: File,
    ctrl_rx: File
}



fn make_inheritable(fd: RawFd) -> io::Result<()> {
    unsafe {
        let flags = libc::fcntl(fd, libc::F_GETFD);
        if flags == -1 {
            return Err(io::Error::last_os_error());
        }
        if libc::fcntl(fd, libc::F_SETFD, flags & !libc::FD_CLOEXEC) == -1 {
            return Err(io::Error::last_os_error());
        }
    }
    Ok(())
}

impl FuzzTarget {
    pub fn spawn(argv: &[&str], shm_id: &str) -> anyhow::Result<FuzzTarget> {
        let (cr_read, cr_write) = pipe()?;
        let (cw_read, cw_write) = pipe()?;
        let (dr_read, dr_write) = pipe()?;
        let (dw_read, dw_write) = pipe()?;

        let mut cmd = Command::new(argv[0]);
        cmd.args(&argv[1..])
            .env("REPRL_MODE", "1")
            .env("SHM_ID", shm_id)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());

        unsafe {
            cmd.pre_exec(move || {
                let dup = |fd: RawFd, target: RawFd| -> io::Result<()> {
                    if libc::dup2(fd, target) == -1 {
                        return Err(io::Error::last_os_error());
                    }
                    libc::close(fd);
                    make_inheritable(target)?;
                    Ok(())
                };
                dup(cr_read, REPRL_CRFD)?;
                dup(cw_write, REPRL_CWFD)?;
                dup(dr_read, REPRL_DRFD)?;
                dup(dw_write, REPRL_DWFD)?;
                Ok(())
            });
        }

        let child = cmd.spawn()?;

        // Parent doesn’t need the child ends anymore.
        unsafe {
            libc::close(cr_read);
            libc::close(cw_write);
            libc::close(dr_read);
            libc::close(dw_write);
        }

        Ok(Self {
            child,
            ctrl_tx: unsafe { File::from_raw_fd(cr_write) },
            ctrl_rx: unsafe { File::from_raw_fd(cw_read) },
            data_tx: unsafe { File::from_raw_fd(dr_write) },
            data_rx: unsafe { File::from_raw_fd(dw_read) },
        })
    }

    pub fn handshake(&mut self) -> io::Result<()> {
        let mut buf = [0u8; 4];
        self.ctrl_rx.read_exact(&mut buf)?;
        if &buf != b"HELO" {
            return Err(io::Error::new(io::ErrorKind::Other, "bad HELO from child"));
        }
        self.ctrl_tx.write_all(b"HELO")?;
        self.ctrl_tx.flush()
    }

    pub fn execute(&mut self, script: &[u8]) -> io::Result<i32> {
        self.ctrl_tx.write_all(b"exec")?;
        self.ctrl_tx
            .write_all(&(script.len() as u64).to_ne_bytes())?;
        
        self.data_tx.write_all(script)?;
        self.data_tx.flush()?;

        let mut status = [0u8; 4];
        self.ctrl_rx.read_exact(&mut status)?;
        let code = i32::from_ne_bytes(status);
        Ok(code >> 8) // exit status is stored in the upper byte
    }

}

fn pipe() -> io::Result<(RawFd, RawFd)> {
    let mut fds = [0; 2];
    if unsafe { libc::pipe(fds.as_mut_ptr()) } == -1 {
        return Err(io::Error::last_os_error());
    }
    Ok((fds[0], fds[1]))
}