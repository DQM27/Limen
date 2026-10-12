package com.dqm27.limen.movil

import android.os.Bundle
import androidx.activity.enableEdgeToEdge
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat

class MainActivity : TauriActivity() {
  override fun onCreate(savedInstanceState: Bundle?) {
    enableEdgeToEdge()
    super.onCreate(savedInstanceState)
    // Con la pantalla de borde a borde el teclado se dibuja encima del WebView
    // y la página no se entera: se le reserva su altura abajo para que los
    // campos y botones queden visibles.
    ViewCompat.setOnApplyWindowInsetsListener(findViewById(android.R.id.content)) { vista, insets ->
      val teclado = insets.getInsets(WindowInsetsCompat.Type.ime())
      vista.setPadding(0, 0, 0, teclado.bottom)
      insets
    }
  }
}
