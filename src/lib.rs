#[macro_use]
extern crate failure;
extern crate winapi;
#[macro_use]
extern crate log;

use std::thread;
use std::time::{Duration, Instant};

pub use errors::{Error, HcbResult};
pub use procext::{MonitoredProcess, MonitoredThread};
use win::Process;

pub mod errors;
pub mod procext;
pub mod win;

/// Returns a handle to the Rocket League process.
pub fn rl_process() -> HcbResult<Process> {
    Process::all()?
        .find(|p| {
            p.name()
                .map(|name| name == "RocketLeague.exe")
                .unwrap_or(false)
        })
        .ok_or(Error::NoProcess)
}

fn wait_for_three_threads(
    process: &mut MonitoredProcess,
    poll_interval: Duration,
) -> HcbResult<[u32; 3]> {
    loop {
        if let Some(active_threads) = process.thread_ids_by_activity().get(0..3) {
            return Ok([active_threads[0], active_threads[1], active_threads[2]]);
        }
        thread::sleep(poll_interval);
        process.update()?;
    }
}

fn top_three_ideal(ids: &[u32; 3], process: &mut MonitoredProcess) -> HcbResult<[u32; 3]> {
    let mut get_ideal = |i| {
        process
            .threads_mut()
            .get_mut(&ids[i])
            .unwrap()
            .thread()
            .ideal_processor()
    };
    Ok([get_ideal(0)?, get_ideal(1)?, get_ideal(2)?])
}

fn set_top_three_ideal(ids: &[u32; 3], process: &mut MonitoredProcess) -> HcbResult<()> {
    println!("{:?}", ids);
    let mut core_num = 0;
    for core in (0u32..3) {
        core_num = ((core + 1) * 2) - 1;
        let s: String = core_num.to_string();
        println!("{}", s);
        process
            .threads_mut()
            .get_mut(&ids[core as usize])
            .unwrap()
            .thread_mut()
            .set_ideal_processor(core_num)?;
    }
    Ok(())
}

/// Monitors the Rocket League process, assigning its three most active threads to separate cores.
pub fn manage_rl_threads(poll_interval: Duration, settling_period: Duration) -> HcbResult<()> {
    let mut process = rl_process().and_then(|p| MonitoredProcess::new(p))?;
    info!("Process found.");

    // The threads which have had affinity assigned.
    let mut set_top_three: Option<[u32; 3]> = None;
    // The top three threads at the moment of the last poll.
    let mut prev_top_three = wait_for_three_threads(&mut process, poll_interval)?;

    // When the thread order last changed.
    let mut last_changed = Instant::now();
    let changing_soon_fraction = settling_period / 10;
    let changing_soon_period = changing_soon_fraction * 8;
    let mut notified_changing_soon = false;
    // Whether the current top three threads equal the ones with set affinities.
    let mut stable = false;

    loop {
        process.update()?;
        let mut current_top_three = wait_for_three_threads(&mut process, poll_interval)?;
        current_top_three.sort_unstable();
        if prev_top_three != current_top_three {
            prev_top_three = current_top_three;
            last_changed = Instant::now();
            match set_top_three {
                Some(set) if set == prev_top_three => {
                    debug!(
                        "Previously set top three threads returned: {:?}",
                        prev_top_three
                    );
                    stable = true;
                }
                Some(_) | None => {
                    debug!("Top three threads changed: {:?}", prev_top_three);
                    stable = false;
                }
            }
        } else {
            if !stable && !notified_changing_soon && last_changed.elapsed() > changing_soon_period {
                info!(
                    "Threads appear to have settled. Assigning affinities on the next poll if stable after {} seconds.",
                    (changing_soon_fraction * 2).as_secs()
                );
                notified_changing_soon = true;
            }
            if !stable && last_changed.elapsed() > settling_period {
                info!("Assigning thread affinities.");
                set_top_three_ideal(&prev_top_three, &mut process)?;
                set_top_three = Some(prev_top_three);
                stable = true;
                notified_changing_soon = false;
            }
        }
        if stable && &top_three_ideal(&current_top_three, &mut process)? != &[1, 3, 5] {
            info!("Correcting affinities.");
            set_top_three_ideal(&prev_top_three, &mut process)?;
        }
        thread::sleep(poll_interval)
    }
}

//#[cfg(test)]
//mod tests {
//    use super::*;
//
//    #[test]
//    fn finds_rl() {
//        let process = rl_process().unwrap();
//        println!("{:?}", process)
//    }
//}
