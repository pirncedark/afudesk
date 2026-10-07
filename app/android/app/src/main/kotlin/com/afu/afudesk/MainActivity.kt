package com.afu.afudesk

import android.os.Build
import android.content.Intent
import android.net.Uri
import android.provider.Settings
import androidx.core.content.FileProvider
import java.io.File
import android.os.VibrationEffect
import android.os.Vibrator
import android.view.InputDevice
import android.view.KeyEvent
import android.view.MotionEvent
import io.flutter.embedding.engine.FlutterEngine
import io.flutter.embedding.android.FlutterActivity
import io.flutter.plugin.common.MethodChannel

class MainActivity : FlutterActivity() {
    private lateinit var channel: MethodChannel
    private val pressed = mutableSetOf<String>()
    private var axes = floatArrayOf(0f, 0f, 0f, 0f, 0f, 0f)
    private var kolVibrator: Vibrator? = null
    private var pendingApk: File? = null

    override fun configureFlutterEngine(flutterEngine: FlutterEngine) {
        super.configureFlutterEngine(flutterEngine)
        MethodChannel(flutterEngine.dartExecutor.binaryMessenger, "afudesk/guncelle").setMethodCallHandler { call, result ->
            try {
                when (call.method) {
                    "surum" -> result.success(packageManager.getPackageInfo(packageName, 0).versionName)
                    "klasor" -> result.success(File(cacheDir, "updates").apply { mkdirs() }.absolutePath)
                    "kur" -> {
                        val apk = File(call.arguments as String).canonicalFile
                        require(apk.parentFile == File(cacheDir, "updates").canonicalFile && apk.name == "AfuDesk.apk" && apk.exists())
                        pendingApk = apk
                        if (Build.VERSION.SDK_INT >= 26 && !packageManager.canRequestPackageInstalls()) {
                            startActivityForResult(Intent(Settings.ACTION_MANAGE_UNKNOWN_APP_SOURCES, Uri.parse("package:$packageName")), 804)
                        } else installUpdate(apk)
                        result.success(null)
                    }
                    else -> result.notImplemented()
                }
            } catch (_: Exception) { result.error("update", "Güncelleme kurulamadı. Yeniden dene.", null) }
        }
        channel = MethodChannel(flutterEngine.dartExecutor.binaryMessenger, "afudesk/kol")
        channel.setMethodCallHandler { call, result ->
            if (call.method == "vibrate") {
                val duration = (call.argument<Int>("duration") ?: 35).toLong().coerceIn(1L, 500L)
                val vibrator = if (Build.VERSION.SDK_INT >= 31) getSystemService(android.os.VibratorManager::class.java).defaultVibrator else getSystemService(Vibrator::class.java)
                if (Build.VERSION.SDK_INT >= 26) vibrator.vibrate(VibrationEffect.createOneShot(duration, VibrationEffect.DEFAULT_AMPLITUDE)) else vibrator.vibrate(duration)
                kolVibrator?.takeIf { it.hasVibrator() }?.let {
                    if (Build.VERSION.SDK_INT >= 26) it.vibrate(VibrationEffect.createOneShot(duration, VibrationEffect.DEFAULT_AMPLITUDE)) else it.vibrate(duration)
                }
                result.success(null)
            } else if (call.method == "veriKlasoru") {
                // Kalıcı cihaz kimliği ve kayıtlı cihazlar (uygulamaya özel klasör).
                result.success(filesDir.absolutePath)
            } else result.notImplemented()
        }
    }

    private fun installUpdate(apk: File) {
        val uri = FileProvider.getUriForFile(this, "$packageName.updates", apk)
        startActivity(Intent(Intent.ACTION_VIEW).setDataAndType(uri, "application/vnd.android.package-archive")
            .addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION))
        pendingApk = null
    }

    override fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?) {
        super.onActivityResult(requestCode, resultCode, data)
        if (requestCode == 804) {
            val apk = pendingApk
            pendingApk = null
            if (apk != null && Build.VERSION.SDK_INT >= 26 && packageManager.canRequestPackageInstalls()) {
                try { installUpdate(apk) } catch (_: Exception) {
                    android.widget.Toast.makeText(this, "Güncelleme kurulamadı. Yeniden dene.", android.widget.Toast.LENGTH_LONG).show()
                }
            } else android.widget.Toast.makeText(this, "Kurulum izni verilmedi. Güncelle düğmesine yeniden dokun.", android.widget.Toast.LENGTH_LONG).show()
        }
    }

    private fun taninanTusu(code: Int): String? = when (code) {
        KeyEvent.KEYCODE_DPAD_UP -> "up"; KeyEvent.KEYCODE_DPAD_DOWN -> "down"
        KeyEvent.KEYCODE_DPAD_LEFT -> "left"; KeyEvent.KEYCODE_DPAD_RIGHT -> "right"
        KeyEvent.KEYCODE_BUTTON_A -> "a"; KeyEvent.KEYCODE_BUTTON_B -> "b"
        KeyEvent.KEYCODE_BUTTON_X -> "x"; KeyEvent.KEYCODE_BUTTON_Y -> "y"
        KeyEvent.KEYCODE_BUTTON_L1 -> "lb"; KeyEvent.KEYCODE_BUTTON_R1 -> "rb"
        KeyEvent.KEYCODE_BUTTON_L2 -> "lt"; KeyEvent.KEYCODE_BUTTON_R2 -> "rt"
        KeyEvent.KEYCODE_BUTTON_START -> "start"; KeyEvent.KEYCODE_BUTTON_SELECT -> "back"
        KeyEvent.KEYCODE_BUTTON_THUMBL -> "leftStick"; KeyEvent.KEYCODE_BUTTON_THUMBR -> "rightStick"
        else -> null
    }

    private fun gonderKol(e: MotionEvent) {
        axes = floatArrayOf(
            e.getAxisValue(MotionEvent.AXIS_X), e.getAxisValue(MotionEvent.AXIS_Y),
            e.getAxisValue(MotionEvent.AXIS_Z).takeIf { it != 0f } ?: e.getAxisValue(MotionEvent.AXIS_RX),
            e.getAxisValue(MotionEvent.AXIS_RZ).takeIf { it != 0f } ?: e.getAxisValue(MotionEvent.AXIS_RY),
            maxOf(e.getAxisValue(MotionEvent.AXIS_LTRIGGER), e.getAxisValue(MotionEvent.AXIS_BRAKE)).coerceIn(0f, 1f),
            maxOf(e.getAxisValue(MotionEvent.AXIS_RTRIGGER), e.getAxisValue(MotionEvent.AXIS_GAS)).coerceIn(0f, 1f)
        )
        val hatX = e.getAxisValue(MotionEvent.AXIS_HAT_X)
        val hatY = e.getAxisValue(MotionEvent.AXIS_HAT_Y)
        for (d in listOf("up", "down", "left", "right")) pressed.remove("axis:$d")
        if (hatY < -0.5f) pressed.add("axis:up"); if (hatY > 0.5f) pressed.add("axis:down")
        if (hatX < -0.5f) pressed.add("axis:left"); if (hatX > 0.5f) pressed.add("axis:right")
        bildirKol()
    }

    private fun bildirKol() {
        if (!::channel.isInitialized) return
        val names = pressed.map { it.removePrefix("axis:") }.distinct()
        channel.invokeMethod("kol", mapOf("keys" to names, "lx" to axes[0], "ly" to axes[1], "rx" to axes[2], "ry" to axes[3], "lt" to axes[4], "rt" to axes[5]))
    }

    override fun dispatchKeyEvent(event: KeyEvent): Boolean {
        val tus = taninanTusu(event.keyCode)
        if (tus != null && ((event.device?.sources ?: 0) and (InputDevice.SOURCE_GAMEPAD or InputDevice.SOURCE_JOYSTICK)) != 0) {
            kolVibrator = event.device?.vibrator
            if (event.action == KeyEvent.ACTION_DOWN) pressed.add(tus) else if (event.action == KeyEvent.ACTION_UP) pressed.remove(tus)
            bildirKol()
            return true
        }
        return super.dispatchKeyEvent(event)
    }

    override fun dispatchGenericMotionEvent(event: MotionEvent): Boolean {
        if (event.action == MotionEvent.ACTION_MOVE && ((event.device?.sources ?: 0) and InputDevice.SOURCE_JOYSTICK) != 0) {
            kolVibrator = event.device?.vibrator
            gonderKol(event)
            return true
        }
        return super.dispatchGenericMotionEvent(event)
    }
}
