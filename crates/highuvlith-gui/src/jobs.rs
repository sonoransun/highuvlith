//! Background computation with latest-wins coalescing.
//!
//! The UI thread submits requests every frame; a [`Job`] runs at most one
//! computation at a time on a worker thread. Requests that arrive while one
//! is running are coalesced (only the newest is kept), and a request equal
//! to the last one submitted is ignored, so dragging a slider never queues
//! a backlog and the UI never blocks on physics.

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// A finished computation.
#[derive(Debug)]
pub struct Finished<Req, Res> {
    /// The request it answers.
    pub request: Req,
    /// The result, or an error message (including a caught panic).
    pub result: Result<Res, String>,
    /// Wall time of the computation.
    pub elapsed: Duration,
}

type Work<Req, Res> = Arc<dyn Fn(&Req) -> Result<Res, String> + Send + Sync>;

/// One kind of background computation.
pub struct Job<Req, Res> {
    work: Work<Req, Res>,
    running: Arc<AtomicBool>,
    slot: Arc<Mutex<Option<Finished<Req, Res>>>>,
    pending: Option<Req>,
    last_submitted: Option<Req>,
    /// Most recent finished computation, moved here by [`Job::poll`].
    pub latest: Option<Finished<Req, Res>>,
}

impl<Req, Res> Job<Req, Res>
where
    Req: Clone + PartialEq + Send + 'static,
    Res: Send + 'static,
{
    /// A job running `work` on worker threads.
    pub fn new(work: impl Fn(&Req) -> Result<Res, String> + Send + Sync + 'static) -> Self {
        Self {
            work: Arc::new(work),
            running: Arc::new(AtomicBool::new(false)),
            slot: Arc::new(Mutex::new(None)),
            pending: None,
            last_submitted: None,
            latest: None,
        }
    }

    /// Queue `req` unless it equals the last submitted request. Returns
    /// whether it was queued.
    pub fn submit(&mut self, req: Req) -> bool {
        if self.last_submitted.as_ref() == Some(&req) {
            return false;
        }
        self.last_submitted = Some(req.clone());
        self.pending = Some(req);
        true
    }

    /// Forget the last submitted request so an identical one runs again.
    #[cfg(test)]
    pub fn invalidate(&mut self) {
        self.last_submitted = None;
    }

    /// Collect a finished result and start the queued request if the
    /// worker is idle. `notify` runs on the worker thread when it finishes
    /// (e.g. to request a repaint). Returns true when a new result arrived.
    pub fn poll(&mut self, notify: impl Fn() + Send + 'static) -> bool {
        let mut arrived = false;
        if let Some(done) = self.slot.lock().expect("job slot poisoned").take() {
            self.latest = Some(done);
            arrived = true;
        }
        if !self.running.load(Ordering::Acquire) {
            if let Some(req) = self.pending.take() {
                self.running.store(true, Ordering::Release);
                let work = Arc::clone(&self.work);
                let slot = Arc::clone(&self.slot);
                let running = Arc::clone(&self.running);
                std::thread::spawn(move || {
                    let start = Instant::now();
                    let result = catch_unwind(AssertUnwindSafe(|| work(&req)))
                        .unwrap_or_else(|panic| Err(panic_message(panic.as_ref())));
                    *slot.lock().expect("job slot poisoned") = Some(Finished {
                        request: req,
                        result,
                        elapsed: start.elapsed(),
                    });
                    running.store(false, Ordering::Release);
                    notify();
                });
            }
        }
        arrived
    }

    /// A computation is running or queued.
    pub fn is_busy(&self) -> bool {
        self.running.load(Ordering::Acquire) || self.pending.is_some()
    }

    /// The latest successful result.
    pub fn result(&self) -> Option<&Res> {
        self.latest.as_ref().and_then(|f| f.result.as_ref().ok())
    }

    /// The latest error, if the most recent computation failed.
    #[cfg(test)]
    pub fn error(&self) -> Option<&str> {
        self.latest
            .as_ref()
            .and_then(|f| f.result.as_ref().err().map(String::as_str))
    }

    /// Whether the latest finished computation answers `req`.
    pub fn is_current(&self, req: &Req) -> bool {
        self.latest.as_ref().is_some_and(|f| &f.request == req)
    }
}

fn panic_message(panic: &(dyn std::any::Any + Send)) -> String {
    let text = panic
        .downcast_ref::<&str>()
        .map(|s| s.to_string())
        .or_else(|| panic.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "unknown panic".to_string());
    format!("internal error in the computation: {text}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;
    use std::sync::mpsc;

    /// Poll until the job is idle with its result collected.
    fn drain<Req, Res>(job: &mut Job<Req, Res>)
    where
        Req: Clone + PartialEq + Send + 'static,
        Res: Send + 'static,
    {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            job.poll(|| {});
            if !job.is_busy() && job.slot.lock().unwrap().is_none() {
                break;
            }
            assert!(Instant::now() < deadline, "job did not finish");
            std::thread::sleep(Duration::from_millis(1));
        }
    }

    #[test]
    fn runs_and_reports_results() {
        let mut job = Job::new(|x: &i32| Ok::<_, String>(x * 2));
        assert!(job.submit(21));
        drain(&mut job);
        assert_eq!(job.result(), Some(&42));
        assert!(job.is_current(&21));
        assert!(!job.is_current(&20));
        // Identical request: ignored until invalidated.
        assert!(!job.submit(21));
        job.invalidate();
        assert!(job.submit(21));
        drain(&mut job);
    }

    #[test]
    fn coalesces_requests_while_running() {
        let runs = Arc::new(AtomicUsize::new(0));
        let (gate_tx, gate_rx) = mpsc::channel::<()>();
        let gate = Arc::new(Mutex::new(gate_rx));
        let counter = Arc::clone(&runs);
        let mut job = Job::new(move |x: &u32| {
            counter.fetch_add(1, Ordering::SeqCst);
            // The first run blocks until released.
            if *x == 1 {
                gate.lock().unwrap().recv().unwrap();
            }
            Ok::<_, String>(*x)
        });
        job.submit(1);
        job.poll(|| {});
        assert!(job.is_busy());
        // Three submissions while the first runs: only the last survives.
        for x in [2, 3, 4] {
            job.submit(x);
            job.poll(|| {});
        }
        gate_tx.send(()).unwrap();
        drain(&mut job);
        assert_eq!(runs.load(Ordering::SeqCst), 2);
        assert_eq!(job.result(), Some(&4));
    }

    #[test]
    fn errors_and_panics_become_messages() {
        let mut job = Job::new(|x: &i32| {
            if *x < 0 {
                panic!("negative input {x}");
            }
            if *x == 0 {
                return Err("zero".to_string());
            }
            Ok(*x)
        });
        job.submit(0);
        drain(&mut job);
        assert_eq!(job.error(), Some("zero"));
        job.submit(-1);
        drain(&mut job);
        let msg = job.error().unwrap();
        assert!(msg.contains("negative input -1"), "{msg}");
        // The job keeps working after a panic.
        job.submit(5);
        drain(&mut job);
        assert_eq!(job.result(), Some(&5));
    }
}
