package com.afu.afudesk

import android.content.Context
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import android.util.AtomicFile
import java.io.File
import java.security.KeyStore
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec

/** AES-GCM anahtarı Keystore dışına çıkmaz. noBackupFilesDir, cihazlar arası
 * restore ile eşleşme/PIN yetkilerinin geri gelmesini önler. Açık metin yedeği yok. */
class GoogleGuvenliDepo(context: Context) {
    private val file = AtomicFile(File(context.noBackupFilesDir, "google-hesap.v1"))
    private val alias = "afudesk.google.v1"
    private val max = 1024 * 1024

    private fun store(): KeyStore = KeyStore.getInstance("AndroidKeyStore").apply { load(null) }
    private fun key(create: Boolean): SecretKey {
        (store().getKey(alias, null) as? SecretKey)?.let { return it }
        check(create) // Kayıp anahtarda yeni anahtar üretip eski kaydı kabul etme.
        return KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, "AndroidKeyStore").apply {
            init(KeyGenParameterSpec.Builder(alias, KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT)
                .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
                .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
                .setRandomizedEncryptionRequired(true).build())
        }.generateKey()
    }

    @Synchronized fun yaz(veri: ByteArray) {
        require(veri.size <= max)
        val cipher = Cipher.getInstance("AES/GCM/NoPadding")
        cipher.init(Cipher.ENCRYPT_MODE, key(true))
        cipher.updateAAD(alias.toByteArray(Charsets.UTF_8))
        val encrypted = cipher.doFinal(veri)
        val out = file.startWrite()
        try {
            out.write(byteArrayOf(1))
            out.write(cipher.iv) // GCM varsayılan 12 bayt IV, her yazmada rastgele.
            out.write(encrypted)
            file.finishWrite(out)
        } catch (e: Exception) { file.failWrite(out); throw e }
    }

    @Synchronized fun oku(): ByteArray? {
        if (!file.baseFile.exists()) return null
        val bytes = file.openRead().use { input ->
            val out = java.io.ByteArrayOutputStream()
            val buffer = ByteArray(8192)
            while (true) {
                val count = input.read(buffer)
                if (count < 0) break
                require(out.size() + count <= max + 29)
                out.write(buffer, 0, count)
            }
            out.toByteArray()
        }
        require(bytes.size >= 29 && bytes[0].toInt() == 1)
        val cipher = Cipher.getInstance("AES/GCM/NoPadding")
        cipher.init(Cipher.DECRYPT_MODE, key(false), GCMParameterSpec(128, bytes.copyOfRange(1, 13)))
        cipher.updateAAD(alias.toByteArray(Charsets.UTF_8))
        return cipher.doFinal(bytes.copyOfRange(13, bytes.size))
    }

    @Synchronized fun sil() {
        // Önce anahtarı sil: dosya silme hatasında bile eski kayıt açılamaz.
        store().deleteEntry(alias)
        file.delete()
        check(!file.baseFile.exists())
    }
}
