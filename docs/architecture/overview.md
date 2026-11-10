# Visao arquitetural

## Control plane e data plane
O control plane autentica solicitacoes, valida cotas, mantem estado desejado e agenda workloads. O data plane transfere objetos, processa dados e treina modelos em nos de trabalho. O Android e cliente e nao pode declarar hardware disponivel.

## Modulos APOLO AX
- AX Core: tipos canonicos e invariantes.
- AX Scheduler: selecao de nos e reservas atomicas.
- AX Node Agent: descoberta de CPU, RAM, discos, GPUs e runtimes.
- AX Compute: execucao em workers isolados.
- AX Storage: objetos, blocos, integridade e replicacao.
- AX Ingestion: validacao, filas, deduplicacao e indexacao.
- AX Token Engine: contadores persistentes e cotas.
- AX Training: PyTorch/JAX e checkpoints.
- AX Telemetry: metricas observadas com origem e timestamp.

## Perfis comerciais
AX-1, AX-8, AX-128 e AX-10000 sao perfis de servico, nao tipos de hardware. Cada perfil declarara vCPU, RAM, VRAM, armazenamento, rede e aceleradores requeridos. A contratacao so confirma apos reserva fisica. Quota logica, capacidade reservada e capacidade fisica observada sao metricas distintas.

## Persistencia e seguranca
PostgreSQL e fonte de verdade para usuarios, projetos, cotas, reservas, jobs e auditoria. Object storage guarda datasets e checkpoints. Redis nao sera a unica persistencia de estado critico. Codigo de clientes roda apenas em workers isolados com limites de CPU, memoria, disco, rede e duracao.

## Evolucao
Comecar com fronteiras modulares e implantacao pequena. Kubernetes entra quando multiplos nos e requisitos operacionais justificarem sua complexidade. Docker Compose e somente ambiente local.
