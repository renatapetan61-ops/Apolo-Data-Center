package com.apolo.datacenter

import org.json.JSONArray
import org.json.JSONObject
import java.net.HttpURLConnection
import java.net.URL

data class ProjectInfo(
    val id: String,
    val name: String,
    val description: String,
    val createdAt: String
)

object ApiClient {
    fun fetchProjects(endpoint: String, token: String): List<ProjectInfo> {
        val response = request(endpoint, "/api/v1/projects", "GET", token, null)
        val json = JSONArray(response)
        return (0 until json.length()).map { index ->
            val item = json.getJSONObject(index)
            ProjectInfo(
                id = item.getString("id"),
                name = item.getString("name"),
                description = item.optString("description"),
                createdAt = item.optString("created_at")
            )
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

    private fun request(
        endpoint: String,
        path: String,
        method: String,
        token: String,
        body: String?
    ): String {
        require(token.length >= 32) { "Configure um token de API com pelo menos 32 caracteres." }
        val base = endpoint.trim().trimEnd('/')
        require(base.startsWith("https://") || base.startsWith("http://10.0.2.2") ||
            base.startsWith("http://127.0.0.1") || base.startsWith("http://localhost")) {
            "Conexões remotas exigem HTTPS."
        }
        val connection = (URL(base + path).openConnection() as HttpURLConnection).apply {
            requestMethod = method
            connectTimeout = 10_000
            readTimeout = 15_000
            setRequestProperty("Accept", "application/json")
            setRequestProperty("Authorization", "Bearer $token")
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
                    401 -> "Token de API inválido ou não autorizado."
                    503 -> "API ou banco de dados ainda não configurado."
                    404 -> "Projeto não encontrado."
                    else -> "Servidor respondeu HTTP $code."
                }
            }
            return response
        } finally {
            connection.disconnect()
        }
    }
}
