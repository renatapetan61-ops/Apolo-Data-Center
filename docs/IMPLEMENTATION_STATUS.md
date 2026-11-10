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
