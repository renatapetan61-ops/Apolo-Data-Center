package com.apolo.datacenter

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

internal enum class ProjectAction { LOAD, CREATE, DELETE }

@Composable
internal fun ProjectsScreen(
    endpoint: String,
    token: String,
    onAction: (ProjectAction, String, String, String, (Result<List<ProjectInfo>>) -> Unit) -> Unit
) {
    var name by remember { mutableStateOf("") }
    var description by remember { mutableStateOf("") }
    var projects by remember { mutableStateOf<List<ProjectInfo>>(emptyList()) }
    var message by remember { mutableStateOf("Carregue os projetos do servidor.") }
    var loading by remember { mutableStateOf(false) }

    fun execute(action: ProjectAction, id: String = "") {
        if (token.length < 32) {
            message = "Configure um token de API com pelo menos 32 caracteres em Ajustes."
            return
        }
        loading = true
        message = "Acessando a API e o PostgreSQL…"
        onAction(action, name, description, id) { result ->
            loading = false
            result.onSuccess { loaded ->
                projects = loaded
                if (action == ProjectAction.CREATE) {
                    name = ""
                    description = ""
                }
                message = "Dados recebidos do servidor: ${loaded.size} projeto(s)."
            }.onFailure { error ->
                message = error.message ?: "Falha ao acessar projetos."
            }
        }
    }

    LazyColumn(
        modifier = Modifier.fillMaxSize(),
        contentPadding = PaddingValues(18.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp)
    ) {
        item {
            Text("APOLO AX / WORKSPACES", color = Color(0xFF78A9FF), fontSize = 10.sp, fontWeight = FontWeight.Bold)
            Text("Projetos", color = Color(0xFFEAF1FC), fontSize = 27.sp, fontWeight = FontWeight.Bold)
            Text("Projetos persistidos no PostgreSQL do servidor.", color = Color(0xFF9BAEC8), fontSize = 13.sp)
        }
        item {
            Card(colors = CardDefaults.cardColors(containerColor = Color(0xFF141F31)), shape = RoundedCornerShape(16.dp)) {
                Column(Modifier.padding(15.dp), verticalArrangement = Arrangement.spacedBy(10.dp)) {
                    OutlinedTextField(name, { name = it }, modifier = Modifier.fillMaxWidth(), label = { Text("Nome do projeto") }, singleLine = true)
                    OutlinedTextField(description, { description = it }, modifier = Modifier.fillMaxWidth(), label = { Text("Descrição") }, minLines = 2)
                    Button(onClick = { execute(ProjectAction.CREATE) }, enabled = !loading && name.isNotBlank(), modifier = Modifier.fillMaxWidth()) {
                        Text("Criar projeto")
                    }
                    OutlinedButton(onClick = { execute(ProjectAction.LOAD) }, enabled = !loading, modifier = Modifier.fillMaxWidth()) {
                        Text("Atualizar lista")
                    }
                }
            }
        }
        item { Text(message, color = Color(0xFF9BAEC8), fontSize = 12.sp) }
        if (loading) item { LinearProgressIndicator(Modifier.fillMaxWidth()) }
        items(projects, key = { it.id }) { project ->
            Card(colors = CardDefaults.cardColors(containerColor = Color(0xFF141F31)), shape = RoundedCornerShape(14.dp)) {
                Column(Modifier.fillMaxWidth().padding(14.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                    Text(project.name, color = Color(0xFFEAF1FC), fontWeight = FontWeight.SemiBold)
                    if (project.description.isNotBlank()) Text(project.description, color = Color(0xFF9BAEC8), fontSize = 12.sp)
                    Text(project.id, color = Color(0xFF9BAEC8), fontSize = 9.sp)
                    TextButton(onClick = { execute(ProjectAction.DELETE, project.id) }, enabled = !loading) {
                        Text("Excluir projeto")
                    }
                }
            }
        }
    }
}
