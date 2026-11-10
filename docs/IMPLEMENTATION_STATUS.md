# Registro de implementação

Data de referência: 2026-11-10

## Diretriz de desenvolvimento

O APOLO DATA CENTER terá serviços e control planes próprios, implementados no repositório. Bibliotecas, protocolos e runtimes externos poderão ser usados como componentes de baixo nível, sem substituir os serviços APOLO. A camada AX poderá virtualizar e agregar recursos de múltiplos dispositivos; capacidade física só será reconhecida após descoberta, verificação e provisionamento de hardware real.

O GitHub Actions instala e verifica Rust/Cargo, Python, Go e Java nos runners. Isso não instala ferramentas automaticamente no Android/Termux do usuário.

## Fase 1 — Fundação
Status: estrutura versionada.
Entregue: workspace Rust, crates modulares, arquitetura control/data plane, ADR, Compose local e GitHub Actions.

## Fase 2 — AX Core
Status: implementação inicial validada em CI.
- ByteCount u128 para representar capacidades lógicas muito grandes.
- Separação entre quota lógica, total físico, uso e reservas.
- Capacidade física bloqueada sem inventário verificado.
- ComputeResource e NodeInventory com CPU, RAM, storage, rede e aceleradores.
- Elegibilidade de nós e pré-validação de compatibilidade.
- Perfis AX-1, AX-8, AX-128, AX-10000 e customizados; perfis não representam hardware instalado.

## Fase 3 — AX Storage
Status: primeira implementação local funcional; distribuição e persistência transacional pendentes.
- ObjectStore e adapter LocalFsObjectStore.
- Fragmentação em blocos, manifesto JSON, SHA-256 por bloco/objeto, reconstrução e verificação.
- Quota local em memória, escrita de arquivos por rename e exclusão.
Pendências: backend S3/MinIO, PostgreSQL, quota multi-instância, replicação, erasure coding, auto-repair, KMS, versionamento, retenção e streaming multipart.

## Fase 4 — AX Data Ingestion
Status: pipeline local inicial implementado e validado.
- Detecção de JSON, JSONL, CSV, TXT/Markdown, PDF, DOCX, imagens, áudio e vídeo por extensão.
- Validação básica de JSON/JSONL, CSV, UTF-8 e assinaturas de arquivos conhecidos.
- Limite de bytes, fila limitada/backpressure, dedupe SHA-256 por projeto, rate limit deslizante por projeto e ordenação temporal.
Pendências: fila durável/distribuída, dedupe persistente, análise profunda de arquivos, antivírus/sandbox, OCR, codecs, validação MIME e integração direta ao Storage.

## Fase 5 — AX Scheduler
Status: primeira versão implementada e validada em CI.
- Registro/atualização de inventários e colocação best-fit determinística.
- Reserva atômica em processo de CPU, memória, storage, rede e aceleradores.
- Prevenção de dupla alocação concorrente dentro de uma instância.
- Rejeição de job duplicado ativo, liberação e consulta de reservas.
- Atualizações de inventário são recusadas quando conflitam com recursos reservados.
Limites: estado em memória, sem persistência após restart, sem coordenação multi-instância, leases, heartbeat, fencing, quotas por tenant ou integração Kubernetes.

## Fase 6 — AX Compute
Status: executor local inicial implementado e validado em CI.
- ComputeJob com programa, argumentos, ambiente, diretório, timeout e ID de reserva.
- ExecutionPolicy com allowlist de executáveis canônicos, raiz de sandbox, timeout máximo e limite de stdout/stderr.
- ProcessExecutor exige reserva ativa do Scheduler pertencente ao mesmo job e libera a reserva após a tentativa de execução.
- Execução sem shell, ambiente limpo, bloqueio de PATH/LD_* e diretório confinado à raiz configurada.
- Timeout com encerramento de processo, drenagem de stdout/stderr e saída limitada.
- ExecutionReceipt com código de saída, duração, timeout e saída.
Limites: adaptador de processo local; não é container ou VM, não oferece isolamento forte, não aplica limites físicos via cgroups, não executa em GPU ou nós remotos e não possui worker daemon autenticado. Não expor a workloads não confiáveis em produção.

## Toolchain e CI
GitHub Actions prepara e verifica:
- Rust estável, Cargo, rustfmt e Clippy.
- Python 3.12.
- Go 1.24.
- Java 21.

Execução de validação que incluiu AX Core, Storage, Ingestion, Scheduler e Compute:
https://github.com/renatapetan61-ops/Apolo-Data-Center/actions/runs/42826456655

Resultado: formatter, Clippy com -D warnings e 35 testes unitários aprovados (15 Core, 4 Storage, 6 Ingestion, 6 Scheduler e 4 Compute). O CI mais recente também valida os ajustes de contabilização de rede/aceleradores e liberação da reserva pelo executor.

## Próximas etapas
1. AX Compute worker daemon, isolamento Linux real (cgroups v2, namespaces, seccomp) e agentes de nó autenticados.
2. Persistência transacional e leases para Scheduler.
3. AX Storage distribuído com metadados e cotas persistentes.
4. AX Cluster e comunicação autenticada entre nós.
