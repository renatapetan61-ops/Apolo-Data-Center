# Registro de implementacao

Data de referencia: 2026-11-10

## Fase 1 — Fundacao e arquitetura
Status: estrutura versionada; validacao automatizada inicial executada.

Entregue: workspace Rust, crates ax-core e ax-scheduler, arquitetura control/data plane, ADR, Compose de desenvolvimento, GitHub Actions.

## Fase 2 — AX Core
Status: implementacao inicial do dominio ampliada; validacao CI pendente.

Implementado:
- ByteCount com u128 para quotas logicas de grande escala.
- CapacitySnapshot distingue quota logica, total fisico, uso e reservas.
- Capacidade fisica indisponivel sem inventario verificado.
- ComputeResource com tipos de acelerador, estado, memoria e validacao.
- NodeInventory com snapshot temporal, CPU, RAM, storage, rede e aceleradores.
- Verificacao de elegibilidade do no e compatibilidade com solicitacao de recursos.
- ProfileCode AX-1, AX-8, AX-128, AX-10000 e perfis customizados.
- ResourceRequest exige especificacao concreta; nomes comerciais nao afirmam hardware.
- Testes unitarios de invariantes e compatibilidade de recursos.

Limites:
- NodeInventory e um contrato de dominio; ainda nao existe agente real de descoberta do host.
- can_satisfy e apenas pre-validacao; nao realiza reserva atomica. Isso pertence ao Scheduler.
- Inventario de GPU exige agente autenticado e evidencias do runtime/driver, ainda nao implementado.

## Proxima fase
Fase 3 — AX Storage: metadados de objetos, manifestos de blocos, hash de integridade, versionamento, cotas e adapter S3. Em seguida, Fase 4 — Ingestion.

## Validacao
O workflow Rust CI executa formatacao, Clippy e testes. Os resultados desta revisao ainda aguardam a nova execucao.
