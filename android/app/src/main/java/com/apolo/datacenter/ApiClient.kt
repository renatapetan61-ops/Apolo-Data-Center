package com.apolo.datacenter

import org.json.JSONObject
import org.json.JSONArray
import java.net.HttpURLConnection
import java.net.URL

data class ProjectInfo(val id: String, val name: String, val description: String, val createdAt: String)
data class AuthSession(val accessToken: String, val name: String, val email: String)

object ApiClient {
    fun register(endpoint: String, name: String, email: String, password: String): AuthSession {
        val body = JSONObject().put("name", name).put("email", email).put("password", password).toString()
        return parseSession(request(endpoint, "/api/v1/auth/register", "POST", null, body))
    }
    fun login(endpoint: String, email: String, password: String): AuthSession {
        val body = JSONObject().put("email", email).put("password", password).toString()
        return parseSession(request(endpoint, "/api/v1/auth/login", "POST", null, body))
    }
    private fun parseSession(raw: String): AuthSession {
        val json = JSONObject(raw)
        val user = json.getJSONObject("user")
        return AuthSession(json.getString("access_token"), user.optString("name"), user.getString("email"))
    }
    fun createApiToken(endpoint: String, sessionToken: String, name: String): String {
        val body = JSONObject().put("name", name).toString()
        return JSONObject(request(endpoint, "/api/v1/tokens", "POST", sessionToken, body)).getString("token")
    }
    fun fetchProjects(endpoint: String, token: String): List<ProjectInfo> {
        val json = JSONArray(request(endpoint, "/api/v1/projects", "GET", token, null))
        return (0 until json.length()).map { i ->
            val item = json.getJSONObject(i)
            ProjectInfo(item.getString("id"), item.getString("name"), item.optString("description"), item.optString("created_at"))
        }
    }
    fun createProject(endpoint: String, token: String, name: String, description: String): List<ProjectInfo> {
        val body = JSONObject().put("name", name).put("description", description).toString()
        request(endpoint, "/api/v1/projects", "POST", token, body)
        return fetchProjects(endpoint, token)
    }
    fun deleteProject(endpoint: String, token: String, projectId: String): List<ProjectInfo> {
        request(endpoint, "/api/v1/projects/$projectId", "DELETE", token, null)
        return fetchProjects(endpoint, token)
    }
    private fun request(endpoint: String, path: String, method: String, token: String?, body: String?): String {
        require(token.isNullOrBlank() || token.length >= 32) { "Token de autenticação inválido." }
        val base = endpoint.trim().trimEnd('/')
        require(base.startsWith("https://") || base.startsWith("http://10.0.2.2") ||
            base.startsWith("http://127.0.0.1") || base.startsWith("http://localhost")) {
            "Conexões remotas exigem HTTPS."
        }
        val connection = (URL(base + path).openConnection() as HttpURLConnection).apply {
            requestMethod = method
            connectTimeout = 12_000
            readTimeout = 20_000
            setRequestProperty("Accept", "application/json")
            if (!token.isNullOrBlank()) setRequestProperty("Authorization", "Bearer $token")
            if (body != null) {
                doOutput = true
                setRequestProperty("Content-Type", "application/json; charset=utf-8")
            }
        }
        try {
            if (body != null) connection.outputStream.use { it.write(body.toByteArray(Charsets.UTF_8)) }
            val code = connection.responseCode
            val stream = if (code in 200..299) connection.inputStream else connection.errorStream
            val response = stream?.bufferedReader()?.use { it.readText() }.orEmpty()
            check(code in 200..299) {
                when (code) {
                    400 -> "Confira os dados. A senha deve ter pelo menos 10 caracteres."
                    401 -> "E-mail, senha ou token inválido."
                    404 -> "Recurso não encontrado."
                    409 -> "Este e-mail já possui cadastro."
                    503 -> "O servidor ainda não está configurado com PostgreSQL. O cadastro/login precisa do banco de dados."
                    else -> "Servidor respondeu HTTP $code."
                }
            }
            return response
        } finally { connection.disconnect() }
    }
}
