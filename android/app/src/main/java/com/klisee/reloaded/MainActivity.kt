package io.reloadedfork.app

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import io.reloadedfork.app.discord.DiscordRpcManager
import io.reloadedfork.app.ui.App
import io.reloadedfork.app.ui.ReloadedTheme
import io.reloadedfork.app.ui.Strings

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        val repo = GameRepository(applicationContext)
        val prefs = Preferences(applicationContext)
        Strings.init(applicationContext)
        Strings.update(prefs)
        // El token y la conexión con Discord viven en el manager, compartido con el juego.
        DiscordRpcManager.init(applicationContext)
        // If the player left it on, show "En el lanzador" while the launcher is open too.
        DiscordRpcManager.startIfEnabled()
        setContent {
            ReloadedTheme { App(repo, prefs) }
        }
    }
}
