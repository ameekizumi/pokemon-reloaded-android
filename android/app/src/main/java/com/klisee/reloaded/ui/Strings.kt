package io.reloadedfork.app.ui

import android.content.Context
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import io.reloadedfork.app.Preferences
import org.json.JSONObject

/**
 * UI translation. Spanish is the source; `ui_en.json` holds the English strings. The language is
 * the same one used for the game and the Rich Presence (auto by phone, or forced in Ajustes).
 */
object Strings {
    var english by mutableStateOf(false)
        private set

    private var map: Map<String, String> = emptyMap()
    // Keys containing "{}" match any text in that spot ("Slot {}" -> "Slot 2"), and keys that end
    // with a space or ": " translate just the fixed lead of a longer, partly dynamic message.
    private var patterns: List<Pair<Regex, String>> = emptyList()
    private var prefixes: List<Pair<String, String>> = emptyList()
    private val cache = HashMap<String, String>()

    fun init(context: Context) {
        if (map.isNotEmpty()) return
        map = runCatching {
            val text = context.assets.open("ui_en.json").bufferedReader().use { it.readText() }
            val json = JSONObject(text)
            json.keys().asSequence().associateWith { json.getString(it) }
        }.getOrDefault(emptyMap())
        patterns = map.filterKeys { "{}" in it }.map { (k, v) ->
            Regex(k.split("{}").joinToString("(.+?)") { Regex.escape(it) }, RegexOption.DOT_MATCHES_ALL) to v
        }
        prefixes = map.filterKeys { "{}" !in it && it.length >= 6 && (it.endsWith(" ") || it.endsWith(": ")) }
            .toList().sortedByDescending { it.first.length }
        cache.clear()
    }

    fun update(prefs: Preferences) {
        english = prefs.resolvedGameLanguage() == "en"
    }

    fun tr(text: String): String {
        if (!english) return text
        map[text]?.let { return it }
        cache[text]?.let { return it }
        var out = text
        for ((re, en) in patterns) {
            val m = re.matchEntire(text) ?: continue
            var i = 0
            out = Regex("\\{\\}").replace(en) { m.groupValues[++i] }
            break
        }
        if (out == text) {
            for ((k, v) in prefixes) {
                if (text.length > k.length && text.startsWith(k)) { out = v + text.substring(k.length); break }
            }
        }
        cache[text] = out
        return out
    }
}
