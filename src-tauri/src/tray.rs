use crate::gpu::GpuData;
use crate::settings::{InlineGpuMode, InlineMetric, Settings, TempUnit};

pub fn format_tray_title(data: &GpuData, settings: &Settings) -> String {
    if data.gpus.is_empty() {
        return "GPU: N/A".to_string();
    }

    match &settings.inline_gpu_mode {
        InlineGpuMode::Single => {
            let idx = settings.selected_gpu_index.unwrap_or(0) as usize;
            let gpu = data.gpus.get(idx).unwrap_or(&data.gpus[0]);
            format_gpu_line(gpu, &settings.inline_metrics, &settings.temp_unit)
        }
        InlineGpuMode::All => {
            if data.gpus.len() == 1 {
                format_gpu_line(&data.gpus[0], &settings.inline_metrics, &settings.temp_unit)
            } else {
                data.gpus
                    .iter()
                    .map(|gpu| {
                        format!(
                            "GPU {}: {}",
                            gpu.index,
                            format_metrics(gpu, &settings.inline_metrics, &settings.temp_unit)
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("  ")
            }
        }
        InlineGpuMode::Aggregate => {
            format_aggregate(&data, &settings.inline_metrics, &settings.temp_unit)
        }
    }
}

fn format_gpu_line(
    gpu: &crate::gpu::GpuInfo,
    metrics: &[InlineMetric],
    temp_unit: &TempUnit,
) -> String {
    format_metrics(gpu, metrics, temp_unit)
}

fn format_metrics(
    gpu: &crate::gpu::GpuInfo,
    metrics: &[InlineMetric],
    temp_unit: &TempUnit,
) -> String {
    metrics
        .iter()
        .map(|m| format_single_metric(m, gpu, temp_unit))
        .collect::<Vec<_>>()
        .join(" | ")
}

fn format_single_metric(
    metric: &InlineMetric,
    gpu: &crate::gpu::GpuInfo,
    temp_unit: &TempUnit,
) -> String {
    match metric {
        InlineMetric::Utilization => format!("GPU: {}%", gpu.utilization_gpu),
        InlineMetric::Temperature => {
            let temp = convert_temp(gpu.temperature, temp_unit);
            format!("Temp: {}°{}", temp, temp_unit_str(temp_unit))
        }
        InlineMetric::FanSpeed => {
            gpu.fan_speed
                .map(|f| format!("Fan: {}%", f))
                .unwrap_or_else(|| "Fan: N/A".to_string())
        }
        InlineMetric::MemoryUsage => {
            format!(
                "Mem: {} / {}",
                format_bytes(gpu.memory_used),
                format_bytes(gpu.memory_total)
            )
        }
        InlineMetric::PowerUsage => format!("Power: {}W", gpu.power_usage),
    }
}

fn format_aggregate(
    data: &GpuData,
    metrics: &[InlineMetric],
    temp_unit: &TempUnit,
) -> String {
    metrics
        .iter()
        .map(|m| match m {
            InlineMetric::Utilization => {
                let avg = data.gpus.iter().map(|g| g.utilization_gpu).sum::<u32>()
                    / data.gpus.len() as u32;
                format!("GPU: {}%", avg)
            }
            InlineMetric::Temperature => {
                let max = data.gpus.iter().map(|g| g.temperature).max().unwrap_or(0);
                format!("Temp: {}°{}", convert_temp(max, temp_unit), temp_unit_str(temp_unit))
            }
            InlineMetric::FanSpeed => {
                let max = data.gpus.iter().filter_map(|g| g.fan_speed).max();
                max.map(|f| format!("Fan: {}%", f))
                    .unwrap_or_else(|| "Fan: N/A".to_string())
            }
            InlineMetric::MemoryUsage => {
                let used: u64 = data.gpus.iter().map(|g| g.memory_used).sum();
                let total: u64 = data.gpus.iter().map(|g| g.memory_total).sum();
                format!("Mem: {} / {}", format_bytes(used), format_bytes(total))
            }
            InlineMetric::PowerUsage => {
                let total: u32 = data.gpus.iter().map(|g| g.power_usage).sum();
                format!("Power: {}W", total)
            }
        })
        .collect::<Vec<_>>()
        .join(" | ")
}

fn format_bytes(bytes: u64) -> String {
    let gb = bytes as f64 / (1024.0 * 1024.0 * 1024.0);
    if gb >= 1.0 {
        format!("{:.1}G", gb)
    } else {
        format!("{}M", bytes / (1024 * 1024))
    }
}

fn convert_temp(celsius: u32, unit: &TempUnit) -> u32 {
    match unit {
        TempUnit::C => celsius,
        TempUnit::F => celsius * 9 / 5 + 32,
    }
}

fn temp_unit_str(unit: &TempUnit) -> &'static str {
    match unit {
        TempUnit::C => "C",
        TempUnit::F => "F",
    }
}
