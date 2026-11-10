# AX Storage — estado tecnico

## Contratos atuais
`ObjectStore` define put/get/delete_prefix. `LocalFsObjectStore` implementa armazenamento local para desenvolvimento e testes. Chaves sao geradas internamente com UUID e indices de blocos; nomes de arquivos fornecidos pelo cliente ficam apenas nos metadados.

## Fluxo de escrita
1. Validar projeto, nome e timestamp.
2. Adquirir lock de contabilidade local e verificar a quota configurada.
3. Gerar ObjectId e dividir o payload em blocos.
4. Gravar cada bloco em arquivo temporario e renomear para a chave final.
5. Calcular SHA-256 de cada bloco e do objeto completo.
6. Persistir manifesto JSON por ultimo.
7. Atualizar o contador local.

## Fluxo de leitura
1. Ler e desserializar o manifesto.
2. Buscar os blocos pela ordem do manifesto.
3. Conferir tamanho e SHA-256 de cada bloco.
4. Conferir tamanho e SHA-256 do objeto reconstruido.

## Limites conhecidos
- Implementacao sincrona e local.
- O payload completo e reconstruido em memoria na leitura.
- Contador de quota e local a uma instancia; nao sobrevive a restart.
- Nao ha replicacao, multipart, versionamento ou criptografia em repouso.
- O adapter nao deve ser exposto diretamente a clientes; deve ser chamado pelo servico autenticado.

## Proxima evolucao
Implementar adapter S3-compatible com MinIO, metadados PostgreSQL, reservas atomicas de quota e streaming de blocos. A distribuicao multi-node deve ser adicionada apenas com politica de consistencia, durabilidade e recuperacao testada.
