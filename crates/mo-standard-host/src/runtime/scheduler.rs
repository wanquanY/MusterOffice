use super::{HostClock, RuntimeOptions, StandardHostConfig};
use crate::StandardHost;
use mo_operation_service::{CallContext, Failure, FailureCode};
use std::{
    panic::{AssertUnwindSafe, catch_unwind},
    sync::{Arc, Condvar, Mutex, MutexGuard},
    thread::{self, JoinHandle},
    time::Duration,
};

struct State {
    started: bool,
    stopping: bool,
    generation: u64,
    error: Option<Failure>,
}
struct Shared {
    state: Mutex<State>,
    changed: Condvar,
}
fn interrupted(message: &'static str) -> Failure {
    Failure::new(FailureCode::ExecutionInterrupted, message)
}
impl Shared {
    fn lock(&self) -> MutexGuard<'_, State> {
        match self.state.lock() {
            Ok(state) => state,
            Err(poisoned) => {
                let mut state = poisoned.into_inner();
                state.stopping = true;
                state
                    .error
                    .get_or_insert_with(|| interrupted("scheduler synchronization failed"));
                self.changed.notify_all();
                state
            }
        }
    }
    fn notify(&self) {
        let mut state = self.lock();
        state.generation = state.generation.wrapping_add(1);
        self.changed.notify_all();
    }
    fn fail(&self, error: Failure) {
        let mut state = self.lock();
        state.error.get_or_insert(error);
        state.stopping = true;
        self.changed.notify_all();
    }
    fn wait(&self, generation: u64, timeout: Duration) {
        let result = self
            .changed
            .wait_timeout_while(self.lock(), timeout, |state| {
                !state.stopping && state.generation == generation
            });
        if result.is_err() {
            drop(result);
            self.fail(interrupted("scheduler synchronization failed"));
        }
    }
}
pub(super) struct Scheduler {
    shared: Arc<Shared>,
    threads: Vec<JoinHandle<()>>,
}
impl Scheduler {
    pub fn start(
        config: &StandardHostConfig,
        context: CallContext,
        options: RuntimeOptions,
        clock: HostClock,
    ) -> Result<Self, Failure> {
        // Establish every connection before any task may run. Partial startup
        // must not return failure after secretly launching a subset of workers.
        let connections = (0..options.workers)
            .map(|_| config.connect())
            .collect::<Result<Vec<_>, _>>()?;
        let shared = Arc::new(Shared {
            state: Mutex::new(State {
                started: false,
                stopping: false,
                generation: 0,
                error: None,
            }),
            changed: Condvar::new(),
        });
        let mut scheduler = Self {
            shared,
            threads: Vec::new(),
        };
        for (index, host) in connections.into_iter().enumerate() {
            let shared = scheduler.shared.clone();
            let context = context.clone();
            let clock = clock.clone();
            let thread = thread::Builder::new()
                .name(format!("mo-executor-{index}"))
                .spawn(move || {
                    let result = catch_unwind(AssertUnwindSafe(|| {
                        work(host, &context, options, clock, shared.clone())
                    }));
                    match result {
                        Ok(Ok(())) => (),
                        Ok(Err(error)) => shared.fail(error),
                        Err(_) => shared.fail(interrupted(
                            "native executor panicked; lease recovery is required",
                        )),
                    }
                })
                .map_err(|_| interrupted("could not start native executor"))?;
            scheduler.threads.push(thread);
        }
        {
            let mut state = scheduler.shared.lock();
            state.started = true;
        }
        scheduler.shared.notify();
        Ok(scheduler)
    }
    pub fn health(&self) -> Result<(), Failure> {
        let state = self.shared.lock();
        if let Some(error) = &state.error {
            return Err(error.clone());
        }
        if state.stopping {
            return Err(interrupted("native scheduler is stopped"));
        }
        Ok(())
    }
    pub fn generation(&self) -> u64 {
        self.shared.lock().generation
    }
    pub fn notify(&self) {
        self.shared.notify()
    }
    pub fn wait(&self, generation: u64, timeout: Duration) {
        self.shared.wait(generation, timeout)
    }
    pub fn shutdown(&mut self) -> Result<(), Failure> {
        {
            let mut state = self.shared.lock();
            state.stopping = true;
            self.shared.changed.notify_all();
        }
        for thread in self.threads.drain(..) {
            if thread.join().is_err() {
                self.shared.fail(interrupted("native executor join failed"));
            }
        }
        match &self.shared.lock().error {
            Some(error) => Err(error.clone()),
            None => Ok(()),
        }
    }
}
impl Drop for Scheduler {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}

fn work(
    mut host: StandardHost,
    context: &CallContext,
    options: RuntimeOptions,
    clock: HostClock,
    shared: Arc<Shared>,
) -> Result<(), Failure> {
    loop {
        let generation = {
            let state = shared.lock();
            if state.stopping {
                return Ok(());
            }
            if !state.started {
                let generation = state.generation;
                drop(state);
                shared.wait(generation, options.idle_poll);
                continue;
            }
            state.generation
        };
        host.recover_expired(context, clock(), options.recovery_batch)?;
        match host.run_next(context, clock(), clock.as_ref(), &|| false)? {
            Some(_) => shared.notify(),
            None => shared.wait(generation, options.idle_poll),
        }
    }
}
