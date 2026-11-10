# Registro de implementacao

Data de referencia: 2026-11-10

## Diretriz de desenvolvimento

O APOLO DATA CENTER é um produto de software desenvolvido no repositório, com serviços e control planes próprios. Dependências externas serão usadas como bibliotecas, protocolos, runtimes ou infraestrutura quando necessário; isso não significa que os serviços do APOLO já existam. Ferramentas de compilação e teste são provisionadas nos ambientes de CI. A disponibilidade de hardware físico e servidores de produção é uma etapa separada.

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
Status: primeira implementação em Rust criada; aguardando CI.

Implementado no primeiro corte:
- Registro e atualização de inventários.
- Colocação best-fit determinística.
- Reserva atômica em processo de CPU, memória, armazenamento, rede e aceleradores.
- Prevenção de dupla alocação concorrente dentro de uma instância.
- Rejeição de jobs duplicados ativos.
- Liberação e consulta de reservas.
- Bloqueio de atualização de inventário incompatível com reservas existentes.

Pendências:
- Persistência PostgreSQL, leases, heartbeat, fencing tokens e recuperação após restart.
- Coordenação multi-instância, prioridades, fairness, quotas por tenant e preempção.
- Execução real de workloads e integração com containers/Kubernetes.
- Autenticação e atestação dos agentes de nós.

## Limites e garantias
- Storage atual e local, não distribuído; quotas são locais a uma instância.
- Ingestion atual mantém payloads e dedupe em memória; restart perde fila e histórico.
- Scheduler atual mantém nós e reservas em memória; restart perde estado e não coordena processos distintos.
- Queue capacity é compartilhada por instância; rate limit é por projeto.
- Áudio/vídeo passam apenas por verificação básica de payload não vazio; não há decodificação de codec.
- Nenhum hardware físico, GPU, throughput ou capacidade de cluster é provisionado por estes módulos.

## Validação automatizada anterior
GitHub Actions executado com sucesso para AX Core, Storage e Ingestion:
- Rust formatter executado.
- Clippy aprovado com `-D warnings`.
- 25 testes unitários aprovados: 15 AX Core, 4 AX Storage e 6 AX Ingestion.

Execução: https://github.com/renatapetan61-ops/Apolo-Data-Center/actions/runs/42822969708

## Próxima etapa
Fase 6 — AX Compute: protocolo de execução de workloads, ciclo de vida, isolamento, limites e adaptador de runtime. O Scheduler atual apenas reserva recursos; ainda não executa jobs.
