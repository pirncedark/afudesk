package com.afu.afudesk

import android.content.Context
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import android.util.Base64
import java.security.KeyStore
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec
import io.flutter.plugin.common.BinaryMessenger
import io.flutter.plugin.common.MethodChannel

/** One encrypted UTF-8 session bundle; never logs arguments or plaintext. */
internal class GoogleSecureStorage(context: Context, messenger: BinaryMessenger) {
    private val prefs = context.getSharedPreferences("afudesk_google_secure", Context.MODE_PRIVATE)
    private val keyStore = KeyStore.getInstance("AndroidKeyStore").apply { load(null) }
    private val alias = "afudesk.google.session.v1"
    private val aad = alias.toByteArray(Charsets.UTF_8)

    init {
        MethodChannel(messenger, "afudesk/google_config").setMethodCallHandler { call, result ->
            if (call.method == "read") {
                result.success(mapOf("webClientId" to context.getString(R.string.afudesk_google_web_client_id)))
            } else result.notImplemented()
        }
        MethodChannel(messenger, "afudesk/secure_storage").setMethodCallHandler { call, result ->
            try {
                when (call.method) {
                    "write" -> {
                        val value = call.arguments as? String ?: error("argument")
                        val bytes = value.toByteArray(Charsets.UTF_8)
                        require(bytes.size in 1..65536)
                        try {
                            val cipher = Cipher.getInstance("AES/GCM/NoPadding")
                            cipher.init(Cipher.ENCRYPT_MODE, key(create = true))
                            cipher.updateAAD(aad)
                            val blob = byteArrayOf(1) + cipher.iv + cipher.doFinal(bytes)
                            check(prefs.edit().putString("bundle", Base64.encodeToString(blob, Base64.NO_WRAP)).commit())
                        } finally { bytes.fill(0) }
                        result.success(null)
                    }
                    "read" -> {
                        val encoded = prefs.getString("bundle", null)
                        if (encoded == null) result.success(null) else {
                            val blob = Base64.decode(encoded, Base64.NO_WRAP)
                            require(blob.size in 30..65565 && blob[0] == 1.toByte())
                            val cipher = Cipher.getInstance("AES/GCM/NoPadding")
                            cipher.init(Cipher.DECRYPT_MODE, key(create = false), GCMParameterSpec(128, blob.copyOfRange(1, 13)))
                            cipher.updateAAD(aad)
                            val bytes = cipher.doFinal(blob.copyOfRange(13, blob.size))
                            try { result.success(bytes.toString(Charsets.UTF_8)) } finally { bytes.fill(0) }
                        }
                    }
                    "clear" -> {
                        // Remove the key first, so a failed preferences write cannot recover secrets.
                        keyStore.deleteEntry(alias)
                        check(prefs.edit().clear().commit())
                        result.success(null)
                    }
                    else -> result.notImplemented()
                }
            } catch (_: Exception) {
                result.error("secure_storage", "Oturum kaydı açılamadı; yeniden giriş yap.", null)
            }
        }
    }

    private fun key(create: Boolean): SecretKey {
        (keyStore.getKey(alias, null) as? SecretKey)?.let { return it }
        check(create) // A missing key must never silently replace a restored session key.
        return KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, "AndroidKeyStore").apply {
            init(KeyGenParameterSpec.Builder(alias, KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT)
                .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
                .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
                .setKeySize(256)
                .setUserAuthenticationRequired(false)
                .build())
        }.generateKey()
    }
}
