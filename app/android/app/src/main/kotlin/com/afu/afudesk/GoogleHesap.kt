package com.afu.afudesk

import android.os.CancellationSignal
import androidx.core.content.ContextCompat
import androidx.credentials.ClearCredentialStateRequest
import androidx.credentials.CredentialManager
import androidx.credentials.CredentialManagerCallback
import androidx.credentials.CustomCredential
import androidx.credentials.GetCredentialRequest
import androidx.credentials.GetCredentialResponse
import androidx.credentials.exceptions.ClearCredentialException
import androidx.credentials.exceptions.GetCredentialException
import com.google.android.libraries.identity.googleid.GetSignInWithGoogleOption
import com.google.android.libraries.identity.googleid.GoogleIdTokenCredential
import io.flutter.embedding.android.FlutterActivity
import io.flutter.plugin.common.MethodChannel

/** Yalnız platform sınırı. Dönen ID token Rust/JWKS doğrulamasından geçmeden
 * hesap veya onaysız bağlantı sayılmaz; token loglanmaz veya burada saklanmaz. */
class GoogleHesap(private val activity: FlutterActivity) {
    private val manager = CredentialManager.create(activity)
    private val executor = ContextCompat.getMainExecutor(activity)
    private val depo = GoogleGuvenliDepo(activity)
    private var pending: CancellationSignal? = null
    private var generation = 0L

    fun handle(call: io.flutter.plugin.common.MethodCall, result: MethodChannel.Result) {
        try {
            when (call.method) {
                "giris" -> {
                    if (pending != null) { fail(result); return }
                    val requestGeneration = ++generation
                    val nonce = requireNotNull(call.argument<String>("nonce"))
                    require(nonce.length in 43..256)
                    val id = BuildConfig.GOOGLE_WEB_CLIENT_ID
                    check(id.endsWith(".apps.googleusercontent.com"))
                    val option = GetSignInWithGoogleOption.Builder(id).setNonce(nonce).build()
                    val request = GetCredentialRequest.Builder().addCredentialOption(option).build()
                    val signal = CancellationSignal()
                    pending = signal
                    manager.getCredentialAsync(activity, request, signal, executor,
                        object : CredentialManagerCallback<GetCredentialResponse, GetCredentialException> {
                            override fun onResult(response: GetCredentialResponse) {
                                if (generation != requestGeneration) { fail(result); return }
                                pending = null
                                try {
                                    val credential = response.credential
                                    check(credential is CustomCredential && credential.type == GoogleIdTokenCredential.TYPE_GOOGLE_ID_TOKEN_CREDENTIAL)
                                    result.success(GoogleIdTokenCredential.createFrom(credential.data).idToken)
                                } catch (_: Exception) { fail(result) }
                            }
                            override fun onError(e: GetCredentialException) {
                                if (generation == requestGeneration) pending = null
                                fail(result)
                            }
                        })
                }
                "depoOku" -> result.success(depo.oku())
                "depoYaz" -> {
                    depo.yaz(requireNotNull(call.argument<ByteArray>("veri")))
                    result.success(null)
                }
                "cikis" -> {
                    generation++
                    pending?.cancel()
                    pending = null
                    depo.sil()
                    manager.clearCredentialStateAsync(ClearCredentialStateRequest(), null, executor,
                        object : CredentialManagerCallback<Void?, ClearCredentialException> {
                            override fun onResult(value: Void?) { result.success(null) }
                            override fun onError(e: ClearCredentialException) { fail(result) }
                        })
                }
                else -> result.notImplemented()
            }
        } catch (_: Exception) { pending = null; fail(result) }
    }

    fun close() { generation++; pending?.cancel(); pending = null }

    private fun fail(result: MethodChannel.Result) {
        result.error("google", "Google işlemi tamamlanamadı. Yeniden dene.", null)
    }
}
