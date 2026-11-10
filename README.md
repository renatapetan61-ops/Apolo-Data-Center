# APOLO DATA CENTER

Plataforma de computacao distribuida, armazenamento elastico e treinamento de IA. Nucleo: APOLO AX.

Estado: fundacao arquitetural em implementacao. Capacidade fisica somente sera anunciada apos descoberta por agentes de nos reais.

Principios: separar control plane/data plane; distinguir quota logica, capacidade fisica observada e capacidade reservada; nunca executar codigo de cliente no processo principal; confirmar reservas somente apos validacao atomica do inventario.

Stack planejada: Rust (AX), Python (workers PyTorch/JAX), Kotlin (Android), PostgreSQL, Redis e object storage S3-compatible.

Consulte docs/architecture/overview.md e docs/adr/0001-architecture-boundaries.md.

## API e painel web complementar (MVP de control plane)

O primeiro aplicativo utilizável está em `apps/ax-api`:
- API HTTP Rust com `/healthz`, `/api/v1/overview` e `/api/v1/nodes`.
- Painel web em português servido na rota `/`.
- O painel mostra apenas inventários realmente registrados no scheduler em memória; não cria nós fictícios nem anuncia capacidade física inexistente.
- A API oferece cadastro, listagem e exclusão de projetos com persistência PostgreSQL e token Bearer obrigatório.\n- O scheduler e inventário de nós continuam single-process e voláteis; agentes remotos, multi-tenancy, TLS de produção e execução de jobs via HTTP ainda não estão implementados.

## Execução local com Docker Compose

1. Copie `infrastructure/.env.example` para `infrastructure/.env` e substitua todas as senhas e o `APOLO_API_TOKEN` por segredos aleatórios únicos. O token precisa ter pelo menos 32 caracteres.
2. Na raiz do repositório, execute `docker compose --env-file infrastructure/.env -f infrastructure/compose.yaml up --build -d`.
3. Abra `http://127.0.0.1:8080`; verifique `http://127.0.0.1:8080/healthz`.
4. Os serviços auxiliares estão publicados somente em loopback. A API usa PostgreSQL para persistir projetos; Redis e MinIO ainda não estão integrados ao runtime.

Não publique esta versão diretamente na internet. Antes disso são necessários autenticação, autorização, TLS, persistência, rate limiting, auditoria, hardening e revisão de segurança.


## Aplicativo Android nativo (Kotlin + Jetpack Compose)

O aplicativo principal para o usuário é Android nativo e está em `android/`:
- Kotlin + Android Gradle Plugin, sem depender do painel web para funcionar como aplicativo.
- Tela nativa com configuração do endpoint, conexão à API, health check, overview e inventário de nós.
- Rejeita HTTP em hosts remotos; HTTPS é obrigatório fora de loopback/emulador local.
- Build Android separado para APK Debug e Release no GitHub Actions.

O painel web em `apps/ax-api` é apenas uma interface complementar de administração/API, não o aplicativo Android.

### Compilar APK

O workflow `.github/workflows/android-ci.yml` executa `assembleDebug` e `assembleRelease` e publica os APKs como artifact do GitHub Actions. Para instalar o Debug, baixe o artifact da execução concluída e instale o arquivo APK no Android. O Release produzido nesta etapa não está assinado para distribuição; a assinatura exige configurar um keystore e secrets de assinatura no GitHub.

O endpoint inicial `https://api.apolo-datacenter.example` é um placeholder. Troque pelo endereço HTTPS real do servidor quando a API estiver implantada. A API atual ainda usa scheduler em memória e não registra agentes remotos.

A integração JNI entre Kotlin e bibliotecas Rust, armazenamento local seguro de credenciais, autenticação de usuário e push de inventário de agentes ainda não foram implementados.


## Workspaces de projetos, Kimi e workers AX

O backend adiciona APIs autenticadas para:
- Arquivos, datasets e checkpoints por projeto, com SHA-256 e metadados no PostgreSQL.
- Modelos e scaffold PyTorch gerado ao criar um registro de LLM: model_config.json, model.py, train.py e README-LLM.md.
- Assistente Kimi via NVIDIA NIM. Configure NVIDIA_NIM_API_KEY como variável secreta no serviço Railway. O identificador padrão é moonshotai/kimi-k3.1-pro.
- Jobs, métricas, checkpoints e protocolo de registro, heartbeat, claim e conclusão de workers AX.

### Armazenamento persistente

Monte um volume persistente no serviço API e configure APOLO_STORAGE_DIR para o caminho de montagem. Sem essa variável, uploads e geração de arquivos são recusados. O volume atualmente provisionado para produção é /data/apolo (500 MB); monitore e amplie antes de datasets grandes.

### Worker real

O agente está em workers/ax-python. Instale as dependências e execute-o em uma máquina Linux real com PyTorch:
1. python -m pip install -r workers/ax-python/requirements.txt
2. Defina APOLO_AX_API e APOLO_AX_TOKEN (token API dedicado).
3. Execute python workers/ax-python/worker.py.

O agente reporta hardware, envia heartbeat, reivindica jobs, executa o train.py do projeto e envia métricas/checkpoints. Ele espera dataset JSONL tokenizado com input_ids. O worker não é implantado automaticamente pelo backend; ele precisa rodar em hardware que você controle.

### Limites da execução atual

O scaffold gerado é um modelo Transformer pequeno de partida, não um LLM pré-treinado nem uma cópia do Kimi. O endpoint de treino exige dataset e worker online com pelo menos 2 CPUs e 4 GiB RAM e bloqueia registros acima de 25 milhões de parâmetros nesse executor inicial. Treinos maiores, distribuídos multi-GPU, MoE em escala e modelos de centenas de bilhões/trilhões exigem workers especializados, armazenamento e comunicação distribuída adicionais; não são simulados.

## APOLO VTA — Virtual Tensor Accelerator

APOLO VTA é um orquestrador virtual de recursos de IA, não uma TPU emulada nem um acelerador de hardware. Ele agrega telemetria dos workers AX, estima memória para BF16 + AdamW e rejeita escalas fisicamente impossíveis.

- Inventário de workers com CPU/RAM/VRAM livre: `GET /api/v1/workers`.
- Avaliação de capacidade: `GET /api/v1/vta/plan?parameters=70000000000`.
- CLI de planejamento: `python workers/virtual-accelerator/apolo_vta.py --parameters 70000000000`.
- Worker envia telemetria de RAM livre e VRAM CUDA livre nos heartbeats.
- Testes: workflow `APOLO VTA CI`.

**Limite atual:** o planejador é planning-only. O executor de treinamento AX continua single-rank; treinamento FSDP2/tensor/pipeline paralelo multi-node ainda precisa ser implementado e validado com rede e aceleradores reais. O VTA não cria capacidade física nem promete ser mais rápido que uma TPU. Consulte `workers/virtual-accelerator/README.md`.
