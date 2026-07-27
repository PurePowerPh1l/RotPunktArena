//! Simulator transport — synthetic ENQ/NAK + shot frames without hardware.

use super::{Transport, TransportKind};
use crate::protocol::{self, build_synthetic_shot_frame, ACK, ENQ, NAK, STX};
use parking_lot::{Condvar, Mutex};
use std::collections::VecDeque;
use std::io;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Shared control for injecting shots from Tauri commands.
#[derive(Clone)]
pub struct SimulatorControl {
    pub pending_shots: Arc<Mutex<VecDeque<Vec<u8>>>>,
    pub auto_fire: Arc<AtomicBool>,
    pub shot_counter: Arc<AtomicUsize>,
    wake_lock: Arc<Mutex<()>>,
    wake_cvar: Arc<Condvar>,
}

impl Default for SimulatorControl {
    fn default() -> Self {
        Self {
            pending_shots: Arc::new(Mutex::new(VecDeque::new())),
            auto_fire: Arc::new(AtomicBool::new(false)),
            shot_counter: Arc::new(AtomicUsize::new(0)),
            wake_lock: Arc::new(Mutex::new(())),
            wake_cvar: Arc::new(Condvar::new()),
        }
    }
}

impl SimulatorControl {
    pub fn queue_shot(&self, frame: Vec<u8>) {
        self.pending_shots.lock().push_back(frame);
        self.notify();
    }

    #[allow(dead_code)]
    pub fn queue_synthetic(
        &self,
        value_ascii: &str,
        distance_ascii: &str,
        x_ascii: &str,
        y_ascii: &str,
    ) -> Result<(), String> {
        let mut frame = build_synthetic_shot_frame(value_ascii, distance_ascii, x_ascii, y_ascii)?;
        protocol::stamp_frame_nonce(&mut frame);
        self.queue_shot(frame);
        Ok(())
    }

    pub fn set_auto_fire(&self, on: bool) {
        self.auto_fire.store(on, Ordering::SeqCst);
        self.notify();
    }

    pub fn notify(&self) {
        let _g = self.wake_lock.lock();
        self.wake_cvar.notify_all();
    }

    pub fn wait_timeout(&self, timeout: Duration) {
        let mut g = self.wake_lock.lock();
        let _ = self.wake_cvar.wait_for(&mut g, timeout);
    }

    pub fn pending_count(&self) -> usize {
        self.pending_shots.lock().len()
    }
}

pub struct SimulatorTransport {
    name: String,
    open: bool,
    rx: VecDeque<u8>,
    control: SimulatorControl,
    last_auto: Instant,
}

impl SimulatorTransport {
    pub fn new(control: SimulatorControl) -> Self {
        Self {
            name: "simulator".into(),
            open: false,
            rx: VecDeque::new(),
            control,
            last_auto: Instant::now(),
        }
    }

    fn enqueue_bytes(&mut self, data: &[u8]) {
        self.rx.extend(data.iter().copied());
    }

    fn maybe_auto_shot(&mut self) {
        if !self.control.auto_fire.load(Ordering::SeqCst) {
            return;
        }
        if self.last_auto.elapsed() < Duration::from_secs(2) {
            return;
        }
        self.last_auto = Instant::now();
        let n = self.control.shot_counter.fetch_add(1, Ordering::SeqCst) + 1;
        let angle = (n as f64) * 0.7;
        // Spread across calibrated face (ring ~10 … ~2); same scale as aim_coords_to_ascii.
        let r = 200.0 + ((n * 97) % 2100) as f64;
        let x = r * angle.cos();
        let y = r * angle.sin();
        let (value, dist, x_s, y_s) = protocol::aim_coords_to_ascii(x, y);
        if let Ok(mut frame) = build_synthetic_shot_frame(&value, &dist, &x_s, &y_s) {
            protocol::stamp_frame_nonce(&mut frame);
            self.control.queue_shot(frame);
        }
    }
}

impl Transport for SimulatorTransport {
    fn kind(&self) -> TransportKind {
        TransportKind::Simulator
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn open(&mut self) -> io::Result<()> {
        self.open = true;
        self.rx.clear();
        Ok(())
    }

    fn close(&mut self) -> io::Result<()> {
        self.open = false;
        self.rx.clear();
        Ok(())
    }

    fn write_all(&mut self, data: &[u8]) -> io::Result<()> {
        if !self.open {
            return Err(io::Error::new(io::ErrorKind::NotConnected, "simulator closed"));
        }
        for &b in data {
            match b {
                ENQ => {
                    self.maybe_auto_shot();
                    let shot = self.control.pending_shots.lock().pop_front();
                    if let Some(frame) = shot {
                        self.enqueue_bytes(&frame);
                    } else {
                        self.enqueue_bytes(&[NAK]);
                    }
                }
                ACK => {}
                STX | NAK | protocol::DC1 => {}
                _ => {}
            }
        }
        Ok(())
    }

    fn read_timeout(&mut self, buf: &mut [u8], timeout: Duration) -> io::Result<usize> {
        if !self.open {
            return Err(io::Error::new(io::ErrorKind::NotConnected, "simulator closed"));
        }
        if self.rx.is_empty() {
            std::thread::sleep(timeout.min(Duration::from_millis(5)));
            if self.rx.is_empty() {
                return Ok(0);
            }
        }
        let n = buf.len().min(self.rx.len());
        for i in 0..n {
            buf[i] = self.rx.pop_front().unwrap();
        }
        Ok(n)
    }
}
