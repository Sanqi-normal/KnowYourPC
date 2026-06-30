use std::cmp::Ordering;
use std::time::Duration;

use sysinfo::{ProcessesToUpdate, System};
use tauri::Emitter;

use crate::models::*;

pub fn start_perf_monitor(app_handle: tauri::AppHandle) {
    std::thread::spawn(move || {
        let mut sys = System::new();

        sys.refresh_cpu_all();
        sys.refresh_memory();

        let mut first = true;

        loop {
            std::thread::sleep(Duration::from_secs(1));

            sys.refresh_cpu_all();
            sys.refresh_memory();

            let mem_used = sys.used_memory() as f64;
            let mem_total = sys.total_memory() as f64;

            sys.refresh_processes(ProcessesToUpdate::All, false);
            let mut processes: Vec<ProcessInfo> = sys
                .processes()
                .iter()
                .filter(|(_, p)| {
                    let name = p.name().to_string_lossy();
                    !name.is_empty()
                })
                .map(|(pid, p)| ProcessInfo {
                    pid: pid.as_u32(),
                    name: p.name().to_string_lossy().to_string(),
                    cpu_percent: p.cpu_usage(),
                    memory_mb: p.memory() as f64 / 1_048_576.0,
                })
                .collect();

            // O(n) top-K instead of O(n log n) full sort
            if processes.len() > 20 {
                let (_, _, _) = processes.select_nth_unstable_by(20, |a, b| {
                    b.cpu_percent
                        .partial_cmp(&a.cpu_percent)
                        .unwrap_or(Ordering::Equal)
                });
                processes.truncate(20);
            }
            processes.sort_by(|a, b| {
                b.cpu_percent
                    .partial_cmp(&a.cpu_percent)
                    .unwrap_or(Ordering::Equal)
            });

            let snapshot = PerfSnapshot {
                cpu_percent: sys.global_cpu_usage(),
                gpu_percent: 0.0,
                memory_used_gb: mem_used / 1_073_741_824.0,
                memory_total_gb: mem_total / 1_073_741_824.0,
                memory_percent: if mem_total > 0.0 {
                    ((mem_used / mem_total) * 100.0) as f32
                } else {
                    0.0
                },
                disk_read_mbps: 0.0,
                disk_write_mbps: 0.0,
                net_recv_kbps: 0.0,
                net_sent_kbps: 0.0,
                top_processes: processes,
            };

            if first {
                first = false;
                continue;
            }

            let _ = app_handle.emit("perf-update", &snapshot);
        }
    });
}
