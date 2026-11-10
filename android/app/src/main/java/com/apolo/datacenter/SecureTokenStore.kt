package com.apolo.datacenter

import android.content.Context
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import android.util.Base64
import java.security.KeyStore
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec

object SecureTokenStore {
    private const val API_ALIAS = "apolo_api_token_v1"
    private const val SESSION_ALIAS = "apolo_session_token_v1"
    private const val PREFS = "apolo_secure_settings"
    private const val API_TOKEN = "encrypted_api_token"
    private const val SESSION_TOKEN = "encrypted_session_token"
    private const val IV_LENGTH = 12

    private fun key(alias: String): SecretKey {
        val store = KeyStore.getInstance("AndroidKeyStore").apply { load(null) }
        val existing = store.getKey(alias, null) as? SecretKey
        if (existing != null) return existing
        val generator = KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, "AndroidKeyStore")
        generator.init(KeyGenParameterSpec.Builder(alias, KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT)
            .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
            .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
            .setRandomizedEncryptionRequired(true).build())
        return generator.generateKey()
    }

    private fun readValue(context: Context, pref: String, alias: String): String {
        val encoded = context.getSharedPreferences(PREFS, Context.MODE_PRIVATE).getString(pref, null) ?: return ""
        return runCatching {
            val payload = Base64.decode(encoded, Base64.NO_WRAP)
            require(payload.size > IV_LENGTH)
            val cipher = Cipher.getInstance("AES/GCM/NoPadding")
            cipher.init(Cipher.DECRYPT_MODE, key(alias), GCMParameterSpec(128, payload.copyOfRange(0, IV_LENGTH)))
            String(cipher.doFinal(payload.copyOfRange(IV_LENGTH, payload.size)), Charsets.UTF_8)
        }.getOrDefault("")
    }

    private fun writeValue(context: Context, value: String, pref: String, alias: String) {
        val prefs = context.getSharedPreferences(PREFS, Context.MODE_PRIVATE)
        if (value.isBlank()) { prefs.edit().remove(pref).apply(); return }
        require(value.length >= 32) { "O token precisa ter pelo menos 32 caracteres." }
        val cipher = Cipher.getInstance("AES/GCM/NoPadding")
        cipher.init(Cipher.ENCRYPT_MODE, key(alias))
        val encrypted = cipher.iv + cipher.doFinal(value.toByteArray(Charsets.UTF_8))
        prefs.edit().putString(pref, Base64.encodeToString(encrypted, Base64.NO_WRAP)).apply()
    }

    fun read(context: Context): String = readValue(context, API_TOKEN, API_ALIAS)
    fun write(context: Context, value: String) = writeValue(context, value, API_TOKEN, API_ALIAS)
    fun readSession(context: Context): String = readValue(context, SESSION_TOKEN, SESSION_ALIAS)
    fun writeSession(context: Context, value: String) = writeValue(context, value, SESSION_TOKEN, SESSION_ALIAS)
}
