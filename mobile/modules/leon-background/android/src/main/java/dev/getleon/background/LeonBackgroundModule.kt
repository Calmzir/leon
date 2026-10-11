package dev.getleon.background

import android.Manifest
import android.content.Context
import android.os.Build
import expo.modules.interfaces.permissions.Permissions
import expo.modules.kotlin.Promise
import expo.modules.kotlin.exception.Exceptions
import expo.modules.kotlin.modules.Module
import expo.modules.kotlin.modules.ModuleDefinition

class LeonBackgroundModule : Module() {
  private val context: Context
    get() = appContext.reactContext ?: throw Exceptions.ReactContextLost()

  override fun definition() = ModuleDefinition {
    Name("LeonBackground")

    /** Keeps the process in the foreground, saying `title` and `text`. Whether it is held. */
    Function("hold") { title: String, text: String, url: String? ->
      KeepAliveService.hold(context, title, text, url)
    }

    Function("release") {
      KeepAliveService.release(context)
    }

    Function("alert") { tag: String, title: String, text: String, url: String? ->
      Notices.alert(context, tag, title, text, url)
    }

    Function("dismiss") { tag: String ->
      Notices.dismiss(context, tag)
    }

    Function("canNotify") {
      Notices.allowed(context)
    }

    AsyncFunction("askToNotify") { promise: Promise ->
      if (Build.VERSION.SDK_INT < Build.VERSION_CODES.TIRAMISU) {
        promise.resolve(mapOf("granted" to true))
      } else {
        Permissions.askForPermissionsWithPermissionsManager(
          appContext.permissions,
          promise,
          Manifest.permission.POST_NOTIFICATIONS,
        )
      }
    }
  }
}
