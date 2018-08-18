//! **Safeword** is a library for gracefully ending a [Tokio][tokio]-based application upon
//! receiving a signal.
//!
//! This could be useful for cleaning up after a program cleanly exits. For example, you might have
//! a server that listens on a Unix domain socket, which does not automatically delete the socket
//! path after the object is dropped. You can run your application with [`Safeword::run`] instead
//! of [`tokio::run`] and know whether your application was asked to stop, or stopped for another
//! reason (such as the future finishing earlier than you expected).
//!
//! Use this library with [`Safeword::run`]. Inspect the cause of why your code might have failed
//! with [`Shutdown`].

extern crate tokio;
extern crate tokio_signal;

use std::error::Error;
use std::fmt::{self, Debug, Display};
use std::io;
use tokio::prelude::future::{self, Either, Future};
use tokio::prelude::stream::Stream;
use tokio::runtime::Runtime;
use tokio_signal::unix::libc::{c_int, SIGINT, SIGTERM};

/// Describes the possible reasons for the runtime to unexpectedly stop (that is, not stop because
/// of a signal).
#[derive(Debug)]
pub enum Shutdown<T, E> {
    /// The future passed to [`Safeword::run`] unexpectedly finished.
    FutureFinished(T),
    /// The future passed to [`Safeword::run`] failed.
    FutureErr(E),
    /// The [`Runtime`] creation failed.
    NoRuntime(io::Error),
    /// A Unix signal handler failed.
    SignalError(io::Error),
}

impl<T, E> Display for Shutdown<T, E>
where
    E: Display,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Shutdown::FutureFinished(_) => write!(f, "unexpectedly finished!"),
            Shutdown::FutureErr(err) => err.fmt(f),
            Shutdown::NoRuntime(err) | Shutdown::SignalError(err) => Display::fmt(err, f),
        }
    }
}

impl<T, E> Error for Shutdown<T, E>
where
    T: Debug,
    E: Error,
{
    fn cause(&self) -> Option<&Error> {
        match self {
            Shutdown::FutureErr(err) => Some(err),
            Shutdown::NoRuntime(err) | Shutdown::SignalError(err) => Some(err),
            _ => None,
        }
    }
}

/// A modified [Tokio][tokio] runtime that exits early on a signal.
///
/// The [`Default`] impl returns a `Safeword` that exits on SIGINT (Ctrl-C) or SIGTERM (what init
/// systems normally use to terminate a process).
#[derive(Debug)]
pub struct Safeword {
    signals: Vec<c_int>,
}

impl Safeword {
    /// Create a `Safeword` that has no configured signals.
    pub fn new() -> Safeword {
        Safeword {
            signals: Vec::new(),
        }
    }

    /// Exit early on a Unix signal.
    pub fn signal(mut self, signal: c_int) -> Safeword {
        self.signals.push(signal);
        self
    }

    /// Run the given [`Future`].
    ///
    /// Returns `Ok(())` if the runtime was terminated by a configured signal. Returns `Err` if
    /// anything else happens, including the `Future` exiting of its own volition, or if something
    /// internal to Safeword fails.
    pub fn run<F>(self, future: F) -> Result<(), Shutdown<F::Item, F::Error>>
    where
        F: Future + Send + 'static,
        F::Item: Send,
        F::Error: Send,
    {
        match Runtime::new()
            .map_err(Shutdown::NoRuntime)?
            .block_on(
                future.select2(future::select_all(self.signals.into_iter().map(|signal| {
                    tokio_signal::unix::Signal::new(signal)
                        .flatten_stream()
                        .into_future()
                        .map(|_| ())
                        .map_err(|(err, _)| Shutdown::SignalError(err))
                }))),
            ) {
            Ok(Either::A((x, _))) => Err(Shutdown::FutureFinished(x)),
            Ok(Either::B(_)) => Ok(()),
            Err(Either::A((err, _))) => Err(Shutdown::FutureErr(err)),
            Err(Either::B(((err, _, _), _))) => Err(err),
        }
    }
}

impl Default for Safeword {
    fn default() -> Safeword {
        Safeword {
            signals: vec![SIGINT, SIGTERM],
        }
    }
}
