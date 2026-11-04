package io.reloadedfork.app

/** Prevents save backups/restores from racing against a still-running native game. */
object GameSession {
    @Volatile var active: Boolean = false
        internal set
}
