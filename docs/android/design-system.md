# APOLO Data Center — Android Design System

## Direção visual
- Aplicativo Android nativo para operar APOLO AX; não é site nem WebView.
- Estética de infraestrutura técnica premium, escura e legível.
- Paleta: Navy #0B1220; superfícies #141F31 e #1B2A40; azul #78A9FF; texto #EAF1FC; secundário #9BAEC8; sucesso #62D6A5; alerta #F3C46B.
- Tipografia Roboto/Roboto Flex do sistema Android.
- Material 3, cards de 16–20dp, espaçamento 8/14/18dp e navegação inferior.
- Acessibilidade: alvos mínimos de 48dp, contraste alto, fontes ampliadas e rótulos além dos símbolos.

## Telas
1. Início: conexão, status control plane, nós registrados apenas após resposta real, atalhos e aviso de capacidade verificada.
2. Dados: Object Storage, upload/ingestão, integridade e replicação; ações dependem da API autenticada de storage.
3. Compute: scheduler, jobs, AX ISA e treinamento; sem simular execução ou hardware.
4. Projetos: datasets, jobs e histórico; requer API persistente e autenticação.
5. Ajustes: endpoint HTTPS configurável, teste de conectividade e URL salva localmente.

## Comportamento e segurança
- GET /healthz, /api/v1/overview e /api/v1/nodes.
- Falha de rede não é confundida com inventário vazio.
- HTTP permitido somente para loopback/emulador; HTTPS obrigatório remotamente.
- Não solicitar tokens até o backend de identidade existir.
- Capacidade exibida deve vir de inventário real verificado.

## Figma
Arquivo criado: https://www.figma.com/design/YGYE7mg7AJVBKisaFC4Z7I
A edição de camadas está bloqueada temporariamente pelo limite de chamadas MCP do plano Starter. Este documento e a implementação Compose registram o design até a sincronização visual no Figma.
