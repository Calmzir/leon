package dev.getleon.background

import android.app.Service
import android.content.Context
import android.content.Intent
import android.content.pm.ServiceInfo
import android.os.Build
import android.os.IBinder
import android.os.PowerManager
import androidx.core.app.ServiceCompat
import androidx.core.content.ContextCompat

/**
 * Holds the process in the foreground while an agent works for this phone.
 * Android cuts the network of an application a few seconds after it leaves
 * the front; a foreground service is what keeps the link to the computer
 * open, so that a question or the end of a turn still arrives.
 *
 * It does nothing itself: the link lives in the application's process.
 */
class KeepAliveService : Service() {
  /**
   * The link says it is alive every few seconds, on a timer; a sleeping
   * processor stops timers, and the relay then takes the phone for gone.
   * Held only while a turn runs, and never longer than `AWAKE_MS`.
   */
  private var awake: PowerManager.WakeLock? = null

  private fun stayAwake() {
    if (awake?.isHeld == true) return
    val power = getSystemService(Context.POWER_SERVICE) as PowerManager
    awake = power.newWakeLock(PowerManager.PARTIAL_WAKE_LOCK, "leon:link").apply {
      setReferenceCounted(false)
      acquire(AWAKE_MS)
    }
  }

  private fun letSleep() {
    try {
      if (awake?.isHeld == true) awake?.release()
    } catch (gone: RuntimeException) {
      // It ran out by itself.
    }
    awake = null
  }

  override fun onBind(intent: Intent?): IBinder? = null

  override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
    Notices.channels(this)
    val notice =
      Notices.working(
        this,
        intent?.getStringExtra(TITLE) ?: "Leon",
        intent?.getStringExtra(TEXT) ?: "",
        intent?.getStringExtra(URL),
      )
    val type = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) ServiceInfo.FOREGROUND_SERVICE_TYPE_DATA_SYNC else 0
    try {
      ServiceCompat.startForeground(this, Notices.WORKING_ID, notice, type)
      running = true
      starting = false
      if (unwanted) stopSelf() else stayAwake()
    } catch (refused: Exception) {
      // Asked for from behind another application, or out of the day's allowance.
      running = false
      starting = false
      stopSelf()
    }
    // Brought back without the application there is no link to keep.
    return START_NOT_STICKY
  }

  /** The day's allowance of this kind of service ran out (Android 15 on). */
  override fun onTimeout(startId: Int, fgsType: Int) {
    letSleep()
    stopSelf()
  }

  /** The person swiped the application away: its link goes with it. */
  override fun onTaskRemoved(rootIntent: Intent?) {
    stopSelf()
  }

  override fun onDestroy() {
    running = false
    starting = false
    letSleep()
    super.onDestroy()
  }

  companion object {
    private const val TITLE = "title"
    private const val TEXT = "text"
    private const val URL = "url"
    private const val AWAKE_MS = 6 * 60 * 60 * 1000L

    @Volatile
    var running = false
      private set

    /** Asked for and not yet up: stopping it now is an error to Android. */
    @Volatile
    private var starting = false

    /** Released while it was starting: it stops itself once it is up. */
    @Volatile
    private var unwanted = false

    /** Starts it, or changes what it says. Whether it is up afterwards. */
    fun hold(context: Context, title: String, text: String, url: String?): Boolean {
      unwanted = false
      if (running) {
        // Only the words change; starting again is refused from behind.
        if (Notices.allowed(context)) {
          Notices.manager(context).notify(Notices.WORKING_ID, Notices.working(context, title, text, url))
        }
        return true
      }
      val intent =
        Intent(context, KeepAliveService::class.java)
          .putExtra(TITLE, title)
          .putExtra(TEXT, text)
          .putExtra(URL, url)
      return try {
        starting = true
        ContextCompat.startForegroundService(context, intent)
        true
      } catch (refused: Exception) {
        starting = false
        false
      }
    }

    fun release(context: Context) {
      if (starting) {
        unwanted = true
        return
      }
      context.stopService(Intent(context, KeepAliveService::class.java))
    }
  }
}
