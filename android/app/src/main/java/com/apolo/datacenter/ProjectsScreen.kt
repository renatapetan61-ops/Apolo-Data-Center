package com.apolo.datacenter

import android.net.Uri
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.graphics.Color
import org.json.JSONObject

internal enum class ProjectAction { LOAD, CREATE, DELETE }

@Composable
internal fun ProjectsScreen(
    modifier: Modifier = Modifier, endpoint: String, token: String,
    onAction: (ProjectAction, String, String, String, (Result<List<ProjectInfo>>) -> Unit) -> Unit,
    onCreateToken: () -> Unit,
    onWorkspaceAction: (String, String, String, String, (Result<String>) -> Unit) -> Unit,
    onUpload: (String, Uri, String, (Result<String>) -> Unit) -> Unit
) {
    var name by remember { mutableStateOf("") }
    var description by remember { mutableStateOf("") }
    var projects by remember { mutableStateOf<List<ProjectInfo>>(emptyList()) }
    var message by remember { mutableStateOf("Carregue os projetos do servidor.") }
    var loading by remember { mutableStateOf(false) }
    var selected by remember { mutableStateOf<ProjectInfo?>(null) }
    var files by remember { mutableStateOf<List<JSONObject>>(emptyList()) }
    var models by remember { mutableStateOf<List<JSONObject>>(emptyList()) }
    var jobs by remember { mutableStateOf<List<JSONObject>>(emptyList()) }
    var assistantInput by remember { mutableStateOf("") }
    var assistantHistory by remember { mutableStateOf(listOf<Pair<String, String>>()) }
    var assistantOutput by remember { mutableStateOf("Assistente de engenharia de LLMs APOLO AX · Kimi K3.1 Pro via NVIDIA NIM. Descreva o modelo que deseja projetar.") }
    var modelName by remember { mutableStateOf("") }
    var parameters by remember { mutableStateOf("1000000") }
    var vtaParameters by remember { mutableStateOf("1000000") }
    var vtaReport by remember { mutableStateOf("Consulte a VTA para estimar a capacidade real dos workers conectados.") }
    var fileKind by remember { mutableStateOf("dataset") }
    var busy by remember { mutableStateOf(false) }
    val picker = rememberLauncherForActivityResult(ActivityResultContracts.OpenDocument()) { uri ->
        val p = selected
        if (uri != null && p != null) {
            busy = true
            onUpload(p.id, uri, fileKind) { result ->
                busy = false
                result.onSuccess { message = "Arquivo enviado e registrado." }.onFailure { message = it.message ?: "Falha no upload." }
                onWorkspaceAction(p.id, "LOAD", "", "") { refresh ->
                    refresh.onSuccess { raw -> val j=JSONObject(raw); files=(0 until j.getJSONArray("files").length()).map { j.getJSONArray("files").getJSONObject(it) } }
                }
            }
        }
    }
    fun loadProjects(action: ProjectAction, id: String = "") {
        // Trava a operação no estado da tela para ignorar toques repetidos.
        if (loading) return
        if (token.length < 32) { message = "Configure o token da API em Ajustes."; return }
        if (action == ProjectAction.CREATE && name.isBlank()) {
            message = "Informe o nome do projeto."
            return
        }
        loading = true
        message = if (action == ProjectAction.CREATE) "Criando um único projeto…" else "Atualizando projetos…"
        onAction(action,name.trim(),description.trim(),id) { result ->
            loading = false
            result.onSuccess { loaded ->
                projects = loaded
                if (action == ProjectAction.CREATE) { name = ""; description = "" }
                message = if (action == ProjectAction.CREATE) "Projeto criado com sucesso. ${loaded.size} projeto(s) na conta."
                    else loaded.size.toString()+" projeto(s) carregado(s)."
            }.onFailure { error ->
                message = error.message ?: "Falha ao acessar projetos."
            }
        }
    }
    fun refreshWorkspace(p: ProjectInfo) {
        busy=true
        onWorkspaceAction(p.id,"LOAD","","") { result ->
            busy=false
            result.onSuccess { raw ->
                val j=JSONObject(raw)
                files=(0 until j.getJSONArray("files").length()).map { j.getJSONArray("files").getJSONObject(it) }
                models=(0 until j.getJSONArray("models").length()).map { j.getJSONArray("models").getJSONObject(it) }
                jobs=(0 until j.getJSONArray("jobs").length()).map { j.getJSONArray("jobs").getJSONObject(it) }
            }.onFailure { message=it.message ?: "Falha ao abrir workspace." }
        }
    }
    // Recarrega automaticamente a lista do servidor ao entrar ou trocar de conta.
    LaunchedEffect(token) {
        if (token.length >= 32) loadProjects(ProjectAction.LOAD)
    }
    LazyColumn(modifier.fillMaxSize(),contentPadding=PaddingValues(18.dp),verticalArrangement=Arrangement.spacedBy(12.dp)) {
        if(selected==null) {
            item {
                Text("APOLO AX / WORKSPACES",color=Color(0xFF78A9FF),fontSize=10.sp,fontWeight=FontWeight.Bold)
                Row(Modifier.fillMaxWidth(),horizontalArrangement=Arrangement.SpaceBetween) {
                    Text("Projetos",color=Color(0xFFEAF1FC),fontSize=27.sp,fontWeight=FontWeight.Bold)
                    Button(onClick=onCreateToken){Text("Criar Token")}
                }
                Text("Projetos persistidos no PostgreSQL.",color=Color(0xFF9BAEC8),fontSize=13.sp)
            }
            item {
                Card(colors=CardDefaults.cardColors(containerColor=Color(0xFF141F31)),shape=RoundedCornerShape(16.dp)) {
                    Column(Modifier.padding(15.dp),verticalArrangement=Arrangement.spacedBy(10.dp)) {
                        OutlinedTextField(name,{name=it},modifier=Modifier.fillMaxWidth(),label={Text("Nome do projeto")},singleLine=true)
                        OutlinedTextField(description,{description=it},modifier=Modifier.fillMaxWidth(),label={Text("Descrição")},minLines=2)
                        Button(onClick={loadProjects(ProjectAction.CREATE)},enabled=!loading&&name.isNotBlank(),modifier=Modifier.fillMaxWidth()){Text("Criar projeto")}
                        OutlinedButton(onClick={loadProjects(ProjectAction.LOAD)},enabled=!loading,modifier=Modifier.fillMaxWidth()){Text("Atualizar lista")}
                    }
                }
            }
            item{Text(message,color=Color(0xFF9BAEC8),fontSize=12.sp)}
            if(loading)item{LinearProgressIndicator(Modifier.fillMaxWidth())}
            items(projects,key={it.id}) { p ->
                Card(colors=CardDefaults.cardColors(containerColor=Color(0xFF141F31)),shape=RoundedCornerShape(14.dp)) {
                    Column(Modifier.fillMaxWidth().padding(14.dp),verticalArrangement=Arrangement.spacedBy(6.dp)) {
                        Text(p.name,color=Color(0xFFEAF1FC),fontWeight=FontWeight.SemiBold)
                        if(p.description.isNotBlank())Text(p.description,color=Color(0xFF9BAEC8),fontSize=12.sp)
                        Button(onClick={selected=p;refreshWorkspace(p)},modifier=Modifier.fillMaxWidth()){Text("Abrir área de trabalho")}
                        TextButton(onClick={loadProjects(ProjectAction.DELETE,p.id)},enabled=!loading){Text("Excluir projeto")}
                    }
                }
            }
        } else {
            val p=selected!!
            item {
                TextButton(onClick={selected=null}){Text("‹ Voltar aos projetos")}
                Text("ÁREA DE TRABALHO",color=Color(0xFF78A9FF),fontSize=10.sp,fontWeight=FontWeight.Bold)
                Text(p.name,color=Color(0xFFEAF1FC),fontSize=25.sp,fontWeight=FontWeight.Bold)
                Text(p.description,color=Color(0xFF9BAEC8),fontSize=12.sp)
                if(busy)LinearProgressIndicator(Modifier.fillMaxWidth())
            }
            // O projeto abre diretamente no laboratório de criação com Kimi.
            item {
                Text("CHAT DE ENGENHARIA · KIMI K3.1 PRO",color=Color(0xFF78A9FF),fontWeight=FontWeight.Bold,fontSize=12.sp)
                Text(assistantOutput,color=Color(0xFFEAF1FC),fontSize=13.sp)
                OutlinedTextField(assistantInput,{assistantInput=it},modifier=Modifier.fillMaxWidth(),label={Text("Descreva o LLM que deseja criar do zero")},minLines=4)
                Button(onClick={
                    val q=assistantInput.trim()
                    if(q.isNotEmpty()){
                        val history=org.json.JSONArray().apply {
                            assistantHistory.takeLast(30).forEach { (role, content) ->
                                put(JSONObject().put("role",role).put("content",content))
                            }
                        }
                        val payload=JSONObject().put("message",q).put("history",history).toString()
                        assistantInput="";busy=true;message="Enviando seu prompt ao Kimi K3.1 Pro…"
                        onWorkspaceAction(p.id,"CHAT",payload,""){r->
                            r.onSuccess{answer->
                                assistantOutput=answer
                                assistantHistory=(assistantHistory+("user" to q)+("assistant" to answer)).takeLast(30)
                                message="Kimi respondeu. Gerando agora os arquivos iniciais do LLM…"
                                val generatedName=modelName.ifBlank { "${p.name} LLM" }
                                onWorkspaceAction(p.id,"MODEL",generatedName,parameters+"\n"+q+"\n\nProjeto técnico definido pelo Kimi:\n"+answer){generated->
                                    busy=false
                                    generated.onSuccess{
                                        assistantOutput=answer+"\n\n[APOLO] Estrutura inicial do LLM gerada e salva no projeto. Isso é código-fonte inicial, não um modelo já treinado."
                                        message="Arquivos iniciais do LLM salvos. Configure um dataset e worker para iniciar treinamento."
                                        refreshWorkspace(p)
                                    }.onFailure{err->
                                        message=err.message?:"Falha ao gerar os arquivos do LLM."
                                        assistantOutput=answer+"\n\n[APOLO] A resposta do Kimi foi recebida, mas a geração dos arquivos falhou: "+message
                                    }
                                }
                            }.onFailure{err->
                                busy=false;assistantOutput=err.message?:"Falha ao enviar o prompt ao Kimi/NVIDIA NIM."
                                message=assistantOutput
                            }
                        }
                    }
                },enabled=assistantInput.isNotBlank()&&!busy,modifier=Modifier.fillMaxWidth()){
                    Text(if(busy)"Processando…" else "Enviar prompt e criar LLM")
                }
                Text("Ao enviar, o backend consulta o Kimi via NVIDIA NIM e tenta gerar model.py, train.py, configuração e README. O treino só começa após preparar dados e conectar um worker compatível.",color=Color(0xFF9BAEC8),fontSize=11.sp)
            }
            item {
                Text("ARQUIVOS E DATASETS",color=Color(0xFF78A9FF),fontWeight=FontWeight.Bold,fontSize=11.sp)
                Row(horizontalArrangement=Arrangement.spacedBy(8.dp)) {
                    OutlinedButton(onClick={fileKind="dataset";picker.launch(arrayOf("*/*"))}){Text("Adicionar dataset")}
                    OutlinedButton(onClick={fileKind="model";picker.launch(arrayOf("*/*"))}){Text("Adicionar modelo")}
                }
                OutlinedButton(onClick={fileKind="file";picker.launch(arrayOf("*/*"))},modifier=Modifier.fillMaxWidth()){Text("Adicionar arquivo")}
            }
            if(files.isEmpty())item{Text("Nenhum arquivo enviado.",color=Color(0xFF9BAEC8),fontSize=12.sp)}
            items(files){ f ->
                Card(colors=CardDefaults.cardColors(containerColor=Color(0xFF141F31))){Column(Modifier.padding(12.dp)){
                    Text(f.optString("filename"),color=Color(0xFFEAF1FC),fontWeight=FontWeight.SemiBold)
                    Text(f.optString("kind")+" · "+f.optLong("size_bytes")+" bytes · SHA-256 "+f.optString("sha256").take(12)+"…",color=Color(0xFF9BAEC8),fontSize=10.sp)
                }}
            }
            item {
                Text("MODELOS",color=Color(0xFF78A9FF),fontWeight=FontWeight.Bold,fontSize=11.sp)
                OutlinedTextField(modelName,{modelName=it},modifier=Modifier.fillMaxWidth(),label={Text("Nome do LLM")})
                OutlinedTextField(parameters,{parameters=it},modifier=Modifier.fillMaxWidth(),label={Text("Parâmetros (quantidade)")})
                Button(onClick={
                    busy=true
                    onWorkspaceAction(p.id,"MODEL",modelName,parameters+"\n"+assistantOutput){r->busy=false;r.onSuccess{message="Kimi gerou e armazenou os arquivos do LLM."}.onFailure{message=it.message?:"Erro"};refreshWorkspace(p)}
                },enabled=modelName.isNotBlank(),modifier=Modifier.fillMaxWidth()){Text("Gerar arquivos do LLM com Kimi")}
                models.forEach { model -> Text("• "+model.optString("name")+" · "+model.optString("architecture")+" · "+model.optString("status"),color=Color(0xFFEAF1FC),fontSize=12.sp) }
            }
            item {
                Text("APOLO VTA · ACELERADOR VIRTUAL",color=Color(0xFF78A9FF),fontWeight=FontWeight.Bold,fontSize=11.sp)
                Text("Planeja o uso combinado de memória e aceleradores dos workers reais. Não cria hardware nem inventa FLOPS.",color=Color(0xFF9BAEC8),fontSize=11.sp)
                OutlinedTextField(vtaParameters,{vtaParameters=it},modifier=Modifier.fillMaxWidth(),label={Text("Parâmetros desejados")})
                Button(onClick={
                    busy=true
                    onWorkspaceAction(p.id,"VTA",vtaParameters,""){r->
                        busy=false
                        r.onSuccess{raw->
                            val v=JSONObject(raw)
                            vtaReport="Viável pela estimativa: "+v.optString("feasible_by_memory_estimate")+" · Limite estimado: "+v.optString("estimated_parameter_limit")+" parâmetros · Workers online: "+v.optInt("active_workers")+" · GPUs: "+v.optInt("gpu_workers")+"\n"+v.optString("reason")
                        }.onFailure{vtaReport=it.message?:"Falha ao consultar APOLO VTA."}
                    }
                },enabled=!busy&&vtaParameters.isNotBlank(),modifier=Modifier.fillMaxWidth()){Text("Analisar capacidade da VTA")}
                Text(vtaReport,color=Color(0xFFEAF1FC),fontSize=11.sp)
            }

            item {
                Text("TREINAMENTO E CHECKPOINTS",color=Color(0xFF78A9FF),fontWeight=FontWeight.Bold,fontSize=11.sp)
                Text("A solicitação é bloqueada quando não há worker AX online com heartbeat recente. Nenhum treino é simulado.",color=Color(0xFF9BAEC8),fontSize=11.sp)
                Button(onClick={
                    val mid=models.firstOrNull()?.optString("id").orEmpty();busy=true
                    onWorkspaceAction(p.id,"TRAIN",mid,""){r->busy=false;r.onSuccess{message="Job colocado na fila."}.onFailure{message=it.message?:"Worker indisponível"};refreshWorkspace(p)}
                },modifier=Modifier.fillMaxWidth()){Text("Solicitar treinamento")}
                jobs.forEach { job ->
                    Card(colors=CardDefaults.cardColors(containerColor=Color(0xFF141F31))){Column(Modifier.padding(12.dp)){
                        Text("Job "+job.optString("id").take(8)+" · "+job.optString("status"),color=Color(0xFFEAF1FC),fontWeight=FontWeight.SemiBold)
                        Text("Métricas: "+(job.optJSONObject("metrics")?.toString() ?: "{}"),color=Color(0xFF9BAEC8),fontSize=10.sp)
                        Text("Checkpoint: "+job.optString("checkpoint_key","não gerado"),color=Color(0xFF9BAEC8),fontSize=10.sp)
                    }}
                }
            }
            item{Text(message,color=Color(0xFF9BAEC8),fontSize=12.sp)}
        }
    }
}
