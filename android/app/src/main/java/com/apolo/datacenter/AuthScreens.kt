package com.apolo.datacenter

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.compose.ui.platform.LocalClipboardManager
import androidx.compose.ui.graphics.Color

@Composable
internal fun AuthScreen(
    endpoint: String,
    loading: Boolean,
    message: String,
    onSubmit: (String, String, String, Boolean) -> Unit
) {
    var registering by remember { mutableStateOf(false) }
    var name by remember { mutableStateOf("") }
    var email by remember { mutableStateOf("") }
    var password by remember { mutableStateOf("") }
    Column(
        Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(24.dp),
        verticalArrangement = Arrangement.Center,
        horizontalAlignment = Alignment.CenterHorizontally
    ) {
        Text("APOLO", color = Color(0xFF78A9FF), fontSize = 13.sp, fontWeight = FontWeight.Black, letterSpacing = 3.sp)
        Text("Data Center", color = Color(0xFFEAF1FC), fontSize = 30.sp, fontWeight = FontWeight.Bold)
        Text(if (registering) "Crie sua conta para continuar" else "Entre na sua conta", color = Color(0xFF9BAEC8), fontSize = 13.sp)
        Spacer(Modifier.height(24.dp))
        Card(colors = CardDefaults.cardColors(containerColor = Color(0xFF141F31)), shape = RoundedCornerShape(20.dp)) {
            Column(Modifier.fillMaxWidth().padding(18.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
                Text(if (registering) "CADASTRO" else "LOGIN", color = Color(0xFF78A9FF), fontSize = 11.sp, fontWeight = FontWeight.Bold)
                if (registering) OutlinedTextField(name, { name = it }, modifier = Modifier.fillMaxWidth(), label = { Text("Nome") }, singleLine = true)
                OutlinedTextField(email, { email = it }, modifier = Modifier.fillMaxWidth(), label = { Text("E-mail") }, singleLine = true)
                OutlinedTextField(password, { password = it }, modifier = Modifier.fillMaxWidth(), label = { Text("Senha (mínimo 10 caracteres)") }, singleLine = true, visualTransformation = androidx.compose.ui.text.input.PasswordVisualTransformation())
                if (message.isNotBlank()) Text(message, color = Color(0xFFF3C46B), fontSize = 12.sp)
                Button(
                    onClick = { onSubmit(name.trim(), email.trim(), password, registering) },
                    enabled = !loading && email.contains("@") && password.length >= 10 && (!registering || name.isNotBlank()),
                    modifier = Modifier.fillMaxWidth()
                ) { if (loading) CircularProgressIndicator(Modifier.size(18.dp), strokeWidth = 2.dp) else Text(if (registering) "Criar conta" else "Entrar") }
                TextButton(onClick = { registering = !registering }, modifier = Modifier.fillMaxWidth()) {
                    Text(if (registering) "Já tenho uma conta · Fazer login" else "Não tenho conta · Cadastrar")
                }
            }
        }
        Spacer(Modifier.height(16.dp))
        Text("Servidor: $endpoint", color = Color(0xFF9BAEC8), fontSize = 10.sp)
    }
}

@Composable
internal fun CreateTokenScreen(
    loading: Boolean,
    message: String,
    token: String,
    onBack: () -> Unit,
    onGenerate: (String) -> Unit,
    onUseInSettings: (String) -> Unit
) {
    var label by remember { mutableStateOf("Meu token do aplicativo") }
    val clipboard = LocalClipboardManager.current
    Column(Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(18.dp), verticalArrangement = Arrangement.spacedBy(14.dp)) {
        TextButton(onClick = onBack) { Text("‹ Voltar para Projetos") }
        Text("SEGURANÇA / API", color = Color(0xFF78A9FF), fontSize = 11.sp, fontWeight = FontWeight.Bold)
        Text("Criar Token", color = Color(0xFFEAF1FC), fontSize = 27.sp, fontWeight = FontWeight.Bold)
        Text("Gere uma credencial individual para conectar seus aplicativos à API APOLO.", color = Color(0xFF9BAEC8), fontSize = 13.sp)
        Card(colors = CardDefaults.cardColors(containerColor = Color(0xFF141F31)), shape = RoundedCornerShape(16.dp)) {
            Column(Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(10.dp)) {
                OutlinedTextField(label, { label = it }, modifier = Modifier.fillMaxWidth(), label = { Text("Nome do token") }, singleLine = true)
                Button(onClick = { onGenerate(label.trim()) }, enabled = !loading && label.isNotBlank() && token.isBlank(), modifier = Modifier.fillMaxWidth()) {
                    if (loading) CircularProgressIndicator(Modifier.size(18.dp), strokeWidth = 2.dp) else Text("Gerar token da API")
                }
                if (token.isNotBlank()) {
                    Text("TOKEN GERADO — COPIE AGORA", color = Color(0xFF62D6A5), fontSize = 10.sp, fontWeight = FontWeight.Bold)
                    OutlinedTextField(token, {}, modifier = Modifier.fillMaxWidth(), readOnly = true, minLines = 3)
                    OutlinedButton(onClick = { clipboard.setText(AnnotatedString(token)) }, modifier = Modifier.fillMaxWidth()) { Text("Copiar token") }
                    Button(onClick = { onUseInSettings(token) }, modifier = Modifier.fillMaxWidth()) { Text("Usar este token em Ajustes") }
                    Text("Por segurança, o token completo é exibido somente nesta tela. Guarde-o em local seguro.", color = Color(0xFFF3C46B), fontSize = 11.sp)
                }
            }
        }
        if (message.isNotBlank()) Text(message, color = Color(0xFF9BAEC8), fontSize = 12.sp)
    }
}
