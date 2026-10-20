package com.afu.afudesk

import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import java.io.File
import org.junit.After
import org.junit.Assert.*
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class GoogleGuvenliDepoTest {
    private val context = InstrumentationRegistry.getInstrumentation().targetContext
    private val depo = GoogleGuvenliDepo(context)
    private val path = File(context.noBackupFilesDir, "google-hesap.v1")

    @Before fun temizle() { depo.sil() }
    @After fun bitir() { depo.sil() }

    @Test fun sifreli_ve_her_yazmada_farkli() {
        val secret = "test-account-and-pairing-record".toByteArray()
        depo.yaz(secret)
        val first = path.readBytes()
        assertFalse(first.toString(Charsets.ISO_8859_1).contains(String(secret)))
        assertArrayEquals(secret, GoogleGuvenliDepo(context).oku())
        depo.yaz(secret)
        assertFalse(first.contentEquals(path.readBytes()))
        assertArrayEquals(secret, depo.oku())
    }

    @Test fun bozuk_kayit_yetki_vermez() {
        depo.yaz("test".toByteArray())
        val bytes = path.readBytes()
        bytes[bytes.lastIndex] = (bytes.last().toInt() xor 1).toByte()
        path.writeBytes(bytes)
        try { depo.oku(); fail("Bozuk kayıt açılmamalı") } catch (_: java.security.GeneralSecurityException) { }
    }

    @Test fun cikis_anahtari_da_siler_eski_dosya_geri_gelse_acilmaz() {
        depo.yaz("test".toByteArray())
        val bytes = path.readBytes()
        depo.sil()
        assertNull(depo.oku())
        path.writeBytes(bytes)
        try { depo.oku(); fail("Eski kayıt açılamamalı") } catch (_: IllegalStateException) { }
    }
}
