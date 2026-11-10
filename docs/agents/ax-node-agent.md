# AX Node Agent — primeira etapa

## Objetivo

O AX Node Agent é o limite de comunicação entre um nó de computação APOLO e o plano de controle. Ele registra a identidade do nó, recebe inventário reportado, atualiza heartbeats e permite detectar agentes sem comunicação.

## Implementado nesta etapa

- Tipos serializáveis de registro e heartbeat.
- Contrato `CredentialVerifier` para autenticação delegada a um verificador real.
- Registro único por identificador.
- Validação do inventário e vínculo entre identidade do agente e `node_id`.
- Sequência crescente de heartbeat para rejeitar repetição de mensagens.
- Timestamp monotônico para rejeitar heartbeat atrasado.
- Expiração de lease que marca agentes sem heartbeat como offline.
- Inventário enviado pelo próprio agente nunca é considerado prova de hardware: `hardware_verified` é forçado para `false`.

## Segurança e limites explícitos

Esta entrega é uma máquina de estados em processo, não um daemon de rede. Ainda não implementa TLS/mTLS, armazenamento seguro de credenciais, rotação/revogação de certificados, persistência, assinatura de mensagens, atestação de hardware, detecção de relógio adulterado ou isolamento do processo agente. O `CredentialVerifier` é uma interface: o projeto ainda precisa fornecer uma implementação de produção. Não exponha esta API diretamente à internet.

O inventário não pode se tornar elegível para o Scheduler apenas por declarar-se saudável. A atestação independente e a verificação do hardware são etapas posteriores.

## Próximas entregas

1. Adaptador de transporte HTTPS com mTLS e identidade de certificado vinculada ao agente.
2. Persistência transacional de identidade, sequência e leases.
3. Probe Linux de CPU, memória, discos, rede e aceleradores, com origem e confiança de cada medição.
4. Atestação independente e política de admissão do nó.
5. Daemon com usuário sem privilégios e integração com cgroups v2, namespaces e seccomp.
6. Canal de comando remoto autenticado; execução de jobs só após isolamento forte.
