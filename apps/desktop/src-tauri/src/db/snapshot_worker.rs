//! One bounded snapshot queue; maintenance invalidates old jobs and waits for I/O.
use super::Database;
use parking_lot::{Mutex, RwLock, RwLockWriteGuard};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, SyncSender, TrySendError};
use std::thread::JoinHandle;

static GENERATION: RwLock<u64> = RwLock::new(0);
static WORKER: Mutex<Option<Service>> = Mutex::new(None);
static SHUTTING_DOWN: AtomicBool = AtomicBool::new(false);
const QUEUE_CAPACITY: usize = 16;
#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SnapshotHealth {
    pub completed: u64,
    pub queue_drops: u64,
    pub last_completed_at: Option<String>,
    pub last_error: Option<String>,
}
static HEALTH: Mutex<SnapshotHealth> = Mutex::new(SnapshotHealth {
    completed: 0,
    queue_drops: 0,
    last_completed_at: None,
    last_error: None,
});
pub(crate) fn snapshot_health() -> SnapshotHealth {
    HEALTH.lock().clone()
}
struct Service {
    sender: SyncSender<Job>,
    worker: JoinHandle<()>,
}

struct Job {
    path: PathBuf,
    session: String,
    sequence: Option<i64>,
    generation: u64,
}

pub(super) fn enqueue(db: &Database, session: &str, sequence: Option<i64>) {
    if db.snapshot_dir().is_none() {
        return;
    }
    let generation = *GENERATION.read();
    let mut service = WORKER.lock();
    if SHUTTING_DOWN.load(Ordering::SeqCst) {
        return;
    }
    if service.is_none() {
        let (send, receive) = mpsc::sync_channel::<Job>(QUEUE_CAPACITY);
        match std::thread::Builder::new()
            .name("reddot-snapshots".into())
            .spawn(move || {
                while let Ok(job) = receive.recv() {
                    let generation = GENERATION.read();
                    if *generation != job.generation {
                        continue;
                    }
                    // Keep the barrier until the connection and all publication I/O finish.
                    let result = Database::open(&job.path).and_then(|db| {
                        let sequence = match job.sequence {
                            Some(sequence) => sequence,
                            None => db.last_event_sequence(&job.session)?,
                        };
                        db.write_session_snapshot(&job.session, sequence)
                            .map(|_| ())
                    });
                    let mut health = HEALTH.lock();
                    match result {
                        Ok(()) => {
                            health.completed += 1;
                            health.last_completed_at = Some(chrono::Utc::now().to_rfc3339());
                            health.last_error = None;
                        }
                        Err(error) => {
                            eprintln!("Snapshot fehlgeschlagen ({}): {error}", job.session);
                            health.last_error = Some(error);
                        }
                    }
                }
            }) {
            Ok(worker) => {
                *service = Some(Service {
                    sender: send,
                    worker,
                })
            }
            Err(error) => {
                HEALTH.lock().last_error = Some(error.to_string());
                eprintln!("Snapshot-Worker konnte nicht starten: {error}");
                return;
            }
        }
    }
    if let Some(active) = service.as_ref() {
        match active.sender.try_send(Job {
            path: db.path().to_path_buf(),
            session: session.into(),
            sequence,
            generation,
        }) {
            Ok(()) => {}
            Err(TrySendError::Full(_)) => {
                let mut health = HEALTH.lock();
                health.queue_drops += 1;
                health.last_error = Some(format!(
                    "Snapshot-Queue voll (max. {QUEUE_CAPACITY}); bitte erneut sichern"
                ));
                eprintln!("{}", health.last_error.as_deref().unwrap_or_default());
            }
            Err(TrySendError::Disconnected(_)) => {
                if let Some(active) = service.take() {
                    if active.worker.join().is_err() {
                        eprintln!("Snapshot-Worker ist fehlgeschlagen");
                    }
                }
                eprintln!("Snapshot-Worker beendet; nächste Anforderung startet ihn neu");
            }
        }
    }
}

/// Final application shutdown: finish active I/O, invalidate queued work and join.
pub(crate) fn shutdown_snapshots() {
    SHUTTING_DOWN.store(true, Ordering::SeqCst);
    let barrier = pause_snapshots();
    let service = WORKER.lock().take();
    drop(barrier); // queued jobs must acquire the read lock before they can exit.
    if let Some(Service { sender, worker }) = service {
        drop(sender);
        if worker.join().is_err() {
            eprintln!("Snapshot-Worker beim Shutdown fehlgeschlagen");
        }
    }
}

pub(crate) fn pause_snapshots() -> RwLockWriteGuard<'static, u64> {
    let mut generation = GENERATION.write();
    *generation = generation.wrapping_add(1);
    generation
}
