//!
//! A simple timer capable of measuring time intervals.
//!

#![allow(dead_code)]

type TimeStamp = chrono::DateTime<chrono::Utc>;

///
/// A simple timer capable of measuring time intervals between invokations of
/// `[start]` and `[stop]` methods.
///
#[derive(Clone, Default, Debug)]
pub struct Timer {
    /// Start time,
    start: Option<TimeStamp>,
    /// End time,
    end: Option<TimeStamp>,
}

impl Timer {
    /// Starts the timer. This sets the `start` time to the current time.
    ///
    /// # Returns
    ///
    /// An Ok result if the timer was successfully started, or an error if
    /// the timer was already started or stopped before.
    ///
    pub fn start(&mut self) -> anyhow::Result<()> {
        match (self.start, self.end) {
            (None, None) => {
                self.start = Some(chrono::offset::Utc::now());
                Ok(())
            }
            _ => anyhow::bail!("Malformed timer state: {self:?}"),
        }
    }

    /// Stops the timer from ticking. Assumes the timer has been started with
    /// `[start]`.
    ///
    /// # Returns
    ///
    /// An Ok result if the timer was successfully stopped, or an error if
    /// the timer has not been started or if it was already stopped.
    ///
    pub fn stop(&mut self) -> anyhow::Result<()> {
        match (self.start, self.end) {
            (Some(_), None) => {
                self.end = Some(chrono::offset::Utc::now());
                Ok(())
            }
            _ => anyhow::bail!("Malformed timer state: {self:?}"),
        }
    }

    ///
    /// Checks if timer is ticking now.
    ///
    /// # Returns
    ///
    /// `true` if the timer has been started and is currently running, `false` otherwise.
    ///
    pub fn is_started(&self) -> bool {
        self.start.is_some()
    }

    /// Returns true if the timer was started and then subsequently stopped.
    ///
    /// # Returns
    ///
    /// `true` if the timer has finished (started and stopped), `false` otherwise.
    ///
    pub fn is_finished(&self) -> bool {
        self.end.is_some()
    }

    /// Returns the start time of this [`Timer`].
    ///
    /// # Returns
    ///
    /// An `Option` containing the start `TimeStamp` if the timer has been started,
    /// or `None` if it has not been started.
    ///
    pub fn get_start(&self) -> Option<TimeStamp> {
        self.start
    }

    /// Returns the end time of this [`Timer`].
    ///
    /// # Returns
    ///
    /// An `Option` containing the end `TimeStamp` if the timer has been stopped,
    /// or `None` if it is still running or has not been started.
    ///
    pub fn get_end(&self) -> Option<TimeStamp> {
        self.end
    }

    /// Returns the elapsed time between the start and end of the timer.
    ///
    /// # Returns
    ///
    /// An Ok result with the `chrono::TimeDelta` representing the elapsed time
    /// if the timer was started (and optionally stopped), or an error if the
    /// timer was not started correctly.
    ///
    pub fn elapsed(&self) -> anyhow::Result<chrono::TimeDelta> {
        match (self.start, self.end) {
            (Some(start), Some(end)) => Ok(end - start),
            (Some(start), None) => Ok(chrono::Utc::now() - start),
            _ => anyhow::bail!("Malformed timer state: {self:?}"),
        }
    }
}
