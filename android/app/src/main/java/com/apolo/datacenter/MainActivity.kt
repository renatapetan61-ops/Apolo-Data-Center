package com.apolo.datacenter

import android.app.Activity
import android.os.Bundle
import android.graphics.Typeface
import android.view.Gravity
import android.view.View
import android.widget.Button
import android.widget.EditText
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.TextView
import org.json.JSONArray
import org.json.JSONObject
import java.net.HttpURLConnection
import java.net.URL
import java.util.concurrent.Executors

class MainActivity : Activity() {
    private val executor = Executors.newSingleThreadExecutor()
    private lateinit var endpoint: EditText
    private lateinit var status: TextView
    private lateinit var nodes: TextView
    private lateinit var details: TextView

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        window.statusBarColor = android.graphics.Color.rgb(11, 18, 32)
        window.navigationBarColor = android.graphics.Color.rgb(11, 18, 32)

        val root = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(24, 28, 24, 24)
            setBackgroundColor(android.graphics.Color.rgb(11, 18, 32))
        }
        fun label(value: String, size: Float, color: Int, bold: Boolean = false) =
            TextView(this).apply {
                text = value
                textSize = size
                setTextColor(color)
                if (bold) typeface = Typeface.DEFAULT_BOLD
                setPadding(0, 8, 0, 8)
            }
        val blue = android.graphics.Color.rgb(138, 180, 255)
        val white = android.graphics.Color.rgb(232, 238, 248)
        val muted = android.graphics.Color.rgb(159, 176, 200)

        root.addView(label("APOLO / INFRASTRUCTURE", 12f, blue, true))
        root.addView(label("Data Center", 30f, white, true))
        root.addView(label("Aplicativo Android nativo · Control plane AX", 14f, muted))
        status = label("Aguardando conexão", 14f, blue, true)
        root.addView(status)

        root.addView(label("Endereço da API", 13f, muted, true))
        endpoint = EditText(this).apply {
            setSingleLine(true)
            setText("https://api.apolo-datacenter.example")
            setTextColor(white)
            setHintTextColor(muted)
            hint = "https://seu-servidor"
            setPadding(14, 10, 14, 10)
        }
        root.addView(endpoint, LinearLayout.LayoutParams(-1, -2))

        root.addView(Button(this).apply {
            text = "Conectar e atualizar"
            setOnClickListener { refresh() }
        })
        nodes = label("Nós registrados: —", 22f, white, true)
        root.addView(nodes)
        root.addView(label("Inventário real retornado pelo servidor", 13f, muted))
        details = label("Nenhum dado carregado.", 13f, white)
        val scroll = ScrollView(this)
        scroll.addView(details)
        root.addView(scroll, LinearLayout.LayoutParams(-1, 0, 1f))
        root.addView(label(
            "A capacidade física só aparece após inventário verificado. Esta versão não cria nem simula hardware.",
            12f, muted
        ))
        setContentView(root)
    }

    private fun refresh() {
        val base = endpoint.text.toString().trim().trimEnd('/')
        if (!base.startsWith("https://") &&
            !base.startsWith("http://10.0.2.2") &&
            !base.startsWith("http://127.0.0.1") &&
            !base.startsWith("http://localhost")) {
            status.text = "Use HTTPS para conexões remotas."
            return
        }
        status.text = "Conectando…"
        executor.execute {
            try {
                val health = get("$base/healthz")
                val overview = JSONObject(get("$base/api/v1/overview"))
                val inventory = JSONArray(get("$base/api/v1/nodes"))
                runOnUiThread {
                    status.text = "API online · ${health.optString("status")}"
                    nodes.text = "Nós registrados: ${overview.optInt("node_count")}"
                    details.text = buildString {
                        append("Serviço: ").append(overview.optString("service")).append("\n")
                        append("Control plane: ").append(overview.optString("control_plane")).append("\n")
                        append("Persistência: ").append(overview.optString("persistence")).append("\n\n")
                        if (inventory.length() == 0) append("Nenhum nó registrado.")
                        else for (i in 0 until inventory.length()) {
                            append(inventory.getJSONObject(i).toString(2)).append("\n\n")
                        }
                    }
                }
            } catch (error: Exception) {
                runOnUiThread {
                    status.text = "Falha de conexão: ${error.javaClass.simpleName}"
                    nodes.text = "Nós registrados: indisponível"
                    details.text = "Confira o endereço, a conexão de rede e se a API está ativa com HTTPS."
                }
            }
        }
    }

    private fun get(address: String): String {
        val connection = (URL(address).openConnection() as HttpURLConnection).apply {
            requestMethod = "GET"
            connectTimeout = 8_000
            readTimeout = 8_000
            setRequestProperty("Accept", "application/json")
        }
        try {
            val code = connection.responseCode
            val stream = if (code in 200..299) connection.inputStream else connection.errorStream
            val body = stream.bufferedReader().use { it.readText() }
            if (code !in 200..299) error("HTTP $code")
            return body
        } finally {
            connection.disconnect()
        }
    }

    override fun onDestroy() {
        executor.shutdownNow()
        super.onDestroy()
    }
}
