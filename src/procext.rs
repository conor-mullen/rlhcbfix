use std::collections::{HashMap, HashSet, hash_map::Entry};

use win::{Process, Thread};
use {Error, HcbResult};

#[derive(Debug)]
pub struct MonitoredProcess {
    process: Process,
    threads: HashMap<u32, MonitoredThread>,
    thread_ids: HashSet<u32>,
    thread_activity: Vec<u32>,
}

impl MonitoredProcess {
    pub fn new(process: Process) -> HcbResult<MonitoredProcess> {
        let mut mproc = MonitoredProcess {
            process,
            threads: HashMap::new(),
            thread_ids: HashSet::new(),
            thread_activity: Vec::new(),
        };
        mproc.update()?;
        Ok(mproc)
    }

    pub fn process(&self) -> &Process {
        &self.process
    }

    pub fn process_mut(&mut self) -> &mut Process {
        &mut self.process
    }

    pub fn threads(&self) -> &HashMap<u32, MonitoredThread> {
        &self.threads
    }

    pub fn threads_mut(&mut self) -> &mut HashMap<u32, MonitoredThread> {
        &mut self.threads
    }

    pub fn thread_ids_by_activity(&self) -> &[u32] {
        &self.thread_activity
    }

    pub fn update(&mut self) -> HcbResult<()> {
        self.thread_ids.clear();
        self.thread_activity.clear();
        if !self.process.running() {
            self.threads.clear();
            return Err(Error::NoProcess);
        }
        for thread_id in self.process.thread_ids()? {
            let thread_updated = MonitoredProcess::get_or_add_thread(self.threads.entry(thread_id))
                .and_then(|thread| thread.update());
            if let Ok(_) = thread_updated {
                self.thread_ids.insert(thread_id);
                self.thread_activity.push(thread_id);
            }
        }
        let thread_ids = &self.thread_ids;
        self.threads.retain(|id, _| thread_ids.contains(id));
        let threads = &self.threads;
        self.thread_activity
            .sort_unstable_by(|lt_id, rt_id| threads[rt_id].delta().cmp(&threads[lt_id].delta()));
        Ok(())
    }

    fn get_or_add_thread<'a>(
        entry: Entry<'a, u32, MonitoredThread>,
    ) -> HcbResult<&'a mut MonitoredThread> {
        let id = *entry.key();
        Ok(entry.or_insert(MonitoredThread::new(Thread::from_id(id)?)?))
    }
}

#[derive(Debug)]
pub struct MonitoredThread {
    thread: Thread,
    cycles: u64,
    delta: u64,
}

impl MonitoredThread {
    pub fn new(thread: Thread) -> HcbResult<MonitoredThread> {
        let cycles = thread.cycle_time()?;
        Ok(MonitoredThread {
            thread,
            cycles,
            delta: 0,
        })
    }

    pub fn update(&mut self) -> HcbResult<u64> {
        let new_cycles = self.thread.cycle_time()?;
        self.delta = new_cycles - self.cycles;
        self.cycles = new_cycles;
        Ok(self.delta)
    }

    pub fn thread(&self) -> &Thread {
        &self.thread
    }

    pub fn thread_mut(&mut self) -> &mut Thread {
        &mut self.thread
    }

    pub fn delta(&self) -> u64 {
        self.delta
    }
}
