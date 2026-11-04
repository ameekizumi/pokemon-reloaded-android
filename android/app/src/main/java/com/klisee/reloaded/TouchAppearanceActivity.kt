package io.reloadedfork.app

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import io.reloadedfork.app.ui.TouchAppearanceScreen
import io.reloadedfork.app.ui.ReloadedTheme

class TouchAppearanceActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContent { ReloadedTheme { TouchAppearanceScreen(Preferences(this)) { finish() } } }
    }
}
