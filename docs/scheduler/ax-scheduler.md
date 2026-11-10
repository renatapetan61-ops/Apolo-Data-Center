# AX Scheduler — primeira implementação

## Escopo entregue

O crate `apolo-ax-scheduler` fornece um scheduler Rust thread-safe para uma única instância de processo:

- registro e atualização de inventários de nós;
- elegibilidade somente para nós saudáveis e com hardware verificado;
- colocação determinística best-fit;
- reservas de CPU, memória, armazenamento, rede e IDs de aceleradores;
- prevenção de dupla alocação concorrente dentro do processo;
- verificação de capacidade agregada de rede;
- preservação de aceleradores já reservados durante atualização do inventário;
- rejeição de job duplicado enquanto existir reserva ativa;
- liberação explícita de reservas e consulta/listagem;
- rejeição de atualização de inventário que conflite com reservas ativas.

## Limites explícitos

O estado é mantido em memória e protegido por `Mutex`. Reiniciar o processo perde nós e reservas. O mutex não coordena processos ou máquinas diferentes. Ainda não existe lease/heartbeat, expiração automática, fila de prioridades, persistência PostgreSQL, consenso, recuperação após falha, integração Kubernetes ou autenticação de agentes. A reserva é uma alocação lógica de recursos anunciados pelo agente; não inicia contêiner, VM ou workload.

A validação de hardware depende de agente confiável e processo de atestação ainda não implementados. Não trate inventários de teste como capacidade física comercializável.

## Testes

Testes unitários cobrem reserva/liberação, dupla alocação, job duplicado, nó não verificado, conflito de atualização e concorrência de reservas CPU.
