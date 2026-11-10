# AX Compute — primeira implementação

## Arquitetura entregue

O crate `apolo-ax-compute` contém um executor local de processos nativos:

- `ComputeJob` descreve job, reserva, executável, argumentos, ambiente, diretório e timeout.
- `ExecutionPolicy` define allowlist de executáveis por caminho canônico, raiz de sandbox, timeout máximo e limite de saída.
- `ProcessExecutor` consulta o AX Scheduler e só inicia o processo se a reserva existir e pertencer ao mesmo job.
- O executável é invocado diretamente com argumentos separados; não há interpolação de shell.
- O ambiente é limpo antes da execução e bloqueia PATH e variáveis LD_*.
- Diretório de trabalho deve estar dentro da raiz configurada.
- Timeout encerra o processo; stdout/stderr são drenados em threads e limitados em bytes.
- `ExecutionReceipt` registra nó, duração, código de saída, timeout e saída limitada.

## Limites de segurança e execução

Este é um primeiro adaptador de processo local, não um sandbox forte. A verificação de diretório e allowlist reduz a superfície, mas não substitui isolamento por namespaces/cgroups/seccomp, usuário sem privilégios, filesystem dedicado, limites de CPU/RAM reais, rede isolada ou container/VM. O processo executa com as permissões do usuário do serviço.

O executor ainda não:
- lança container, VM, GPU kernel ou workload remoto;
- reserva CPU/RAM fisicamente via kernel/cgroups;
- faz cancelamento externo, retries, fila de execução ou persistência de logs;
- implementa isolamento multi-tenant seguro;
- distribui tarefas entre múltiplos nós.

A reserva do scheduler é uma autorização lógica; não equivale a enforcement físico de recursos do sistema operacional. Não exponha este executor a usuários não confiáveis em produção.

## Próximos incrementos

1. Worker daemon AX com autenticação mútua, heartbeat e canal de controle.
2. Backend de isolamento Linux com cgroups v2, namespaces, seccomp e filesystem por job.
3. Adaptadores container/OCI e VM com políticas explícitas.
4. Execução remota via agentes, com leases/fencing e cancelamento.
5. Persistência de estados e logs, métricas, auditoria e quotas por tenant.
6. Adaptadores de GPU e runtimes de ML com detecção verificada de dispositivos.
