package com.gabrielfranicevich.watch

import android.content.Context
import androidx.test.core.app.ActivityScenario
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import org.json.JSONObject
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class NativeAndroidAcceptanceTest {
  @Test
  fun applicationShellLaunches() {
    ActivityScenario.launch(MainActivity::class.java).use { scenario ->
      scenario.onActivity { activity ->
        assertTrue(activity.isFinishing.not())
      }
    }
  }

  @Test
  fun packagedRustBridgeLoadsAndValidatesRequests() {
    val context = ApplicationProvider.getApplicationContext<Context>()
    val historyPath = context.filesDir.resolve("history/ani-hsts").absolutePath
    NativeWatch(historyPath).use { native ->
      val emptyQuery = JSONObject(native.search(" ", null))
      assertTrue(emptyQuery.getString("error").contains("empty"))

      val unknownProvider = JSONObject(
        native.episodes("not-a-provider", "id", "Title", "https://example.test/title")
      )
      assertTrue(unknownProvider.getString("error").contains("unknown provider"))
    }
  }
}
