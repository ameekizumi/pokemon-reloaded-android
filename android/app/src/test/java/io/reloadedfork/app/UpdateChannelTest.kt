package io.reloadedfork.app

import org.junit.Assert.assertEquals
import org.junit.Test

class UpdateChannelTest {
    @Test fun everyBuildUsesTheForkReleaseChannel() {
        val fork = "lionelmendozaparsons020-lab/pokemon-reloaded-android"
        assertEquals(fork, Update.repositoryFor("1.0.0"))
        assertEquals(fork, Update.repositoryFor("1.0.0-ui.1"))
    }
}
