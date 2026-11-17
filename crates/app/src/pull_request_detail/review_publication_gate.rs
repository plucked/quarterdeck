use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};

const ACTIVE: u8 = 0;
const CANCELLED: u8 = 1;
const PUBLISHING: u8 = 2;
const INVALIDATED_PUBLICATION: u8 = 3;

/// Arbitrates cancellation and publication without blocking the UI on SQLite.
/// Whichever operation wins the transition from Active owns the outcome.
#[derive(Default)]
pub(super) struct ReviewPublicationGate {
    phase: AtomicU8,
    cancelled: AtomicBool,
}

impl ReviewPublicationGate {
    /// The process runner polls this flag while preparing and invoking Claude.
    pub fn cancellation_flag(&self) -> &AtomicBool {
        &self.cancelled
    }

    /// Returns false once the completed guide has entered its save transaction.
    /// The UI then retains the task until its short publication step finishes.
    pub fn cancel(&self) -> bool {
        match self
            .phase
            .compare_exchange(ACTIVE, CANCELLED, Ordering::SeqCst, Ordering::SeqCst)
        {
            Ok(_) | Err(CANCELLED) => {
                self.cancelled.store(true, Ordering::SeqCst);
                true
            }
            Err(_) => false,
        }
    }

    /// Reserves publication; a previously accepted cancellation cannot be undone.
    pub fn begin_publication(&self) -> bool {
        self.phase
            .compare_exchange(ACTIVE, PUBLISHING, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
    }

    /// Restores cancellation and retry after SQLite rolls its publication back.
    /// A refresh during publication permanently revokes that snapshot instead.
    pub fn failed_publication(&self) {
        self.phase
            .try_update(Ordering::SeqCst, Ordering::SeqCst, |phase| match phase {
                PUBLISHING => Some(ACTIVE),
                INVALIDATED_PUBLICATION => Some(CANCELLED),
                _ => None,
            })
            .ok();
    }

    /// Revokes a changed snapshot even when its transaction is already publishing.
    /// False keeps the task alive to observe that transaction's success or rollback.
    pub fn invalidate(&self) -> bool {
        // A failed transaction must remember refresh even when immediate cancellation loses.
        self.cancelled.store(true, Ordering::SeqCst);
        matches!(
            self.phase
                .try_update(Ordering::SeqCst, Ordering::SeqCst, |phase| match phase {
                    ACTIVE | CANCELLED => Some(CANCELLED),
                    PUBLISHING | INVALIDATED_PUBLICATION => Some(INVALIDATED_PUBLICATION),
                    _ => None,
                }),
            Ok(ACTIVE | CANCELLED)
        )
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::Ordering;
    use std::sync::{Arc, Barrier};

    use super::ReviewPublicationGate;

    /// Cancellation and publication cannot both claim the same active attempt.
    #[test]
    fn cancellation_and_publication_have_one_winner() {
        for _ in 0..64 {
            let gate = Arc::new(ReviewPublicationGate::default());
            let barrier = Arc::new(Barrier::new(2));
            let publisher_gate = gate.clone();
            let publisher_barrier = barrier.clone();
            let publisher = std::thread::spawn(move || {
                publisher_barrier.wait();
                publisher_gate.begin_publication()
            });
            barrier.wait();
            let cancelled = gate.cancel();
            let saved = publisher.join().unwrap();
            assert_ne!(cancelled, saved);
            assert_eq!(gate.cancellation_flag().load(Ordering::SeqCst), cancelled);
        }
    }

    /// A rolled-back save permits both another publication attempt and cancellation.
    #[test]
    fn failed_publication_can_retry_or_cancel() {
        let gate = ReviewPublicationGate::default();
        assert!(gate.begin_publication());
        assert!(!gate.cancel());
        gate.failed_publication();
        assert!(gate.begin_publication());
        gate.failed_publication();
        assert!(gate.cancel());
        assert!(!gate.begin_publication());
    }

    /// A refreshed snapshot cannot become retryable after an in-flight save fails.
    #[test]
    fn invalidation_during_publication_survives_rollback() {
        let gate = ReviewPublicationGate::default();
        assert!(gate.begin_publication());
        assert!(!gate.invalidate());
        gate.failed_publication();
        assert!(gate.cancellation_flag().load(Ordering::SeqCst));
        assert!(!gate.begin_publication());
        assert!(gate.cancel());
    }

    /// Refresh and rollback always leave the old snapshot permanently revoked.
    #[test]
    fn invalidation_and_rollback_cannot_restore_a_stale_snapshot() {
        for _ in 0..64 {
            let gate = Arc::new(ReviewPublicationGate::default());
            assert!(gate.begin_publication());
            let (sender, receiver) = std::sync::mpsc::channel();
            let publisher_gate = gate.clone();
            let publisher = std::thread::spawn(move || {
                receiver
                    .recv_timeout(std::time::Duration::from_secs(2))
                    .unwrap();
                publisher_gate.failed_publication();
            });
            sender.send(()).unwrap();
            gate.invalidate();
            publisher.join().unwrap();
            assert!(gate.cancellation_flag().load(Ordering::SeqCst));
            assert!(!gate.begin_publication());
        }
    }
}
