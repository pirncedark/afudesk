package com.afu.afudesk

import android.os.Build
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

    override fun configureFlutterEngine(flutterEngine: FlutterEngine) {
        super.configureFlutterEngine(flutterEngine)
        channel = MethodChannel(flutterEngine.dartExecutor.binaryMessenger, "afudesk/kol")
        channel.setMethodCallHandler { call, result ->
            if (call.method == "vibrate") {
                val duration = (call.argument<Int>("duration") ?: 35).toLong().coerceIn(1L, 500L)
                val vibrator = if (Build.VERSION.SDK_INT >= 31) getSystemService(android.os.VibratorManager::class.java).defaultVibrator else getSystemService(Vibrator::class.java)
                if (Build.VERSION.SDK_INT >= 26) vibrator.vibrate(VibrationEffect.createOneShot(duration, VibrationEffect.DEFAULT_AMPLITUDE)) else vibrator.vibrate(duration)
                result.success(null)
            } else result.notImplemented()
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
            e.getAxisValue(MotionEvent.AXIS_LTRIGGER).coerceIn(0f, 1f),
            e.getAxisValue(MotionEvent.AXIS_RTRIGGER).coerceIn(0f, 1f)
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
            if (event.action == KeyEvent.ACTION_DOWN) pressed.add(tus) else if (event.action == KeyEvent.ACTION_UP) pressed.remove(tus)
            bildirKol()
            return true
        }
        return super.dispatchKeyEvent(event)
    }

    override fun dispatchGenericMotionEvent(event: MotionEvent): Boolean {
        if (event.action == MotionEvent.ACTION_MOVE && ((event.device?.sources ?: 0) and InputDevice.SOURCE_JOYSTICK) != 0) {
            gonderKol(event)
            return true
        }
        return super.dispatchGenericMotionEvent(event)
    }
}
