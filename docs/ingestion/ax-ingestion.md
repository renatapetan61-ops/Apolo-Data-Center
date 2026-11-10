# AX Data Ingestion — estado tecnico

## Pipeline atual
1. Validar projeto, nome, timestamp e tamanho maximo.
2. Identificar formato pela extensao.
3. Aplicar validacao basica de estrutura/assinatura.
4. Calcular SHA-256.
5. Verificar capacidade da fila limitada.
6. Aplicar limite deslizante de entradas por hora por projeto.
7. Rejeitar duplicidade por projeto e hash.
8. Enfileirar o payload e retornar recibo com formato, bytes, hash e profundidade.

## Formatos
- JSON e JSONL/NDJSON: parse JSON.
- CSV: UTF-8, virgula e quebra de linha.
- TXT/Markdown: UTF-8.
- PDF e DOCX: assinatura inicial do formato.
- PNG/JPEG/WebP: assinatura inicial.
- Audio e video: payload nao vazio; codecs nao sao decodificados.

## Backpressure e limites
A fila tem capacidade maxima configuravel. Quando cheia, o produtor recebe QueueFull e deve aguardar/repetir com estrategia de backoff. O limite horario e configuravel por projeto e usa timestamp recebido pelo servico; a camada HTTP futura deve atribuir esse timestamp no servidor, nunca confiar no relogio enviado pelo cliente.

## Limites conhecidos
Estado da fila, deduplicacao e janela de taxa sao mantidos em memoria. Nao ha persistencia, distribuicao entre replicas ou recuperacao apos restart. A fila atual e compartilhada por instancia. Content-Type e preservado, mas ainda nao e validado contra a extensao. Validacoes de assinatura nao substituem parsers completos nem antivírus.

## Proxima evolucao
Adicionar adaptador duravel (PostgreSQL para metadados e broker para filas), idempotencia persistente, limites por bytes e tenant, verificacao MIME, scanning isolado, e encaminhamento de objetos ao AX Storage por streaming.
