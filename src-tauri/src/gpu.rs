use nvml_wrapper::Nvml;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct GpuInfo {
    pub index: u32,
    pub name: String,
    pub utilization_gpu: u32,
    pub utilization_memory: u32,
    pub temperature: u32,
    pub fan_speed: Option<u32>,
    pub memory_used: u64,
    pub memory_total: u64,
    pub power_usage: u32,
    pub power_limit: u32,
    pub clock_gpu: u32,
    pub clock_memory: u32,
    pub processes: Vec<GpuProcess>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GpuProcess {
    pub pid: u32,
    pub name: String,
    pub memory_used: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct GpuData {
    pub gpus: Vec<GpuInfo>,
    pub driver_version: String,
    pub cuda_version: Option<String>,
}

pub struct GpuMonitor {
    nvml: Nvml,
}

impl GpuMonitor {
    pub fn new() -> Result<Self, String> {
        let nvml = Nvml::init().map_err(|e| format!("Failed to initialize NVML: {}", e))?;
        Ok(Self { nvml })
    }

    pub fn poll(&self) -> Result<GpuData, String> {
        let device_count = self
            .nvml
            .device_count()
            .map_err(|e| format!("Failed to get device count: {}", e))?;

        let driver_version = self
            .nvml
            .sys_driver_version()
            .map_err(|e| format!("Failed to get driver version: {}", e))?;

        let cuda_version = self.nvml.sys_cuda_driver_version().ok().map(|v| {
            let major = v / 1000;
            let minor = (v % 1000) / 10;
            format!("{}.{}", major, minor)
        });

        let mut gpus = Vec::new();

        for i in 0..device_count {
            let device = self
                .nvml
                .device_by_index(i)
                .map_err(|e| format!("Failed to get device {}: {}", i, e))?;

            let name = device.name().unwrap_or_else(|_| "Unknown GPU".to_string());

            let utilization = device.utilization_rates().ok();
            let utilization_gpu = utilization.as_ref().map(|u| u.gpu).unwrap_or(0);
            let utilization_memory = utilization.as_ref().map(|u| u.memory).unwrap_or(0);

            let temperature = device
                .temperature(nvml_wrapper::enum_wrappers::device::TemperatureSensor::Gpu)
                .unwrap_or(0);

            let fan_speed = device.fan_speed(0).ok();

            let memory_info = device.memory_info().ok();
            let memory_used = memory_info.as_ref().map(|m| m.used).unwrap_or(0);
            let memory_total = memory_info.as_ref().map(|m| m.total).unwrap_or(0);

            let power_usage = device.power_usage().unwrap_or(0); // milliwatts
            let power_limit = device.enforced_power_limit().unwrap_or(0); // milliwatts

            let clock_gpu = device
                .clock_info(nvml_wrapper::enum_wrappers::device::Clock::Graphics)
                .unwrap_or(0);
            let clock_memory = device
                .clock_info(nvml_wrapper::enum_wrappers::device::Clock::Memory)
                .unwrap_or(0);

            let running_processes = device.running_compute_processes().unwrap_or_default();
            let graphics_processes = device.running_graphics_processes().unwrap_or_default();

            let mut processes = Vec::new();
            let mut seen_pids = std::collections::HashSet::new();

            for proc in running_processes.iter().chain(graphics_processes.iter()) {
                if seen_pids.insert(proc.pid) {
                    let proc_name = std::fs::read_to_string(format!("/proc/{}/comm", proc.pid))
                        .map(|s| s.trim().to_string())
                        .unwrap_or_else(|_| format!("PID {}", proc.pid));

                    processes.push(GpuProcess {
                        pid: proc.pid,
                        name: proc_name,
                        memory_used: match proc.used_gpu_memory {
                            nvml_wrapper::enums::device::UsedGpuMemory::Used(bytes) => bytes,
                            _ => 0,
                        },
                    });
                }
            }

            gpus.push(GpuInfo {
                index: i,
                name,
                utilization_gpu,
                utilization_memory,
                temperature,
                fan_speed,
                memory_used,
                memory_total,
                power_usage: power_usage / 1000, // convert to watts
                power_limit: power_limit / 1000,
                clock_gpu,
                clock_memory,
                processes,
            });
        }

        Ok(GpuData {
            gpus,
            driver_version,
            cuda_version,
        })
    }
}
