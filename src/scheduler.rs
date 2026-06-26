use crate::{interrupts, serial};
use core::cell::UnsafeCell;
use x86_64::instructions::interrupts as cpu_interrupts;

pub const QUEUE_CAPACITY: usize = 8;

#[derive(Clone, Copy)]
pub struct Snapshot {
    pub initialized: bool,
    pub queue_capacity: u64,
    pub queued_tasks: u64,
    pub current_task: u64,
    pub last_task: u64,
    pub context_switches: u64,
    pub cooperative_yields: u64,
    pub timer_ticks: u64,
    pub accounted_ticks: u64,
    pub failed_enqueues: u64,
    pub last_switch_tick: u64,
}

struct Scheduler {
    initialized: bool,
    queue: [u64; QUEUE_CAPACITY],
    head: usize,
    len: usize,
    current_task: u64,
    last_task: u64,
    context_switches: u64,
    cooperative_yields: u64,
    timer_ticks: u64,
    accounted_ticks: u64,
    failed_enqueues: u64,
    last_switch_tick: u64,
}

impl Scheduler {
    const fn new() -> Self {
        Self {
            initialized: false,
            queue: [0; QUEUE_CAPACITY],
            head: 0,
            len: 0,
            current_task: 0,
            last_task: 0,
            context_switches: 0,
            cooperative_yields: 0,
            timer_ticks: 0,
            accounted_ticks: 0,
            failed_enqueues: 0,
            last_switch_tick: 0,
        }
    }

    fn init(&mut self) {
        if self.initialized {
            return;
        }

        self.queue = [0; QUEUE_CAPACITY];
        self.head = 0;
        self.len = 0;
        self.current_task = 0;
        self.last_task = 0;
        self.context_switches = 0;
        self.cooperative_yields = 0;
        self.timer_ticks = 0;
        self.accounted_ticks = 0;
        self.failed_enqueues = 0;
        self.last_switch_tick = interrupts::ticks();
        self.initialized = true;

        serial::log("sched", "scheduler ready");
        serial::log_u64("sched", "queue capacity", QUEUE_CAPACITY as u64);
    }

    fn snapshot(&self) -> Snapshot {
        Snapshot {
            initialized: self.initialized,
            queue_capacity: QUEUE_CAPACITY as u64,
            queued_tasks: self.len as u64,
            current_task: self.current_task,
            last_task: self.last_task,
            context_switches: self.context_switches,
            cooperative_yields: self.cooperative_yields,
            timer_ticks: self.timer_ticks,
            accounted_ticks: self.accounted_ticks,
            failed_enqueues: self.failed_enqueues,
            last_switch_tick: self.last_switch_tick,
        }
    }

    fn enqueue(&mut self, task_id: u64) -> bool {
        if task_id == 0 {
            self.failed_enqueues = self.failed_enqueues.saturating_add(1);
            return false;
        }

        if self.contains(task_id) {
            return true;
        }

        if self.len >= QUEUE_CAPACITY {
            self.failed_enqueues = self.failed_enqueues.saturating_add(1);
            serial::log_u64("sched", "queue full for task", task_id);
            return false;
        }

        let tail = (self.head + self.len) % QUEUE_CAPACITY;
        self.queue[tail] = task_id;
        self.len += 1;
        serial::log_u64("sched", "task queued", task_id);

        true
    }

    fn begin_task(&mut self, task_id: u64) {
        self.remove(task_id);
        self.current_task = task_id;
        self.last_task = task_id;
        self.context_switches = self.context_switches.saturating_add(1);
        self.last_switch_tick = interrupts::ticks();
        serial::log_u64("sched", "context switch", task_id);
    }

    fn finish_task(&mut self, task_id: u64) {
        if self.current_task == task_id {
            self.refresh_switch_tick();
            self.current_task = 0;
        }

        self.remove(task_id);
        self.last_task = task_id;
        serial::log_u64("sched", "task complete", task_id);
    }

    fn yield_current(&mut self) -> u64 {
        self.cooperative_yields = self.cooperative_yields.saturating_add(1);

        if self.current_task == 0 {
            serial::log("sched", "yield without current task");
            return 0;
        }

        self.refresh_switch_tick();
        serial::log_u64("sched", "cooperative yield", self.current_task);

        self.current_task
    }

    fn on_timer_tick(&mut self) {
        self.timer_ticks = self.timer_ticks.saturating_add(1);

        if self.current_task != 0 {
            self.accounted_ticks = self.accounted_ticks.saturating_add(1);
        }
    }

    fn selftest(&self) -> bool {
        self.initialized
            && self.len <= QUEUE_CAPACITY
            && self.queue_capacity_consistent()
            && !self.queue_has_duplicates()
    }

    fn contains(&self, task_id: u64) -> bool {
        for offset in 0..self.len {
            let index = (self.head + offset) % QUEUE_CAPACITY;
            if self.queue[index] == task_id {
                return true;
            }
        }

        false
    }

    fn remove(&mut self, task_id: u64) -> bool {
        for offset in 0..self.len {
            let index = (self.head + offset) % QUEUE_CAPACITY;
            if self.queue[index] == task_id {
                self.remove_at(offset);
                return true;
            }
        }

        false
    }

    fn remove_at(&mut self, offset: usize) {
        if offset >= self.len {
            return;
        }

        for shift in offset..(self.len - 1) {
            let from = (self.head + shift + 1) % QUEUE_CAPACITY;
            let to = (self.head + shift) % QUEUE_CAPACITY;
            self.queue[to] = self.queue[from];
        }

        let tail = (self.head + self.len - 1) % QUEUE_CAPACITY;
        self.queue[tail] = 0;
        self.len -= 1;

        if self.len == 0 {
            self.head = 0;
        }
    }

    fn refresh_switch_tick(&mut self) {
        self.last_switch_tick = interrupts::ticks();
    }

    fn queue_has_duplicates(&self) -> bool {
        for left in 0..self.len {
            let left_index = (self.head + left) % QUEUE_CAPACITY;
            let left_id = self.queue[left_index];
            if left_id == 0 {
                return true;
            }

            for right in (left + 1)..self.len {
                let right_index = (self.head + right) % QUEUE_CAPACITY;
                if self.queue[right_index] == left_id {
                    return true;
                }
            }
        }

        false
    }

    fn queue_capacity_consistent(&self) -> bool {
        self.len <= QUEUE_CAPACITY && self.head < QUEUE_CAPACITY
    }
}

struct SchedulerStore {
    value: UnsafeCell<Scheduler>,
}

unsafe impl Sync for SchedulerStore {}

static SCHEDULER: SchedulerStore = SchedulerStore {
    value: UnsafeCell::new(Scheduler::new()),
};

pub fn init() -> Snapshot {
    cpu_interrupts::without_interrupts(|| scheduler_mut().init());
    snapshot()
}

pub fn snapshot() -> Snapshot {
    let mut snapshot = Snapshot {
        initialized: false,
        queue_capacity: QUEUE_CAPACITY as u64,
        queued_tasks: 0,
        current_task: 0,
        last_task: 0,
        context_switches: 0,
        cooperative_yields: 0,
        timer_ticks: 0,
        accounted_ticks: 0,
        failed_enqueues: 0,
        last_switch_tick: 0,
    };

    cpu_interrupts::without_interrupts(|| {
        snapshot = scheduler().snapshot();
    });

    snapshot
}

pub fn enqueue_task(task_id: u64) -> bool {
    cpu_interrupts::without_interrupts(|| scheduler_mut().enqueue(task_id))
}

pub fn begin_task(task_id: u64) {
    cpu_interrupts::without_interrupts(|| scheduler_mut().begin_task(task_id));
}

pub fn finish_task(task_id: u64) {
    cpu_interrupts::without_interrupts(|| scheduler_mut().finish_task(task_id));
}

pub fn yield_current() -> u64 {
    cpu_interrupts::without_interrupts(|| scheduler_mut().yield_current())
}

pub fn on_timer_tick() {
    let scheduler = scheduler_mut();
    if scheduler.initialized {
        scheduler.on_timer_tick();
    }
}

pub fn selftest() -> bool {
    cpu_interrupts::without_interrupts(|| scheduler().selftest())
}

fn scheduler() -> &'static Scheduler {
    unsafe { &*SCHEDULER.value.get() }
}

fn scheduler_mut() -> &'static mut Scheduler {
    unsafe { &mut *SCHEDULER.value.get() }
}
