use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(default)]
pub struct Settings {
    pub enabled: bool,
    pub cpu_percent: u32,
    pub ram_percent: u32,
    pub upload_percent: u32,
    pub upload_mbps: u32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            enabled: false,
            cpu_percent: 50,
            ram_percent: 50,
            upload_percent: 50,
            upload_mbps: 20,
        }
    }
}

impl Settings {
    pub fn normalize(&mut self) {
        self.cpu_percent = self.cpu_percent.clamp(1, 100);
        self.ram_percent = self.ram_percent.clamp(1, 100);
        self.upload_percent = self.upload_percent.clamp(1, 100);
        self.upload_mbps = self.upload_mbps.clamp(1, 100_000);
    }

    pub fn upload_bytes_per_second(&self) -> u64 {
        u64::from(self.upload_mbps) * 1_000_000 / 8 * u64::from(self.upload_percent) / 100
    }
}

pub struct Limiter {
    #[cfg(windows)]
    job: Result<windows::Job, String>,
    pub status: Vec<String>,
}

impl Limiter {
    pub fn new(settings: &Settings) -> Self {
        let mut limiter = Self {
            #[cfg(windows)]
            job: windows::Job::attach_current_process(),
            status: Vec::new(),
        };
        limiter.apply(settings);
        limiter
    }

    pub fn apply(&mut self, settings: &Settings) {
        self.status.clear();
        #[cfg(windows)]
        match &self.job {
            Ok(job) => {
                let mut settings = settings.clone();
                settings.normalize();
                for (label, result) in [
                    (format!("CPU {}%", settings.cpu_percent), job.cpu(&settings)),
                    (
                        format!("RAM commit {}% of physical RAM", settings.ram_percent),
                        job.memory(&settings),
                    ),
                    (
                        format!(
                            "Network upload {:.2} Mbps",
                            settings.upload_bytes_per_second() as f64 * 8.0 / 1_000_000.0
                        ),
                        job.network(&settings),
                    ),
                ] {
                    self.status.push(match result {
                        Ok(()) if settings.enabled => format!("{label} budget active"),
                        Ok(()) => format!("{label} budget off"),
                        Err(error) => format!(
                            "{label} update failed. {error}. Previous limit may still apply."
                        ),
                    });
                }
            }
            Err(error) => self
                .status
                .push(format!("Resource limiter unavailable. {error}")),
        }
        #[cfg(not(windows))]
        {
            let _ = settings;
            self.status
                .push("Resource enforcement requires Windows.".into());
        }
    }
}

impl Drop for Limiter {
    fn drop(&mut self) {
        // Release budgets before a UI restart spawns the replacement app in this job.
        self.apply(&Settings::default());
    }
}

#[cfg(windows)]
mod windows {
    use super::Settings;
    use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
    use windows_sys::Win32::System::JobObjects::*;
    use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
    use windows_sys::Win32::System::Threading::GetCurrentProcess;

    pub(super) struct Job(OwnedHandle);

    impl Job {
        fn create() -> Result<Self, String> {
            // Unnamed, non-inheritable handle. Children inherit membership through Windows.
            let handle = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
            if handle.is_null() {
                return Err(std::io::Error::last_os_error().to_string());
            }
            Ok(Self(unsafe { OwnedHandle::from_raw_handle(handle) }))
        }

        pub(super) fn attach_current_process() -> Result<Self, String> {
            let job = Self::create()?;
            if unsafe { AssignProcessToJobObject(job.0.as_raw_handle(), GetCurrentProcess()) } == 0
            {
                return Err(std::io::Error::last_os_error().to_string());
            }
            Ok(job)
        }

        fn set<T>(&self, class: JOBOBJECTINFOCLASS, value: &T) -> Result<(), String> {
            if unsafe {
                SetInformationJobObject(
                    self.0.as_raw_handle(),
                    class,
                    std::ptr::from_ref(value).cast(),
                    std::mem::size_of::<T>() as u32,
                )
            } == 0
            {
                return Err(std::io::Error::last_os_error().to_string());
            }
            Ok(())
        }

        pub(super) fn cpu(&self, settings: &Settings) -> Result<(), String> {
            let value = JOBOBJECT_CPU_RATE_CONTROL_INFORMATION {
                ControlFlags: if settings.enabled {
                    JOB_OBJECT_CPU_RATE_CONTROL_ENABLE | JOB_OBJECT_CPU_RATE_CONTROL_HARD_CAP
                } else {
                    0
                },
                Anonymous: JOBOBJECT_CPU_RATE_CONTROL_INFORMATION_0 {
                    CpuRate: settings.cpu_percent * 100,
                },
            };
            self.set(JobObjectCpuRateControlInformation, &value)
        }

        pub(super) fn memory(&self, settings: &Settings) -> Result<(), String> {
            let mut value = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
            if settings.enabled {
                let mut memory = MEMORYSTATUSEX {
                    dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32,
                    ..Default::default()
                };
                if unsafe { GlobalMemoryStatusEx(&mut memory) } == 0 {
                    return Err(std::io::Error::last_os_error().to_string());
                }
                value.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_JOB_MEMORY;
                value.JobMemoryLimit = (memory.ullTotalPhys / 100 * u64::from(settings.ram_percent))
                    .min(usize::MAX as u64) as usize;
            }
            self.set(JobObjectExtendedLimitInformation, &value)
        }

        pub(super) fn network(&self, settings: &Settings) -> Result<(), String> {
            let value = JOBOBJECT_NET_RATE_CONTROL_INFORMATION {
                MaxBandwidth: settings.upload_bytes_per_second(),
                ControlFlags: if settings.enabled {
                    JOB_OBJECT_NET_RATE_CONTROL_ENABLE | JOB_OBJECT_NET_RATE_CONTROL_MAX_BANDWIDTH
                } else {
                    0
                },
                DscpTag: 0,
            };
            self.set(JobObjectNetRateControlInformation, &value)
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        fn query<T: Default>(job: &Job, class: JOBOBJECTINFOCLASS) -> T {
            let mut value = T::default();
            assert_ne!(
                unsafe {
                    QueryInformationJobObject(
                        job.0.as_raw_handle(),
                        class,
                        std::ptr::from_mut(&mut value).cast(),
                        std::mem::size_of::<T>() as u32,
                        std::ptr::null_mut(),
                    )
                },
                0,
                "{}",
                std::io::Error::last_os_error()
            );
            value
        }

        #[test]
        fn native_limits_apply_update_and_disable() {
            let job = Job::create().unwrap();
            let mut settings = Settings {
                enabled: true,
                ..Default::default()
            };
            for percent in [25, 60] {
                settings.cpu_percent = percent;
                job.cpu(&settings).unwrap();
                job.memory(&settings).unwrap();
                job.network(&settings).unwrap();
                let cpu: JOBOBJECT_CPU_RATE_CONTROL_INFORMATION =
                    query(&job, JobObjectCpuRateControlInformation);
                assert_eq!(unsafe { cpu.Anonymous.CpuRate }, percent * 100);
                assert_ne!(cpu.ControlFlags & JOB_OBJECT_CPU_RATE_CONTROL_HARD_CAP, 0);
                let memory: JOBOBJECT_EXTENDED_LIMIT_INFORMATION =
                    query(&job, JobObjectExtendedLimitInformation);
                assert_ne!(
                    memory.BasicLimitInformation.LimitFlags & JOB_OBJECT_LIMIT_JOB_MEMORY,
                    0
                );
                assert!(memory.JobMemoryLimit > 0);
                let network: JOBOBJECT_NET_RATE_CONTROL_INFORMATION =
                    query(&job, JobObjectNetRateControlInformation);
                assert_eq!(network.MaxBandwidth, settings.upload_bytes_per_second());
            }
            settings.enabled = false;
            job.cpu(&settings).unwrap();
            job.memory(&settings).unwrap();
            job.network(&settings).unwrap();
            let cpu: JOBOBJECT_CPU_RATE_CONTROL_INFORMATION =
                query(&job, JobObjectCpuRateControlInformation);
            assert_eq!(cpu.ControlFlags, 0);
            let memory: JOBOBJECT_EXTENDED_LIMIT_INFORMATION =
                query(&job, JobObjectExtendedLimitInformation);
            assert_eq!(memory.BasicLimitInformation.LimitFlags, 0);
            let network: JOBOBJECT_NET_RATE_CONTROL_INFORMATION =
                query(&job, JobObjectNetRateControlInformation);
            assert_eq!(network.ControlFlags, 0);
        }

        #[test]
        fn descendants_inherit_membership() {
            use std::io::Read;
            use std::process::{Command, Stdio};

            const STAGE: &str = "REDOX_RESOURCE_TEST_STAGE";
            const TEST: &str = "resources::windows::tests::descendants_inherit_membership";
            match std::env::var(STAGE).as_deref() {
                Ok("leaf") => {
                    let mut input = String::new();
                    std::io::stdin().read_to_string(&mut input).unwrap();
                }
                Ok("parent") => {
                    let job = Job::attach_current_process().unwrap();
                    let settings = Settings {
                        enabled: true,
                        ..Default::default()
                    };
                    job.cpu(&settings).unwrap();
                    job.memory(&settings).unwrap();
                    job.network(&settings).unwrap();
                    let mut child = Command::new(std::env::current_exe().unwrap())
                        .args(["--exact", TEST])
                        .env(STAGE, "leaf")
                        .stdin(Stdio::piped())
                        .stdout(Stdio::null())
                        .spawn()
                        .unwrap();
                    let mut member = 0;
                    let success = unsafe {
                        IsProcessInJob(child.as_raw_handle(), job.0.as_raw_handle(), &mut member)
                    };
                    drop(child.stdin.take());
                    let status = child.wait().unwrap();
                    assert_ne!(success, 0);
                    assert_ne!(member, 0);
                    assert!(status.success());
                }
                _ => {
                    let status = Command::new(std::env::current_exe().unwrap())
                        .args(["--exact", TEST, "--nocapture"])
                        .env(STAGE, "parent")
                        .status()
                        .unwrap();
                    assert!(status.success());
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_migrate_normalize_and_convert_bandwidth() {
        let mut settings: Settings = serde_json::from_str("{}").unwrap();
        assert!(!settings.enabled);
        assert_eq!(settings.upload_bytes_per_second(), 1_250_000);
        settings.cpu_percent = 0;
        settings.ram_percent = 200;
        settings.upload_mbps = u32::MAX;
        settings.normalize();
        assert_eq!(settings.cpu_percent, 1);
        assert_eq!(settings.ram_percent, 100);
        assert_eq!(settings.upload_mbps, 100_000);
    }
}
