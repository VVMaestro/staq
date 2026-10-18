package com.vvmaestro.staq

import android.os.Bundle
import android.view.View
import androidx.activity.enableEdgeToEdge
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat
import androidx.core.view.updatePadding

class MainActivity : TauriActivity() {
  override fun onCreate(savedInstanceState: Bundle?) {
    enableEdgeToEdge()
    super.onCreate(savedInstanceState)

    // Edge-to-edge windows aren't resized for the keyboard, so shrink the WebView ourselves.
    // The navigation bar part stays under the WebView, where CSS safe-area padding covers it.
    ViewCompat.setOnApplyWindowInsetsListener(findViewById<View>(android.R.id.content)) { view, insets ->
      val ime = insets.getInsets(WindowInsetsCompat.Type.ime()).bottom
      val nav = insets.getInsets(WindowInsetsCompat.Type.navigationBars()).bottom
      view.updatePadding(bottom = (ime - nav).coerceAtLeast(0))
      insets
    }
  }
}
