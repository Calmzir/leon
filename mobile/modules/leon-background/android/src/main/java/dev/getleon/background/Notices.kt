package dev.getleon.background

import android.Manifest
import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.net.Uri
import android.os.Build
import androidx.core.app.NotificationCompat
import androidx.core.content.ContextCompat

/** The two kinds of notification: the quiet one that says agents work, and the one that calls the person. */
internal object Notices {
  const val WORKING_CHANNEL = "leon.working"
  const val ALERT_CHANNEL = "leon.alerts"
  const val WORKING_ID = 1

  /** The signal of the brand. */
  private const val SIGNAL = 0xFFFFEA00.toInt()

  fun manager(context: Context): NotificationManager =
    context.getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager

  fun allowed(context: Context): Boolean =
    Build.VERSION.SDK_INT < Build.VERSION_CODES.TIRAMISU ||
      ContextCompat.checkSelfPermission(context, Manifest.permission.POST_NOTIFICATIONS) ==
      PackageManager.PERMISSION_GRANTED

  fun channels(context: Context) {
    if (Build.VERSION.SDK_INT < Build.VERSION_CODES.O) return
    val working = NotificationChannel(WORKING_CHANNEL, "Agents working", NotificationManager.IMPORTANCE_LOW)
    working.description = "Shown while an agent you started from this phone is working."
    working.setShowBadge(false)
    val alerts = NotificationChannel(ALERT_CHANNEL, "Agents need you", NotificationManager.IMPORTANCE_HIGH)
    alerts.description = "An agent asked something, finished or stopped with an error."
    manager(context).createNotificationChannels(listOf(working, alerts))
  }

  /** Opens `url` in the application, or the application as it was when there is none. */
  private fun opening(context: Context, url: String?, code: Int): PendingIntent? {
    val intent =
      if (url.isNullOrEmpty()) {
        context.packageManager.getLaunchIntentForPackage(context.packageName)
      } else {
        Intent(Intent.ACTION_VIEW, Uri.parse(url)).setPackage(context.packageName)
      } ?: return null
    intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_SINGLE_TOP)
    return PendingIntent.getActivity(
      context,
      code,
      intent,
      PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
    )
  }

  fun working(context: Context, title: String, text: String, url: String?): Notification =
    NotificationCompat.Builder(context, WORKING_CHANNEL)
      .setSmallIcon(R.drawable.leon_mark)
      .setColor(SIGNAL)
      .setContentTitle(title)
      .setContentText(text)
      .setOngoing(true)
      .setOnlyAlertOnce(true)
      .setShowWhen(false)
      .setCategory(NotificationCompat.CATEGORY_PROGRESS)
      .setForegroundServiceBehavior(NotificationCompat.FOREGROUND_SERVICE_IMMEDIATE)
      .setContentIntent(opening(context, url, WORKING_ID))
      .build()

  fun alert(context: Context, tag: String, title: String, text: String, url: String?) {
    if (!allowed(context)) return
    channels(context)
    val notification =
      NotificationCompat.Builder(context, ALERT_CHANNEL)
        .setSmallIcon(R.drawable.leon_mark)
        .setColor(SIGNAL)
        .setContentTitle(title)
        .setContentText(text)
        .setStyle(NotificationCompat.BigTextStyle().bigText(text))
        .setAutoCancel(true)
        .setPriority(NotificationCompat.PRIORITY_HIGH)
        .setCategory(NotificationCompat.CATEGORY_MESSAGE)
        .setContentIntent(opening(context, url, tag.hashCode()))
        .build()
    manager(context).notify(tag, 0, notification)
  }

  fun dismiss(context: Context, tag: String) = manager(context).cancel(tag, 0)
}
