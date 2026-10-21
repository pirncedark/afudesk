package com.afu.afudesk

import android.os.CancellationSignal
import android.accounts.Account
import android.content.Intent
import com.google.android.gms.auth.api.identity.AuthorizationRequest
import com.google.android.gms.auth.api.identity.AuthorizationResult
import com.google.android.gms.auth.api.identity.Identity
import com.google.android.gms.common.api.Scope
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
    private var authorizationResult: MethodChannel.Result? = null
    private var authorizationToken: String? = null
    private var authorizationGeneration = 0L

    fun handle(call: io.flutter.plugin.common.MethodCall, result: MethodChannel.Result) {
        try {
            when (call.method) {
                "giris" -> {
                    if (pending != null || authorizationResult != null) { fail(result); return }
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
                                    val google = GoogleIdTokenCredential.createFrom(credential.data)
                                    authorizationResult = result
                                    authorizationToken = google.idToken
                                    authorizationGeneration = requestGeneration
                                    val auth = AuthorizationRequest.builder()
                                        .setAccount(Account(google.id, "com.google"))
                                        .setRequestedScopes(listOf(Scope("https://www.googleapis.com/auth/drive.appdata"), Scope("openid")))
                                        .build()
                                    Identity.getAuthorizationClient(activity).authorize(auth)
                                        .addOnSuccessListener { value ->
                                            if (generation != requestGeneration) return@addOnSuccessListener
                                            try {
                                                if (value.hasResolution()) {
                                                    activity.startIntentSenderForResult(requireNotNull(value.pendingIntent).intentSender,
                                                        805, null, 0, 0, 0)
                                                } else authorized(value)
                                            } catch (_: Exception) { authorizationFailed() }
                                        }
                                        .addOnFailureListener { if (generation == requestGeneration) authorizationFailed() }
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
                    authorizationFailed()
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

    fun onActivityResult(requestCode: Int, data: Intent?) {
        if (requestCode != 805 || authorizationResult == null) return
        try {
            check(generation == authorizationGeneration)
            authorized(Identity.getAuthorizationClient(activity).getAuthorizationResultFromIntent(data))
        } catch (_: Exception) { authorizationFailed() }
    }

    private fun authorized(value: AuthorizationResult) {
        val result = authorizationResult ?: return
        try {
            check(generation == authorizationGeneration)
            val access = requireNotNull(value.accessToken)
            val id = requireNotNull(authorizationToken)
            authorizationResult = null; authorizationToken = null
            result.success(mapOf("idToken" to id, "accessToken" to access))
        } catch (_: Exception) { authorizationFailed() }
    }

    private fun authorizationFailed() {
        val result = authorizationResult
        authorizationResult = null; authorizationToken = null
        if (result != null) fail(result)
    }

    fun close() { generation++; pending?.cancel(); pending = null; authorizationFailed() }

    private fun fail(result: MethodChannel.Result) {
        result.error("google", "Google işlemi tamamlanamadı. Yeniden dene.", null)
    }
}
