package com.apolo.datacenter

import android.content.Context
import android.os.Bundle
import android.net.Uri
import android.provider.OpenableColumns
import android.graphics.Color as AndroidColor
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import org.json.JSONArray
import org.json.JSONObject
import java.net.HttpURLConnection
import java.net.URL
import java.util.concurrent.Executors

private val Navy = Color(0xFF0B1220)
private val Panel = Color(0xFF141F31)
private val PanelAlt = Color(0xFF1B2A40)
private val Accent = Color(0xFF78A9FF)
private val TextPrimary = Color(0xFFEAF1FC)
private val TextMuted = Color(0xFF9BAEC8)
private val Good = Color(0xFF62D6A5)
private val Warning = Color(0xFFF3C46B)

private enum class AppTab(val label: String, val symbol: String) {
    HOME("Início", "⌂"), STORAGE("Dados", "▤"), COMPUTE("Compute", "◈"),
    PROJECTS("Projetos", "▦"), SETTINGS("Ajustes", "⚙")
}

private data class NodeInfo(val id: String, val state: String, val detail: String)
private data class ApiSnapshot(
    val status: String,
    val service: String,
    val controlPlane: String,
    val persistence: String,
    val nodes: List<NodeInfo>
)

class MainActivity : ComponentActivity() {
    private val executor = Executors.newFixedThreadPool(4)
    private var resumeTick by mutableIntStateOf(0)

    override fun onResume() {
        super.onResume()
        // Recheck the backend every time the app returns to the foreground.
        resumeTick++
    }
    private val preferences by lazy { getSharedPreferences("apolo_settings", Context.MODE_PRIVATE) }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        window.statusBarColor = AndroidColor.rgb(11, 18, 32)
        window.navigationBarColor = AndroidColor.rgb(11, 18, 32)
        val initialEndpoint = preferences.getString("api_endpoint", "https://apolo-ax-api-production.up.railway.app")
            ?: "https://apolo-ax-api-production.up.railway.app"
        val initialToken = SecureTokenStore.read(this)
        val initialSession = SecureTokenStore.readSession(this)
        setContent {
            ApoloTheme {
                ApoloApplication(
                    initialEndpoint = initialEndpoint,
                    initialToken = initialToken,
                    initialSession = initialSession,
                    resumeTick = resumeTick,
                    onSaveEndpoint = { value -> preferences.edit().putString("api_endpoint", value).apply() },
                    onSaveSession = { value -> SecureTokenStore.writeSession(this, value) },
                    onSaveToken = { value -> SecureTokenStore.write(this, value) },
                    onConnect = { endpoint, callback ->
                        executor.execute {
                            val result = runCatching { fetchSnapshot(endpoint) }
                            runOnUiThread { callback(result) }
                        }
                    },
                    onValidateSession = { endpoint, session, callback ->
                        executor.execute {
                            val result = runCatching { ApiClient.validateSession(endpoint, session) }
                            runOnUiThread { callback(result) }
                        }
                    },
                    onAuthenticate = { endpoint, name, email, password, register, callback ->
                        executor.execute {
                            val result = runCatching {
                                if (register) ApiClient.register(endpoint, name, email, password)
                                else ApiClient.login(endpoint, email, password)
                            }
                            runOnUiThread { callback(result) }
                        }
                    },
                    onGenerateToken = { endpoint, session, label, callback ->
                        executor.execute {
                            val result = runCatching { ApiClient.createApiToken(endpoint, session, label) }
                            runOnUiThread { callback(result) }
                        }
                    },
                    onProjectAction = { endpoint, token, action, name, description, projectId, callback ->
                        executor.execute {
                            val result = runCatching {
                                when (action) {
                                    ProjectAction.LOAD -> ApiClient.fetchProjects(endpoint, token)
                                    ProjectAction.CREATE -> ApiClient.createProject(endpoint, token, name, description)
                                    ProjectAction.DELETE -> ApiClient.deleteProject(endpoint, token, projectId)
                                }
                            }
                            runOnUiThread { callback(result) }
                        }
                    },
                    onWorkspaceAction = { endpoint, token, projectId, action, payload, callback ->
                        executor.execute {
                            val result = runCatching {
                                when (action) {
                                    "LOAD" -> { val w=ApiClient.workspace(endpoint,token,projectId); JSONObject().put("files",w.files).put("models",w.models).put("jobs",w.jobs).toString() }
                                    "CHAT" -> {
                                        val chat = JSONObject(payload)
                                        ApiClient.askKimi(
                                            endpoint, token, projectId,
                                            chat.getString("message"),
                                            chat.optJSONArray("history") ?: JSONArray()
                                        )
                                    }
                                    "MODEL" -> { val rest=payload.substringAfter("\n"); ApiClient.generateModel(endpoint,token,projectId,payload.substringBefore("\n"),rest.substringBefore("\n").toLongOrNull() ?: 0L,rest.substringAfter("\n","")) }
                                    "TRAIN" -> ApiClient.startTraining(endpoint,token,projectId,payload)
                                    "VTA" -> ApiClient.vtaPlan(endpoint,token,payload)
                                    else -> error("Ação desconhecida.")
                                }
                            }
                            runOnUiThread { callback(result) }
                        }
                    },
                    onStorageAction = { endpoint, token, initialize, callback ->
                        executor.execute {
                            val result = runCatching { ApiClient.vtaStorage(endpoint, token, initialize) }
                            runOnUiThread { callback(result) }
                        }
                    },
                    onUploadFile = { endpoint, token, projectId, uri, kind, callback ->
                        executor.execute {
                            val result = runCatching {
                                val bytes=contentResolver.openInputStream(uri)?.use { it.readBytes() } ?: error("Não foi possível ler o arquivo.")
                                val name=contentResolver.query(uri,null,null,null,null)?.use { c -> if(c.moveToFirst()) c.getString(c.getColumnIndexOrThrow(OpenableColumns.DISPLAY_NAME)) else "upload.bin" } ?: "upload.bin"
                                val type=contentResolver.getType(uri) ?: "application/octet-stream"
                                ApiClient.uploadFile(endpoint,token,projectId,name,type,kind,bytes)
                            }
                            runOnUiThread { callback(result) }
                        }
                    }
                )
            }
        }
    }

    private fun fetchSnapshot(base: String): ApiSnapshot {
        val root = base.trim().trimEnd('/')
        require(root.startsWith("https://") || root.startsWith("http://10.0.2.2") ||
            root.startsWith("http://127.0.0.1") || root.startsWith("http://localhost")) {
            "Conexões remotas exigem HTTPS."
        }
        val health = JSONObject(get("$root/healthz"))
        val overview = JSONObject(get("$root/api/v1/overview"))
        val rawNodes = JSONArray(get("$root/api/v1/nodes"))
        val parsedNodes = (0 until rawNodes.length()).map { index ->
            val node = rawNodes.getJSONObject(index)
            NodeInfo(
                id = node.optString("node_id", node.optString("id", "Nó #${index + 1}")),
                state = node.optString("state", node.optString("status", "Desconhecido")),
                detail = node.optString("architecture", node.optString("hostname", "Inventário registrado"))
            )
        }
        return ApiSnapshot(
            status = health.optString("status", "online"),
            service = overview.optString("service", "APOLO AX"),
            controlPlane = overview.optString("control_plane", "—"),
            persistence = overview.optString("persistence", "—"),
            nodes = parsedNodes
        )
    }

    private fun get(address: String): String {
        val connection = (URL(address).openConnection() as HttpURLConnection).apply {
            requestMethod = "GET"
            connectTimeout = 20_000
            readTimeout = 45_000
            setRequestProperty("Accept", "application/json")
        }
        try {
            val code = connection.responseCode
            val stream = if (code in 200..299) connection.inputStream else connection.errorStream
            val body = stream.bufferedReader().use { it.readText() }
            check(code in 200..299) { "Servidor respondeu HTTP $code." }
            return body
        } finally { connection.disconnect() }
    }

    override fun onDestroy() {
        executor.shutdownNow()
        super.onDestroy()
    }
}

@Composable
private fun ApoloTheme(content: @Composable () -> Unit) {
    MaterialTheme(
        colorScheme = darkColorScheme(
            primary = Accent, secondary = Good, background = Navy, surface = Panel,
            onPrimary = Navy, onBackground = TextPrimary, onSurface = TextPrimary
        ),
        content = content
    )
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun ApoloApplication(
    initialEndpoint: String,
    initialToken: String,
    initialSession: String,
    resumeTick: Int,
    onSaveEndpoint: (String) -> Unit,
    onSaveToken: (String) -> Unit,
    onSaveSession: (String) -> Unit,
    onConnect: (String, (Result<ApiSnapshot>) -> Unit) -> Unit,
    onValidateSession: (String, String, (Result<Boolean>) -> Unit) -> Unit,
    onAuthenticate: (String, String, String, String, Boolean, (Result<AuthSession>) -> Unit) -> Unit,
    onGenerateToken: (String, String, String, (Result<String>) -> Unit) -> Unit,
    onProjectAction: (String, String, ProjectAction, String, String, String, (Result<List<ProjectInfo>>) -> Unit) -> Unit,
    onWorkspaceAction: (String, String, String, String, String, (Result<String>) -> Unit) -> Unit,
    onUploadFile: (String, String, String, Uri, String, (Result<String>) -> Unit) -> Unit,
    onStorageAction: (String, String, Boolean, (Result<String>) -> Unit) -> Unit
) {
    var selectedTab by remember { mutableStateOf(AppTab.HOME) }
    var endpoint by remember { mutableStateOf(initialEndpoint) }
    var apiToken by remember { mutableStateOf(initialToken) }
    var snapshot by remember { mutableStateOf<ApiSnapshot?>(null) }
    var loading by remember { mutableStateOf(false) }
    var message by remember { mutableStateOf("API ainda não conectada") }
    var connected by remember { mutableStateOf(false) }
    var authenticated by remember { mutableStateOf(false) }
    var checkingSession by remember { mutableStateOf(initialSession.length >= 32) }
    var sessionToken by remember { mutableStateOf(initialSession) }
    var authLoading by remember { mutableStateOf(false) }
    var authMessage by remember { mutableStateOf("") }
    var showTokenScreen by remember { mutableStateOf(false) }
    var generatedToken by remember { mutableStateOf("") }
    var tokenLoading by remember { mutableStateOf(false) }
    var tokenMessage by remember { mutableStateOf("") }

    fun connect() {
        if (loading) return
        loading = true
        message = "Conectando ao control plane…"
        onConnect(endpoint) { result ->
            loading = false
            result.onSuccess {
                snapshot = it
                connected = true
                message = "API online · ${it.status}"
            }.onFailure {
                connected = false
                message = it.message ?: "Não foi possível conectar"
            }
        }
    }

    // A saved token is not automatically trusted: validate it against the API.
    // If the backend is temporarily unreachable, keep the session and show OFFLINE;
    // only an explicit 401/403 returns the user to login/registration.
    LaunchedEffect(initialSession, endpoint) {
        if (initialSession.length >= 32) {
            checkingSession = true
            onValidateSession(endpoint, initialSession) { result ->
                checkingSession = false
                result.onSuccess { valid ->
                    if (valid) {
                        sessionToken = initialSession
                        authenticated = true
                    } else {
                        sessionToken = ""
                        authenticated = false
                        apiToken = ""
                        onSaveSession("")
                        onSaveToken("")
                        authMessage = "Sua sessão expirou. Entre novamente ou crie uma conta."
                    }
                }.onFailure {
                    sessionToken = initialSession
                    authenticated = true
                    message = "Não foi possível validar a sessão porque o backend está indisponível."
                }
            }
        } else {
            checkingSession = false
            authenticated = false
        }
    }

    // The status is live, not a value persisted from the previous process.
    // Reconnect on first composition, endpoint changes and every foreground return.
    LaunchedEffect(resumeTick, endpoint) {
        connect()
    }

    if (checkingSession) {
        Box(Modifier.fillMaxSize().padding(24.dp), contentAlignment = Alignment.Center) {
            Column(horizontalAlignment = Alignment.CenterHorizontally, verticalArrangement = Arrangement.spacedBy(12.dp)) {
                CircularProgressIndicator(color = Accent)
                Text("Verificando sua sessão segura…", color = TextMuted)
            }
        }
        return
    }

    if (!authenticated) {
        AuthScreen(
            endpoint = endpoint,
            loading = authLoading,
            message = authMessage,
            onSubmit = { name, email, password, register ->
                authLoading = true
                authMessage = if (register) "Criando sua conta…" else "Validando seu acesso…"
                onAuthenticate(endpoint, name, email, password, register) { result ->
                    authLoading = false
                    result.onSuccess { session ->
                        sessionToken = session.accessToken
                        // Sempre substituir a credencial local pela da conta que acabou de entrar.
                        // Evita reaproveitar o token de outra conta e recupera acesso após reinstalação.
                        apiToken = session.apiToken.ifBlank { session.accessToken }
                        onSaveToken(apiToken)
                        onSaveSession(session.accessToken)
                        authenticated = true
                        authMessage = ""
                        message = "Bem-vindo, ${session.name}."
                    }.onFailure { error ->
                        authMessage = error.message ?: "Não foi possível autenticar."
                    }
                }
            }
        )
        return
    }

    Scaffold(
        containerColor = Navy,
        topBar = {
            TopAppBar(
                title = {
                    Column {
                        Text("APOLO", fontWeight = FontWeight.Black, letterSpacing = 2.sp, color = Accent, fontSize = 12.sp)
                        Text("Data Center", fontWeight = FontWeight.Bold, color = TextPrimary, fontSize = 21.sp)
                    }
                },
                actions = {
                    Surface(
                        color = if (connected) Good.copy(alpha = .14f) else Warning.copy(alpha = .14f),
                        shape = RoundedCornerShape(50),
                        modifier = Modifier.padding(end = 16.dp)
                    ) {
                        Text(
                            if (connected) "● ONLINE" else "● OFFLINE",
                            color = if (connected) Good else Warning,
                            fontSize = 10.sp,
                            fontWeight = FontWeight.Bold,
                            modifier = Modifier.padding(horizontal = 10.dp, vertical = 7.dp)
                        )
                    }
                },
                colors = TopAppBarDefaults.topAppBarColors(containerColor = Navy)
            )
        },
        bottomBar = {
            NavigationBar(containerColor = Panel) {
                AppTab.entries.forEach { tab ->
                    NavigationBarItem(
                        selected = selectedTab == tab,
                        onClick = { selectedTab = tab },
                        icon = { Text(tab.symbol, fontSize = 19.sp) },
                        label = { Text(tab.label, fontSize = 10.sp) },
                        colors = NavigationBarItemDefaults.colors(
                            selectedIconColor = Accent, selectedTextColor = Accent,
                            indicatorColor = Accent.copy(alpha = .14f),
                            unselectedIconColor = TextMuted, unselectedTextColor = TextMuted
                        )
                    )
                }
            }
        }
    ) { padding ->
        when (selectedTab) {
            AppTab.HOME -> HomeScreen(Modifier.padding(padding), snapshot, message, loading, ::connect) { selectedTab = it }
            AppTab.STORAGE -> VtaStorageScreen(
                modifier = Modifier.padding(padding),
                token = apiToken,
                endpoint = endpoint,
                onStorageAction = onStorageAction
            )
            AppTab.COMPUTE -> CapabilityScreen(
                Modifier.padding(padding), "Computação AX", "Execução, filas e aceleradores",
                listOf("AX Scheduler", "AX Compute", "AX Accelerator ISA", "Treinamento de modelos"),
                "O scheduler e o runtime AX existem no núcleo Rust. Execução remota multiusuário e aceleradores físicos ainda não estão habilitados."
            )
            AppTab.PROJECTS -> if (showTokenScreen) {
                CreateTokenScreen(
                    modifier = Modifier.padding(padding),
                    loading = tokenLoading,
                    message = tokenMessage,
                    token = generatedToken,
                    onBack = { showTokenScreen = false; generatedToken = ""; tokenMessage = "" },
                    onGenerate = { label ->
                        tokenLoading = true
                        tokenMessage = "Gerando credencial segura no servidor…"
                        onGenerateToken(endpoint, sessionToken, label) { result ->
                            tokenLoading = false
                            result.onSuccess {
                                generatedToken = it
                                tokenMessage = "Token criado com sucesso."
                            }.onFailure { error ->
                                tokenMessage = error.message ?: "Falha ao gerar token."
                            }
                        }
                    },
                    onUseInSettings = { value ->
                        apiToken = value
                        onSaveToken(value)
                        showTokenScreen = false
                        generatedToken = ""
                        selectedTab = AppTab.SETTINGS
                        message = "Token gerado e salvo com proteção do Android Keystore."
                    }
                )
            } else ProjectsScreen(
                modifier = Modifier.padding(padding),
                endpoint = endpoint,
                token = apiToken,
                onCreateToken = { showTokenScreen = true; generatedToken = ""; tokenMessage = "" },
                onAction = { action, name, description, projectId, callback ->
                    onProjectAction(endpoint, apiToken, action, name, description, projectId, callback)
                },
                onWorkspaceAction = { projectId, action, value, extra, callback ->
                    onWorkspaceAction(endpoint, apiToken, projectId, action, value + "\n" + extra, callback)
                },
                onUpload = { projectId, uri, kind, callback -> onUploadFile(endpoint, apiToken, projectId, uri, kind, callback) }
            )
            AppTab.SETTINGS -> SettingsScreen(
                modifier = Modifier.padding(padding),
                endpoint = endpoint,
                onEndpointChange = { endpoint = it },
                token = apiToken,
                onTokenChange = { apiToken = it },
                onSave = {
                    onSaveEndpoint(endpoint)
                    onSaveToken(apiToken)
                    message = "Endereço e token protegidos salvos neste dispositivo."
                },
                onConnect = ::connect,
                message = message,
                loading = loading,
                onLogout = {
                    authenticated = false
                    sessionToken = ""
                    onSaveSession("")
                    authMessage = "Você saiu da sua conta. O token permanece vinculado à sua conta e será restaurado no próximo acesso."
                }
            )
        }
    }
}

@Composable
private fun HomeScreen(
    modifier: Modifier, snapshot: ApiSnapshot?, message: String, loading: Boolean,
    onConnect: () -> Unit, onOpen: (AppTab) -> Unit
) {
    LazyColumn(modifier.fillMaxSize(), contentPadding = PaddingValues(18.dp), verticalArrangement = Arrangement.spacedBy(14.dp)) {
        item {
            Column(verticalArrangement = Arrangement.spacedBy(5.dp)) {
                Text("PAINEL DE CONTROLE", color = Accent, fontSize = 11.sp, fontWeight = FontWeight.Bold, letterSpacing = 1.5.sp)
                Text("Sua infraestrutura,\nem um só lugar.", color = TextPrimary, fontSize = 27.sp, lineHeight = 32.sp, fontWeight = FontWeight.Bold)
                Text("Visão operacional do APOLO AX", color = TextMuted, fontSize = 13.sp)
            }
        }
        item {
            Card(colors = CardDefaults.cardColors(containerColor = Panel), shape = RoundedCornerShape(20.dp)) {
                Column(Modifier.padding(17.dp), verticalArrangement = Arrangement.spacedBy(10.dp)) {
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        Column(Modifier.weight(1f)) {
                            Text("STATUS DO CONTROL PLANE", color = TextMuted, fontSize = 10.sp, fontWeight = FontWeight.Bold, letterSpacing = 1.sp)
                            Text(if (snapshot != null) "Conectado" else "Aguardando conexão", color = TextPrimary, fontSize = 18.sp, fontWeight = FontWeight.SemiBold)
                        }
                        Text(if (snapshot != null) "●" else "○", color = if (snapshot != null) Good else Warning, fontSize = 28.sp)
                    }
                    Text(message, color = TextMuted, fontSize = 12.sp)
                    Button(onClick = onConnect, enabled = !loading, modifier = Modifier.fillMaxWidth()) {
                        if (loading) CircularProgressIndicator(Modifier.size(18.dp), strokeWidth = 2.dp)
                        else Text("Conectar e atualizar")
                    }
                }
            }
        }
        item {
            Row(horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                MetricCard("Nós registrados", snapshot?.nodes?.size?.toString() ?: "—", "Inventário real", Modifier.weight(1f))
                MetricCard("Serviço", if (snapshot != null) "Ativo" else "—", snapshot?.service ?: "Sem leitura", Modifier.weight(1f))
            }
        }
        item { Text("MÓDULOS DA PLATAFORMA", color = TextMuted, fontSize = 11.sp, fontWeight = FontWeight.Bold, letterSpacing = 1.2.sp) }
        item { ModuleCard("Armazenamento", "Dados, objetos e ingestão", "▤") { onOpen(AppTab.STORAGE) } }
        item { ModuleCard("Computação", "Scheduler, jobs e AX Accelerator", "◈") { onOpen(AppTab.COMPUTE) } }
        item { ModuleCard("Projetos de IA", "Datasets e treinamento de modelos", "▦") { onOpen(AppTab.PROJECTS) } }
        if (snapshot != null) {
            item { Text("NÓS DO CLUSTER", color = TextMuted, fontSize = 11.sp, fontWeight = FontWeight.Bold, letterSpacing = 1.2.sp) }
            items(snapshot.nodes) { node ->
                Card(colors = CardDefaults.cardColors(containerColor = Panel), shape = RoundedCornerShape(14.dp)) {
                    Row(Modifier.fillMaxWidth().padding(14.dp), verticalAlignment = Alignment.CenterVertically) {
                        Column(Modifier.weight(1f)) {
                            Text(node.id, color = TextPrimary, fontWeight = FontWeight.SemiBold)
                            Text(node.detail, color = TextMuted, fontSize = 11.sp)
                        }
                        Text(node.state, color = Good, fontSize = 11.sp)
                    }
                }
            }
        }
    }
}

@Composable
private fun MetricCard(title: String, value: String, detail: String, modifier: Modifier = Modifier) {
    Card(modifier, colors = CardDefaults.cardColors(containerColor = Panel), shape = RoundedCornerShape(16.dp)) {
        Column(Modifier.padding(14.dp), verticalArrangement = Arrangement.spacedBy(5.dp)) {
            Text(title, color = TextMuted, fontSize = 11.sp)
            Text(value, color = TextPrimary, fontSize = 24.sp, fontWeight = FontWeight.Bold)
            Text(detail, color = TextMuted, fontSize = 10.sp)
        }
    }
}

@Composable
private fun ModuleCard(title: String, subtitle: String, symbol: String, onClick: () -> Unit) {
    Card(onClick = onClick, colors = CardDefaults.cardColors(containerColor = Panel), shape = RoundedCornerShape(16.dp)) {
        Row(Modifier.fillMaxWidth().padding(16.dp), verticalAlignment = Alignment.CenterVertically) {
            Surface(color = Accent.copy(alpha = .13f), shape = RoundedCornerShape(12.dp)) {
                Text(symbol, color = Accent, fontSize = 22.sp, modifier = Modifier.padding(12.dp))
            }
            Column(Modifier.weight(1f).padding(start = 13.dp)) {
                Text(title, color = TextPrimary, fontWeight = FontWeight.SemiBold)
                Text(subtitle, color = TextMuted, fontSize = 11.sp)
            }
            Text("›", color = Accent, fontSize = 25.sp)
        }
    }
}


@Composable
private fun VtaStorageScreen(
    modifier: Modifier,
    token: String,
    endpoint: String,
    onStorageAction: (String, String, Boolean, (Result<String>) -> Unit) -> Unit
) {
    var status by remember { mutableStateOf<JSONObject?>(null) }
    var loading by remember { mutableStateOf(false) }
    var message by remember { mutableStateOf("Consulte o volume conectado ao APOLO Data Center.") }
    fun refresh(initialize: Boolean) {
        loading = true
        message = if (initialize) "Preparando diretórios VTA 2.0 no volume…" else "Consultando capacidade física do volume…"
        onStorageAction(endpoint, token, initialize) { result ->
            loading = false
            result.onSuccess {
                status = JSONObject(it)
                message = if (initialize) "Estrutura VTA 2.0 criada no volume conectado." else "Status do armazenamento atualizado."
            }.onFailure { error -> message = error.message ?: "Falha ao consultar armazenamento." }
        }
    }
    LazyColumn(modifier.fillMaxSize(), contentPadding = PaddingValues(18.dp), verticalArrangement = Arrangement.spacedBy(14.dp)) {
        item {
            Text("APOLO VTA 2.0", color = Accent, fontSize = 11.sp, fontWeight = FontWeight.Bold, letterSpacing = 1.5.sp)
            Text("Armazenamento interno", color = TextPrimary, fontSize = 26.sp, fontWeight = FontWeight.Bold)
            Text("Volume conectado ao APOLO Data Center · quota lógica configurada de 999 TB", color = TextMuted, fontSize = 13.sp)
        }
        item {
            Card(colors = CardDefaults.cardColors(containerColor = Panel), shape = RoundedCornerShape(18.dp)) {
                Column(Modifier.fillMaxWidth().padding(17.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    Text("CAPACIDADE LÓGICA", color = TextMuted, fontSize = 10.sp, fontWeight = FontWeight.Bold)
                    Text("999 TB", color = Accent, fontSize = 34.sp, fontWeight = FontWeight.Black)
                    Text("Limite lógico de projeto; não representa espaço físico já provisionado.", color = TextMuted, fontSize = 11.sp)
                }
            }
        }
        item {
            Card(colors = CardDefaults.cardColors(containerColor = Panel), shape = RoundedCornerShape(16.dp)) {
                Column(Modifier.fillMaxWidth().padding(16.dp), verticalArrangement = Arrangement.spacedBy(7.dp)) {
                    Text("VOLUME FÍSICO CONECTADO", color = Accent, fontSize = 11.sp, fontWeight = FontWeight.Bold)
                    Text("Total: " + (status?.optLong("physical_filesystem_total_bytes")?.takeIf { it > 0 }?.let { "%.2f GB".format(it / 1_000_000_000.0) } ?: "Aguardando leitura"), color = TextPrimary)
                    Text("Disponível: " + (status?.optLong("physical_filesystem_available_bytes")?.takeIf { it > 0 }?.let { "%.2f GB".format(it / 1_000_000_000.0) } ?: "—"), color = TextPrimary)
                    Text("Utilizado: " + (status?.optLong("physical_filesystem_used_bytes")?.takeIf { it > 0 }?.let { "%.2f GB".format(it / 1_000_000_000.0) } ?: "—"), color = TextPrimary)
                    Text("Montagem: " + (status?.optString("mount_path") ?: "/data/apolo"), color = TextMuted, fontSize = 11.sp)
                    Text("Diretórios VTA prontos: " + if (status?.optBoolean("directories_ready") == true) "Sim" else "Não", color = TextMuted, fontSize = 11.sp)
                }
            }
        }
        item {
            Text("CAMADAS DE ARMAZENAMENTO", color = TextMuted, fontSize = 11.sp, fontWeight = FontWeight.Bold)
            listOf("VTA Cache · blocos quentes", "VTA RAM Spill · spill em disco, não RAM física", "VTA NVMe · pesos e shards", "VTA Archive · datasets e checkpoints", "VTA Models · artefatos de modelos").forEach { label ->
                Card(colors = CardDefaults.cardColors(containerColor = Panel), shape = RoundedCornerShape(12.dp), modifier = Modifier.padding(top = 7.dp)) {
                    Text(label, color = TextPrimary, modifier = Modifier.fillMaxWidth().padding(13.dp), fontSize = 12.sp)
                }
            }
        }
        item {
            Text(message, color = TextMuted, fontSize = 12.sp)
            Button(onClick = { refresh(false) }, enabled = !loading && token.length >= 32, modifier = Modifier.fillMaxWidth()) {
                Text(if (loading) "Aguarde…" else "Atualizar capacidade física")
            }
            OutlinedButton(onClick = { refresh(true) }, enabled = !loading && token.length >= 32, modifier = Modifier.fillMaxWidth()) {
                Text("Conectar / preparar armazenamento VTA 2.0")
            }
            Text(status?.optString("warning") ?: "Os 999 TB são uma quota lógica. O volume real precisa ser ampliado no provedor de infraestrutura para armazenar essa quantidade.", color = Warning, fontSize = 11.sp)
        }
    }
}

@Composable
private fun CapabilityScreen(modifier: Modifier, title: String, subtitle: String, capabilities: List<String>, limitation: String) {
    LazyColumn(modifier.fillMaxSize(), contentPadding = PaddingValues(18.dp), verticalArrangement = Arrangement.spacedBy(14.dp)) {
        item {
            Text("APOLO AX / MÓDULO", color = Accent, fontSize = 10.sp, fontWeight = FontWeight.Bold, letterSpacing = 1.4.sp)
            Text(title, color = TextPrimary, fontSize = 27.sp, fontWeight = FontWeight.Bold)
            Text(subtitle, color = TextMuted, fontSize = 13.sp)
        }
        items(capabilities) { capability ->
            Card(colors = CardDefaults.cardColors(containerColor = Panel), shape = RoundedCornerShape(14.dp)) {
                Row(Modifier.fillMaxWidth().padding(16.dp), verticalAlignment = Alignment.CenterVertically) {
                    Text("◇", color = Accent, fontSize = 19.sp)
                    Text(capability, color = TextPrimary, modifier = Modifier.padding(start = 12.dp), fontWeight = FontWeight.Medium)
                }
            }
        }
    }
}

@Composable
private fun SettingsScreen(
    modifier: Modifier, endpoint: String, onEndpointChange: (String) -> Unit,
    token: String, onTokenChange: (String) -> Unit,
    onSave: () -> Unit, onConnect: () -> Unit, message: String, loading: Boolean,
    onLogout: () -> Unit
) {
    Column(modifier.fillMaxSize().padding(18.dp), verticalArrangement = Arrangement.spacedBy(14.dp)) {
        Text("CONFIGURAÇÕES", color = Accent, fontSize = 11.sp, fontWeight = FontWeight.Bold, letterSpacing = 1.5.sp)
        Text("Conexão e dispositivo", color = TextPrimary, fontSize = 26.sp, fontWeight = FontWeight.Bold)
        Text("Configure o endereço do servidor APOLO Data Center.", color = TextMuted, fontSize = 13.sp)
        OutlinedTextField(
            value = endpoint, onValueChange = onEndpointChange, modifier = Modifier.fillMaxWidth(),
            label = { Text("URL da API") }, placeholder = { Text("https://seu-servidor") }, singleLine = true
        )
        OutlinedTextField(
            value = token, onValueChange = onTokenChange, modifier = Modifier.fillMaxWidth(),
            label = { Text("Token da API (mínimo 32 caracteres)") },
            visualTransformation = androidx.compose.ui.text.input.PasswordVisualTransformation(),
            singleLine = true
        )
        Button(
            onClick = onSave,
            enabled = token.isBlank() || token.length >= 32,
            modifier = Modifier.fillMaxWidth()
        ) { Text("Salvar conexão e token") }
        OutlinedButton(onClick = onConnect, enabled = !loading, modifier = Modifier.fillMaxWidth()) {
            if (loading) CircularProgressIndicator(Modifier.size(17.dp), strokeWidth = 2.dp)
            else Text("Testar conexão")
        }
        Text(message, color = TextMuted, fontSize = 12.sp)
        Card(colors = CardDefaults.cardColors(containerColor = Panel), shape = RoundedCornerShape(16.dp)) {
            Column(Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(7.dp)) {
                Text("SEGURANÇA", color = Accent, fontWeight = FontWeight.Bold, fontSize = 11.sp)
                Text("HTTPS obrigatório em servidores remotos", color = TextPrimary, fontWeight = FontWeight.SemiBold)
                Text("O token é criptografado com AES-GCM usando chave protegida pelo Android Keystore. HTTP só é permitido em loopback/emulador.", color = TextMuted, fontSize = 12.sp)
            }
        }
        OutlinedButton(onClick = onLogout, modifier = Modifier.fillMaxWidth()) { Text("Sair da conta") }
        Spacer(Modifier.weight(1f))
        Text("APOLO Data Center · Android nativo · v0.3.0", color = TextMuted, fontSize = 10.sp)
    }
}
