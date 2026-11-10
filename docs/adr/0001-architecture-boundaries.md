# ADR 0001: Fronteiras arquiteturais iniciais

- Status: aceita
- Data: 2026-11-10

## Decisao
1. Rust implementa dominio AX, scheduler e node agents.
2. PostgreSQL e a fonte de verdade transacional.
3. Object storage guarda payloads grandes; banco guarda metadados.
4. Python executa treinamento em workers, nao no processo central.
5. Kotlin e o cliente Android.
6. Inventario fisico exige probe autenticado e timestamp.
7. Reserva exige verificacao de saude e transacao para impedir dupla alocacao.

## Consequencias
As fronteiras de confianca ficam explicitas. A deteccao de GPU depende de drivers e runtimes instalados no host e sera feita em tempo de execucao.
