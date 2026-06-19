use core::sync::atomic::{AtomicU64, Ordering};

pub struct Snapshot {
    pub timer_irqs: u64,
    pub keyboard_irqs: u64,
    pub default_irqs: u64,
    pub exceptions: u64,
    pub keyboard_scancodes: u64,
    pub keyboard_events: u64,
    pub keyboard_dropped_events: u64,
    pub shell_commands: u64,
    pub shell_errors: u64,
    pub shell_empty_commands: u64,
    pub shell_history_recalls: u64,
    pub vga_cell_writes: u64,
    pub vga_clears: u64,
    pub vga_scrolls: u64,
    pub cursor_toggles: u64,
    pub serial_bytes: u64,
    pub bench_runs: u64,
    pub shell_ready_tick: u64,
}

static TIMER_IRQS: AtomicU64 = AtomicU64::new(0);
static KEYBOARD_IRQS: AtomicU64 = AtomicU64::new(0);
static DEFAULT_IRQS: AtomicU64 = AtomicU64::new(0);
static EXCEPTIONS: AtomicU64 = AtomicU64::new(0);
static KEYBOARD_SCANCODES: AtomicU64 = AtomicU64::new(0);
static KEYBOARD_EVENTS: AtomicU64 = AtomicU64::new(0);
static KEYBOARD_DROPPED_EVENTS: AtomicU64 = AtomicU64::new(0);
static SHELL_COMMANDS: AtomicU64 = AtomicU64::new(0);
static SHELL_ERRORS: AtomicU64 = AtomicU64::new(0);
static SHELL_EMPTY_COMMANDS: AtomicU64 = AtomicU64::new(0);
static SHELL_HISTORY_RECALLS: AtomicU64 = AtomicU64::new(0);
static VGA_CELL_WRITES: AtomicU64 = AtomicU64::new(0);
static VGA_CLEARS: AtomicU64 = AtomicU64::new(0);
static VGA_SCROLLS: AtomicU64 = AtomicU64::new(0);
static CURSOR_TOGGLES: AtomicU64 = AtomicU64::new(0);
static SERIAL_BYTES: AtomicU64 = AtomicU64::new(0);
static BENCH_RUNS: AtomicU64 = AtomicU64::new(0);
static SHELL_READY_TICK: AtomicU64 = AtomicU64::new(0);

pub fn inc_timer_irq() {
    TIMER_IRQS.fetch_add(1, Ordering::Relaxed);
}

pub fn inc_keyboard_irq() {
    KEYBOARD_IRQS.fetch_add(1, Ordering::Relaxed);
}

pub fn inc_default_irq() {
    DEFAULT_IRQS.fetch_add(1, Ordering::Relaxed);
}

pub fn inc_exception() {
    EXCEPTIONS.fetch_add(1, Ordering::Relaxed);
}

pub fn inc_keyboard_scancode() {
    KEYBOARD_SCANCODES.fetch_add(1, Ordering::Relaxed);
}

pub fn inc_keyboard_event() {
    KEYBOARD_EVENTS.fetch_add(1, Ordering::Relaxed);
}

pub fn inc_keyboard_dropped_event() {
    KEYBOARD_DROPPED_EVENTS.fetch_add(1, Ordering::Relaxed);
}

pub fn inc_shell_command() {
    SHELL_COMMANDS.fetch_add(1, Ordering::Relaxed);
}

pub fn inc_shell_error() {
    SHELL_ERRORS.fetch_add(1, Ordering::Relaxed);
}

pub fn inc_shell_empty_command() {
    SHELL_EMPTY_COMMANDS.fetch_add(1, Ordering::Relaxed);
}

pub fn inc_shell_history_recall() {
    SHELL_HISTORY_RECALLS.fetch_add(1, Ordering::Relaxed);
}

pub fn inc_vga_cell_write() {
    VGA_CELL_WRITES.fetch_add(1, Ordering::Relaxed);
}

pub fn inc_vga_clear() {
    VGA_CLEARS.fetch_add(1, Ordering::Relaxed);
}

pub fn inc_vga_scroll() {
    VGA_SCROLLS.fetch_add(1, Ordering::Relaxed);
}

pub fn inc_cursor_toggle() {
    CURSOR_TOGGLES.fetch_add(1, Ordering::Relaxed);
}

pub fn inc_serial_byte() {
    SERIAL_BYTES.fetch_add(1, Ordering::Relaxed);
}

pub fn inc_bench_run() {
    BENCH_RUNS.fetch_add(1, Ordering::Relaxed);
}

pub fn mark_shell_ready(tick: u64) {
    SHELL_READY_TICK.store(tick, Ordering::Release);
}

pub fn snapshot() -> Snapshot {
    Snapshot {
        timer_irqs: TIMER_IRQS.load(Ordering::Relaxed),
        keyboard_irqs: KEYBOARD_IRQS.load(Ordering::Relaxed),
        default_irqs: DEFAULT_IRQS.load(Ordering::Relaxed),
        exceptions: EXCEPTIONS.load(Ordering::Relaxed),
        keyboard_scancodes: KEYBOARD_SCANCODES.load(Ordering::Relaxed),
        keyboard_events: KEYBOARD_EVENTS.load(Ordering::Relaxed),
        keyboard_dropped_events: KEYBOARD_DROPPED_EVENTS.load(Ordering::Relaxed),
        shell_commands: SHELL_COMMANDS.load(Ordering::Relaxed),
        shell_errors: SHELL_ERRORS.load(Ordering::Relaxed),
        shell_empty_commands: SHELL_EMPTY_COMMANDS.load(Ordering::Relaxed),
        shell_history_recalls: SHELL_HISTORY_RECALLS.load(Ordering::Relaxed),
        vga_cell_writes: VGA_CELL_WRITES.load(Ordering::Relaxed),
        vga_clears: VGA_CLEARS.load(Ordering::Relaxed),
        vga_scrolls: VGA_SCROLLS.load(Ordering::Relaxed),
        cursor_toggles: CURSOR_TOGGLES.load(Ordering::Relaxed),
        serial_bytes: SERIAL_BYTES.load(Ordering::Relaxed),
        bench_runs: BENCH_RUNS.load(Ordering::Relaxed),
        shell_ready_tick: SHELL_READY_TICK.load(Ordering::Acquire),
    }
}
