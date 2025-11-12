use std::fs::File;
use std::io::{self, Read, Write};
use std::os::unix::io::{AsRawFd, FromRawFd, RawFd};
use std::os::unix::process::CommandExt;
use std::process::{Child, Command};
use std::thread;
use std::time::{Duration, Instant};

use crate::profiles::profile::JsEngineProfile;

const REPRL_CRFD: RawFd = 100; // child reads control
const REPRL_CWFD: RawFd = 101; // child writes status
const REPRL_DRFD: RawFd = 102; // child reads program bytes
const REPRL_DWFD: RawFd = 103; // child writes fuzzer prints / logs

#[derive(Debug)]
pub struct FuzzProcess {
    pub child: Child,
    crt_executions: usize,
    max_executions: usize,
    timeout: u64,
    path: String,
    args: Vec<String>,
    shm_id: String,
    data_tx: File,
    data_rx: File,
    ctrl_tx: File,
    ctrl_rx: File,
}

#[derive(Debug, Copy, Clone)]
pub struct ExecutionStatus {
    pub exit_code: i32,
    pub signal: i32,
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

impl FuzzProcess {
    pub fn spawn<T: JsEngineProfile>(profile: &T, shm_id: &str) -> anyhow::Result<FuzzProcess> {
        let path = profile.get_path();
        let args = profile.get_args();
        let timeout = profile.get_timeout();
        Self::spawn_with_details(
            path,
            args,
            shm_id.to_string(),
            timeout,
            profile.get_jobs_per_process(),
        )
    }

    fn spawn_with_details(
        path: String,
        args: Vec<String>,
        shm_id: String,
        timeout: u64,
        max_executions: usize,
    ) -> anyhow::Result<FuzzProcess> {
        let (child, ctrl_tx, ctrl_rx, data_tx, data_rx) =
            Self::launch_process(&path, &args, &shm_id)?;

        Ok(Self {
            child,
            crt_executions: 0,
            max_executions: max_executions,
            timeout,
            path,
            args,
            shm_id,
            ctrl_tx,
            ctrl_rx,
            data_tx,
            data_rx,
        })
    }

    fn launch_process(
        path: &str,
        args: &[String],
        shm_id: &str,
    ) -> io::Result<(Child, File, File, File, File)> {
        let (cr_read, cr_write) = pipe()?;
        let (cw_read, cw_write) = pipe()?;
        let (dr_read, dr_write) = pipe()?;
        let (dw_read, dw_write) = pipe()?;

        let mut cmd = Command::new(path);
        cmd.args(args)
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

        unsafe {
            libc::close(cr_read);
            libc::close(cw_write);
            libc::close(dr_read);
            libc::close(dw_write);
        }

        Ok((
            child,
            unsafe { File::from_raw_fd(cr_write) },
            unsafe { File::from_raw_fd(cw_read) },
            unsafe { File::from_raw_fd(dr_write) },
            unsafe { File::from_raw_fd(dw_read) },
        ))
    }

    pub fn restart(&mut self) -> anyhow::Result<()> {
        let _ = self.child.kill();
        let _ = self.child.wait();

        let (child, ctrl_tx, ctrl_rx, data_tx, data_rx) =
            Self::launch_process(&self.path, &self.args, &self.shm_id)?;

        self.child = child;
        self.ctrl_tx = ctrl_tx;
        self.ctrl_rx = ctrl_rx;
        self.data_tx = data_tx;
        self.data_rx = data_rx;

        Ok(())
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

    pub fn execute(&mut self, script: &[u8]) -> io::Result<ExecutionStatus> {
        if self.crt_executions >= self.max_executions {
            while let Err(e) = self.restart() {
                // return Err(io::Error::new(
                //     io::ErrorKind::Other,
                //     format!("failed to restart process: {}", e),
                // ));
                eprintln!("failed to restart process: {}, retrying...", e);
                thread::sleep(Duration::from_millis(100));
            }
            self.handshake()?;
            self.crt_executions = 0;
        }
        self.ctrl_tx.write_all(b"exec")?;
        self.ctrl_tx
            .write_all(&(script.len() as u64).to_ne_bytes())?;

        self.data_tx.write_all(script)?;
        self.data_tx.flush()?;

        let mut status = [0u8; 4];
        self.read_status_with_timeout(&mut status)?;
        let raw = u32::from_ne_bytes(status);
        let signal = (raw & 0xff) as i32;
        let exit_code = ((raw >> 8) & 0xff) as i32;
        self.crt_executions += 1;
        Ok(ExecutionStatus { exit_code, signal })
    }
}

impl FuzzProcess {
    fn read_status_with_timeout(&mut self, buf: &mut [u8]) -> io::Result<()> {
        if self.timeout == 0 {
            self.ctrl_rx.read_exact(buf)?;
            return Ok(());
        }

        let fd = self.ctrl_rx.as_raw_fd();
        let original_flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
        if original_flags == -1 {
            return Err(io::Error::last_os_error());
        }
        let _restore = FdFlagRestore {
            fd,
            flags: original_flags,
        };
        if unsafe { libc::fcntl(fd, libc::F_SETFL, original_flags | libc::O_NONBLOCK) } == -1 {
            return Err(io::Error::last_os_error());
        }

        let deadline = Instant::now() + Duration::from_millis(self.timeout);
        let mut offset = 0;

        while offset < buf.len() {
            match self.ctrl_rx.read(&mut buf[offset..]) {
                Ok(0) => {
                    return Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "child closed status pipe",
                    ));
                }
                Ok(n) => offset += n,
                Err(ref err) if err.kind() == io::ErrorKind::Interrupted => continue,
                Err(ref err) if err.kind() == io::ErrorKind::WouldBlock => {
                    if Instant::now() >= deadline {
                        // Tear down hung child so it doesn't block future jobs.
                        let _ = self.child.kill();
                        let _ = self.child.wait();
                        return Err(io::Error::new(
                            io::ErrorKind::TimedOut,
                            "execution timed out",
                        ));
                    }
                    thread::sleep(Duration::from_millis(1));
                }
                Err(err) => return Err(err),
            }
        }
        Ok(())
    }
}

fn pipe() -> io::Result<(RawFd, RawFd)> {
    let mut fds = [0; 2];
    if unsafe { libc::pipe(fds.as_mut_ptr()) } == -1 {
        return Err(io::Error::last_os_error());
    }
    Ok((fds[0], fds[1]))
}

struct FdFlagRestore {
    fd: RawFd,
    flags: libc::c_int,
}

impl Drop for FdFlagRestore {
    fn drop(&mut self) {
        unsafe {
            libc::fcntl(self.fd, libc::F_SETFL, self.flags);
        };
    }
}
