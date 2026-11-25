use std::time::{Duration, Instant};

use super::error::{AuthenticationTimeoutSnafu, OAuthError};

#[derive(Clone, Copy, Debug)]
pub(super) struct FlowDeadline {
    start: Instant,
    budget: Duration,
}

impl FlowDeadline {
    pub(super) fn new(budget_secs: u64) -> Self {
        Self {
            start: Instant::now(),
            budget: Duration::from_secs(budget_secs),
        }
    }

    pub(super) fn remaining(&self) -> Option<Duration> {
        self.budget
            .checked_sub(self.start.elapsed())
            .filter(|left| !left.is_zero())
    }

    pub(super) fn timeout_error(&self) -> OAuthError {
        AuthenticationTimeoutSnafu {
            elapsed_secs: self.start.elapsed().as_secs(),
        }
        .build()
    }

    pub(super) fn half_of_remaining(&self) -> Option<Self> {
        let half = self.remaining()? / 2;
        (!half.is_zero()).then(|| Self {
            start: self.start,
            budget: self.start.elapsed() + half,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn half_of_remaining_keeps_the_flow_start() {
        let deadline = FlowDeadline {
            start: Instant::now() - Duration::from_secs(40),
            budget: Duration::from_secs(120),
        };

        let half = deadline.half_of_remaining().expect("time is left");

        let remaining = half.remaining().expect("half is not spent");
        assert!(
            remaining <= Duration::from_secs(40) && remaining > Duration::from_secs(39),
            "expected about 40s of the 80s left, got {remaining:?}"
        );
        assert!(
            matches!(
                half.timeout_error(),
                OAuthError::AuthenticationTimeout { elapsed_secs, .. } if elapsed_secs >= 40
            ),
            "the timeout reports the time since the flow started"
        );
    }

    #[test]
    fn spent_deadline_has_nothing_left() {
        let deadline = FlowDeadline {
            start: Instant::now() - Duration::from_secs(10),
            budget: Duration::from_secs(10),
        };

        assert!(deadline.remaining().is_none());
        assert!(deadline.half_of_remaining().is_none());
    }
}
