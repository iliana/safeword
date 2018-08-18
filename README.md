# Safeword

Safeword is a library for gracefully ending a [Tokio](https://tokio.rs)-based application upon receiving a signal.

This could be useful for cleaning up after a program cleanly exits. For example, you might have a server that listens on a Unix domain socket, which does not automatically delete the socket path after the object is dropped. You can run your application with `Safeword::run` instead of `tokio::run` and know whether your application was asked to stop, or stopped for another reason (such as the future finishing earlier than you expected).

See [the examples](https://github.com/ilianaw/safeword/tree/mistress/examples) for how this might be usefully used.
