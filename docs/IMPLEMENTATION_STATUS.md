# Registro de implementacao

Data de referencia: 2026-11-10

## Diretriz de desenvolvimento

O APOLO DATA CENTER terá control plane, serviços, protocolos de agente, mecanismos de storage e runtime de computação próprios. Bibliotecas, compiladores, protocolos e recursos de infraestrutura podem ser usados como fundação técnica, sem confundir isso com serviços APOLO já implementados. O software pode criar capacidade lógica, volumes virtuais, alocações e pools de recursos; capacidade física de CPU, RAM, discos e rede depende de dispositivos reais ou de infraestrutura provisionada.

## Fase 1 — Fundacao e arquitetura
Status: estrutura versionada.

Entregue: workspace Rust, crates modulares, arquitetura control/data plane, ADR, Compose local e GitHub Actions.

## Fase 2 — AX Core
Status: implementacao inicial validada em CI.

Implementado:
- ByteCount u128 para capacidades de escala exabyte.
- Separacao entre quota logica, total fisico, uso e reservas.
- Capacidade fisica bloqueada sem inventario verificado.
- ComputeResource com tipo, memoria, estado e validacao.
- NodeInventory com CPU, RAM, storage, rede, timestamp e aceleradores.
- Elegibilidade de nos e pre-validacao de compatibilidade de recursos.
- Perfis AX-1, AX-8, AX-128, AX-10000 e perfis customizados.
- Solicitacao de recursos explicita; nome comercial nao representa hardware.

## Fase 3 — AX Storage
Status: primeira implementacao local funcional; distribuicao e persistencia transacional pendentes.

Implementado:
- Contrato ObjectStore e adapter LocalFsObjectStore.
- Fragmentacao de objetos em blocos.
- Manifesto JSON com metadados, tamanhos e SHA-256 por bloco e objeto.
- Reconstrucao e verificacao de integridade na leitura.
- Quota local em memoria por instancia de StorageEngine.
- Escrita atomica de arquivos individuais via rename.
- Exclusao de objetos e liberacao do contador local.

Pendencias:
- Adapter S3/MinIO real, metadados PostgreSQL e contabilidade atomica multi-instancia.
- Replicacao, erasure coding, reparo automatico, KMS, versionamento e retencao.
- Upload multipart/resumivel e streaming sem carregar todo objeto na RAM.

## Fase 4 — AX Data Ingestion
Status: pipeline local inicial implementado e validado em CI.

Implementado:
- Deteccao de JSON, JSONL/NDJSON, CSV, TXT/Markdown, PDF, DOCX, imagens, audio e video pela extensao.
- Validacao basica de JSON/JSONL, CSV, UTF-8 e assinaturas conhecidas de PDF, DOCX, PNG, JPEG e WebP.
- Limite configuravel de bytes por entrada.
- Fila limitada com retorno explicito QueueFull para backpressure.
- Deduplicacao SHA-256 por projeto.
- Limite deslizante de entradas por hora por projeto.
- Ordem temporal por projeto para rejeitar timestamp retrocedente.
- Dequeue e metricas de profundidade da fila.

Pendencias:
- Fila duravel/distribuida, persistencia de deduplicacao e recuperacao apos restart.
- Validacao profunda de PDF/DOCX, antivirus/sandbox, OCR, transcodificacao e parsers de audio/video.
- Validacao estrita do MIME type e limites por bytes agregados/tenant.
- Persistencia de metadados e entrega direta ao AX Storage.

## Fase 5 — AX Scheduler
Status: primeira implementacao em Rust validada em CI.

Implementado:
- Registro e atualizacao de inventarios.
- Colocacao best-fit deterministica.
- Reserva atomica em processo de CPU, memoria, armazenamento, rede e aceleradores.
- Prevencao de dupla alocacao concorrente dentro de uma instancia.
- Verificacao agregada de banda de rede.
- Preservacao de aceleradores reservados durante atualizacao de inventario.
- Preservacao de aceleradores reservados durante atualizacao de inventario.
- Rejeicao de jobs duplicados ativos, liberacao e consulta de reservas.

Pendencias:
- Persistencia PostgreSQL, leases, heartbeat, fencing tokens e recuperacao apos restart.
- Coordenacao multi-instancia, prioridades, fairness, quotas por tenant e preempcao.
- Autenticacao e atestacao dos agentes de nos.

## Fase 6 — AX Compute
Status: executor local inicial implementado em Rust e validado em CI.

Implementado:
- ComputeJob e politica de execucao por allowlist de executaveis.
- Validacao da reserva do Scheduler e correspondencia do job.
- Execucao nativa sem shell, ambiente limpo e diretorio de trabalho dentro da raiz permitida.
- Timeout e captura limitada de stdout/stderr.
- Recibo de execucao com codigo de saida, duracao e timeout.
- Liberacao da reserva apos a tentativa de execucao.

Pendencias:
- Isolamento forte com cgroups v2, namespaces, seccomp e usuario sem privilegios.
- Limites reais de CPU, RAM, I/O e rede aplicados pelo sistema operacional.
- Worker daemon, execucao remota, container/VM, GPU e persistencia de jobs/logs.
- Cancelamento externo, retries, fila de execucao e telemetria.

## Validacao CI das fases 5 e 6
Execucao GitHub Actions: https://github.com/renatapetan61-ops/Apolo-Data-Center/actions/runs/42826510376
- Rust formatter: sucesso.
- Clippy com `-D warnings`: sucesso.
- Toolchains Rust/Cargo, Python 3.12, Go 1.24 e Java 21: instaladas e verificadas nos runners.
- 35 testes unitarios aprovados: AX Compute 4, AX Core 15, AX Ingestion 6, AX Scheduler 6 e AX Storage 4.

## Limites e garantias
- Storage atual e local, nao distribuido; quotas sao locais a uma instancia.
- Ingestion atual mantem payloads e dedupe em memoria; restart perde fila e historico.
- Scheduler atual mantem nos e reservas em memoria; restart perde estado e nao coordena processos distintos.
- AX Compute atual executa processos no host e nao e isolamento multi-tenant seguro.
- Audio/video passam apenas por verificacao basica de payload nao vazio; nao ha decodificacao de codec.
- Capacidade logica nao e prova de existencia de hardware, armazenamento fisico ou throughput.
- Nenhum hardware fisico, GPU ou cluster de producao foi provisionado por estes modulos.

## Validacao automatizada anterior
GitHub Actions executado com sucesso para AX Core, Storage e Ingestion:
- Rust formatter executado.
- Clippy aprovado com `-D warnings`.
- 25 testes unitarios aprovados: 15 AX Core, 4 AX Storage e 6 AX Ingestion.

Execucao: https://github.com/renatapetan61-ops/Apolo-Data-Center/actions/runs/42822969708

## Fase 7 — AX Node Agent (primeiro incremento)
Status: máquina de estados local implementada; transporte e atestação pendentes.

Implementado:
- Novo crate `apolo-ax-node-agent` integrado ao workspace.
- Registro de agente com verificação de credencial delegada ao contrato `CredentialVerifier`.
- Validação do inventário e vínculo da identidade do agente com o node_id.
- Heartbeat com sequência crescente e timestamp monotônico para rejeitar replay e mensagens atrasadas.
- Expiração de lease e transição do nó para Offline.
- O agente nunca pode declarar seu próprio hardware como verificado: o registro força `hardware_verified=false`; somente atestação independente futura poderá habilitar o nó para o Scheduler.
- Seis testes unitários adicionados.

Pendências críticas:
- Servidor de transporte HTTPS/mTLS e vínculo de identidade a certificados.
- Implementação de produção do verificador e armazenamento seguro/rotação de credenciais.
- Persistência de agentes, sequência e leases.
- Probe de hardware real e atestação independente.
- Daemon de sistema com usuário sem privilégios, cgroups v2, namespaces e seccomp.
- Canal remoto de comandos; nenhum job remoto é executado nesta fase.

Documentação: `docs/agents/ax-node-agent.md`.

## Proxima etapa
Fase 8 — transporte seguro e persistente do plano de controle: HTTPS/mTLS, identidade criptográfica dos agentes, persistência transacional e integração do registro com Scheduler. Em paralelo, evoluir AX Compute com isolamento real do sistema operacional antes de habilitar execução remota.


## Linha de engenharia paralela — AX Accelerator Architecture v0.1
Status: contrato de ISA e validação de programas implementados; hardware ainda não implementado.

Implementado:
- Novo crate `apolo-ax-accelerator`, integrado ao workspace.
- ISA preliminar própria AX v0.1 com NOP, LOAD, STORE, ADD, MULTIPLY, FMA, DOT PRODUCT, RELU, MATRIX MULTIPLY TILE e BARRIER.
- Programas declaram versão ISA, registradores e região de memória.
- Validação de operandos, registradores e limites de memória com detecção de overflow.
- Descritor de dispositivo com arquitetura, versão ISA, memória e confiança.
- Contrato `AcceleratorBackend` para futuros drivers de FPGA/ASIC.
- Seis testes unitários escritos.

Não implementado:
- RTL SystemVerilog sintetizável, FPGA bitstream, ASIC, placa, driver, compilador, runtime de execução ou benchmark físico.
- Nenhum desempenho de acelerador é declarado ou simulado como medição real.

Documentação: `docs/accelerators/ax-architecture.md`.

## Próxima etapa de infraestrutura
Fase 8 permanece: canal de controle HTTPS/mTLS, identidade criptográfica e persistência transacional de agentes/leases. A linha AX Accelerator avança em paralelo para especificação binária e referência funcional antes do RTL.


## AX Accelerator — Reference Runtime (incremento seguinte)

Status: interpretador funcional inicial em software; validação CI deste incremento pendente.

Implementado:
- Módulo `reference_runtime` executa sequencialmente programas AX validados em CPU comum.
- Registradores de 64 bits e LOAD/STORE little-endian de 1, 2, 4 ou 8 bytes.
- ADD, MULTIPLY, FMA, RELU e DOT PRODUCT escalar com semântica inteira definida.
- NOP e BARRIER como operações sem efeito no modelo sequencial.
- Limite explícito de instruções e validação do tamanho do buffer de memória.
- MATRIX MULTIPLY TILE retorna erro explícito de operação não implementada.

Limites:
- É uma implementação de referência em CPU, não execução em hardware AX.
- Não há métricas de desempenho, paralelismo físico, compilador, ABI congelada, formato binário, driver ou RTL.
- DOT PRODUCT ainda é escalar; semântica vetorial/acumulada e operações matriciais precisam de especificação.
- O interpretador não é sandbox para programas hostis.

## AX Accelerator — Bytecode container experimental v0

Status: encoder/decoder binário implementados; validação CI pendente.

Implementado:
- Cabeçalho `AXBC`, campos little-endian e versão ISA.
- Codificação/decodificação de instruções e operandos.
- Limites de 64 MiB e 1.000.000 instruções.
- Rejeição de magic inválido, opcode desconhecido, truncamento, operandos inválidos e bytes excedentes.\n- Validação ISA reforçada para exigir destino apenas nas operações que produzem resultado e proibi-lo em NOP, BARRIER e STORE.
- Cinco testes de round-trip e entradas malformadas.

Limites: formato experimental, sem ABI congelada, relocations, símbolos, debug info, assinatura ou garantia de compatibilidade futura.

Validação CI do AX Reference Runtime e Bytecode:
- AX Reference Runtime: https://github.com/renatapetan61-ops/Apolo-Data-Center/actions/runs/42828489118 (sucesso).
- AX Bytecode encoder/decoder: https://github.com/renatapetan61-ops/Apolo-Data-Center/actions/runs/42828603754 (sucesso).
- Regras de operandos ISA: https://github.com/renatapetan61-ops/Apolo-Data-Center/actions/runs/42828633347 (sucesso).
- Formatter, Clippy com `-D warnings` e testes do workspace aprovados nessas execuções.


## Aplicativo web APOLO AX — MVP inicial

Status: implementação inicial adicionada; CI Rust e build de container pendentes.

Implementado:
- API HTTP Rust em `apps/ax-api` com health check, overview e consulta de inventário.
- Dashboard web PT-BR servido pela API.
- Testes de health check e de inventário vazio, para evitar inventar capacidade física.
- Dockerfile multi-stage com usuário não-root, filesystem read-only, health check e capacidades Linux removidas.
- Docker Compose com API e serviços auxiliares; portas dos serviços auxiliares limitadas ao loopback do host.
- `.dockerignore` exclui arquivos `.env` e diretórios de build.
- CI de construção de imagem e workflow de publicação no GHCR acionado por tag `v*`.

Limites:
- API ainda não usa PostgreSQL, Redis ou MinIO; scheduler e inventário são voláteis.
- Sem autenticação, RBAC, multi-tenancy, billing, registro remoto de agentes, TLS de produção, execução de workloads pela API ou deployment em provedor público.
- O workflow GHCR publica uma imagem versionada, mas não configura servidor, DNS nem produção automaticamente.

Documentação: `docs/deployment/application-and-deployment.md`.


## Aplicativo Android nativo APOLO Data Center

Status: aplicativo Android nativo em Kotlin + Jetpack Compose/Material 3 implementado; compilação Android via CI pendente.

Implementado:
- Projeto Android Gradle em `android/`, Kotlin, namespace `com.apolo.datacenter`, minSdk 26, target/compileSdk 36.
- Interface nativa Jetpack Compose + Material 3, sem WebView, com navegação Início/Dados/Compute/Projetos/Ajustes.\n- Dashboard com status da API, métricas e inventário retornados pelo backend; falhas de conexão não são confundidas com inventário vazio.\n- Endpoint configurável e salvo localmente, conexão em executor separado, timeouts e HTTPS obrigatório para hosts remotos.\n- Telas de módulos distinguem capacidades já existentes no núcleo Rust de operações que ainda dependem de APIs autenticadas.
- HTTPS obrigatório para hosts remotos; HTTP permitido apenas para loopback e endereço do emulador via Network Security Config.
- Workflow `.github/workflows/android-ci.yml` para construir Debug e Release e guardar APKs como artifacts.

Ainda não implementado:
- JNI/UniFFI para integrar crates Rust dentro do processo Android.
- Autenticação, armazenamento seguro de tokens, enrollment de agentes, push de telemetria e notificações.\n- Integração JNI/UniFFI dos crates Rust, API persistente de storage/jobs/projetos e fluxos funcionais de upload e treinamento.\n- Design Figma: arquivo criado; edição de camadas bloqueada temporariamente pelo limite de chamadas MCP Starter. Especificação versionada em `docs/android/design-system.md`.
- Assinatura de Release por keystore e publicação na Play Store.
- Backend distribuído persistente e endpoint HTTPS real. O domínio de exemplo na tela deve ser substituído após a implantação do servidor.

O painel web é complementar. O aplicativo Android nativo é o cliente principal.


## API persistente de projetos — incremento 2026-11-10

Implementado no backend Rust:
- PostgreSQL opcional via `DATABASE_URL`, pool SQLx e migração automática `apps/ax-api/migrations/0001_projects.sql`.
- Endpoints `GET/POST /api/v1/projects` e `DELETE /api/v1/projects/{project_id}`.
- Validação de nome/descrição e UUIDs; projeto persistido com timestamp no PostgreSQL.
- Token Bearer obrigatório para todos os endpoints de projetos, comparado em tempo constante; token ausente/inválido bloqueia acesso.
- Compose conecta API ao PostgreSQL com healthcheck e injeta token obrigatório via `APOLO_API_TOKEN`; segredo documentado em `infrastructure/.env.example`.
- Testes API cobrem health, ausência de inventário fictício e bloqueio do endpoint de projetos sem configuração/autenticação.

Limitações ainda abertas:
- Scheduler e registro de nós ainda são voláteis e não estão ligados a PostgreSQL.
- Token atual é segredo compartilhado de API, não autenticação de usuário, sessões, RBAC ou multi-tenancy.
- Endpoints de storage, ingestion e compute ainda não foram expostos ao app.
- Redis e MinIO continuam sem integração ao runtime; TLS de produção e agentes remotos ainda faltam.
- APK Debug/Release foram compilados com sucesso pelo workflow Android anterior (run 42834002555); mudanças futuras no app devem disparar nova validação.

## Aplicativo Android — integração de projetos e credenciais

- Tela Kotlin/Compose de Projetos conectada aos endpoints autenticados de listar, criar e excluir projetos.
- Cliente HTTP Android com timeouts, HTTPS obrigatório para hosts remotos e tratamento de respostas 401/404/503.
- Token Bearer salvo com AES-GCM; a chave AES é gerada e mantida no Android Keystore, sem gravar o token em texto puro nas preferências.
- Tela Ajustes permite configurar endpoint e token; operações de projetos exigem token de pelo menos 32 caracteres.
- Build Android para esta integração em validação no workflow CI; não considerar APK atualizado aprovado até o job terminar.
