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
import io.flutter.plugin.common.EventChannel
import android.Manifest
import android.content.pm.PackageManager
import android.media.AudioFormat
import android.media.AudioRecord
import android.media.MediaRecorder

class MainActivity : FlutterActivity() {
    private lateinit var channel: MethodChannel
    private val pressed = mutableSetOf<String>()
    private var axes = floatArrayOf(0f, 0f, 0f, 0f, 0f, 0f)
    private var kolVibrator: Vibrator? = null
    private var pendingApk: File? = null
    private var googleHesap: GoogleHesap? = null
    private var mikrofonIzni: MethodChannel.Result? = null
    private var sesAkisi: EventChannel.EventSink? = null
    private var kayit: AudioRecord? = null
    private var sesIsci: Thread? = null
    @Volatile private var sesCalisiyor = false
    @Volatile private var sesNesli = 0

    private fun mikrofonDurdur() {
        sesNesli++
        sesCalisiyor = false
        val eski = kayit
        try { eski?.stop() } catch (_: Exception) { }
        sesIsci?.join(500)
        sesIsci = null
        // Worker releases its own recorder after the blocking read ends.
        kayit = null
    }

    private fun mikrofonBaslat() {
        if (sesCalisiyor) return
        check(sesAkisi != null)
        val tampon = maxOf(2560, AudioRecord.getMinBufferSize(16000,
            AudioFormat.CHANNEL_IN_MONO, AudioFormat.ENCODING_PCM_16BIT))
        val yeni = AudioRecord(MediaRecorder.AudioSource.VOICE_COMMUNICATION,
            16000, AudioFormat.CHANNEL_IN_MONO, AudioFormat.ENCODING_PCM_16BIT, tampon)
        try {
            check(yeni.state == AudioRecord.STATE_INITIALIZED)
            yeni.startRecording()
            check(yeni.recordingState == AudioRecord.RECORDSTATE_RECORDING)
        } catch (e: Exception) { yeni.release(); throw e }
        kayit = yeni
        sesCalisiyor = true
        val nesil = ++sesNesli
        sesIsci = Thread({
            try {
                var parca = ByteArray(640)
                var dolu = 0
                while (sesCalisiyor && sesNesli == nesil) {
                    val n = yeni.read(parca, dolu, 640 - dolu)
                    if (n <= 0) break
                    dolu += n
                    if (dolu == 640) {
                        val hazir = parca
                        runOnUiThread {
                            if (sesNesli == nesil && sesCalisiyor) sesAkisi?.success(hazir)
                        }
                        parca = ByteArray(640)
                        dolu = 0
                    }
                }
            } catch (_: Exception) {
                // Report on the platform thread below, never crash the audio worker.
            } finally {
                yeni.release()
                runOnUiThread {
                    if (sesNesli == nesil && sesCalisiyor) {
                        sesCalisiyor = false
                        sesAkisi?.error("mikrofon", "Mikrofon durdu. Yeniden basılı tut.", null)
                    }
                }
            }
        }, "AfuDesk-mikrofon").also { it.start() }
    }

    override fun onRequestPermissionsResult(code: Int, permissions: Array<out String>, grants: IntArray) {
        super.onRequestPermissionsResult(code, permissions, grants)
        if (code == 805) {
            mikrofonIzni?.success(grants.isNotEmpty() && grants[0] == PackageManager.PERMISSION_GRANTED)
            mikrofonIzni = null
        }
    }

    override fun onPause() {
        mikrofonDurdur()
        super.onPause()
    }

    override fun configureFlutterEngine(flutterEngine: FlutterEngine) {
        super.configureFlutterEngine(flutterEngine)
        EventChannel(flutterEngine.dartExecutor.binaryMessenger, "afudesk/mikrofon/ses")
            .setStreamHandler(object : EventChannel.StreamHandler {
                override fun onListen(args: Any?, sink: EventChannel.EventSink) { sesAkisi = sink }
                override fun onCancel(args: Any?) { mikrofonDurdur(); sesAkisi = null }
            })
        MethodChannel(flutterEngine.dartExecutor.binaryMessenger, "afudesk/mikrofon")
            .setMethodCallHandler { call, result ->
                try {
                    when (call.method) {
                        "izin" -> {
                            if (Build.VERSION.SDK_INT < 23 || checkSelfPermission(Manifest.permission.RECORD_AUDIO) == PackageManager.PERMISSION_GRANTED) {
                                result.success(true)
                            } else if (mikrofonIzni != null) result.success(false)
                            else {
                                mikrofonIzni = result
                                requestPermissions(arrayOf(Manifest.permission.RECORD_AUDIO), 805)
                            }
                        }
                        "baslat" -> { mikrofonBaslat(); result.success(null) }
                        "durdur" -> { mikrofonDurdur(); result.success(null) }
                        else -> result.notImplemented()
                    }
                } catch (_: Exception) {
                    result.error("mikrofon", "Mikrofon açılamadı. Yeniden basılı tut.", null)
                }
            }
        googleHesap = GoogleHesap(this)
        MethodChannel(flutterEngine.dartExecutor.binaryMessenger, "afudesk/google")
            .setMethodCallHandler { call, result -> googleHesap!!.handle(call, result) }
        GoogleSecureStorage(this, flutterEngine.dartExecutor.binaryMessenger)
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

    override fun onDestroy() {
        mikrofonDurdur()
        mikrofonIzni?.success(false)
        mikrofonIzni = null
        googleHesap?.close()
        super.onDestroy()
    }

    private fun installUpdate(apk: File) {
        val uri = FileProvider.getUriForFile(this, "$packageName.updates", apk)
        startActivity(Intent(Intent.ACTION_VIEW).setDataAndType(uri, "application/vnd.android.package-archive")
            .addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION))
        pendingApk = null
    }

    override fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?) {
        super.onActivityResult(requestCode, resultCode, data)
        googleHesap?.onActivityResult(requestCode, data)
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
